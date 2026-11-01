use std::fmt;
use std::str::FromStr;

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub const fn hex(v: u32) -> Rgb {
        Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
    }

    pub const BLACK: Rgb = Rgb(0, 0, 0);
    pub const WHITE: Rgb = Rgb(255, 255, 255);

    /// Perceived brightness on a 0 to 255000 scale, for picking the darkest of several colors.
    pub fn luma(self) -> u32 {
        299 * u32::from(self.0) + 587 * u32::from(self.1) + 114 * u32::from(self.2)
    }

    /// Moves `percent` of the way from `self` to `other`, per channel, rounding to nearest.
    pub fn mix(self, other: Rgb, percent: u32) -> Rgb {
        let p = percent.min(100);
        let f = |a: u8, b: u8| ((u32::from(a) * (100 - p) + u32::from(b) * p + 50) / 100) as u8;
        Rgb(f(self.0, other.0), f(self.1, other.1), f(self.2, other.2))
    }

    pub fn darken(self, percent_left: u16) -> Rgb {
        let f = |c: u8| (u16::from(c) * percent_left / 100) as u8;
        Rgb(f(self.0), f(self.1), f(self.2))
    }
}

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseColorError(String);

impl fmt::Display for ParseColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "'{}' is not a color, use #rgb or #rrggbb (the # is optional)",
            self.0
        )
    }
}

impl std::error::Error for ParseColorError {}

impl FromStr for Rgb {
    type Err = ParseColorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || ParseColorError(s.to_string());
        let digits = s.strip_prefix('#').unwrap_or(s);
        if !digits.is_ascii() || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(err());
        }
        match digits.len() {
            3 => {
                let v = u32::from_str_radix(digits, 16).map_err(|_| err())?;
                let (r, g, b) = ((v >> 8) & 0xf, (v >> 4) & 0xf, v & 0xf);
                Ok(Rgb((r * 17) as u8, (g * 17) as u8, (b * 17) as u8))
            }
            6 => Ok(Rgb::hex(
                u32::from_str_radix(digits, 16).map_err(|_| err())?,
            )),
            _ => Err(err()),
        }
    }
}

impl Serialize for Rgb {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Rgb {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgba(pub u8, pub u8, pub u8, pub u8);

impl Rgba {
    pub const TRANSPARENT: Rgba = Rgba(0, 0, 0, 0);

    pub fn is_transparent(self) -> bool {
        self.3 == 0
    }

    pub fn rgb(self) -> Rgb {
        Rgb(self.0, self.1, self.2)
    }
}

impl From<Rgb> for Rgba {
    fn from(c: Rgb) -> Self {
        Rgba(c.0, c.1, c.2, 255)
    }
}
