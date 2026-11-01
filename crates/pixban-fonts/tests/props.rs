use proptest::prelude::*;

use pixban_core::{render, CaseMode, Font, Spec};

fn expected_canvas(font: &Font, text: &str) -> (u32, u32) {
    let chars: Vec<char> = match font.case() {
        CaseMode::Upper => text.chars().flat_map(char::to_uppercase).collect(),
        CaseMode::Mixed => text.chars().collect(),
    };
    let mut pen: i64 = 0;
    let mut prev: Option<char> = None;
    let (mut min_x, mut max_x) = (i64::MAX, i64::MIN);
    let (mut top, mut bottom) = (0u32, 0u32);
    for c in chars {
        let g = font.glyph(c).unwrap_or(font.fallback());
        if let Some(p) = prev {
            pen += i64::from(font.kerning(p, c));
        }
        let x = pen + i64::from(g.left());
        min_x = min_x.min(x);
        max_x = max_x.max(x + i64::from(g.width()));
        top = top.max(g.top());
        bottom = bottom.max(g.bottom());
        pen += i64::from(g.advance()) + i64::from(font.spacing());
        prev = Some(c);
    }
    ((max_x - min_x) as u32, font.height() + top + bottom)
}

fn fonts() -> Vec<Font> {
    pixban_fonts::names()
        .map(|n| pixban_fonts::get(n).expect("listed font exists"))
        .collect()
}

fn text_from_font(font: &Font) -> BoxedStrategy<String> {
    let chars = font.chars();
    let one = prop_oneof![
        prop::sample::select(chars),
        any::<char>().prop_map(|c| if c == '\n' { '?' } else { c }),
    ];
    prop::collection::vec(one, 1..40)
        .prop_map(|v| v.into_iter().collect::<String>())
        .boxed()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn random_text_from_every_font_renders_to_the_expected_canvas(
        idx in 0usize..4,
        scale in 1u32..=4,
    ) {
        let all = fonts();
        let font = &all[idx];
        let strategy = text_from_font(font);
        let mut runner = proptest::test_runner::TestRunner::new(ProptestConfig::with_cases(64));
        runner
            .run(&strategy, |text| {
                let spec = Spec {
                    text: text.clone(),
                    scale: Some(scale),
                    ..Spec::default()
                };
                let out = render(&spec, font).expect("random text renders");
                let (w, h) = expected_canvas(font, &text);
                prop_assert_eq!(out.grid.width(), w);
                prop_assert_eq!(out.grid.height(), h);
                prop_assert_eq!(out.pixel_size(), (w * scale, h * scale));
                Ok(())
            })
            .unwrap();
    }
}
