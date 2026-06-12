//! Color and material (FR-2.1).

/// An 8-bit-per-channel RGB color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const BLACK: Self = Self::new(0, 0, 0);
    pub const WHITE: Self = Self::new(255, 255, 255);

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Perceptual luminance in `0.0..=1.0` (Rec. 601 weights). Drives the
    /// ASCII ramp mapping (FR-2.6).
    pub fn luminance(self) -> f32 {
        (0.299 * f32::from(self.r) + 0.587 * f32::from(self.g) + 0.114 * f32::from(self.b)) / 255.0
    }

    /// Scale every channel by `factor` (clamped to `0.0..=1.0`), rounding to
    /// the nearest 8-bit value. Used to apply a shading intensity to a base color.
    pub fn scaled(self, factor: f32) -> Self {
        let f = factor.clamp(0.0, 1.0);
        let ch = |c: u8| (f32::from(c) * f + 0.5) as u8;
        Self::new(ch(self.r), ch(self.g), ch(self.b))
    }
}

/// Surface appearance. Phase 2 carries only a base color; richer materials
/// (textures, PBR) are deferred per the project brief.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Material {
    pub base_color: Rgb,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            base_color: Rgb::new(210, 210, 210),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fr2_1_luminance_endpoints() {
        assert_eq!(Rgb::BLACK.luminance(), 0.0);
        assert!((Rgb::WHITE.luminance() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn fr2_1_luminance_is_weighted_green_heaviest() {
        let r = Rgb::new(255, 0, 0).luminance();
        let g = Rgb::new(0, 255, 0).luminance();
        let b = Rgb::new(0, 0, 255).luminance();
        assert!(g > r && r > b, "expected G>R>B, got {g} {r} {b}");
    }

    #[test]
    fn fr2_1_scaled_clamps_and_rounds() {
        assert_eq!(Rgb::WHITE.scaled(0.0), Rgb::BLACK);
        assert_eq!(Rgb::WHITE.scaled(1.0), Rgb::WHITE);
        assert_eq!(Rgb::WHITE.scaled(2.0), Rgb::WHITE, "factor clamps at 1.0");
        assert_eq!(Rgb::new(100, 100, 100).scaled(0.5), Rgb::new(50, 50, 50));
    }
}
