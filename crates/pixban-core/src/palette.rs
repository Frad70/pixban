use crate::color::Rgb;

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

pub fn find(name: &str) -> Option<&'static Palette> {
    PALETTES.iter().find(|p| p.name.eq_ignore_ascii_case(name))
}

pub fn names() -> impl Iterator<Item = &'static str> {
    PALETTES.iter().map(|p| p.name)
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
