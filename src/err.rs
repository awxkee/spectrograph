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

/// Errors that can occur during the detrending process.
#[derive(Debug)]
pub enum SpectrographError {
    /// Indicates a failure to allocate the memory required for the resulting vector.
    /// The associated value is the requested size (`usize`) of the allocation.
    Allocation(usize),
    Generic(String),
    FftError(String),
    InvalidFrameSize(usize, usize),
    ZeroBaseSized,
    PointerOverflow,
}

impl Display for SpectrographError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SpectrographError::Allocation(size) => {
                f.write_fmt(format_args!("Failed to allocate buffer with size {size}"))
            }
            SpectrographError::Generic(msg) => f.write_str(msg),
            SpectrographError::FftError(msg) => f.write_str(msg),
            SpectrographError::InvalidFrameSize(expected, got) => f.write_fmt(format_args!(
                "Frame size expected to be {expected} but it was {got}"
            )),
            SpectrographError::ZeroBaseSized => f.write_str("Zero sized frames are not supported"),
            SpectrographError::PointerOverflow => {
                f.write_str("Pointer overflow while calculations were performed")
            }
        }
    }
}

impl Error for SpectrographError {}

macro_rules! try_vec {
    () => {
        Vec::new()
    };
    ($elem:expr; $n:expr) => {{
        let mut v = Vec::new();
        v.try_reserve_exact($n)
            .map_err(|_| crate::err::SpectrographError::Allocation($n))?;
        v.resize($n, $elem);
        v
    }};
}

use std::error::Error;
use std::fmt::{Display, Formatter};
pub(crate) use try_vec;
