/*
 * // Copyright (c) Radzivon Bartoshyk 3/2026. All rights reserved.
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
#![allow(clippy::needless_range_loop)]
use crate::mla::{FusedMadd, fmla};

pub(crate) trait Sampler {
    fn sample(
        &self,
        data: &[f32],
        stride: usize,
        width: usize,
        height: usize,
        y: f32,
        x: f32,
    ) -> f32;
}

pub(crate) struct BilinearInterpolator {}

impl Sampler for BilinearInterpolator {
    fn sample(
        &self,
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
}

pub(crate) struct CatmullRomInterpolator {}

impl CatmullRomInterpolator {
    /// Cubic (Catmull-Rom) interpolation for 4 points
    #[inline(always)]
    fn hermite_1d(f: f32, p0: f32, p1: f32, p2: f32, p3: f32) -> f32 {
        let a = fmla(-0.5, p0, fmla(1.5, p1, fmla(-1.5, p2, 0.5 * p3)));
        let b = fmla(-2.5, p1, fmla(2.0, p2, fmla(-0.5, p3, p0)));
        let c = fmla(-0.5, p0, 0.5 * p2);
        let d = p1;

        // Horner's method for efficiency: ((a*f + b)*f + c)*f + d
        f.mla(f.mla(f.mla(a, b), c), d)
    }
}

impl Sampler for CatmullRomInterpolator {
    fn sample(
        &self,
        data: &[f32],
        stride: usize,
        width: usize,
        height: usize,
        y: f32,
        x: f32,
    ) -> f32 {
        let x_int = x.floor() as isize;
        let y_int = y.floor() as isize;
        let fx = x - x_int as f32;
        let fy = y - y_int as f32;

        // We need 4 points for each axis (i-1, i, i+1, i+2)
        let mut py = [0.0f32; 4];

        for i in 0..4 {
            let row_idx = (y_int - 1 + i as isize).clamp(0, height as isize - 1) as usize;
            let row = unsafe { data.get_unchecked(row_idx * stride..) };

            let mut px = [0.0f32; 4];
            for j in 0..4 {
                let col_idx = (x_int - 1 + j as isize).clamp(0, width as isize - 1) as usize;
                px[j] = unsafe { *row.get_unchecked(col_idx) };
            }

            // Interpolate the 4 points in the current row
            py[i] = Self::hermite_1d(fx, px[0], px[1], px[2], px[3]);
        }

        // Interpolate the results of the 4 rows vertically
        Self::hermite_1d(fy, py[0], py[1], py[2], py[3])
    }
}
