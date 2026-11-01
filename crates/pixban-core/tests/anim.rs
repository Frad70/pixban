mod common;

use pixban_core::limits::*;
use pixban_core::{
    render, render_animation, render_frame, Anim, AnimKind, Error, Grid, Outline, Plate, Rgb, Rgba,
    Shadow, Spec,
};

const KINDS: [AnimKind; 7] = AnimKind::ALL;

fn base(text: &str) -> Spec {
    Spec {
        text: text.to_string(),
        scale: Some(2),
        shadow: Some(Shadow::default()),
        ..Spec::default()
    }
}

fn animated(kind: AnimKind, text: &str) -> Spec {
    Spec {
        anim: Some(Anim::new(kind)),
        ..base(text)
    }
}

fn changed(a: &Grid, b: &Grid) -> usize {
    let mut n = 0;
    for y in 0..a.height() {
        for x in 0..a.width() {
            if a.get(x, y) != b.get(x, y) {
                n += 1;
            }
        }
    }
    n
}

fn opaque(grid: &Grid) -> usize {
    let mut n = 0;
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if !grid.get(x, y).is_transparent() {
                n += 1;
            }
        }
    }
    n
}

#[test]
fn every_animation_keeps_one_canvas_and_the_requested_frame_count() {
    let font = common::test_font();
    for kind in KINDS {
        for frames in [None, Some(5)] {
            let mut s = animated(kind, "AI A");
            s.anim.as_mut().unwrap().frames = frames;
            let a = render_animation(&s, &font).unwrap();
            if let Some(n) = frames {
                assert_eq!(a.frames().len(), n as usize, "{kind:?}");
            } else {
                assert!(a.frames().len() >= 2, "{kind:?}");
            }
            let (w, h) = (a.rest().width(), a.rest().height());
            for (i, f) in a.frames().iter().enumerate() {
                assert_eq!((f.width(), f.height()), (w, h), "{kind:?} frame {i}");
            }
            let (l, t, r, b) = a.margins();
            let still = render(&base("AI A"), &font).unwrap();
            assert_eq!(
                a.rest().cropped(l, t, r, b),
                still.grid,
                "{kind:?}: the still logo is the static render"
            );
        }
    }
}

#[test]
fn still_logo_matches_the_static_render_with_plate_and_background() {
    let font = common::test_font();
    for kind in KINDS {
        let mut s = animated(kind, "AI");
        s.plate = Some(Plate::default());
        s.outline = Some(Outline::default());
        s.bg = Some(Rgb(1, 2, 3));
        let a = render_animation(&s, &font).unwrap();
        let (l, t, r, b) = a.margins();
        let mut plain = s.clone();
        plain.anim = None;
        let still = render(&plain, &font).unwrap();
        assert_eq!(a.rest().cropped(l, t, r, b), still.grid, "{kind:?}");
    }
}

#[test]
fn frame_n_is_frame_0() {
    let font = common::test_font();
    for kind in KINDS {
        let mut s = animated(kind, "AI A");
        s.seed = 5;
        s.anim.as_mut().unwrap().frames = Some(7);
        let first = render_frame(&s, &font, 0).unwrap();
        assert_eq!(render_frame(&s, &font, 7).unwrap(), first, "{kind:?}");
        assert_eq!(render_frame(&s, &font, 14).unwrap(), first, "{kind:?}");
        let a = render_animation(&s, &font).unwrap();
        assert_eq!(a.frames()[0], first, "{kind:?}");
        for i in 0..7 {
            assert_eq!(
                a.frames()[i as usize],
                render_frame(&s, &font, i).unwrap(),
                "{kind:?} frame {i}"
            );
        }
    }
}

#[test]
fn smooth_animations_close_their_loop_without_a_jump() {
    let font = common::test_font();
    for kind in [
        AnimKind::Burn,
        AnimKind::Wave,
        AnimKind::Rainbow,
        AnimKind::Shimmer,
    ] {
        let mut s = animated(kind, "AIA");
        s.anim.as_mut().unwrap().frames = Some(24);
        let a = render_animation(&s, &font).unwrap();
        let f = a.frames();
        let steps: Vec<usize> = (0..f.len() - 1)
            .map(|i| changed(&f[i], &f[i + 1]))
            .collect();
        let wrap = changed(&f[f.len() - 1], &f[0]);
        let widest = *steps.iter().max().unwrap();
        assert!(
            wrap <= widest,
            "{kind:?}: the loop point changes {wrap} cells, ordinary steps at most {widest}"
        );
    }
}

#[test]
fn same_settings_give_the_same_bytes() {
    let font = common::test_font();
    for kind in KINDS {
        let mut s = animated(kind, "AI A");
        s.seed = 11;
        let a = render_animation(&s, &font).unwrap();
        let b = render_animation(&s.clone(), &font).unwrap();
        assert_eq!(a.to_svg(), b.to_svg(), "{kind:?}");
        assert_eq!(a.to_gif().unwrap(), b.to_gif().unwrap(), "{kind:?}");
        assert_eq!(a.to_apng().unwrap(), b.to_apng().unwrap(), "{kind:?}");
    }
}

#[test]
fn the_seed_changes_the_random_animations() {
    let font = common::test_font();
    for kind in [AnimKind::Burn, AnimKind::Glitch, AnimKind::Sparkle] {
        let mut one = animated(kind, "AI A");
        one.seed = 1;
        let mut two = one.clone();
        two.seed = 2;
        let a = render_animation(&one, &font).unwrap();
        let b = render_animation(&two, &font).unwrap();
        assert_ne!(a.frames(), b.frames(), "{kind:?}");
    }
}

#[test]
fn glitch_calm_frames_are_the_static_render() {
    let font = common::test_font();
    let mut s = animated(AnimKind::Glitch, "AI A");
    s.anim.as_mut().unwrap().frames = Some(30);
    s.anim.as_mut().unwrap().count = Some(2);
    let a = render_animation(&s, &font).unwrap();
    let calm = a.frames().iter().filter(|f| **f == *a.rest()).count();
    let burst = a.frames().len() - calm;
    assert!(burst >= 1, "no burst at all");
    assert!(
        burst <= 4,
        "{burst} burst frames for 2 bursts of at most 2 frames"
    );
    assert!(calm >= 26);
    let (l, t, r, b) = a.margins();
    let still = render(&base("AI A"), &font).unwrap();
    for f in a.frames().iter().filter(|f| **f == *a.rest()) {
        assert_eq!(f.cropped(l, t, r, b), still.grid);
    }
}

#[test]
fn glitch_makes_room_for_the_longest_shift() {
    let font = common::test_font();
    let mut s = animated(AnimKind::Glitch, "AI");
    s.anim.as_mut().unwrap().amplitude = Some(3);
    s.anim.as_mut().unwrap().count = Some(6);
    let a = render_animation(&s, &font).unwrap();
    let (l, _, r, _) = a.margins();
    assert!(l >= 3 && r >= 3, "room for the longest shift: {l} {r}");
}

#[test]
fn burn_leaves_the_lettering_alone_and_rises_from_its_top_edges() {
    let font = common::test_font();
    let mut s = animated(AnimKind::Burn, "AI");
    s.shadow = None;
    s.fill.color = Some(Rgb(250, 250, 250));
    let a = render_animation(&s, &font).unwrap();
    let rest = a.rest();
    let ink = |x: i64, y: i64| {
        x >= 0
            && y >= 0
            && x < i64::from(rest.width())
            && y < i64::from(rest.height())
            && !rest.get(x as u32, y as u32).is_transparent()
    };
    let mut flames = 0;
    for f in a.frames() {
        for y in 0..f.height() {
            for x in 0..f.width() {
                let cell = f.get(x, y);
                if !rest.get(x, y).is_transparent() {
                    assert_eq!(cell, rest.get(x, y), "lettering changed at {x},{y}");
                } else if !cell.is_transparent() {
                    flames += 1;
                    let below = (y + 1..f.height())
                        .any(|yy| (-1..=1).any(|dx| ink(i64::from(x) + dx, i64::from(yy))));
                    assert!(below, "flame at {x},{y} has no lettering under it");
                }
            }
        }
    }
    assert!(flames > 20, "only {flames} flame cells");
}

#[test]
fn burn_uses_the_palette_of_the_fill() {
    let font = common::test_font();
    let mut s = animated(AnimKind::Burn, "AI");
    s.fill.palette = Some("ice".into());
    let a = render_animation(&s, &font).unwrap();
    let rest = a.rest();
    for f in a.frames() {
        for y in 0..f.height() {
            for x in 0..f.width() {
                let c = f.get(x, y);
                if rest.get(x, y).is_transparent() && !c.is_transparent() {
                    assert!(c.2 >= c.0, "flame cell {c:?} is not blue-ish");
                }
            }
        }
    }
}

#[test]
fn wave_moves_whole_letters_by_whole_cells() {
    let font = common::block_font();
    let mut s = animated(AnimKind::Wave, "bbb");
    s.anim.as_mut().unwrap().amplitude = Some(2);
    s.anim.as_mut().unwrap().frames = Some(12);
    s.shadow = None;
    s.spacing = Some(1);
    let a = render_animation(&s, &font).unwrap();
    let (_, top, _, bottom) = a.margins();
    assert_eq!((top, bottom), (2, 2));
    let rest = a.rest();
    let mut seen = std::collections::BTreeSet::new();
    for f in a.frames() {
        for letter in 0..3u32 {
            let x0 = letter * 4;
            let rows: Vec<u32> = (0..f.height())
                .filter(|&y| (x0..x0 + 3).all(|x| !f.get(x, y).is_transparent()))
                .collect();
            assert_eq!(rows.len(), 3, "a letter keeps its height");
            assert_eq!(rows[2] - rows[0], 2, "and stays in one piece");
            let rest_top = (0..rest.height())
                .find(|&y| !rest.get(x0, y).is_transparent())
                .unwrap();
            let dy = i64::from(rows[0]) - i64::from(rest_top);
            assert!(dy.abs() <= 2);
            seen.insert(dy);
        }
    }
    assert!(seen.len() >= 4, "letters only visited {seen:?}");
}

#[test]
fn shimmer_lights_only_the_lettering_inside_one_diagonal_band() {
    let font = common::test_font();
    let mut s = animated(AnimKind::Shimmer, "AI A");
    s.anim.as_mut().unwrap().band = Some(3);
    let a = render_animation(&s, &font).unwrap();
    let rest = a.rest();
    let mut lit_frames = 0;
    for f in a.frames() {
        let mut sums = Vec::new();
        for y in 0..f.height() {
            for x in 0..f.width() {
                if f.get(x, y) != rest.get(x, y) {
                    let (was, now) = (rest.get(x, y), f.get(x, y));
                    assert!(!was.is_transparent());
                    assert!(now.0 >= was.0 && now.1 >= was.1 && now.2 >= was.2);
                    sums.push(x + y);
                }
            }
        }
        if !sums.is_empty() {
            lit_frames += 1;
            let (lo, hi) = (sums.iter().min().unwrap(), sums.iter().max().unwrap());
            assert!(hi - lo < 3, "band wider than 3 cells: {lo}..{hi}");
        }
    }
    assert!(lit_frames >= 5, "{lit_frames}");
    assert!(
        a.frames().iter().any(|f| f == rest),
        "a pause without any band"
    );
}

#[test]
fn type_reveals_letters_one_by_one_then_clears() {
    let font = common::test_font();
    let s = animated(AnimKind::Type, "AIA");
    let a = render_animation(&s, &font).unwrap();
    let (_, _, right, _) = a.margins();
    assert_eq!(right, 3);
    let rest = a.rest();
    let ink_cells = |f: &Grid| {
        let mut n = 0;
        for y in 0..f.height() {
            for x in 0..f.width() {
                if f.get(x, y) == rest.get(x, y) && !f.get(x, y).is_transparent() {
                    n += 1;
                }
            }
        }
        n
    };
    let counts: Vec<usize> = a.frames().iter().map(ink_cells).collect();
    let rest_cells = opaque(rest);
    assert!(counts.contains(&rest_cells), "the full text holds");
    let peak = counts.iter().position(|&n| n == rest_cells).unwrap();
    assert!(
        counts[..=peak].windows(2).all(|w| w[0] <= w[1]),
        "letters only get added while typing: {counts:?}"
    );
    let distinct: std::collections::BTreeSet<_> = counts[..=peak].iter().collect();
    assert!(distinct.len() >= 4, "{counts:?}");
    let cursor = 2 * 7;
    assert_eq!(
        opaque(&a.frames()[0]),
        cursor,
        "starts with the cursor alone"
    );
    assert!(opaque(a.frames().last().unwrap()) <= cursor, "ends cleared");
}

#[test]
fn rainbow_turns_through_all_hues_and_back() {
    let font = common::test_font();
    let mut s = animated(AnimKind::Rainbow, "A");
    s.shadow = None;
    s.fill.color = Some(Rgb(200, 40, 40));
    s.anim.as_mut().unwrap().hues = Some(6);
    s.anim.as_mut().unwrap().frames = Some(6);
    let a = render_animation(&s, &font).unwrap();
    let colors: Vec<Rgba> = a
        .frames()
        .iter()
        .map(|f| {
            (0..f.height())
                .flat_map(|y| (0..f.width()).map(move |x| (x, y)))
                .map(|(x, y)| f.get(x, y))
                .find(|c| !c.is_transparent())
                .unwrap()
        })
        .collect();
    let mut unique = colors.clone();
    unique.sort_by_key(|c| (c.0, c.1, c.2));
    unique.dedup();
    assert_eq!(unique.len(), 6, "{colors:?}");
}

#[test]
fn sparkle_adds_a_few_cells_next_to_the_lettering() {
    let font = common::test_font();
    let mut s = animated(AnimKind::Sparkle, "AI A");
    s.anim.as_mut().unwrap().count = Some(4);
    let a = render_animation(&s, &font).unwrap();
    let rest = a.rest();
    let mut lit = 0;
    for f in a.frames() {
        let extra = changed(f, rest);
        assert!(extra <= 4 * 5, "{extra} cells for 4 stars");
        if extra > 0 {
            lit += 1;
        }
    }
    assert!(lit >= 6, "stars on only {lit} frames");
}

#[test]
fn settings_outside_the_limits_are_clear_errors() {
    let font = common::test_font();
    let run = |edit: &dyn Fn(&mut Anim)| {
        let mut s = animated(AnimKind::Burn, "AI");
        edit(s.anim.as_mut().unwrap());
        render_animation(&s, &font).unwrap_err().to_string()
    };
    assert!(run(&|a| a.frames = Some(1)).contains("frames 1 is out of range, use 2 to 120"));
    assert!(run(&|a| a.frames = Some(121)).contains("frames 121"));
    assert!(run(&|a| a.frame_ms = Some(19))
        .contains("frame time in ms 19 is out of range, use 20 to 2000"));
    assert!(run(&|a| a.frame_ms = Some(2001)).contains("2001"));
    assert!(run(&|a| a.flame = Some(0)).contains("flame height"));
    assert!(run(&|a| a.flame = Some(MAX_FLAME + 1)).contains("flame height"));
    let wrong = run(&|a| a.band = Some(3));
    assert!(
        wrong.contains("band only applies to shimmer, not to burn"),
        "{wrong}"
    );
    let mut s = animated(AnimKind::Rainbow, "AI");
    s.anim.as_mut().unwrap().hues = Some(2);
    assert!(render_animation(&s, &font).is_err());
    assert!(matches!(
        render_animation(&base("AI"), &font),
        Err(Error::InvalidSetting(_))
    ));
}

#[test]
fn animations_that_would_not_fit_in_memory_are_refused() {
    let font = common::test_font();
    let mut s = animated(AnimKind::Burn, &"A".repeat(300));
    s.anim.as_mut().unwrap().frames = Some(MAX_FRAMES);
    s.scale = Some(1);
    let err = render_animation(&s, &font).unwrap_err();
    assert!(matches!(err, Error::AnimationTooLarge { .. }), "{err}");
    assert!(err.to_string().contains("limits"));
    let mut s = animated(AnimKind::Wave, &"A".repeat(30));
    s.anim.as_mut().unwrap().frames = Some(MAX_FRAMES);
    s.scale = Some(MAX_SCALE);
    let err = render_animation(&s, &font).unwrap_err();
    assert!(matches!(err, Error::AnimationTooLarge { .. }), "{err}");
}

#[test]
fn static_settings_are_untouched_by_the_animation_table() {
    let font = common::test_font();
    let mut s = animated(AnimKind::Wave, "AI");
    let with = render(&s, &font).unwrap().to_svg();
    s.anim = None;
    assert_eq!(render(&s, &font).unwrap().to_svg(), with);
}

#[test]
fn fire_is_cut_into_palette_steps() {
    let font = common::test_font();
    let s = animated(AnimKind::Burn, "AI A");
    let a = render_animation(&s, &font).unwrap();
    let mut colors = std::collections::BTreeSet::new();
    let rest = a.rest();
    for f in a.frames() {
        for y in 0..f.height() {
            for x in 0..f.width() {
                if rest.get(x, y).is_transparent() && !f.get(x, y).is_transparent() {
                    let c = f.get(x, y);
                    colors.insert((c.0, c.1, c.2));
                }
            }
        }
    }
    assert!(
        (4..=20).contains(&colors.len()),
        "{} flame colors",
        colors.len()
    );
}
