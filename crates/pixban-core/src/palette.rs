use crate::color::Rgb;
use crate::error::Error;
use crate::limits::{MAX_PALETTE_STOPS, MIN_PALETTE_STOPS};

#[derive(Debug)]
pub struct Palette {
    pub name: &'static str,
    pub stops: &'static [Rgb],
    pub shadow: Rgb,
}

pub static PALETTES: &[Palette] = &[
    Palette {
        name: "fire",
        stops: &[
            Rgb::hex(0xdf4f07),
            Rgb::hex(0xdf5707),
            Rgb::hex(0xd75f07),
            Rgb::hex(0xcf6f0f),
            Rgb::hex(0xcf7f0f),
            Rgb::hex(0xc78f17),
            Rgb::hex(0xbf9f1f),
        ],
        shadow: Rgb::hex(0x470f07),
    },
    Palette {
        name: "ice",
        stops: &[
            Rgb::hex(0xb4eaff),
            Rgb::hex(0x92ddff),
            Rgb::hex(0x6fd0ff),
            Rgb::hex(0x4cbaf8),
            Rgb::hex(0x349ce8),
            Rgb::hex(0x2c7ed2),
            Rgb::hex(0x2862b4),
        ],
        shadow: Rgb::hex(0x0b2447),
    },
    Palette {
        name: "toxic",
        stops: &[
            Rgb::hex(0xe8ff4a),
            Rgb::hex(0xc9f52f),
            Rgb::hex(0xa5e81f),
            Rgb::hex(0x80d618),
            Rgb::hex(0x5cc015),
            Rgb::hex(0x3ba712),
            Rgb::hex(0x278a10),
        ],
        shadow: Rgb::hex(0x0d2a07),
    },
    Palette {
        name: "neon",
        stops: &[
            Rgb::hex(0xff3cc7),
            Rgb::hex(0xf040e0),
            Rgb::hex(0xc648f5),
            Rgb::hex(0x8f56ff),
            Rgb::hex(0x5f7dff),
            Rgb::hex(0x36a9ff),
            Rgb::hex(0x2fe0ff),
        ],
        shadow: Rgb::hex(0x230a48),
    },
    Palette {
        name: "gold",
        stops: &[
            Rgb::hex(0xffe98a),
            Rgb::hex(0xffdb5c),
            Rgb::hex(0xf7c93a),
            Rgb::hex(0xdca31f),
            Rgb::hex(0xbf8512),
            Rgb::hex(0x9d680d),
            Rgb::hex(0x7a4e09),
        ],
        shadow: Rgb::hex(0x3a2304),
    },
    Palette {
        name: "synthwave",
        stops: &[
            Rgb::hex(0xffe44d),
            Rgb::hex(0xffc93c),
            Rgb::hex(0xff9a3c),
            Rgb::hex(0xff5c8a),
            Rgb::hex(0xe0359f),
            Rgb::hex(0xa43bd0),
            Rgb::hex(0x6a3ee0),
        ],
        shadow: Rgb::hex(0x2a0b4a),
    },
    Palette {
        name: "mono",
        stops: &[
            Rgb::hex(0xdcdcdc),
            Rgb::hex(0xcacaca),
            Rgb::hex(0xb8b8b8),
            Rgb::hex(0xa6a6a6),
            Rgb::hex(0x949494),
            Rgb::hex(0x828282),
            Rgb::hex(0x707070),
        ],
        shadow: Rgb::hex(0x202020),
    },
    Palette {
        name: "rainbow",
        stops: &[
            Rgb::hex(0xff3b3b),
            Rgb::hex(0xff9a2e),
            Rgb::hex(0xffe033),
            Rgb::hex(0x4ddb52),
            Rgb::hex(0x2ec8ff),
            Rgb::hex(0x4a6bff),
            Rgb::hex(0xa255ff),
        ],
        shadow: Rgb::hex(0x1a1033),
    },
];

/// Ordered ramps for the extended group. The hardware ramps use published values:
/// PICO-8 (lexaloffle.com manual), Game Boy (the common DMG greens), NES (the widely used
/// 2C02 NTSC table, as in FCEUX), C64 (Pepto's measured palette), CGA (IBM 5153 RGBI colors).
/// The themed ramps and every default shadow color are our own picks.
pub static EXTENDED: &[Palette] = &[
    Palette {
        name: "pico8-warm",
        stops: &[
            Rgb::hex(0xFFF1E8),
            Rgb::hex(0xFFEC27),
            Rgb::hex(0xFFA300),
            Rgb::hex(0xFF004D),
            Rgb::hex(0x7E2553),
        ],
        shadow: Rgb::hex(0x1D2B53),
    },
    Palette {
        name: "pico8-cool",
        stops: &[
            Rgb::hex(0x00E436),
            Rgb::hex(0x29ADFF),
            Rgb::hex(0x83769C),
            Rgb::hex(0x1D2B53),
        ],
        shadow: Rgb::hex(0x0B1230),
    },
    Palette {
        name: "gameboy",
        stops: &[
            Rgb::hex(0x9BBC0F),
            Rgb::hex(0x8BAC0F),
            Rgb::hex(0x306230),
            Rgb::hex(0x0F380F),
        ],
        shadow: Rgb::hex(0x071C07),
    },
    Palette {
        name: "nes",
        stops: &[
            Rgb::hex(0xFCE0A8),
            Rgb::hex(0xFCA044),
            Rgb::hex(0xF87858),
            Rgb::hex(0xF83800),
            Rgb::hex(0xA81000),
            Rgb::hex(0x503000),
        ],
        shadow: Rgb::hex(0x1C0A00),
    },
    Palette {
        name: "c64",
        stops: &[
            Rgb::hex(0xB8C76F),
            Rgb::hex(0x9AD284),
            Rgb::hex(0x70A4B2),
            Rgb::hex(0x6C5EB5),
            Rgb::hex(0x6F3D86),
            Rgb::hex(0x352879),
        ],
        shadow: Rgb::hex(0x1B1440),
    },
    Palette {
        name: "cga",
        stops: &[
            Rgb::hex(0xFFFFFF),
            Rgb::hex(0x55FFFF),
            Rgb::hex(0xFF55FF),
            Rgb::hex(0xAA00AA),
            Rgb::hex(0x0000AA),
        ],
        shadow: Rgb::hex(0x000000),
    },
    Palette {
        name: "lava",
        stops: &[
            Rgb::hex(0xFFF1A8),
            Rgb::hex(0xFFD24A),
            Rgb::hex(0xFF9F1C),
            Rgb::hex(0xFF5D0A),
            Rgb::hex(0xE02F00),
            Rgb::hex(0xA31500),
            Rgb::hex(0x5C0A00),
        ],
        shadow: Rgb::hex(0x1F0502),
    },
    Palette {
        name: "ocean",
        stops: &[
            Rgb::hex(0xD6F5F2),
            Rgb::hex(0x8FE3E1),
            Rgb::hex(0x4FC8D6),
            Rgb::hex(0x2AA3C9),
            Rgb::hex(0x1E78B4),
            Rgb::hex(0x15508F),
            Rgb::hex(0x0C2F66),
        ],
        shadow: Rgb::hex(0x061630),
    },
    Palette {
        name: "forest",
        stops: &[
            Rgb::hex(0xD4E8A0),
            Rgb::hex(0xA5D06B),
            Rgb::hex(0x6DB34A),
            Rgb::hex(0x3F9140),
            Rgb::hex(0x2A7040),
            Rgb::hex(0x1B5036),
            Rgb::hex(0x0F3326),
        ],
        shadow: Rgb::hex(0x07170F),
    },
    Palette {
        name: "sunset",
        stops: &[
            Rgb::hex(0xFFE29A),
            Rgb::hex(0xFFB86B),
            Rgb::hex(0xFF8A5C),
            Rgb::hex(0xF25E7A),
            Rgb::hex(0xC23E8A),
            Rgb::hex(0x7B3FA0),
            Rgb::hex(0x3F3A8A),
        ],
        shadow: Rgb::hex(0x1A1040),
    },
    Palette {
        name: "pastel",
        stops: &[
            Rgb::hex(0xFFB3BA),
            Rgb::hex(0xFFDFBA),
            Rgb::hex(0xFFFFBA),
            Rgb::hex(0xBAFFC9),
            Rgb::hex(0xBAE1FF),
            Rgb::hex(0xD5BAFF),
        ],
        shadow: Rgb::hex(0x5A4A78),
    },
    Palette {
        name: "vaporwave",
        stops: &[
            Rgb::hex(0xFFFB96),
            Rgb::hex(0x05FFA1),
            Rgb::hex(0x01CDFE),
            Rgb::hex(0xB967FF),
            Rgb::hex(0xFF71CE),
        ],
        shadow: Rgb::hex(0x2D1B4E),
    },
    Palette {
        name: "ember",
        stops: &[
            Rgb::hex(0xCFC6BC),
            Rgb::hex(0xE8B98A),
            Rgb::hex(0xF29A4A),
            Rgb::hex(0xE36A24),
            Rgb::hex(0xB73E14),
            Rgb::hex(0x7A220E),
            Rgb::hex(0x3D1008),
        ],
        shadow: Rgb::hex(0x1A0604),
    },
];

pub fn find(name: &str) -> Option<&'static Palette> {
    PALETTES
        .iter()
        .chain(EXTENDED)
        .find(|p| p.name.eq_ignore_ascii_case(name))
}

pub fn names() -> impl Iterator<Item = &'static str> {
    PALETTES.iter().map(|p| p.name)
}

pub fn extended_names() -> impl Iterator<Item = &'static str> {
    EXTENDED.iter().map(|p| p.name)
}

/// Stops and shadow of a palette picked by name or written out as colors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    pub stops: Vec<Rgb>,
    pub shadow: Rgb,
}

impl From<&Palette> for Resolved {
    fn from(p: &Palette) -> Self {
        Resolved {
            stops: p.stops.to_vec(),
            shadow: p.shadow,
        }
    }
}

/// A palette string is custom when it lists colors: `#ff0000,#ffff00` or `#ff0000,#ffff00/#330000`.
pub fn is_custom(spec: &str) -> bool {
    spec.contains([',', '#', '/'])
}

pub fn parse_custom(spec: &str) -> Result<Resolved, Error> {
    let bad = |m: String| Error::InvalidPalette(m);
    let (list, shadow) = match spec.split_once('/') {
        Some((list, shadow)) => (list, Some(shadow)),
        None => (spec, None),
    };
    let mut stops = Vec::new();
    for (i, item) in list.split(',').enumerate() {
        let item = item.trim();
        let color = item
            .parse::<Rgb>()
            .map_err(|e| bad(format!("item {} {e}", i + 1)))?;
        stops.push(color);
    }
    if !(MIN_PALETTE_STOPS..=MAX_PALETTE_STOPS).contains(&stops.len()) {
        return Err(bad(format!(
            "{} colors given, use {MIN_PALETTE_STOPS} to {MAX_PALETTE_STOPS}",
            stops.len()
        )));
    }
    let explicit = match shadow {
        Some(s) => Some(
            s.trim()
                .parse::<Rgb>()
                .map_err(|e| bad(format!("shadow color {e}")))?,
        ),
        None => None,
    };
    let darkest = stops.iter().copied().min_by_key(|c| c.luma()).unwrap();
    Ok(Resolved {
        shadow: explicit.unwrap_or_else(|| darkest.darken(30)),
        stops,
    })
}

/// Looks up a palette by name, or parses a custom color list.
pub fn resolve(spec: &str) -> Result<Resolved, Error> {
    if is_custom(spec) {
        return parse_custom(spec);
    }
    find(spec)
        .map(Resolved::from)
        .ok_or_else(|| Error::UnknownPalette(spec.to_string()))
}

/// Index of the stop used for position `i` of `len` along a gradient over `stops` colors.
pub fn stop_index(i: u32, len: u32, stops: usize) -> usize {
    if len <= 1 || stops <= 1 {
        return 0;
    }
    let i = u64::from(i.min(len - 1));
    let (n1, l1) = ((stops - 1) as u64, u64::from(len - 1));
    ((i * n1 * 2 + l1) / (l1 * 2)) as usize
}
