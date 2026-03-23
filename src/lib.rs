/*
 * // Copyright (c) Radzivon Bartoshyk 02/2026. All rights reserved.
 * //
 * // Redistribution and use in source and binary forms, with or without modification,
 * // are permitted provided that the following conditions are met:
 * //
 * // 1.  Redistributions of source code must retain the above copyright notice, this
 * // list of conditions and the following disclaimer.
 * //
 * // 2.  Redistributions in binary form must reproduce the above copyright notice,
 * // this list of conditions and the following disclaimer in the documentation
 * // and/or other materials provided with the distribution.
 * //
 * // 3.  Neither the name of the copyright holder nor the names of its
 * // contributors may be used to endorse or promote products derived from
 * // this software without specific prior written permission.
 * //
 * // THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
 * // AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * // IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
 * // DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
 * // FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * // DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
 * // SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
 * // CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
 * // OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
 * // OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */
#![allow(clippy::excessive_precision, clippy::manual_clamp)]
use num_traits::real::Real;
use num_traits::{AsPrimitive, MulAdd, Num, Zero};
use pic_scale::{ImageSize, Resampling, ResamplingFunction};
use std::fmt::{Debug, Display};
use std::ops::{Add, AddAssign, Div, Mul, MulAssign, Neg, Sub};
use std::sync::Arc;

mod colormap;
mod err;
mod mla;
mod normalizer;
mod spectrograph;

pub use colormap::Colormap;

pub trait SpectroSample:
    MulAdd<Self, Output = Self>
    + AddAssign
    + MulAssign
    + 'static
    + Copy
    + Clone
    + Send
    + Sync
    + Num
    + Default
    + Neg<Output = Self>
    + Add<Self, Output = Self>
    + Mul<Self, Output = Self>
    + Div<Self, Output = Self>
    + Sub<Self, Output = Self>
    + Debug
    + Display
    + Zero
    + PartialOrd
    + AsPrimitive<f32>
    + Real
{
}

impl SpectroSample for f32 {}
impl SpectroSample for f64 {}

/// Configuration for spectrograph/scalogram rendering.
///
/// Controls output dimensions, color mapping, normalization, and interpolation
/// quality. Pass this to the renderer to produce a final image.
#[derive(Clone)]
pub struct SpectrographOptions {
    /// Width of the output image in pixels.
    pub out_width: usize,
    /// Height of the output image in pixels.
    pub out_height: usize,
    /// Colormap applied to normalized magnitude values.
    pub colormap: Colormap,
    /// Strategy used to normalize raw magnitude data before colormapping.
    pub normalizer: Normalizer,
    /// Interpolation algorithm used when resampling the spectrograph data
    /// to the output dimensions.
    pub interpolator: Interpolator,
    pub context: Option<Arc<SpectrographContext>>,
}

#[derive(Clone)]
pub struct SpectrographContext {
    scaler: Arc<Resampling<u16, 1>>,
    scaler_accurate: Arc<Resampling<f32, 1>>,
}

impl SpectrographContext {
    pub fn source_size(&self) -> ImageSize {
        self.scaler.source_size()
    }

    pub fn target_size(&self) -> ImageSize {
        self.scaler.target_size()
    }
}

#[derive(Debug, Default, PartialOrd, PartialEq, Copy, Clone)]
pub enum Normalizer {
    #[default]
    Power,
    Magnitude,
    /// 10*log10(power), mapped to [0,1] between floor_db and 0 dB
    DecibelsDb {
        /// Noise floor in dB, e.g. -80.0. Values below are clamped to 0.
        floor_db: f32,
    },
    /// Square root of power — perceptually between Power and Magnitude,
    /// compresses dynamic range less aggressively than Power
    PowerSqrt,
    /// log1p(magnitude) — log compression without needing a dB floor,
    /// good when signal has a lot of near-zero bins
    LogMagnitude,
    /// Each bin normalized by the mean power across the frame —
    /// emphasizes spectral shape rather than absolute energy.
    /// Good for comparing frames with very different loudness.
    MeanNormalized,
    /// Each bin divided by its local neighborhood max (per-bin across time).
    /// Makes quiet features visible that global normalization would crush.
    LocalMax {
        /// Half-window size in bins
        radius: usize,
    },
}

/// Interpolation algorithm used when resampling spectrograph/scalogram data.
///
/// CWT scalograms contain continuous, smooth energy distributions across time-frequency space.
/// Higher quality filters better preserve ridge continuity and avoid ringing artifacts
/// along frequency ridges, at the cost of increased computation.
#[derive(Debug, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Interpolator {
    /// Lanczos2 — fast resampling suitable for real-time or preview rendering.
    ///
    /// Good enough for most scalogram visualizations where exact ridge shape
    /// is not critical. May introduce slight blurring at sharp energy transitions.
    #[default]
    Fast,
    /// Mitchell-Netravalli — balanced quality/performance for general use.
    ///
    /// A good default for final renders. The Mitchell filter's balance between
    /// blurring and ringing makes it well suited for the smooth energy envelopes
    /// typical in CWT output, avoiding over-sharpening of ridge artifacts.
    HighQuality,
    /// Lanczos5 — maximum fidelity for high-resolution scalogram export.
    ///
    /// Preserves fine frequency ridge detail and sharp onset transients
    /// at the cost of longer computation. Recommended when the output will
    /// be analyzed further rather than just viewed.
    SuperHighQuality,
}

impl Interpolator {
    pub(crate) fn to_pic_scale(self) -> ResamplingFunction {
        match self {
            Interpolator::Fast => ResamplingFunction::Lanczos2,
            Interpolator::HighQuality => ResamplingFunction::MitchellNetravalli,
            Interpolator::SuperHighQuality => ResamplingFunction::Lanczos5Jinc,
        }
    }
}

pub fn create_context(
    interpolator: Interpolator,
    in_width: usize,
    in_height: usize,
    out_width: usize,
    out_height: usize,
) -> Result<SpectrographContext, SpectrographError> {
    let resizer = pic_scale::Scaler::new(interpolator.to_pic_scale());
    let plan = resizer
        .plan_planar_resampling16(
            ImageSize::new(in_width, in_height),
            ImageSize::new(out_width, out_height),
            12,
        )
        .map_err(|x| SpectrographError::Generic(x.to_string()))?;
    let plan2 = resizer
        .plan_planar_resampling_f32(
            ImageSize::new(in_width, in_height),
            ImageSize::new(out_width, out_height),
        )
        .map_err(|x| SpectrographError::Generic(x.to_string()))?;
    Ok(SpectrographContext {
        scaler: plan,
        scaler_accurate: plan2,
    })
}

pub struct SpectrographFrame<'a, T: ToOwned>
where
    [T]: ToOwned,
{
    pub data: std::borrow::Cow<'a, [T]>,
    pub width: usize,
    pub height: usize,
}

use crate::err::SpectrographError;
pub use spectrograph::{
    rgb_real_spectrograph_f32, rgb_real_spectrograph_f64, rgb_spectrograph_f32,
    rgb_spectrograph_f64, rgba_real_spectrograph_f32, rgba_real_spectrograph_f64,
    rgba_spectrograph_f32, rgba_spectrograph_f64,
};
