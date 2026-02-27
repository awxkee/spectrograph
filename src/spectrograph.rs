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
use crate::err::{SpectrographError, try_vec};
use crate::mla::fmla;
use crate::normalizer::{normalize_power, normalize_real};
use crate::{SpectroSample, SpectrographOptions, SpectrographFrame};
use num_complex::Complex;
use num_traits::AsPrimitive;

#[inline(always)]
fn normalized_to_intensity(x: f32) -> f32 {
    x * 255.
}

struct ColormapHandle<'a> {
    r_slice: &'a [f32],
    g_slice: &'a [f32],
    b_slice: &'a [f32],
    cap: f32,
}

impl ColormapHandle<'_> {
    #[inline(always)]
    fn interpolate(&self, x: f32) -> [u8; 3] {
        let a = (x * self.cap).floor();
        let b = self.cap.min(a.ceil());
        let f = fmla(x, self.cap, -a);
        let new_r0 = unsafe { *self.r_slice.get_unchecked(a as usize) };
        let new_g0 = unsafe { *self.g_slice.get_unchecked(a as usize) };
        let new_b0 = unsafe { *self.b_slice.get_unchecked(a as usize) };

        let new_r1 = unsafe { self.r_slice.get_unchecked(b as usize) };
        let new_g1 = unsafe { self.g_slice.get_unchecked(b as usize) };
        let new_b1 = unsafe { self.b_slice.get_unchecked(b as usize) };
        [
            normalized_to_intensity(fmla(new_r1 - new_r0, f, new_r0)).round() as u8,
            normalized_to_intensity(fmla(new_g1 - new_g0, f, new_g0)).round() as u8,
            normalized_to_intensity(fmla(new_b1 - new_b0, f, new_b0)).round() as u8,
        ]
    }
}

#[inline]
fn bilinear_sample(
    data: &[f32],
    stride: usize,
    width: usize,
    height: usize,
    y: f32,
    x: f32,
) -> f32 {
    let x0 = x.floor() as isize;
    let y0 = y.floor() as isize;
    let x1 = (x.ceil() as isize).min(width as isize - 1);
    let y1 = (y.ceil() as isize).min(height as isize - 1);

    let fx = x - x0 as f32;
    let fy = y - y0 as f32;

    let y0_row = unsafe { data.get_unchecked(y0 as usize * stride..) };
    let y1_row = unsafe { data.get_unchecked(y1 as usize * stride..) };

    let v00 = unsafe { *y0_row.get_unchecked(x0 as usize) };
    let v10 = unsafe { *y0_row.get_unchecked(x1 as usize) };
    let v01 = unsafe { *y1_row.get_unchecked(x0 as usize) };
    let v11 = unsafe { *y1_row.get_unchecked(x1 as usize) };

    let v0 = fmla(fx, v10 - v00, v00);
    let v1 = fmla(fx, v11 - v01, v01);

    fmla(fy, v1 - v0, v0)
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

    let src_h = frame.height;
    let src_w = frame.width;

    let norm = normalize_power(frame.data.as_ref(), options.normalizer)?;
    let mut img = try_vec![0u8; out_width * out_height * N];

    let sx = (src_w - 1) as f32 / (out_width - 1) as f32;
    let sy = (src_h - 1) as f32 / (out_height - 1) as f32;

    let handle = ColormapHandle {
        r_slice,
        g_slice,
        b_slice,
        cap: r_slice.len() as f32 - 1.,
    };

    for (oy, row) in img.chunks_exact_mut(out_width * N).enumerate() {
        // flip vertically (high freq on top)
        let src_y = (out_height - 1 - oy) as f32 * sy;

        for (ox, px) in row.as_chunks_mut::<N>().0.iter_mut().enumerate() {
            let src_x = ox as f32 * sx;

            let v = bilinear_sample(&norm, frame.width, frame.width, frame.height, src_y, src_x);
            let new_rgb = handle.interpolate(v);
            px[0] = new_rgb[0];
            px[1] = new_rgb[1];
            px[2] = new_rgb[2];
            if N == 4 {
                px[3] = 255;
            }
        }
    }

    Ok(img)
}

fn draw_scalogram_real_color_impl<T: SpectroSample, const N: usize>(
    frame: &SpectrographFrame<T>,
    spectrograph_options: SpectrographOptions,
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

    let out_width = spectrograph_options.out_width;
    let out_height = spectrograph_options.out_height;

    let _ = (out_height as isize)
        .checked_mul(out_width as isize)
        .ok_or(SpectrographError::PointerOverflow)?;

    let (r_slice, g_slice, b_slice) = spectrograph_options.colormap.colorset();

    assert_eq!(r_slice.len(), g_slice.len());
    assert_eq!(r_slice.len(), b_slice.len());

    let src_h = frame.height;
    let src_w = frame.width;

    let norm = normalize_real(frame.data.as_ref(), spectrograph_options.normalizer)?;
    let mut img = try_vec![0u8; out_width * out_height * N];

    let sx = (src_w - 1) as f32 / (out_width - 1) as f32;
    let sy = (src_h - 1) as f32 / (out_height - 1) as f32;

    let handle = ColormapHandle {
        r_slice,
        g_slice,
        b_slice,
        cap: r_slice.len() as f32 - 1.,
    };

    for (oy, row) in img.chunks_exact_mut(out_width * N).enumerate() {
        // flip vertically (high freq on top)
        let src_y = (out_height - 1 - oy) as f32 * sy;

        for (ox, px) in row.as_chunks_mut::<N>().0.iter_mut().enumerate() {
            let src_x = ox as f32 * sx;

            let v = bilinear_sample(&norm, frame.width, frame.width, frame.height, src_y, src_x);
            let new_rgb = handle.interpolate(v);
            px[0] = new_rgb[0];
            px[1] = new_rgb[1];
            px[2] = new_rgb[2];
            if N == 4 {
                px[3] = 255;
            }
        }
    }

    Ok(img)
}

pub fn rgb_spectrograph_color_f32(
    frame: &SpectrographFrame<Complex<f32>>,
    options: SpectrographOptions,
) -> Result<Vec<u8>, SpectrographError> {
    draw_scalogram_color_impl::<f32, 3>(frame, options)
}

pub fn rgb_spectrograph_color_f64(
    frame: &SpectrographFrame<Complex<f64>>,
    options: SpectrographOptions,
) -> Result<Vec<u8>, SpectrographError> {
    draw_scalogram_color_impl::<f64, 3>(frame, options)
}

pub fn rgba_spectrograph_color_f32(
    frame: &SpectrographFrame<Complex<f32>>,
    options: SpectrographOptions,
) -> Result<Vec<u8>, SpectrographError> {
    draw_scalogram_color_impl::<f32, 4>(frame, options)
}

pub fn rgba_spectrograph_color_f64(
    frame: &SpectrographFrame<Complex<f64>>,
    options: SpectrographOptions,
) -> Result<Vec<u8>, SpectrographError> {
    draw_scalogram_color_impl::<f64, 4>(frame, options)
}

pub fn rgb_real_spectrograph_color_f32(
    frame: &SpectrographFrame<f32>,
    options: SpectrographOptions,
) -> Result<Vec<u8>, SpectrographError> {
    draw_scalogram_real_color_impl::<f32, 3>(frame, options)
}

pub fn rgb_real_spectrograph_color_f64(
    frame: &SpectrographFrame<f64>,
    options: SpectrographOptions,
) -> Result<Vec<u8>, SpectrographError> {
    draw_scalogram_real_color_impl::<f64, 3>(frame, options)
}

pub fn rgba_real_spectrograph_color_f32(
    frame: &SpectrographFrame<f32>,
    options: SpectrographOptions,
) -> Result<Vec<u8>, SpectrographError> {
    draw_scalogram_real_color_impl::<f32, 4>(frame, options)
}

pub fn rgba_real_spectrograph_color_f64(
    frame: &SpectrographFrame<f64>,
    options: SpectrographOptions,
) -> Result<Vec<u8>, SpectrographError> {
    draw_scalogram_real_color_impl::<f64, 4>(frame, options)
}
