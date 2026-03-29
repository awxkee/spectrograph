/*
 * // Copyright (c) Radzivon Bartoshyk 12/2025. All rights reserved.
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
use crate::cache::colormap_lut_12bit;
use crate::err::{SpectrographError, try_vec};
use crate::mla::fmla;
use crate::normalizer::{
    normalize_power, normalize_power_f32, normalize_real, normalize_real_12bit,
};
use crate::{Interpolator, SpectroSample, SpectrographFrame, SpectrographOptions};
use num_complex::Complex;
use num_traits::AsPrimitive;
use pic_scale::{ImageSize, ImageStore, ImageStoreMut, Resampling};
use std::sync::Arc;

#[inline(always)]
fn normalized_to_intensity(x: f32) -> f32 {
    x * 255.
}

pub(crate) struct ColormapHandle<'a> {
    pub(crate) r_slice: &'a [f32],
    pub(crate) g_slice: &'a [f32],
    pub(crate) b_slice: &'a [f32],
    pub(crate) cap: f32,
}

impl ColormapHandle<'_> {
    #[inline(always)]
    pub(crate) fn interpolate(&self, x: f32) -> [u8; 3] {
        let a = (x * self.cap).floor();
        let b = self.cap.min(a.ceil());
        let f = fmla(x, self.cap, -a);
        let new_r0 = self.r_slice[a as usize];
        let new_g0 = self.g_slice[a as usize];
        let new_b0 = self.b_slice[a as usize];

        let new_r1 = self.r_slice[b as usize];
        let new_g1 = self.g_slice[b as usize];
        let new_b1 = self.b_slice[b as usize];
        [
            normalized_to_intensity(fmla(new_r1 - new_r0, f, new_r0)).round() as u8,
            normalized_to_intensity(fmla(new_g1 - new_g0, f, new_g0)).round() as u8,
            normalized_to_intensity(fmla(new_b1 - new_b0, f, new_b0)).round() as u8,
        ]
    }
}

fn validate_plan<T: Copy, const N: usize>(
    plan: Arc<Resampling<T, N>>,
    src: ImageSize,
    dst: ImageSize,
) -> Result<(), SpectrographError> {
    if plan.source_size() != src {
        return Err(SpectrographError::Generic(
            format!(
                "Invalid source size in passed context, expected {:?}, but got {:?}",
                src,
                plan.source_size(),
            )
            .to_string(),
        ));
    }
    if plan.target_size() != dst {
        return Err(SpectrographError::Generic(
            format!(
                "Invalid target size in passed context, expected {:?}, but got {:?}",
                dst,
                plan.target_size()
            )
            .to_string(),
        ));
    }
    Ok(())
}

#[inline(always)]
fn apply_lut_row<const N: usize>(src_row: &[u16], dst_row: &mut [u8], lut: &[[u8; 3]]) {
    for (&src_px, px) in src_row
        .iter()
        .zip(dst_row.as_chunks_mut::<N>().0.iter_mut())
    {
        let new_rgb = lut[src_px as usize];
        px[0] = new_rgb[0];
        px[1] = new_rgb[1];
        px[2] = new_rgb[2];
        if N == 4 {
            px[3] = 255;
        }
    }
}

#[inline(always)]
fn apply_lut_row_f32<const N: usize>(src_row: &[f32], dst_row: &mut [u8], lut: &[[u8; 3]; 65536]) {
    for (&src_px, px) in src_row
        .iter()
        .zip(dst_row.as_chunks_mut::<N>().0.iter_mut())
    {
        let new_rgb = lut[((src_px.min(1.) * 8192.0) as u16) as usize];
        px[0] = new_rgb[0];
        px[1] = new_rgb[1];
        px[2] = new_rgb[2];
        if N == 4 {
            px[3] = 255;
        }
    }
}

fn build_f32_lut(handle: &ColormapHandle<'_>) -> Box<[[u8; 3]; 65536]> {
    let mut lut = Box::new([[0u8; 3]; 65536]);
    for (i, dst) in lut[..8193].iter_mut().enumerate() {
        *dst = handle.interpolate(i as f32 / 8192.0);
    }
    lut
}

fn draw_scalogram_color_impl<T: SpectroSample, const N: usize>(
    frame: &SpectrographFrame<Complex<T>>,
    options: SpectrographOptions,
) -> Result<Vec<u8>, SpectrographError>
where
    f64: AsPrimitive<T>,
{
    if frame.width == 0 || frame.height == 0 {
        return Err(SpectrographError::ZeroBaseSized);
    }

    if frame.data.len()
        != (frame.width as isize)
            .checked_mul(frame.height as isize)
            .ok_or(SpectrographError::PointerOverflow)? as usize
    {
        return Err(SpectrographError::InvalidFrameSize(
            frame.width * frame.height,
            frame.data.len(),
        ));
    }

    let out_width = options.out_width;
    let out_height = options.out_height;

    let _ = (out_height as isize)
        .checked_mul(out_width as isize)
        .ok_or(SpectrographError::PointerOverflow)?;

    let (r_slice, g_slice, b_slice) = options.colormap.colorset();

    assert_eq!(r_slice.len(), g_slice.len());
    assert_eq!(r_slice.len(), b_slice.len());

    let handle = ColormapHandle {
        r_slice,
        g_slice,
        b_slice,
        cap: r_slice.len() as f32 - 1.,
    };

    if options.interpolator != Interpolator::Fast {
        let norm = normalize_power_f32(frame.data.as_ref(), options.normalizer, frame.width)?;
        let mut img = try_vec![0u8; out_width * out_height * N];
        let resizing_plan = options
            .context
            .map(|x| Ok(x.scaler_accurate.clone()))
            .unwrap_or_else(|| {
                let resizer = pic_scale::Scaler::new(options.interpolator.to_pic_scale())
                    .set_multi_step_upsampling(true);
                resizer
                    .plan_planar_resampling_f32(
                        ImageSize::new(frame.width, frame.height),
                        ImageSize::new(out_width, out_height),
                    )
                    .map_err(|x| SpectrographError::Generic(x.to_string()))
            })?;

        validate_plan(
            resizing_plan.clone(),
            ImageSize::new(frame.width, frame.height),
            ImageSize::new(out_width, out_height),
        )?;

        let mut resized = ImageStoreMut::<f32, 1>::alloc(out_width, out_height);
        let s_frame = ImageStore::<f32, 1>::borrow(&norm, frame.width, frame.height)
            .map_err(|x| SpectrographError::Generic(x.to_string()))?;
        resizing_plan
            .resample(&s_frame, &mut resized)
            .map_err(|x| SpectrographError::Generic(x.to_string()))?;

        let lut = build_f32_lut(&handle);

        for (src_row, dst_row) in resized
            .buffer
            .borrow()
            .chunks_exact(out_width)
            .zip(img.chunks_exact_mut(out_width * N))
        {
            apply_lut_row_f32::<N>(src_row, dst_row, &lut);
        }

        return Ok(img);
    }

    let norm = normalize_power(frame.data.as_ref(), options.normalizer, frame.width)?;
    let mut img = try_vec![0u8; out_width * out_height * N];

    let resizing_plan = options
        .context
        .map(|x| Ok(x.scaler.clone()))
        .unwrap_or_else(|| {
            let resizer = pic_scale::Scaler::new(options.interpolator.to_pic_scale())
                .set_multi_step_upsampling(true);
            resizer
                .plan_planar_resampling16(
                    ImageSize::new(frame.width, frame.height),
                    ImageSize::new(out_width, out_height),
                    12,
                )
                .map_err(|x| SpectrographError::Generic(x.to_string()))
        })?;

    validate_plan(
        resizing_plan.clone(),
        ImageSize::new(frame.width, frame.height),
        ImageSize::new(out_width, out_height),
    )?;

    let mut resized = ImageStoreMut::<u16, 1>::alloc_with_depth(out_width, out_height, 12);
    let s_frame = ImageStore::<u16, 1>::borrow(&norm, frame.width, frame.height)
        .map_err(|x| SpectrographError::Generic(x.to_string()))?;
    resizing_plan
        .resample(&s_frame, &mut resized)
        .map_err(|x| SpectrographError::Generic(x.to_string()))?;

    let lut = colormap_lut_12bit(options.colormap);

    for (src_row, dst_row) in resized
        .buffer
        .borrow()
        .chunks_exact(out_width)
        .zip(img.chunks_exact_mut(out_width * N))
    {
        apply_lut_row::<N>(src_row, dst_row, lut);
    }

    Ok(img)
}

fn draw_scalogram_real_color_impl<T: SpectroSample, const N: usize>(
    frame: &SpectrographFrame<T>,
    options: SpectrographOptions,
) -> Result<Vec<u8>, SpectrographError>
where
    f64: AsPrimitive<T>,
{
    if frame.width == 0 || frame.height == 0 {
        return Err(SpectrographError::ZeroBaseSized);
    }
    if frame.data.len()
        != (frame.width as isize)
            .checked_mul(frame.height as isize)
            .ok_or(SpectrographError::PointerOverflow)? as usize
    {
        return Err(SpectrographError::InvalidFrameSize(
            frame.width * frame.height,
            frame.data.len(),
        ));
    }

    let out_width = options.out_width;
    let out_height = options.out_height;

    let _ = (out_height as isize)
        .checked_mul(out_width as isize)
        .ok_or(SpectrographError::PointerOverflow)?;

    let (r_slice, g_slice, b_slice) = options.colormap.colorset();

    assert_eq!(r_slice.len(), g_slice.len());
    assert_eq!(r_slice.len(), b_slice.len());

    let mut img = try_vec![0u8; out_width * out_height * N];

    let handle = ColormapHandle {
        r_slice,
        g_slice,
        b_slice,
        cap: r_slice.len() as f32 - 1.,
    };

    if options.interpolator == Interpolator::SuperHighQuality {
        let norm = normalize_real(frame.data.as_ref(), options.normalizer, frame.width)?;
        let resizing_plan = options
            .context
            .map(|x| Ok(x.scaler_accurate.clone()))
            .unwrap_or_else(|| {
                let resizer = pic_scale::Scaler::new(options.interpolator.to_pic_scale())
                    .set_multi_step_upsampling(true);
                resizer
                    .plan_planar_resampling_f32(
                        ImageSize::new(frame.width, frame.height),
                        ImageSize::new(out_width, out_height),
                    )
                    .map_err(|x| SpectrographError::Generic(x.to_string()))
            })?;

        validate_plan(
            resizing_plan.clone(),
            ImageSize::new(frame.width, frame.height),
            ImageSize::new(out_width, out_height),
        )?;

        let mut resized = ImageStoreMut::<f32, 1>::alloc_with_depth(out_width, out_height, 12);
        let s_frame = ImageStore::<f32, 1>::borrow(&norm, frame.width, frame.height)
            .map_err(|x| SpectrographError::Generic(x.to_string()))?;
        resizing_plan
            .resample(&s_frame, &mut resized)
            .map_err(|x| SpectrographError::Generic(x.to_string()))?;

        let lut = build_f32_lut(&handle);

        for (src_row, dst_row) in resized
            .buffer
            .borrow()
            .chunks_exact(out_width)
            .zip(img.chunks_exact_mut(out_width * N))
        {
            apply_lut_row_f32::<N>(src_row, dst_row, &lut);
        }

        return Ok(img);
    }

    let norm = normalize_real_12bit(frame.data.as_ref(), options.normalizer, frame.width)?;

    let resizing_plan = options
        .context
        .map(|x| Ok(x.scaler.clone()))
        .unwrap_or_else(|| {
            let resizer = pic_scale::Scaler::new(options.interpolator.to_pic_scale())
                .set_multi_step_upsampling(true);
            resizer
                .plan_planar_resampling16(
                    ImageSize::new(frame.width, frame.height),
                    ImageSize::new(out_width, out_height),
                    12,
                )
                .map_err(|x| SpectrographError::Generic(x.to_string()))
        })?;

    validate_plan(
        resizing_plan.clone(),
        ImageSize::new(frame.width, frame.height),
        ImageSize::new(out_width, out_height),
    )?;

    let mut resized = ImageStoreMut::<u16, 1>::alloc_with_depth(out_width, out_height, 12);
    let s_frame = ImageStore::<u16, 1>::borrow(&norm, frame.width, frame.height)
        .map_err(|x| SpectrographError::Generic(x.to_string()))?;
    resizing_plan
        .resample(&s_frame, &mut resized)
        .map_err(|x| SpectrographError::Generic(x.to_string()))?;

    let lut = colormap_lut_12bit(options.colormap);

    for (src_row, dst_row) in resized
        .buffer
        .borrow()
        .chunks_exact(out_width)
        .zip(img.chunks_exact_mut(out_width * N))
    {
        apply_lut_row::<N>(src_row, dst_row, lut);
    }

    Ok(img)
}

fn rgbx_spectrograph_f32<const S: usize>(
    frame: &SpectrographFrame<Complex<f32>>,
    options: SpectrographOptions,
) -> Result<Vec<u8>, SpectrographError> {
    draw_scalogram_color_impl::<f32, S>(frame, options)
}

fn rgbx_spectrograph_f64<const S: usize>(
    frame: &SpectrographFrame<Complex<f64>>,
    options: SpectrographOptions,
) -> Result<Vec<u8>, SpectrographError> {
    draw_scalogram_color_impl::<f64, S>(frame, options)
}

fn rgbx_real_spectrograph_f32<const S: usize>(
    frame: &SpectrographFrame<f32>,
    options: SpectrographOptions,
) -> Result<Vec<u8>, SpectrographError> {
    draw_scalogram_real_color_impl::<f32, S>(frame, options)
}

fn rgbx_real_spectrograph_f64<const S: usize>(
    frame: &SpectrographFrame<f64>,
    options: SpectrographOptions,
) -> Result<Vec<u8>, SpectrographError> {
    draw_scalogram_real_color_impl::<f64, S>(frame, options)
}

pub fn rgb_spectrograph_f32(
    frame: &SpectrographFrame<Complex<f32>>,
    options: SpectrographOptions,
) -> Result<Vec<u8>, SpectrographError> {
    rgbx_spectrograph_f32::<3>(frame, options)
}

pub fn rgb_spectrograph_f64(
    frame: &SpectrographFrame<Complex<f64>>,
    options: SpectrographOptions,
) -> Result<Vec<u8>, SpectrographError> {
    rgbx_spectrograph_f64::<3>(frame, options)
}

pub fn rgba_spectrograph_f32(
    frame: &SpectrographFrame<Complex<f32>>,
    options: SpectrographOptions,
) -> Result<Vec<u8>, SpectrographError> {
    rgbx_spectrograph_f32::<4>(frame, options)
}

pub fn rgba_spectrograph_f64(
    frame: &SpectrographFrame<Complex<f64>>,
    options: SpectrographOptions,
) -> Result<Vec<u8>, SpectrographError> {
    rgbx_spectrograph_f64::<4>(frame, options)
}

pub fn rgb_real_spectrograph_f32(
    frame: &SpectrographFrame<f32>,
    options: SpectrographOptions,
) -> Result<Vec<u8>, SpectrographError> {
    rgbx_real_spectrograph_f32::<3>(frame, options)
}

pub fn rgb_real_spectrograph_f64(
    frame: &SpectrographFrame<f64>,
    options: SpectrographOptions,
) -> Result<Vec<u8>, SpectrographError> {
    rgbx_real_spectrograph_f64::<3>(frame, options)
}

pub fn rgba_real_spectrograph_f32(
    frame: &SpectrographFrame<f32>,
    options: SpectrographOptions,
) -> Result<Vec<u8>, SpectrographError> {
    rgbx_real_spectrograph_f32::<4>(frame, options)
}

pub fn rgba_real_spectrograph_f64(
    frame: &SpectrographFrame<f64>,
    options: SpectrographOptions,
) -> Result<Vec<u8>, SpectrographError> {
    rgbx_real_spectrograph_f64::<4>(frame, options)
}
