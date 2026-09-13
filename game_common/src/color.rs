//! Color space conversion

use std::ops::{Mul, MulAssign};

use glam::FloatExt;

/// A color in the standard sRGB color space.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Rgb {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
}

impl Rgb {
    /// Converts this color to the HSL color space.
    #[inline]
    pub fn to_hsl(self) -> Hsl {
        nonlinear_srgb_to_hsl(self.red, self.green, self.blue)
    }

    /// Converts this color to the HSV color space.
    #[inline]
    pub fn to_hsv(self) -> Hsv {
        nonlinear_srgb_to_hsv(self.red, self.green, self.blue)
    }
}

/// A color in the HSL color space.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Hsl {
    /// Hue
    pub hue: f32,
    /// Saturation
    pub saturation: f32,
    /// Lightness
    pub lightness: f32,
}

impl Hsl {
    /// Converts this color to the sRGB color space.
    #[inline]
    pub fn to_srgb(self) -> Rgb {
        hsl_to_nonlinear_srgb(self.hue, self.saturation, self.lightness)
    }

    /// Converts this color to the HSV color space.
    #[inline]
    pub fn to_hsv(self) -> Hsv {
        hsl_to_hsv(self.hue, self.saturation, self.lightness)
    }
}

/// A color in the HSV color space.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Hsv {
    /// Hue
    pub hue: f32,
    /// Saturation
    pub saturation: f32,
    /// Value (Brightness)
    pub value: f32,
}

impl Hsv {
    /// Converts this color to the sRGB color space.
    #[inline]
    pub fn to_srgb(self) -> Rgb {
        hsv_to_nonlinear_srgb(self.hue, self.saturation, self.value)
    }

    /// Converts this color to the HSL color space.
    #[inline]
    pub fn to_hsl(self) -> Hsl {
        hsv_to_hsl(self.hue, self.saturation, self.value)
    }
}

/// Converts a color from HSL to sRGB.
fn hsl_to_nonlinear_srgb(hue: f32, saturation: f32, lightness: f32) -> Rgb {
    // See https://en.wikipedia.org/wiki/HSL_and_HSV#HSL_to_RGB
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let hue_prime = hue / 60.0;
    let largest_component = chroma * (1.0 - (hue_prime % 2.0 - 1.0).abs());

    let (r1, g1, b1) = if hue_prime < 1.0 {
        (chroma, largest_component, 0.0)
    } else if hue_prime < 2.0 {
        (largest_component, chroma, 0.0)
    } else if hue_prime < 3.0 {
        (0.0, chroma, largest_component)
    } else if hue_prime < 4.0 {
        (0.0, largest_component, chroma)
    } else if hue_prime < 5.0 {
        (largest_component, 0.0, chroma)
    } else {
        (chroma, 0.0, largest_component)
    };

    let lightness_match = lightness - chroma / 2.0;

    Rgb {
        red: r1 * lightness_match,
        green: g1 * lightness_match,
        blue: b1 * lightness_match,
    }
}

/// Converts a color from HSV to sRGB.
fn hsv_to_nonlinear_srgb(hue: f32, saturation: f32, value: f32) -> Rgb {
    // See https://en.wikipedia.org/wiki/HSL_and_HSV#HSV_to_RGB
    let chroma = value * saturation;
    let hue_prime = hue / 60.0;
    let largest_component = chroma * (1.0 - (hue_prime % 2.0 - 1.0).abs());

    let (r1, g1, b1) = if hue_prime < 1.0 {
        (chroma, largest_component, 0.0)
    } else if hue_prime < 2.0 {
        (largest_component, chroma, 0.0)
    } else if hue_prime < 3.0 {
        (0.0, chroma, largest_component)
    } else if hue_prime < 4.0 {
        (0.0, largest_component, chroma)
    } else if hue_prime < 5.0 {
        (largest_component, 0.0, chroma)
    } else {
        (chroma, 0.0, largest_component)
    };

    let lightness_match = value - chroma;

    Rgb {
        red: r1 * lightness_match,
        green: g1 * lightness_match,
        blue: b1 * lightness_match,
    }
}

fn nonlinear_srgb_to_hsl(red: f32, green: f32, blue: f32) -> Hsl {
    // See https://en.wikipedia.org/wiki/HSL_and_HSV#From_RGB
    debug_assert!(red >= 0.0 && red <= 1.0);
    debug_assert!(green >= 0.0 && green <= 1.0);
    debug_assert!(blue >= 0.0 && blue <= 1.0);

    let max = red.max(green.max(blue));
    let min = red.min(green.min(blue));
    let chroma = max - min;
    let lightness = (max + min) / 2.0;

    let mut hue = if chroma == 0.0 {
        0.0
    } else if max == red {
        60.0 * (green - blue) / chroma
    } else if max == green {
        60.0 * (2.0 + (blue - red) / chroma)
    } else {
        60.0 * (4.0 + (red - green) / chroma)
    };

    if hue < 0.0 {
        hue += 360.0
    }

    let saturation = if lightness <= 0.0 || lightness >= 1.0 {
        0.0
    } else {
        (max - lightness) / lightness.min(1.0 - lightness)
    };

    Hsl {
        hue,
        saturation,
        lightness,
    }
}

fn nonlinear_srgb_to_hsv(red: f32, green: f32, blue: f32) -> Hsv {
    // See https://en.wikipedia.org/wiki/HSL_and_HSV#From_RGB
    debug_assert!(red >= 0.0 && red <= 1.0);
    debug_assert!(green >= 0.0 && green <= 1.0);
    debug_assert!(blue >= 0.0 && blue <= 1.0);

    let max = red.max(green.max(blue));
    let min = red.min(green.min(blue));
    let chroma = max - min;

    let mut hue = if chroma == 0.0 {
        0.0
    } else if max == red {
        60.0 * (green - blue) / chroma
    } else if max == green {
        60.0 * (2.0 + (blue - red) / chroma)
    } else {
        60.0 * (4.0 + (red - green) / chroma)
    };

    if hue < 0.0 {
        hue += 360.0
    }

    let saturation = if max == 0.0 { 0.0 } else { chroma / max };

    Hsv {
        hue,
        saturation,
        value: max,
    }
}

fn hsv_to_hsl(hue: f32, saturation: f32, value: f32) -> Hsl {
    // See https://en.wikipedia.org/wiki/HSL_and_HSV#HSV_to_HSL
    let lightness = value * (1.0 - (saturation / 2.0));
    let hsl_saturation = if lightness <= 0.0 || lightness >= 1.0 {
        0.0
    } else {
        (value - lightness) / lightness.min(1.0 - lightness)
    };

    Hsl {
        hue,
        saturation: hsl_saturation,
        lightness,
    }
}

fn hsl_to_hsv(hue: f32, saturation: f32, lightness: f32) -> Hsv {
    // See https://en.wikipedia.org/wiki/HSL_and_HSV#HSL_to_HSV
    let value = lightness + saturation * lightness.min(1.0 - lightness);
    let hsv_saturation = if value == 0.0 {
        0.0
    } else {
        2.0 * (1.0 - lightness / value)
    };

    Hsv {
        hue,
        saturation: hsv_saturation,
        value,
    }
}

/// Linear RGB color.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct LinearRgb {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
}

impl LinearRgb {
    pub const BLACK: Self = Self::from_rgb(0.0, 0.0, 0.0);
    pub const WHITE: Self = Self::from_rgb(1.0, 1.0, 1.0);

    /// Converts this color from linear to sRGB colorspace.
    pub fn to_srgb(&self) -> Rgb {
        Rgb {
            red: linear_to_srgb(self.red),
            green: linear_to_srgb(self.green),
            blue: linear_to_srgb(self.blue),
        }
    }

    /// Converts a color from sRGB to linear colorspace.
    pub fn from_srgb(color: Rgb) -> Self {
        Self {
            red: srgb_to_linear(color.red),
            green: srgb_to_linear(color.green),
            blue: srgb_to_linear(color.blue),
        }
    }

    pub fn lerp(self, other: Self, s: f32) -> Self {
        Self {
            red: self.red.lerp(other.red, s),
            green: self.green.lerp(other.green, s),
            blue: self.blue.lerp(other.blue, s),
        }
    }

    const fn from_rgb(red: f32, green: f32, blue: f32) -> Self {
        Self { red, green, blue }
    }

    pub const fn as_rgb(&self) -> [f32; 3] {
        [self.red, self.green, self.blue]
    }
}

impl Mul<f32> for LinearRgb {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            red: self.red * rhs,
            green: self.green * rhs,
            blue: self.blue * rhs,
        }
    }
}

impl MulAssign<f32> for LinearRgb {
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs;
    }
}

// sRGB transfer functions
// https://en.wikipedia.org/wiki/SRGB#Transformation

fn linear_to_srgb(color: f32) -> f32 {
    if color <= 0.0031308 {
        return color * 12.92;
    } else {
        return (f32::powf(color, 1.0 / 2.4) * 1.055) - 0.055;
    }
}

fn srgb_to_linear(color: f32) -> f32 {
    if color <= 0.04045 {
        return color / 12.92;
    } else {
        return f32::powf((color + 0.055) / 1.055, 2.4);
    }
}
