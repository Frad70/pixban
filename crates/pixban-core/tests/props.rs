mod common;

use pixban_core::palette;
use pixban_core::{render, Direction, Error, Fill, Rgb, Shadow, Spec};
use proptest::prelude::*;

fn expected_width(text: &str) -> u32 {
    let font = common::test_font();
    let chars: Vec<char> = text.chars().flat_map(char::to_uppercase).collect();
    let glyphs: u32 = chars
        .iter()
        .map(|&c| font.glyph(c).unwrap_or(font.fallback()).width())
        .sum();
    let kerns: i32 = chars.windows(2).map(|w| font.kerning(w[0], w[1])).sum();
    (glyphs as i32 + (font.spacing() * (chars.len() as u32 - 1)) as i32 + kerns) as u32
}

fn arb_rgb() -> impl Strategy<Value = Rgb> {
    any::<[u8; 3]>().prop_map(|[r, g, b]| Rgb(r, g, b))
}

fn arb_spec() -> impl Strategy<Value = Spec> {
    let names: Vec<String> = palette::names().map(String::from).collect();
    (
        prop_oneof!["(?s).{0,60}", "[AaIiДд ?]{0,40}"],
        prop::option::of(1u32..=10),
        prop::option::of(arb_rgb()),
        prop::option::of(prop::sample::select(names)),
        prop::bool::ANY,
        prop::option::of(arb_rgb()),
        prop::option::of((-12i32..=12, -12i32..=12, prop::option::of(arb_rgb()))),
    )
        .prop_map(
            |(text, scale, bg, palette, horizontal, color, shadow)| Spec {
                text,
                font: "test".into(),
                scale,
                bg,
                fill: Fill {
                    palette,
                    direction: if horizontal {
                        Direction::Horizontal
                    } else {
                        Direction::Vertical
                    },
                    color,
                },
                shadow: shadow.map(|(dx, dy, color)| Shadow { dx, dy, color }),
            },
        )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn valid_specs_render_with_the_expected_canvas(spec in arb_spec()) {
        let font = common::test_font();
        let result = render(&spec, &font);
        if spec.text.is_empty() {
            prop_assert_eq!(result.unwrap_err(), Error::EmptyText);
        } else if spec.text.contains('\n') {
            prop_assert_eq!(result.unwrap_err(), Error::MultiLine);
        } else {
            let out = result.unwrap();
            let (dx, dy) = spec.shadow.as_ref().map_or((0, 0), |s| (s.dx, s.dy));
            prop_assert_eq!(out.grid.width(), expected_width(&spec.text) + dx.unsigned_abs());
            prop_assert_eq!(out.grid.height(), 7 + dy.unsigned_abs());
            let scale = spec.scale.unwrap_or(8);
            prop_assert_eq!(out.pixel_size(), (out.grid.width() * scale, out.grid.height() * scale));

            let svg = out.to_svg();
            prop_assert!(roxmltree::Document::parse(&svg).is_ok());

            let png = out.to_png().unwrap();
            let reader = png::Decoder::new(std::io::Cursor::new(png)).read_info().unwrap();
            prop_assert_eq!(reader.info().width, out.pixel_size().0);
            prop_assert_eq!(reader.info().height, out.pixel_size().1);
        }
    }

    #[test]
    fn arbitrary_settings_never_panic(
        text in prop_oneof!["(?s).{0,40}", "[AaIiДд ?]{0,40}"],
        scale in prop::option::of(any::<u32>()),
        dx in any::<i32>(),
        dy in any::<i32>(),
        palette in prop::option::of("(?s).{0,8}"),
    ) {
        let font = common::test_font();
        let spec = Spec {
            text,
            scale,
            shadow: Some(Shadow { dx, dy, color: None }),
            fill: Fill { palette, ..Fill::default() },
            ..Spec::default()
        };
        if let Ok(out) = render(&spec, &font) {
            prop_assert_eq!(out.grid.width(), expected_width(&spec.text) + dx.unsigned_abs());
            prop_assert_eq!(out.grid.height(), 7 + dy.unsigned_abs());
        }
    }
}
