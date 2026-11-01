mod common;

use std::collections::BTreeSet;
use std::io::Cursor;

use pixban_core::{
    render, render_animation, Anim, AnimKind, Animated, Fill, Grid, Outline, Plate, Rgb, Shadow,
    Spec, Texture, TextureKind,
};

fn spec(kind: AnimKind, text: &str) -> Spec {
    Spec {
        text: text.to_string(),
        scale: Some(3),
        shadow: Some(Shadow::default()),
        anim: Some(Anim::new(kind)),
        ..Spec::default()
    }
}

fn pixel(grid: &Grid, scale: u32, px: u32, py: u32) -> [u8; 4] {
    let c = grid.get(px / scale, py / scale);
    [c.0, c.1, c.2, c.3]
}

struct GifFrame {
    delay_ms: u32,
    pixels: Vec<[u8; 4]>,
}

/// Decodes with the `gif` crate and replays the frames onto a canvas the way a viewer does.
fn decode_gif(bytes: &[u8]) -> (u32, u32, gif::Repeat, Vec<GifFrame>) {
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = options.read_info(Cursor::new(bytes)).unwrap();
    let (w, h) = (u32::from(decoder.width()), u32::from(decoder.height()));
    let repeat = decoder.repeat();
    let mut canvas = vec![[0u8; 4]; (w * h) as usize];
    let mut out = Vec::new();
    while let Some(frame) = decoder.read_next_frame().unwrap() {
        let (fx, fy) = (u32::from(frame.left), u32::from(frame.top));
        let (fw, fh) = (u32::from(frame.width), u32::from(frame.height));
        for y in 0..fh {
            for x in 0..fw {
                let i = ((y * fw + x) * 4) as usize;
                let p = [
                    frame.buffer[i],
                    frame.buffer[i + 1],
                    frame.buffer[i + 2],
                    frame.buffer[i + 3],
                ];
                if p[3] != 0 {
                    canvas[((fy + y) * w + fx + x) as usize] = p;
                }
            }
        }
        out.push(GifFrame {
            delay_ms: u32::from(frame.delay) * 10,
            pixels: canvas.clone(),
        });
        if frame.dispose == gif::DisposalMethod::Background {
            for y in 0..fh {
                for x in 0..fw {
                    canvas[((fy + y) * w + fx + x) as usize] = [0; 4];
                }
            }
        }
    }
    (w, h, repeat, out)
}

fn colors_of(a: &Animated) -> BTreeSet<[u8; 3]> {
    let mut set = BTreeSet::new();
    for f in a.frames() {
        for y in 0..f.height() {
            for x in 0..f.width() {
                let c = f.get(x, y);
                if !c.is_transparent() {
                    set.insert([c.0, c.1, c.2]);
                }
            }
        }
    }
    set
}

#[test]
fn gif_round_trip_is_exact_when_the_colors_fit() {
    let font = common::test_font();
    for kind in AnimKind::ALL {
        for bg in [None, Some(Rgb(13, 17, 23))] {
            let mut s = spec(kind, "AI A");
            s.bg = bg;
            s.anim.as_mut().unwrap().frame_ms = Some(60);
            let a = render_animation(&s, &font).unwrap();
            assert!(colors_of(&a).len() <= 255, "{kind:?}");
            let (w, h, repeat, frames) = decode_gif(&a.to_gif().unwrap());
            assert_eq!((w, h), a.pixel_size(), "{kind:?}");
            assert_eq!(repeat, gif::Repeat::Infinite);
            assert_eq!(frames.len(), a.frames().len(), "{kind:?}");
            for (i, (got, want)) in frames.iter().zip(a.frames()).enumerate() {
                assert_eq!(got.delay_ms, 60);
                for py in 0..h {
                    for px in 0..w {
                        let expected = pixel(want, 3, px, py);
                        let seen = got.pixels[(py * w + px) as usize];
                        assert_eq!(
                            seen, expected,
                            "{kind:?} bg {bg:?} frame {i} pixel {px},{py}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn gif_delays_are_whole_tens_of_milliseconds_and_never_below_20() {
    let font = common::test_font();
    for (asked, stored) in [
        (20, 20),
        (24, 20),
        (25, 30),
        (75, 80),
        (80, 80),
        (2000, 2000),
    ] {
        let mut s = spec(AnimKind::Wave, "AI");
        s.anim.as_mut().unwrap().frame_ms = Some(asked);
        let a = render_animation(&s, &font).unwrap();
        assert_eq!(a.gif_frame_ms(), stored, "asked {asked}");
        let (_, _, _, frames) = decode_gif(&a.to_gif().unwrap());
        assert!(frames.iter().all(|f| f.delay_ms == stored), "asked {asked}");
        assert_eq!(a.frame_ms(), asked);
    }
}

#[test]
fn gif_with_more_than_256_colors_uses_a_palette_and_keeps_transparency() {
    let font = common::test_font();
    let mut s = spec(AnimKind::Rainbow, "AI A");
    s.fill = Fill {
        palette: Some("rainbow".into()),
        per_letter: false,
        ..Fill::default()
    };
    s.texture = Some(Texture {
        kind: TextureKind::Noise,
        strength: 20,
        period: 2,
    });
    s.bevel = Some(pixban_core::Bevel { amount: 30 });
    s.anim.as_mut().unwrap().hues = Some(36);
    let a = render_animation(&s, &font).unwrap();
    let wanted = colors_of(&a);
    assert!(wanted.len() > 255, "only {} colors", wanted.len());
    let bytes = a.to_gif().unwrap();
    assert_eq!(bytes, a.to_gif().unwrap(), "the quantizer is deterministic");
    let (w, h, _, frames) = decode_gif(&bytes);
    assert_eq!(frames.len(), 36);
    let mut seen = BTreeSet::new();
    for (got, want) in frames.iter().zip(a.frames()) {
        for py in 0..h {
            for px in 0..w {
                let expected = pixel(want, 3, px, py);
                let p = got.pixels[(py * w + px) as usize];
                assert_eq!(p[3] == 0, expected[3] == 0, "transparency is exact");
                if p[3] != 0 {
                    seen.insert([p[0], p[1], p[2]]);
                    let d: i32 = (0..3)
                        .map(|k| (i32::from(p[k]) - i32::from(expected[k])).pow(2))
                        .sum();
                    assert!(d < 70 * 70, "{p:?} for {expected:?}");
                }
            }
        }
    }
    assert!(seen.len() <= 255, "{} colors in the file", seen.len());
}

#[test]
fn apng_round_trip_keeps_frames_delays_and_alpha() {
    let font = common::test_font();
    for kind in AnimKind::ALL {
        for bg in [None, Some(Rgb(1, 2, 3))] {
            let mut s = spec(kind, "AI A");
            s.bg = bg;
            s.anim.as_mut().unwrap().frame_ms = Some(75);
            let a = render_animation(&s, &font).unwrap();
            let bytes = a.to_apng().unwrap();
            let mut reader = png::Decoder::new(Cursor::new(&bytes)).read_info().unwrap();
            let actl = reader.info().animation_control.unwrap();
            assert_eq!(actl.num_frames as usize, a.frames().len(), "{kind:?}");
            assert_eq!(actl.num_plays, 0, "loops forever");
            let (w, h) = a.pixel_size();
            let mut canvas = vec![[0u8; 4]; (w * h) as usize];
            for (i, want) in a.frames().iter().enumerate() {
                let mut buf = vec![0; reader.output_buffer_size().unwrap()];
                let info = reader.next_frame(&mut buf).unwrap();
                let fc = reader.info().frame_control.unwrap();
                assert_eq!((fc.delay_num, fc.delay_den), (75, 1000));
                assert_eq!(fc.blend_op, png::BlendOp::Source);
                if i == 0 {
                    assert_eq!((fc.x_offset, fc.y_offset), (0, 0));
                    assert_eq!(
                        (info.width, info.height),
                        (w, h),
                        "the default image is frame 0"
                    );
                }
                for y in 0..fc.height {
                    for x in 0..fc.width {
                        let j = ((y * fc.width + x) * 4) as usize;
                        canvas[((fc.y_offset + y) * w + fc.x_offset + x) as usize] =
                            [buf[j], buf[j + 1], buf[j + 2], buf[j + 3]];
                    }
                }
                for py in 0..h {
                    for px in 0..w {
                        assert_eq!(
                            canvas[(py * w + px) as usize],
                            pixel(want, 3, px, py),
                            "{kind:?} bg {bg:?} frame {i} pixel {px},{py}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn apng_default_image_is_frame_0_for_plain_viewers() {
    let font = common::test_font();
    let a = render_animation(&spec(AnimKind::Wave, "AI"), &font).unwrap();
    let bytes = a.to_apng().unwrap();
    let mut reader = png::Decoder::new(Cursor::new(&bytes)).read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!((info.width, info.height), a.pixel_size());
    let still = a.frame_png(0).unwrap();
    let mut other = png::Decoder::new(Cursor::new(&still)).read_info().unwrap();
    let mut buf2 = vec![0; other.output_buffer_size().unwrap()];
    other.next_frame(&mut buf2).unwrap();
    assert_eq!(buf, buf2);
}

// Paints the rectangles of an SVG the way a viewer shows it with motion switched off.
fn rects(d: &str) -> Vec<(i64, i64, i64, i64)> {
    let mut out = Vec::new();
    let mut tokens: Vec<(char, Vec<i64>)> = Vec::new();
    let mut cmd = ' ';
    let mut num = String::new();
    let mut nums: Vec<i64> = Vec::new();
    let flush = |num: &mut String, nums: &mut Vec<i64>| {
        if !num.is_empty() {
            nums.push(num.parse().unwrap());
            num.clear();
        }
    };
    for c in d.chars() {
        if c.is_ascii_alphabetic() {
            flush(&mut num, &mut nums);
            if cmd != ' ' {
                tokens.push((cmd, std::mem::take(&mut nums)));
            }
            cmd = c;
        } else if c == ' ' {
            flush(&mut num, &mut nums);
        } else {
            num.push(c);
        }
    }
    flush(&mut num, &mut nums);
    tokens.push((cmd, nums));
    let (mut x, mut y, mut w, mut h) = (0, 0, 0, 0);
    for (c, n) in tokens {
        match c {
            'M' => {
                x = n[0];
                y = n[1];
            }
            'h' if w == 0 => w = n[0],
            'v' => h = n[0],
            'z' => {
                out.push((x, y, w, h));
                w = 0;
            }
            _ => {}
        }
    }
    out
}

fn static_state(svg: &str, w: u32, h: u32) -> Vec<Option<[u8; 3]>> {
    let doc = roxmltree::Document::parse(svg).unwrap();
    let mut grid = vec![None; (w * h) as usize];
    for node in doc.descendants().filter(|n| n.has_tag_name("path")) {
        if node.ancestors().any(|a| a.has_tag_name("clipPath")) {
            continue;
        }
        let off = node.ancestors().any(|a| {
            a.attribute("class")
                .is_some_and(|c| c.split(' ').any(|t| t == "h" || t == "b"))
        });
        if off {
            continue;
        }
        let fill: Rgb = node.attribute("fill").unwrap().parse().unwrap();
        for (x, y, rw, rh) in rects(node.attribute("d").unwrap()) {
            for yy in y..y + rh {
                for xx in x..x + rw {
                    grid[(yy as u32 * w + xx as u32) as usize] = Some([fill.0, fill.1, fill.2]);
                }
            }
        }
    }
    grid
}

#[test]
fn animated_svg_is_clean_and_its_still_state_is_the_static_logo() {
    let font = common::test_font();
    for kind in AnimKind::ALL {
        for bg in [None, Some(Rgb(13, 17, 23))] {
            let mut s = spec(kind, "AI A");
            s.bg = bg;
            s.plate = (bg.is_some()).then(Plate::default);
            s.outline = Some(Outline::default());
            let a = render_animation(&s, &font).unwrap();
            let svg = a.to_svg();
            let doc = roxmltree::Document::parse(&svg).unwrap_or_else(|e| panic!("{kind:?}: {e}"));
            let root = doc.root_element();
            assert_eq!(root.attribute("shape-rendering"), Some("crispEdges"));
            for n in doc.descendants().filter(|n| n.is_element()) {
                let name = n.tag_name().name();
                assert!(
                    ["svg", "title", "style", "path", "g", "clipPath"].contains(&name),
                    "{kind:?}: unexpected <{name}>"
                );
                for attr in n.attributes() {
                    assert!(!attr.name().starts_with("on"), "{kind:?} handler");
                    assert!(
                        !attr.value().contains("http"),
                        "{kind:?} link in {}",
                        attr.name()
                    );
                }
            }
            assert!(!svg.contains("<script") && !svg.contains("<image") && !svg.contains("href"));
            assert!(!svg.contains("<!--") && !svg.contains("<animate") && !svg.contains("<set"));
            let css: String = doc
                .descendants()
                .filter(|n| n.has_tag_name("style"))
                .filter_map(|n| n.text())
                .collect();
            assert!(
                css.contains("@media (prefers-reduced-motion:reduce)")
                    && css.contains("animation:none"),
                "{kind:?}: {css}"
            );
            assert!(css.contains("@keyframes"));
            assert!(css.contains("infinite"));
            let (w, h) = (a.rest().width(), a.rest().height());
            let still = static_state(&svg, w, h);
            for y in 0..h {
                for x in 0..w {
                    let want = a.rest().get(x, y);
                    let want = (!want.is_transparent()).then_some([want.0, want.1, want.2]);
                    assert_eq!(
                        still[(y * w + x) as usize],
                        want,
                        "{kind:?} bg {bg:?} cell {x},{y}"
                    );
                }
            }
            let (l, t, r, b) = a.margins();
            let mut plain = s.clone();
            plain.anim = None;
            assert_eq!(
                a.rest().cropped(l, t, r, b),
                render(&plain, &font).unwrap().grid
            );
        }
    }
}

#[test]
fn animated_svg_has_the_total_duration_and_one_keyframe_per_change() {
    let font = common::test_font();
    let mut s = spec(AnimKind::Wave, "AI");
    s.anim.as_mut().unwrap().frames = Some(8);
    s.anim.as_mut().unwrap().frame_ms = Some(125);
    let svg = render_animation(&s, &font).unwrap().to_svg();
    assert!(svg.contains("animation-duration:1000ms"), "{svg}");
    assert!(svg.contains("step-end"));
}

#[test]
fn settings_round_trip_through_toml_into_the_same_files() {
    let font = common::test_font();
    for kind in AnimKind::ALL {
        let mut anim = Anim::new(kind);
        anim.frames = Some(6);
        anim.frame_ms = Some(70);
        match kind {
            AnimKind::Burn => anim.flame = Some(5),
            AnimKind::Wave => anim.amplitude = Some(2),
            AnimKind::Shimmer => anim.band = Some(4),
            AnimKind::Glitch => {
                anim.amplitude = Some(3);
                anim.count = Some(2);
            }
            AnimKind::Sparkle => anim.count = Some(3),
            AnimKind::Rainbow => {
                anim.hues = Some(6);
                anim.frames = Some(8);
            }
            AnimKind::Type => {}
        }
        let mut s = spec(kind, "AI A");
        s.seed = 99;
        s.bg = Some(Rgb(4, 5, 6));
        s.anim = Some(anim);
        let text = toml::to_string(&s).unwrap();
        let back: Spec = toml::from_str(&text).unwrap();
        assert_eq!(back, s, "{kind:?}\n{text}");
        let (a, b) = (
            render_animation(&s, &font).unwrap(),
            render_animation(&back, &font).unwrap(),
        );
        assert_eq!(a.to_svg(), b.to_svg(), "{kind:?}");
        assert_eq!(a.to_gif().unwrap(), b.to_gif().unwrap(), "{kind:?}");
        assert_eq!(a.to_apng().unwrap(), b.to_apng().unwrap(), "{kind:?}");
    }
}

#[test]
fn unknown_animation_settings_are_rejected() {
    assert!(toml::from_str::<Spec>("text = \"a\"\n[anim]\nkind = \"burn\"\nspeed = 3\n").is_err());
    assert!(toml::from_str::<Spec>("text = \"a\"\n[anim]\nkind = \"melt\"\n").is_err());
    assert!(toml::from_str::<Spec>("text = \"a\"\n[anim]\nframes = 4\n").is_err());
    let ok: Spec = toml::from_str("text = \"a\"\n[anim]\nkind = \"type\"\n").unwrap();
    assert_eq!(ok.anim, Some(Anim::new(AnimKind::Type)));
}
