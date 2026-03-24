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

use crate::Colormap;
use crate::spectrograph::ColormapHandle;
use std::sync::OnceLock;

pub(crate) fn colormap_lut_12bit(colormap: Colormap) -> &'static [[u8; 3]; 65536] {
    fn build(colormap: Colormap) -> Box<[[u8; 3]; 65536]> {
        let (r_slice, g_slice, b_slice) = colormap.colorset();
        let handle = ColormapHandle {
            r_slice,
            g_slice,
            b_slice,
            cap: r_slice.len() as f32 - 1.,
        };
        let mut lut = Box::new([[0u8; 3]; 65536]);
        const S: f32 = 1. / 4095.;
        for (i, dst) in lut[..4096].iter_mut().enumerate() {
            *dst = handle.interpolate(i as f32 * S);
        }
        lut
    }

    macro_rules! cached {
        ($variant:expr) => {{
            static CELL: OnceLock<Box<[[u8; 3]; 65536]>> = OnceLock::new();
            CELL.get_or_init(|| build($variant))
        }};
    }

    match colormap {
        Colormap::Inferno => cached!(Colormap::Inferno),
        Colormap::Magma => cached!(Colormap::Magma),
        Colormap::Plasma => cached!(Colormap::Plasma),
        Colormap::Viridis => cached!(Colormap::Viridis),
        Colormap::Turbo => cached!(Colormap::Turbo),
        Colormap::Jet => cached!(Colormap::Jet),
        Colormap::Cividis => cached!(Colormap::Cividis),
        Colormap::Ocean => cached!(Colormap::Ocean),
        Colormap::Pink => cached!(Colormap::Pink),
        Colormap::Spring => cached!(Colormap::Spring),
        Colormap::Summer => cached!(Colormap::Summer),
        Colormap::Twilight => cached!(Colormap::Twilight),
        Colormap::TwilightShifted => cached!(Colormap::TwilightShifted),
        Colormap::Winter => cached!(Colormap::TwilightShifted),
    }
}
