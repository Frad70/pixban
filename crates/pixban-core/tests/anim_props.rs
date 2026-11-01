mod common;

use std::io::Cursor;

use pixban_core::{
    render, render_animation, render_frame, Anim, AnimKind, Outline, Plate, Rgb, Shadow, Spec,
};
use proptest::prelude::*;

fn arb_anim() -> impl Strategy<Value = Anim> {
    (
        prop::sample::select(AnimKind::ALL.to_vec()),
        prop::option::of(2u32..=6),
        prop::option::of(20u32..=200),
        1u32..=6,
        1u32..=4,
        1u32..=5,
        1u32..=6,
        3u32..=8,
    )
        .prop_map(
            |(kind, frames, frame_ms, flame, amplitude, band, count, hues)| {
                let mut a = Anim::new(kind);
                a.frames = frames;
                a.frame_ms = frame_ms;
                match kind {
                    AnimKind::Burn => a.flame = Some(flame),
                    AnimKind::Wave => a.amplitude = Some(amplitude),
                    AnimKind::Shimmer => a.band = Some(band),
                    AnimKind::Glitch => {
                        a.amplitude = Some(amplitude);
                        a.count = Some(count);
                    }
                    AnimKind::Sparkle => a.count = Some(count),
                    AnimKind::Rainbow => a.hues = Some(hues),
                    AnimKind::Type => {}
                }
                a
            },
        )
}

fn arb_spec() -> impl Strategy<Value = Spec> {
    (
        "[AaIiДд ?]{1,6}",
        1u32..=3,
        any::<u32>(),
        prop::option::of(any::<[u8; 3]>()),
        prop::option::of(Just(Shadow::default())),
        prop::option::of(Just(Outline::default())),
        prop::option::of(Just(Plate::default())),
        arb_anim(),
    )
        .prop_map(
            |(text, scale, seed, bg, shadow, outline, plate, anim)| Spec {
                text,
                font: "test".into(),
                scale: Some(scale),
                seed,
                bg: bg.map(|[r, g, b]| Rgb(r, g, b)),
                shadow,
                outline,
                plate,
                anim: Some(anim),
                ..Spec::default()
            },
        )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(80))]

    #[test]
    fn random_animations_render_and_encode(spec in arb_spec()) {
        let font = common::test_font();
        let a = render_animation(&spec, &font).unwrap();
        let n = a.frames().len();
        let wanted = spec.anim.as_ref().unwrap().frames;
        if let Some(w) = wanted {
            prop_assert_eq!(n, w as usize);
        }
        prop_assert!((2..=120).contains(&n));
        let (w, h) = (a.rest().width(), a.rest().height());
        for f in a.frames() {
            prop_assert_eq!((f.width(), f.height()), (w, h));
        }
        prop_assert_eq!(&render_frame(&spec, &font, n as u32).unwrap(), &a.frames()[0]);

        let (l, t, r, b) = a.margins();
        let mut plain = spec.clone();
        plain.anim = None;
        prop_assert_eq!(a.rest().cropped(l, t, r, b), render(&plain, &font).unwrap().grid);

        let gif = a.to_gif().unwrap();
        let mut decoder = gif::DecodeOptions::new().read_info(Cursor::new(&gif)).unwrap();
        let mut seen = 0;
        while decoder.read_next_frame().unwrap().is_some() {
            seen += 1;
        }
        prop_assert_eq!(seen, n);

        let apng = a.to_apng().unwrap();
        let reader = png::Decoder::new(Cursor::new(&apng)).read_info().unwrap();
        prop_assert_eq!(reader.info().animation_control.unwrap().num_frames as usize, n);

        let svg = a.to_svg();
        prop_assert!(roxmltree::Document::parse(&svg).is_ok());
        prop_assert!(!svg.contains("<script"));
        prop_assert_eq!(&a.to_svg(), &svg);
    }
}
