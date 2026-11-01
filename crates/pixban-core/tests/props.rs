mod common;

use pixban_core::limits::*;
use pixban_core::palette;
use pixban_core::{
    render, Align, Bevel, Connectivity, Direction, Dither, Extrude, Fill, Outline, Plate, Rgb,
    Shadow, Spec, Texture, TextureKind,
};
use proptest::prelude::*;

fn line_width(line: &str, spacing: i32) -> u32 {
    let font = common::test_font();
    let chars: Vec<char> = line.chars().flat_map(char::to_uppercase).collect();
    if chars.is_empty() {
        return 0;
    }
    let glyphs: i32 = chars
        .iter()
        .map(|&c| font.glyph(c).unwrap_or(font.fallback()).width() as i32)
        .sum();
    let kerns: i32 = chars.windows(2).map(|w| font.kerning(w[0], w[1])).sum();
    (glyphs + spacing * (chars.len() as i32 - 1) + kerns) as u32
}

/// The canvas a spec must produce, worked out from the documented growth rules.
fn expected_canvas(spec: &Spec) -> (u32, u32) {
    let text = spec.text.replace("\r\n", "\n");
    let lines: Vec<&str> = text.trim_end_matches('\n').split('\n').collect();
    let spacing = spec.spacing.unwrap_or(2);
    let width = lines.iter().map(|l| line_width(l, spacing)).max().unwrap();
    let height = 7 * lines.len() as u32 + spec.line_gap * (lines.len() as u32 - 1);
    let ring = spec.outline.as_ref().map_or(0, |o| o.width)
        + spec.plate.as_ref().map_or(0, |p| p.border)
        + spec
            .padding
            .unwrap_or(if spec.plate.is_some() { 2 } else { 0 });
    let (ex, ey) = spec.extrude.as_ref().map_or((0, 0), |e| {
        (
            (e.dx * e.length as i32).unsigned_abs(),
            (e.dy * e.length as i32).unsigned_abs(),
        )
    });
    let (sx, sy) = spec
        .shadow
        .as_ref()
        .map_or((0, 0), |s| (s.dx.unsigned_abs(), s.dy.unsigned_abs()));
    (width + 2 * ring + ex + sx, height + 2 * ring + ey + sy)
}

fn arb_rgb() -> impl Strategy<Value = Rgb> {
    any::<[u8; 3]>().prop_map(|[r, g, b]| Rgb(r, g, b))
}

fn arb_text() -> impl Strategy<Value = String> {
    (
        prop::collection::vec("[AaIiДд ?]{0,8}", 0..=2),
        "[AaIiДд ?]{1,8}",
    )
        .prop_map(|(mut head, last)| {
            head.push(last);
            head.join("\n")
        })
}

fn arb_palette() -> impl Strategy<Value = Option<String>> {
    let names: Vec<String> = palette::names()
        .chain(palette::extended_names())
        .map(String::from)
        .collect();
    prop::option::of(prop_oneof![
        prop::sample::select(names),
        prop::collection::vec(arb_rgb(), 2..=16).prop_map(|c| c
            .iter()
            .map(Rgb::to_string)
            .collect::<Vec<_>>()
            .join(",")),
    ])
}

fn arb_fill() -> impl Strategy<Value = Fill> {
    (
        arb_palette(),
        prop_oneof![
            Just(Direction::Vertical),
            Just(Direction::Horizontal),
            Just(Direction::Diagonal)
        ],
        prop::option::of(arb_rgb()),
        prop_oneof![
            Just(Dither::None),
            Just(Dither::Bayer2),
            Just(Dither::Bayer4)
        ],
        prop::bool::ANY,
        prop::collection::vec(arb_rgb(), 0..=MAX_LETTER_COLORS),
    )
        .prop_map(
            |(palette, direction, color, dither, per_letter, letter_colors)| Fill {
                palette,
                direction,
                color,
                dither,
                per_letter,
                letter_colors,
            },
        )
}

#[derive(Clone, Debug)]
struct Effects {
    shadow: Option<Shadow>,
    outline: Option<Outline>,
    extrude: Option<Extrude>,
    bevel: Option<Bevel>,
    texture: Option<Texture>,
    plate: Option<Plate>,
}

fn arb_effects() -> impl Strategy<Value = Effects> {
    let shadow = prop::option::of((-12i32..=12, -12i32..=12, prop::option::of(arb_rgb())));
    let outline = prop::option::of((
        1u32..=MAX_OUTLINE_WIDTH,
        prop::bool::ANY,
        prop::option::of(arb_rgb()),
    ));
    let extrude = prop::option::of((
        1u32..=MAX_EXTRUDE,
        (-1i32..=1, -1i32..=1).prop_filter("direction", |d| *d != (0, 0)),
        1u32..=100,
        prop::option::of(arb_rgb()),
    ));
    let bevel = prop::option::of(1u32..=100);
    let texture = prop::option::of((0usize..3, 1u32..=100, 2u32..=MAX_TEXTURE_PERIOD));
    let plate = prop::option::of((
        prop::option::of(arb_rgb()),
        0u32..=MAX_PLATE_BORDER,
        prop::option::of(arb_rgb()),
        prop::bool::ANY,
    ));
    (shadow, outline, extrude, bevel, texture, plate).prop_map(
        |(shadow, outline, extrude, bevel, texture, plate)| Effects {
            shadow: shadow.map(|(dx, dy, color)| Shadow { dx, dy, color }),
            outline: outline.map(|(width, eight, color)| Outline {
                width,
                connectivity: if eight {
                    Connectivity::Eight
                } else {
                    Connectivity::Four
                },
                color,
            }),
            extrude: extrude.map(|(length, (dx, dy), step, color)| Extrude {
                length,
                dx,
                dy,
                step,
                color,
            }),
            bevel: bevel.map(|amount| Bevel { amount }),
            texture: texture.map(|(kind, strength, period)| Texture {
                kind: [
                    TextureKind::Scanlines,
                    TextureKind::Checker,
                    TextureKind::Noise,
                ][kind],
                strength,
                period,
            }),
            plate: plate.map(|(color, border, border_color, cut_corners)| Plate {
                color,
                border,
                border_color,
                cut_corners,
            }),
        },
    )
}

fn arb_spec() -> impl Strategy<Value = Spec> {
    (
        arb_text(),
        (
            (1u32..=2).prop_map(Some),
            prop::option::of(arb_rgb()),
            any::<u32>(),
            prop::option::of(0u32..=MAX_PADDING),
        ),
        (
            prop_oneof![Just(Align::Left), Just(Align::Center), Just(Align::Right)],
            0u32..=MAX_LINE_GAP,
            prop::option::of(MIN_SPACING..=8),
        ),
        arb_fill(),
        arb_effects(),
    )
        .prop_map(
            |(text, (scale, bg, seed, padding), (align, line_gap, spacing), fill, effects)| Spec {
                text,
                font: "test".into(),
                scale,
                bg,
                seed,
                padding,
                align,
                line_gap,
                spacing,
                fill,
                shadow: effects.shadow,
                outline: effects.outline,
                extrude: effects.extrude,
                bevel: effects.bevel,
                texture: effects.texture,
                plate: effects.plate,
            },
        )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    #[test]
    fn valid_specs_render_with_the_expected_canvas(spec in arb_spec()) {
        let font = common::test_font();
        let out = render(&spec, &font).unwrap();
        let (w, h) = expected_canvas(&spec);
        prop_assert_eq!((out.grid.width(), out.grid.height()), (w, h));
        let scale = spec.scale.unwrap_or(8);
        prop_assert_eq!(out.pixel_size(), (w * scale, h * scale));

        let svg = out.to_svg();
        prop_assert!(roxmltree::Document::parse(&svg).is_ok());
        prop_assert_eq!(&svg, &render(&spec, &font).unwrap().to_svg(), "not deterministic");

        let png = out.to_png().unwrap();
        let reader = png::Decoder::new(std::io::Cursor::new(png)).read_info().unwrap();
        prop_assert_eq!((reader.info().width, reader.info().height), out.pixel_size());
    }

    #[test]
    fn svg_has_one_path_per_color(spec in arb_spec()) {
        let out = render(&spec, &common::test_font()).unwrap();
        let svg = out.to_svg();
        let doc = roxmltree::Document::parse(&svg).unwrap();
        let mut fills: Vec<&str> = doc
            .descendants()
            .filter(|n| n.has_tag_name("path"))
            .map(|n| n.attribute("fill").unwrap())
            .collect();
        let total = fills.len();
        fills.sort_unstable();
        fills.dedup();
        prop_assert_eq!(total, fills.len());
    }
}

fn small_or_any_i32() -> impl Strategy<Value = i32> {
    prop_oneof![3 => -40i32..=40, 1 => any::<i32>()]
}

fn small_or_any_u32() -> impl Strategy<Value = u32> {
    prop_oneof![3 => 0u32..=70, 1 => any::<u32>()]
}

fn wild_spec() -> impl Strategy<Value = Spec> {
    (
        prop_oneof!["(?s).{0,40}", "[AaIiДд ?\n]{0,40}"],
        (
            prop::option::of(small_or_any_u32()),
            prop::option::of(small_or_any_u32()),
            small_or_any_u32(),
            prop::option::of(small_or_any_i32()),
        ),
        prop::option::of("(?s).{0,12}"),
        prop::option::of((small_or_any_i32(), small_or_any_i32())),
        prop::option::of((small_or_any_u32(), prop::bool::ANY)),
        prop::option::of((
            small_or_any_u32(),
            small_or_any_i32(),
            small_or_any_i32(),
            small_or_any_u32(),
        )),
        prop::option::of(small_or_any_u32()),
        prop::option::of((small_or_any_u32(), small_or_any_u32())),
        prop::option::of(small_or_any_u32()),
        prop::collection::vec(arb_rgb(), 0..=80),
    )
        .prop_map(
            |(
                text,
                (scale, padding, line_gap, spacing),
                palette,
                shadow,
                outline,
                extrude,
                bevel,
                texture,
                plate,
                letters,
            )| Spec {
                text,
                scale,
                padding,
                line_gap,
                spacing,
                fill: Fill {
                    palette,
                    letter_colors: letters,
                    ..Fill::default()
                },
                shadow: shadow.map(|(dx, dy)| Shadow {
                    dx,
                    dy,
                    color: None,
                }),
                outline: outline.map(|(width, eight)| Outline {
                    width,
                    connectivity: if eight {
                        Connectivity::Eight
                    } else {
                        Connectivity::Four
                    },
                    color: None,
                }),
                extrude: extrude.map(|(length, dx, dy, step)| Extrude {
                    length,
                    dx,
                    dy,
                    step,
                    color: None,
                }),
                bevel: bevel.map(|amount| Bevel { amount }),
                texture: texture.map(|(strength, period)| Texture {
                    kind: TextureKind::Noise,
                    strength,
                    period,
                }),
                plate: plate.map(|border| Plate {
                    border,
                    ..Plate::default()
                }),
                ..Spec::default()
            },
        )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn arbitrary_settings_never_panic(spec in wild_spec()) {
        let font = common::test_font();
        match render(&spec, &font) {
            Ok(out) => {
                let (w, h) = expected_canvas(&spec);
                prop_assert_eq!((out.grid.width(), out.grid.height()), (w, h));
            }
            Err(e) => {
                prop_assert!(!e.to_string().is_empty());
            }
        }
    }
}
