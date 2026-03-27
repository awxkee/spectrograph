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
use crate::colormap::afmhot::{AFMHOT_B, AFMHOT_G, AFMHOT_R};
use crate::colormap::bone::{BONE_B, BONE_G, BONE_R};
use crate::colormap::cividis::{CIVIDIS_B, CIVIDIS_G, CIVIDIS_R};
use crate::colormap::cool::{COOL_B, COOL_G, COOL_R};
use crate::colormap::cool_warm::{COOLWARM_B, COOLWARM_G, COOLWARM_R};
use crate::colormap::copper::{COPPER_B, COPPER_G, COPPER_R};
use crate::colormap::cubehelix::{CUBEHELIX_B, CUBEHELIX_G, CUBEHELIX_R};
use crate::colormap::gnuplot::{GNUPLOT2_B, GNUPLOT2_G, GNUPLOT2_R};
use crate::colormap::grays::{GRAYS_B, GRAYS_G, GRAYS_R};
use crate::colormap::hot::{HOT_B, HOT_G, HOT_R};
use crate::colormap::hsv::{HSV_B, HSV_G, HSV_R};
use crate::colormap::inferno::{INFERNO_B, INFERNO_G, INFERNO_R};
use crate::colormap::jet::{JET_B, JET_G, JET_R};
use crate::colormap::magma::{MAGMA_B, MAGMA_G, MAGMA_R};
use crate::colormap::ocean::{OCEAN_B, OCEAN_G, OCEAN_R};
use crate::colormap::pink::{PINK_B, PINK_G, PINK_R};
use crate::colormap::plasma::{PLASMA_B, PLASMA_G, PLASMA_R};
use crate::colormap::rdbu::{RDBU_B, RDBU_G, RDBU_R};
use crate::colormap::spectral::{SPECTRAL_B, SPECTRAL_G, SPECTRAL_R};
use crate::colormap::spring::{SPRING_B, SPRING_G, SPRING_R};
use crate::colormap::summer::{SUMMER_B, SUMMER_G, SUMMER_R};
use crate::colormap::terrain::{TERRAIN_B, TERRAIN_G, TERRAIN_R};
use crate::colormap::turbo::{TURBO_B, TURBO_G, TURBO_R};
use crate::colormap::twilight::{TWILIGHT_B, TWILIGHT_G, TWILIGHT_R};
use crate::colormap::twilight_shifted::{TWILIGHT_S_B, TWILIGHT_S_G, TWILIGHT_S_R};
use crate::colormap::viridis::{VIRIDIS_B, VIRIDIS_G, VIRIDIS_R};
use crate::colormap::winter::{WINTER_B, WINTER_G, WINTER_R};

mod afmhot;
mod bone;
mod cividis;
mod cool;
mod cool_warm;
mod copper;
mod cubehelix;
mod gnuplot;
mod grays;
mod hot;
mod hsv;
mod inferno;
mod jet;
mod magma;
mod ocean;
mod pink;
mod plasma;
mod rdbu;
mod spectral;
mod spring;
mod summer;
mod terrain;
mod turbo;
mod twilight;
mod twilight_shifted;
mod viridis;
mod winter;

#[derive(Copy, Clone, Hash, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum Colormap {
    Turbo,
    Jet,
    Cividis,
    Inferno,
    Magma,
    Ocean,
    Pink,
    Plasma,
    Spring,
    Summer,
    Twilight,
    TwilightShifted,
    Viridis,
    Winter,
    Hot,
    CoolWarm,
    Cool,
    Grays,
    Spectral,
    Copper,
    Bone,
    Hsv,
    Afmhot,
    Rdbu,
    Cubehelix,
    Gnuplot2,
    Terrain,
}

impl Colormap {
    pub(crate) fn colorset(self) -> (&'static [f32], &'static [f32], &'static [f32]) {
        match self {
            Colormap::Turbo => (TURBO_R.as_slice(), TURBO_G.as_slice(), TURBO_B.as_slice()),
            Colormap::Jet => (JET_R.as_slice(), JET_G.as_slice(), JET_B.as_slice()),
            Colormap::Cividis => (
                CIVIDIS_R.as_slice(),
                CIVIDIS_G.as_slice(),
                CIVIDIS_B.as_slice(),
            ),
            Colormap::Inferno => (
                INFERNO_R.as_slice(),
                INFERNO_G.as_slice(),
                INFERNO_B.as_slice(),
            ),
            Colormap::Magma => (MAGMA_R.as_slice(), MAGMA_G.as_slice(), MAGMA_B.as_slice()),
            Colormap::Ocean => (OCEAN_R.as_slice(), OCEAN_G.as_slice(), OCEAN_B.as_slice()),
            Colormap::Pink => (PINK_R.as_slice(), PINK_G.as_slice(), PINK_B.as_slice()),
            Colormap::Plasma => (
                PLASMA_R.as_slice(),
                PLASMA_G.as_slice(),
                PLASMA_B.as_slice(),
            ),
            Colormap::Spring => (
                SPRING_R.as_slice(),
                SPRING_G.as_slice(),
                SPRING_B.as_slice(),
            ),
            Colormap::Summer => (
                SUMMER_R.as_slice(),
                SUMMER_G.as_slice(),
                SUMMER_B.as_slice(),
            ),
            Colormap::Twilight => (
                TWILIGHT_R.as_slice(),
                TWILIGHT_G.as_slice(),
                TWILIGHT_B.as_slice(),
            ),
            Colormap::TwilightShifted => (
                TWILIGHT_S_R.as_slice(),
                TWILIGHT_S_G.as_slice(),
                TWILIGHT_S_B.as_slice(),
            ),
            Colormap::Viridis => (
                VIRIDIS_R.as_slice(),
                VIRIDIS_G.as_slice(),
                VIRIDIS_B.as_slice(),
            ),
            Colormap::Winter => (
                WINTER_R.as_slice(),
                WINTER_G.as_slice(),
                WINTER_B.as_slice(),
            ),
            Colormap::Hot => (HOT_R.as_slice(), HOT_G.as_slice(), HOT_B.as_slice()),
            Colormap::Cool => (COOL_R.as_slice(), COOL_G.as_slice(), COOL_B.as_slice()),
            Colormap::Grays => (GRAYS_R.as_slice(), GRAYS_G.as_slice(), GRAYS_B.as_slice()),
            Colormap::CoolWarm => (
                COOLWARM_R.as_slice(),
                COOLWARM_G.as_slice(),
                COOLWARM_B.as_slice(),
            ),
            Colormap::Spectral => (
                SPECTRAL_R.as_slice(),
                SPECTRAL_G.as_slice(),
                SPECTRAL_B.as_slice(),
            ),
            Colormap::Copper => (
                COPPER_R.as_slice(),
                COPPER_G.as_slice(),
                COPPER_B.as_slice(),
            ),
            Colormap::Bone => (BONE_R.as_slice(), BONE_G.as_slice(), BONE_B.as_slice()),
            Colormap::Hsv => (HSV_R.as_slice(), HSV_G.as_slice(), HSV_B.as_slice()),
            Colormap::Afmhot => (
                AFMHOT_R.as_slice(),
                AFMHOT_G.as_slice(),
                AFMHOT_B.as_slice(),
            ),
            Colormap::Rdbu => (RDBU_R.as_slice(), RDBU_G.as_slice(), RDBU_B.as_slice()),
            Colormap::Cubehelix => (
                CUBEHELIX_R.as_slice(),
                CUBEHELIX_G.as_slice(),
                CUBEHELIX_B.as_slice(),
            ),
            Colormap::Gnuplot2 => (
                GNUPLOT2_R.as_slice(),
                GNUPLOT2_G.as_slice(),
                GNUPLOT2_B.as_slice(),
            ),
            Colormap::Terrain => (
                TERRAIN_R.as_slice(),
                TERRAIN_G.as_slice(),
                TERRAIN_B.as_slice(),
            ),
        }
    }
}
