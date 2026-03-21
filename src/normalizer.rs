/*
 * // Copyright (c) Radzivon Bartoshyk 2/2026. All rights reserved.
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
use crate::{Normalizer, SpectroSample};
use num_complex::Complex;
use num_traits::{AsPrimitive, One, Zero};
use pxfm::{f_log1pf, f_log10f};

pub(crate) fn normalize_power<T: SpectroSample>(
    coeffs: &[Complex<T>],
    normalizer: Normalizer,
    width: usize,
) -> Result<Vec<f32>, SpectrographError>
where
    f64: AsPrimitive<T>,
{
    let mut output = try_vec![f32::zero(); coeffs.len()];

    match normalizer {
        Normalizer::Power => {
            let mut max = T::zero();
            for (dst, v) in output
                .rchunks_exact_mut(width)
                .zip(coeffs.chunks_exact(width))
            {
                for (dst, v) in dst.iter_mut().zip(v.iter()) {
                    let p = fmla(v.re, v.re, v.im * v.im);
                    max = max.max(p);
                    *dst = p.as_();
                }
            }
            let inv: f32 = if max > T::zero() {
                1f32 / max.as_()
            } else {
                0.0
            };
            for v in output.iter_mut() {
                *v = (*v * inv).clamp(f32::zero(), f32::one());
            }
        }

        Normalizer::Magnitude => {
            let mut max = T::zero();
            for (dst, v) in output
                .rchunks_exact_mut(width)
                .zip(coeffs.chunks_exact(width))
            {
                for (dst, v) in dst.iter_mut().zip(v.iter()) {
                    let m = fmla(v.re, v.re, v.im * v.im).sqrt();
                    max = max.max(m);
                    *dst = m.as_();
                }
            }
            let inv = if max > T::zero() {
                1.0 / max.as_()
            } else {
                0.0
            };
            for v in output.iter_mut() {
                *v = (*v * inv).clamp(f32::zero(), f32::one());
            }
        }

        Normalizer::PowerSqrt => {
            // sqrt(power) = magnitude, but normalized against sqrt(max_power)
            // rather than max_magnitude — subtly different scaling
            let mut max_power = T::zero();
            for (dst, v) in output
                .rchunks_exact_mut(width)
                .zip(coeffs.chunks_exact(width))
            {
                for (dst, v) in dst.iter_mut().zip(v.iter()) {
                    let p = fmla(v.re, v.re, v.im * v.im);
                    max_power = max_power.max(p);
                    *dst = p.as_();
                }
            }
            let inv = if max_power > T::zero() {
                1f32 / max_power.sqrt().as_()
            } else {
                0f32
            };
            for v in output.iter_mut() {
                *v = ((*v).sqrt() * inv).clamp(f32::zero(), f32::one());
            }
        }

        Normalizer::DecibelsDb { floor_db } => {
            let floor = floor_db;
            let recip_floor = -1f32 / floor;
            for (dst, v) in output
                .rchunks_exact_mut(width)
                .zip(coeffs.chunks_exact(width))
            {
                for (dst, v) in dst.iter_mut().zip(v.iter()) {
                    let power: f32 = fmla(v.re, v.re, v.im * v.im).as_();
                    let db = if power > 1e-10 {
                        10.0 * f_log10f(power)
                    } else {
                        floor
                    };
                    // map [floor_db, 0] → [0, 1]
                    *dst = ((db - floor) * recip_floor).min(1.0).max(0.0);
                }
            }
            // already normalized by construction, no second pass needed
        }

        Normalizer::LogMagnitude => {
            let mut max = 0.0f32;
            for (dst, v) in output
                .rchunks_exact_mut(width)
                .zip(coeffs.chunks_exact(width))
            {
                for (dst, v) in dst.iter_mut().zip(v.iter()) {
                    let m: f32 = fmla(v.re, v.re, v.im * v.im).as_();
                    let lm = f_log1pf(m); // log(1 + magnitude²)
                    max = max.max(lm);
                    *dst = lm;
                }
            }
            let inv = if max > 0.0 { 1.0 / max } else { 0.0 };
            for v in output.iter_mut() {
                *v = (*v * inv).clamp(f32::zero(), f32::one());
            }
        }

        Normalizer::MeanNormalized => {
            let mut sum = 0.0f32;
            for (dst, v) in output
                .rchunks_exact_mut(width)
                .zip(coeffs.chunks_exact(width))
            {
                for (dst, v) in dst.iter_mut().zip(v.iter()) {
                    let p: f32 = fmla(v.re, v.re, v.im * v.im).as_();
                    sum += p;
                    *dst = p;
                }
            }
            let mean = if coeffs.is_empty() {
                1.0
            } else {
                sum / coeffs.len() as f32
            };
            let inv = if mean > 1e-10 { 1.0 / mean } else { 0.0 };
            // clamp to [0,1] since individual bins can exceed the mean
            for v in output.iter_mut() {
                *v = (*v * inv).clamp(0.0, 1.0);
            }
        }

        Normalizer::LocalMax { radius } => {
            // First pass: fill output with power
            for (dst, v) in output
                .rchunks_exact_mut(width)
                .zip(coeffs.chunks_exact(width))
            {
                for (dst, v) in dst.iter_mut().zip(v.iter()) {
                    *dst = fmla(v.re, v.re, v.im * v.im).as_();
                }
            }
            // Second pass: divide each bin by its local max
            let source = output.clone();
            for (i, dst) in output.iter_mut().enumerate() {
                let lo = i.saturating_sub(radius);
                let hi = (i + radius + 1).min(source.len());
                let local_max = source[lo..hi].iter().cloned().fold(0.0f32, f32::max);
                *dst = if local_max > 1e-10 {
                    (*dst / local_max).clamp(0.0, 1.0)
                } else {
                    0.0
                };
            }
        }
    }
    Ok(output)
}

pub(crate) fn normalize_real<T: SpectroSample>(
    frame: &[T],
    normalizer: Normalizer,
    width: usize,
) -> Result<Vec<f32>, SpectrographError>
where
    f64: AsPrimitive<T>,
{
    let mut output = try_vec![f32::zero(); frame.len()];

    match normalizer {
        Normalizer::Power => {
            let mut max = T::zero();
            for (dst, v) in output
                .rchunks_exact_mut(width)
                .zip(frame.chunks_exact(width))
            {
                for (dst, &v) in dst.iter_mut().zip(v.iter()) {
                    let p = v * v;
                    max = max.max(p);
                    *dst = p.as_();
                }
            }
            let inv = if max > T::zero() {
                1.0 / max.as_()
            } else {
                0.0
            };
            for v in output.iter_mut() {
                *v = (*v * inv).clamp(f32::zero(), f32::one());
            }
        }

        Normalizer::Magnitude => {
            let mut max = T::zero();
            for (dst, v) in output
                .rchunks_exact_mut(width)
                .zip(frame.chunks_exact(width))
            {
                for (dst, v) in dst.iter_mut().zip(v.iter()) {
                    let m = v.abs();
                    max = max.max(m);
                    *dst = m.as_();
                }
            }
            let inv = if max > T::zero() {
                1.0 / max.as_()
            } else {
                0.0
            };
            for v in output.iter_mut() {
                *v = (*v * inv).clamp(f32::zero(), f32::one());
            }
        }

        Normalizer::PowerSqrt => {
            let mut max_power = T::zero();
            for (dst, v) in output
                .rchunks_exact_mut(width)
                .zip(frame.chunks_exact(width))
            {
                for (dst, &v) in dst.iter_mut().zip(v.iter()) {
                    let p = v * v;
                    max_power = max_power.max(p);
                    *dst = p.as_();
                }
            }
            let inv = if max_power > T::zero() {
                1.0 / max_power.as_().sqrt()
            } else {
                0.0
            };
            for v in output.iter_mut() {
                *v = ((*v).sqrt() * inv).clamp(f32::zero(), f32::one());
            }
        }

        Normalizer::DecibelsDb { floor_db } => {
            let floor = floor_db;
            // find peak power for relative normalization (librosa-style)
            let max_power = frame.iter().map(|&v| (v * v).as_()).fold(0.0f32, f32::max);
            let max_power_recip = if max_power != 0. {
                max_power.recip()
            } else {
                0.
            };

            for (dst, v) in output
                .rchunks_exact_mut(width)
                .zip(frame.chunks_exact(width))
            {
                for (dst, &v) in dst.iter_mut().zip(v.iter()) {
                    let power: f32 = (v * v).as_();
                    let db = if max_power > 1e-10 && power > 1e-10 {
                        10.0 * f_log10f(power * max_power_recip)
                    } else {
                        floor
                    };
                    *dst = ((db - floor) / (-floor)).clamp(0.0, 1.0);
                }
            }
        }

        Normalizer::LogMagnitude => {
            let mut max = 0.0f32;
            for (dst, v) in output
                .rchunks_exact_mut(width)
                .zip(frame.chunks_exact(width))
            {
                for (dst, &v) in dst.iter_mut().zip(v.iter()) {
                    let p: f32 = (v * v).as_();
                    let lm = f_log1pf(p);
                    max = max.max(lm);
                    *dst = lm;
                }
            }
            let inv = if max > 0.0 { 1.0 / max } else { 0.0 };
            for v in output.iter_mut() {
                *v = (*v * inv).clamp(f32::zero(), f32::one());
            }
        }

        Normalizer::MeanNormalized => {
            let mut sum = 0.0f32;
            for (dst, v) in output
                .rchunks_exact_mut(width)
                .zip(frame.chunks_exact(width))
            {
                for (dst, &v) in dst.iter_mut().zip(v.iter()) {
                    let p: f32 = (v * v).as_();
                    sum += p;
                    *dst = p;
                }
            }
            let mean = if frame.is_empty() {
                1.0
            } else {
                sum / frame.len() as f32
            };
            let inv = if mean > 1e-10 { 1.0 / mean } else { 0.0 };
            for v in output.iter_mut() {
                *v = (*v * inv).clamp(0.0, 1.0);
            }
        }

        Normalizer::LocalMax { radius } => {
            for (dst, v) in output
                .rchunks_exact_mut(width)
                .zip(frame.chunks_exact(width))
            {
                for (dst, &v) in dst.iter_mut().zip(v.iter()) {
                    *dst = (v * v).as_();
                }
            }
            let source = output.clone();
            for (i, dst) in output.iter_mut().enumerate() {
                let lo = i.saturating_sub(radius);
                let hi = (i + radius + 1).min(source.len());
                let local_max = source[lo..hi].iter().cloned().fold(0.0f32, f32::max);
                *dst = if local_max > 1e-10 {
                    (*dst / local_max).clamp(0.0, 1.0)
                } else {
                    0.0
                };
            }
        }
    }

    Ok(output)
}
