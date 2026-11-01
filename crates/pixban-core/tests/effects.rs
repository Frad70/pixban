mod common;

use common::{art, block_font, dot_font, tall_font, test_font};
use pixban_core::limits::*;
use pixban_core::{
    render, Align, Bevel, Connectivity, Dither, Error, Extrude, Fill, Outline, Plate, Rendered,
    Rgb, Shadow, Spec, Texture, TextureKind,
};

const RED: Rgb = Rgb(255, 0, 0);
const BLUE: Rgb = Rgb(0, 0, 255);

fn spec(text: &str) -> Spec {
    Spec {
        text: text.to_string(),
        fill: Fill {
            color: Some(RED),
            ..Fill::default()
        },
        ..Spec::default()
    }
}

fn outline(width: u32, connectivity: Connectivity) -> Option<Outline> {
    Some(Outline {
        width,
        connectivity,
        color: Some(BLUE),
    })
}

fn extrude(length: u32, dx: i32, dy: i32) -> Option<Extrude> {
    Some(Extrude {
        length,
        dx,
        dy,
        ..Extrude::default()
    })
}

fn plain_shadow(dx: i32, dy: i32) -> Option<Shadow> {
    Some(Shadow {
        dx,
        dy,
        color: None,
    })
}

fn size(out: &Rendered) -> (u32, u32) {
    (out.grid.width(), out.grid.height())
}

fn ink(text: &str) -> usize {
    let font = test_font();
    text.chars()
        .filter(|c| *c != '\n')
        .map(|c| {
            let g = font.glyph(c).unwrap_or(font.fallback());
            (0..g.height())
                .flat_map(|y| (0..g.width()).map(move |x| (x, y)))
                .filter(|&(x, y)| g.get(x, y))
                .count()
        })
        .sum()
}

fn count_color(out: &Rendered, c: Rgb) -> usize {
    (0..out.grid.height())
        .flat_map(|y| (0..out.grid.width()).map(move |x| (x, y)))
        .filter(|&(x, y)| out.grid.get(x, y).rgb() == c && !out.grid.get(x, y).is_transparent())
        .count()
}

#[test]
fn outline_four_connected_is_a_diamond_ring() {
    let mut s = spec("a");
    s.outline = outline(1, Connectivity::Four);
    let out = render(&s, &dot_font()).unwrap();
    assert_eq!(art(&out, &[(RED, '#'), (BLUE, 'o')]), [".o.", "o#o", ".o."]);
    s.outline = outline(2, Connectivity::Four);
    let out = render(&s, &dot_font()).unwrap();
    assert_eq!(
        art(&out, &[(RED, '#'), (BLUE, 'o')]),
        ["..o..", ".ooo.", "oo#oo", ".ooo.", "..o.."]
    );
}

#[test]
fn outline_eight_connected_is_a_square_ring() {
    let mut s = spec("a");
    s.outline = outline(1, Connectivity::Eight);
    let out = render(&s, &dot_font()).unwrap();
    assert_eq!(art(&out, &[(RED, '#'), (BLUE, 'o')]), ["ooo", "o#o", "ooo"]);
    s.outline = outline(2, Connectivity::Eight);
    let out = render(&s, &dot_font()).unwrap();
    assert_eq!(
        art(&out, &[(RED, '#'), (BLUE, 'o')]),
        ["ooooo", "ooooo", "oo#oo", "ooooo", "ooooo"]
    );
}

#[test]
fn outline_width_four_fits_and_default_color_is_the_palette_shadow() {
    let mut s = spec("a");
    s.fill = Fill::default();
    s.outline = Some(Outline {
        width: 4,
        ..Outline::default()
    });
    let out = render(&s, &dot_font()).unwrap();
    assert_eq!(size(&out), (9, 9));
    assert_eq!(out.grid.get(0, 0).rgb(), Rgb::hex(0x470f07));
}

#[test]
fn extrusion_steps_get_darker_by_a_fixed_amount() {
    let mut s = spec("b");
    s.fill.color = Some(Rgb(200, 200, 200));
    s.extrude = Some(Extrude {
        length: 3,
        dx: 1,
        dy: 0,
        step: 25,
        color: Some(Rgb(0, 0, 0)),
    });
    let out = render(&s, &dot_font()).unwrap();
    assert_eq!(size(&out), (5, 1));
    let row: Vec<Rgb> = (0..5).map(|x| out.grid.get(x, 0).rgb()).collect();
    assert_eq!(
        row,
        [
            Rgb(200, 200, 200),
            Rgb(200, 200, 200),
            Rgb(150, 150, 150),
            Rgb(100, 100, 100),
            Rgb(50, 50, 50)
        ]
    );
}

#[test]
fn extrusion_starts_from_the_darkest_fill_color() {
    let mut s = spec("a");
    s.fill = Fill {
        palette: Some("#ffffff,#808080".into()),
        ..Fill::default()
    };
    s.extrude = Some(Extrude {
        length: 1,
        dx: 1,
        dy: 0,
        step: 50,
        color: Some(Rgb(0, 0, 0)),
    });
    let out = render(&s, &dot_font()).unwrap();
    assert_eq!(out.grid.get(1, 0).rgb(), Rgb(64, 64, 64));
}

#[test]
fn extrusion_toward_the_top_left_grows_the_canvas_on_that_side() {
    let mut s = spec("a");
    s.extrude = Some(Extrude {
        length: 2,
        dx: -1,
        dy: -1,
        step: 50,
        color: Some(Rgb(0, 0, 0)),
    });
    let out = render(&s, &dot_font()).unwrap();
    assert_eq!(size(&out), (3, 3));
    assert_eq!(out.grid.get(2, 2).rgb(), RED);
    assert_eq!(out.grid.get(1, 1).rgb(), Rgb(128, 0, 0));
    assert_eq!(out.grid.get(0, 0).rgb(), Rgb(0, 0, 0));
    assert!(out.grid.get(2, 0).is_transparent());
}

#[test]
fn extrusion_covers_the_outline_and_stays_below_it() {
    let mut s = spec("a");
    s.outline = outline(1, Connectivity::Four);
    s.extrude = Some(Extrude {
        length: 1,
        dx: 1,
        dy: 0,
        step: 100,
        color: Some(Rgb(9, 9, 9)),
    });
    let out = render(&s, &dot_font()).unwrap();
    assert_eq!(
        art(&out, &[(RED, '#'), (BLUE, 'o'), (Rgb(9, 9, 9), 'e')]),
        [".oe.", "o#oe", ".oe."]
    );
}

#[test]
fn bevel_lights_the_top_left_and_shades_the_bottom_right() {
    let mut s = spec("b");
    s.fill.color = Some(Rgb(100, 100, 100));
    s.bevel = Some(Bevel { amount: 50 });
    let out = render(&s, &block_font()).unwrap();
    let (l, n, d) = (Rgb(178, 178, 178), Rgb(100, 100, 100), Rgb(50, 50, 50));
    assert_eq!(
        art(&out, &[(l, 'L'), (n, 'n'), (d, 'D')]),
        ["LLn", "LnD", "nDD"]
    );
    assert_eq!(size(&out), (3, 3));
}

#[test]
fn bevel_works_on_top_of_a_gradient() {
    let mut s = spec("b");
    s.fill = Fill {
        palette: Some("#000000,#ffffff".into()),
        ..Fill::default()
    };
    s.bevel = Some(Bevel { amount: 50 });
    let out = render(&s, &block_font()).unwrap();
    assert_eq!(out.grid.get(0, 0).rgb(), Rgb(128, 128, 128));
    assert_eq!(out.grid.get(2, 0).rgb(), Rgb(0, 0, 0));
    assert_eq!(out.grid.get(1, 1).rgb(), Rgb(255, 255, 255));
    assert_eq!(out.grid.get(2, 2).rgb(), Rgb(128, 128, 128));
    assert_eq!(out.grid.get(0, 2).rgb(), Rgb(255, 255, 255));
}

#[test]
fn bayer_dithering_mixes_only_the_two_stops_in_a_fixed_pattern() {
    let mut s = spec("b");
    s.fill = Fill {
        palette: Some("#000000,#ffffff".into()),
        dither: Dither::Bayer2,
        ..Fill::default()
    };
    let out = render(&s, &tall_font()).unwrap();
    let (k, w) = (Rgb(0, 0, 0), Rgb(255, 255, 255));
    assert_eq!(
        art(&out, &[(k, 'k'), (w, 'w')]),
        ["kkkk", "kkkk", "wkwk", "kwkw", "wwww"]
    );
}

#[test]
fn dithering_off_keeps_whole_rows_of_one_stop() {
    let mut s = spec("b");
    s.fill = Fill {
        palette: Some("#000000,#ffffff".into()),
        ..Fill::default()
    };
    let out = render(&s, &tall_font()).unwrap();
    let (k, w) = (Rgb(0, 0, 0), Rgb(255, 255, 255));
    assert_eq!(
        art(&out, &[(k, 'k'), (w, 'w')]),
        ["kkkk", "kkkk", "wwww", "wwww", "wwww"]
    );
}

#[test]
fn gradient_directions_follow_the_line() {
    let mut s = spec("b");
    s.fill = Fill {
        palette: Some("#000000,#ffffff".into()),
        direction: pixban_core::Direction::Diagonal,
        ..Fill::default()
    };
    let out = render(&s, &block_font()).unwrap();
    let (k, w) = (Rgb(0, 0, 0), Rgb(255, 255, 255));
    assert_eq!(art(&out, &[(k, 'k'), (w, 'w')]), ["kkw", "kww", "www"]);
}

#[test]
fn plate_draws_border_padding_and_cut_corners() {
    let mut s = spec("a");
    s.padding = Some(1);
    s.plate = Some(Plate {
        color: Some(Rgb(1, 1, 1)),
        border: 1,
        border_color: Some(Rgb(2, 2, 2)),
        cut_corners: false,
    });
    let legend = [(RED, '#'), (Rgb(1, 1, 1), 'p'), (Rgb(2, 2, 2), 'b')];
    let out = render(&s, &dot_font()).unwrap();
    assert_eq!(
        art(&out, &legend),
        ["bbbbb", "bpppb", "bp#pb", "bpppb", "bbbbb"]
    );
    s.plate.as_mut().unwrap().cut_corners = true;
    let out = render(&s, &dot_font()).unwrap();
    assert_eq!(
        art(&out, &legend),
        [".bbb.", "bpppb", "bp#pb", "bpppb", ".bbb."]
    );
    s.plate.as_mut().unwrap().border = 0;
    let out = render(&s, &dot_font()).unwrap();
    assert_eq!(art(&out, &legend), [".p.", "p#p", ".p."]);
}

#[test]
fn plate_without_padding_set_gets_two_cells_and_padding_works_alone() {
    let mut s = spec("a");
    s.plate = Some(Plate::default());
    assert_eq!(size(&render(&s, &dot_font()).unwrap()), (7, 7));
    s.plate = None;
    s.padding = Some(3);
    let out = render(&s, &dot_font()).unwrap();
    assert_eq!(size(&out), (7, 7));
    assert_eq!(out.grid.get(3, 3).rgb(), RED);
    assert!(out.grid.get(0, 0).is_transparent());
}

#[test]
fn layers_stack_background_shadow_extrusion_outline_fill() {
    let mut s = spec("a");
    s.bg = Some(Rgb(1, 1, 1));
    s.shadow = Some(Shadow {
        dx: 2,
        dy: 0,
        color: Some(Rgb(5, 5, 5)),
    });
    s.extrude = Some(Extrude {
        length: 3,
        dx: 1,
        dy: 0,
        step: 100,
        color: Some(Rgb(9, 9, 9)),
    });
    s.outline = outline(1, Connectivity::Four);
    let out = render(&s, &dot_font()).unwrap();
    let legend = [
        (Rgb(1, 1, 1), 'b'),
        (Rgb(5, 5, 5), 'S'),
        (Rgb(9, 9, 9), 'E'),
        (BLUE, 'O'),
        (RED, 'F'),
    ];
    assert_eq!(art(&out, &legend), ["bOEEESSb", "OFOEEESS", "bOEEESSb"]);
}

#[test]
fn bevel_is_the_last_layer_and_textures_sit_under_it() {
    let mut s = spec("b");
    s.fill.color = Some(Rgb(100, 100, 100));
    s.texture = Some(Texture {
        kind: TextureKind::Scanlines,
        strength: 50,
        period: 2,
    });
    s.bevel = Some(Bevel { amount: 50 });
    let out = render(&s, &block_font()).unwrap();
    assert_eq!(out.grid.get(1, 1).rgb(), Rgb(50, 50, 50));
    assert_eq!(out.grid.get(0, 0).rgb(), Rgb(178, 178, 178));
    assert_eq!(
        out.grid.get(0, 1).rgb(),
        Rgb(50, 50, 50).mix(Rgb::WHITE, 50)
    );
}

#[test]
fn per_letter_colors_cycle_and_skip_spaces() {
    let mut s = spec("a a\nb");
    s.fill.letter_colors = vec![Rgb(1, 0, 0), Rgb(2, 0, 0)];
    let out = render(&s, &dot_font()).unwrap();
    assert_eq!(out.grid.get(0, 0).rgb(), Rgb(1, 0, 0));
    assert_eq!(out.grid.get(4, 0).rgb(), Rgb(2, 0, 0));
    assert_eq!(out.grid.get(0, 2).rgb(), Rgb(1, 0, 0));
    assert_eq!(out.grid.get(1, 2).rgb(), Rgb(1, 0, 0));
}

#[test]
fn one_palette_stop_per_letter_spreads_over_the_text() {
    let mut s = spec("aaa");
    s.fill = Fill {
        palette: Some("#ff0000,#00ff00,#0000ff".into()),
        per_letter: true,
        ..Fill::default()
    };
    let out = render(&s, &dot_font()).unwrap();
    let colors: Vec<Rgb> = [0, 2, 4]
        .iter()
        .map(|&x| out.grid.get(x, 0).rgb())
        .collect();
    assert_eq!(colors, [Rgb(255, 0, 0), Rgb(0, 255, 0), Rgb(0, 0, 255)]);
}

#[test]
fn gradients_restart_on_every_line() {
    let mut s = spec("bb\nbb");
    s.fill = Fill {
        palette: Some("#000000,#ffffff".into()),
        ..Fill::default()
    };
    s.line_gap = 0;
    let out = render(&s, &block_font()).unwrap();
    assert_eq!(size(&out), (7, 6));
    assert_eq!(out.grid.get(0, 0).rgb(), Rgb(0, 0, 0));
    assert_eq!(out.grid.get(0, 2).rgb(), Rgb(255, 255, 255));
    assert_eq!(out.grid.get(0, 3).rgb(), Rgb(0, 0, 0));
}

#[test]
fn noise_is_seeded_and_uses_three_shades() {
    let mut s = spec("b");
    s.texture = Some(Texture {
        kind: TextureKind::Noise,
        strength: 20,
        period: 2,
    });
    let mut render_seed = |seed| {
        s.seed = seed;
        render(&s, &tall_font()).unwrap()
    };
    let a = render_seed(1);
    assert_eq!(a.to_svg(), render_seed(1).to_svg());
    assert_ne!(a.to_svg(), render_seed(2).to_svg());
    let mut colors: Vec<Rgb> = Vec::new();
    for y in 0..a.grid.height() {
        for x in 0..a.grid.width() {
            let c = a.grid.get(x, y).rgb();
            if !colors.contains(&c) {
                colors.push(c);
            }
        }
    }
    assert!(colors.len() <= 3, "{colors:?}");
}

fn growth_cases() -> Vec<(&'static str, Spec, (u32, u32))> {
    let mut cases = Vec::new();
    let base = (13, 7);
    let mut s = spec("AI");
    s.outline = outline(3, Connectivity::Four);
    cases.push(("outline 3", s, (base.0 + 6, base.1 + 6)));
    let mut s = spec("AI");
    s.extrude = extrude(6, 1, 0);
    cases.push(("extrude right", s, (base.0 + 6, base.1)));
    let mut s = spec("AI");
    s.extrude = extrude(5, -1, 1);
    cases.push(("extrude down left", s, (base.0 + 5, base.1 + 5)));
    let mut s = spec("AI");
    s.shadow = plain_shadow(-3, 2);
    cases.push(("shadow", s, (base.0 + 3, base.1 + 2)));
    let mut s = spec("AI");
    s.padding = Some(5);
    cases.push(("padding", s, (base.0 + 10, base.1 + 10)));
    let mut s = spec("AI");
    s.plate = Some(Plate {
        border: 3,
        ..Plate::default()
    });
    cases.push(("plate border 3", s, (base.0 + 10, base.1 + 10)));
    let mut s = spec("AI");
    s.bevel = Some(Bevel::default());
    s.texture = Some(Texture::default());
    s.fill.dither = Dither::Bayer4;
    cases.push(("bevel texture dither", s, base));
    let mut s = spec("AI");
    s.outline = outline(2, Connectivity::Eight);
    s.extrude = extrude(4, -1, -1);
    s.shadow = plain_shadow(3, -2);
    s.padding = Some(1);
    s.plate = Some(Plate {
        border: 2,
        ..Plate::default()
    });
    cases.push((
        "everything",
        s,
        (
            base.0 + 2 * (2 + 1 + 2) + 4 + 3,
            base.1 + 2 * (2 + 1 + 2) + 4 + 2,
        ),
    ));
    let mut s = spec("A\nII");
    s.line_gap = 2;
    s.align = Align::Right;
    s.spacing = Some(0);
    s.outline = outline(1, Connectivity::Eight);
    cases.push(("lines", s, (12 + 2, 7 + 2 + 7 + 2)));
    cases
}

#[test]
fn canvas_grows_for_every_effect_and_combination() {
    for (name, s, want) in growth_cases() {
        let out = render(&s, &test_font()).unwrap();
        assert_eq!(size(&out), want, "{name}");
    }
}

#[test]
fn text_is_never_clipped() {
    for (name, mut s, _) in growth_cases() {
        s.fill.color = Some(Rgb(250, 251, 252));
        s.fill.palette = None;
        s.fill.dither = Dither::None;
        s.bevel = None;
        s.texture = None;
        if let Some(plate) = s.plate.as_mut() {
            plate.border_color = Some(Rgb(1, 2, 3));
        }
        let out = render(&s, &test_font()).unwrap();
        assert_eq!(
            count_color(&out, Rgb(250, 251, 252)),
            ink(&s.text),
            "{name}"
        );
    }
}

#[test]
fn limits_are_enforced_with_clear_errors() {
    let font = test_font();
    let msg = |s: &Spec| render(s, &font).unwrap_err().to_string();

    let mut s = spec("A");
    s.outline = outline(5, Connectivity::Four);
    assert!(
        msg(&s).contains("outline width 5 is out of range, use 1 to 4"),
        "{}",
        msg(&s)
    );
    s.outline = outline(0, Connectivity::Four);
    assert!(msg(&s).contains("outline width"));

    let mut s = spec("A");
    s.extrude = extrude(33, 1, 1);
    assert!(msg(&s).contains("extrusion length 33 is out of range, use 1 to 32"));
    s.extrude = extrude(MAX_EXTRUDE, 1, 1);
    assert!(render(&s, &font).is_ok());
    s.extrude = extrude(3, 2, 0);
    assert!(msg(&s).contains("extrusion dx 2"));
    s.extrude = extrude(3, 0, 0);
    assert!(msg(&s).contains("points nowhere"));

    let mut s = spec("A");
    s.padding = Some(65);
    assert!(msg(&s).contains("padding 65 is out of range, use 0 to 64"));
    s.padding = Some(MAX_PADDING);
    assert!(render(&s, &font).is_ok());

    let s = spec(&vec!["A"; 17].join("\n"));
    assert_eq!(
        render(&s, &font).unwrap_err(),
        Error::TooManyLines { lines: 17, max: 16 }
    );
    assert!(render(&spec(&vec!["A"; 16].join("\n")), &font).is_ok());

    let mut s = spec("A");
    s.fill.letter_colors = vec![RED; 65];
    assert!(msg(&s).contains("number of letter colors 65"));
    s.fill.letter_colors = vec![RED; 64];
    assert!(render(&s, &font).is_ok());

    let mut s = spec("A");
    s.bevel = Some(Bevel { amount: 0 });
    assert!(msg(&s).contains("bevel amount"));
    s = spec("A");
    s.texture = Some(Texture {
        strength: 101,
        ..Texture::default()
    });
    assert!(msg(&s).contains("texture strength"));
    s.texture = Some(Texture {
        period: 1,
        ..Texture::default()
    });
    assert!(msg(&s).contains("texture period"));
    s = spec("A");
    s.plate = Some(Plate {
        border: 9,
        ..Plate::default()
    });
    assert!(msg(&s).contains("plate border"));
    s = spec("A");
    s.spacing = Some(-3);
    assert!(msg(&s).contains("letter spacing"));
    s = spec("A");
    s.fill.palette = Some("#ff0000,#nope".into());
    assert!(msg(&s).contains("item 2"));
}

#[test]
fn canvas_caps_still_apply_to_effects() {
    let font = test_font();
    let mut s = spec(&"A".repeat(1000));
    s.scale = Some(MAX_SCALE);
    s.outline = outline(4, Connectivity::Eight);
    assert!(matches!(
        render(&s, &font),
        Err(Error::OutputTooLarge { .. })
    ));
    let mut s = spec(&"A".repeat(600));
    s.spacing = Some(32);
    s.padding = Some(64);
    assert!(matches!(
        render(&s, &font),
        Err(Error::CanvasTooLarge { .. })
    ));
}

#[test]
fn default_spec_stays_effect_free() {
    let font = test_font();
    let out = render(&spec("AI"), &font).unwrap();
    assert_eq!(size(&out), (13, 7));
    assert_eq!(out.grid.background(), None);
    let colors = |o: &Rendered| {
        let mut v: Vec<Rgb> = Vec::new();
        for y in 0..o.grid.height() {
            for x in 0..o.grid.width() {
                if !o.grid.get(x, y).is_transparent() && !v.contains(&o.grid.get(x, y).rgb()) {
                    v.push(o.grid.get(x, y).rgb());
                }
            }
        }
        v.len()
    };
    assert_eq!(colors(&out), 1);
}

#[test]
fn settings_round_trip_through_toml_with_every_option_set() {
    let mut s = spec("A\nI");
    s.font = "test".into();
    s.scale = Some(3);
    s.bg = Some(Rgb(1, 2, 3));
    s.seed = 77;
    s.padding = Some(2);
    s.align = Align::Center;
    s.line_gap = 3;
    s.spacing = Some(-1);
    s.fill = Fill {
        palette: Some("#ff0000,#00ff00/#001100".into()),
        direction: pixban_core::Direction::Diagonal,
        color: Some(RED),
        dither: Dither::Bayer4,
        per_letter: true,
        letter_colors: vec![RED, BLUE],
    };
    s.shadow = Some(Shadow {
        dx: -1,
        dy: 2,
        color: Some(Rgb(4, 4, 4)),
    });
    s.outline = outline(2, Connectivity::Four);
    s.extrude = Some(Extrude {
        length: 5,
        dx: -1,
        dy: 1,
        step: 20,
        color: Some(Rgb(5, 5, 5)),
    });
    s.bevel = Some(Bevel { amount: 40 });
    s.texture = Some(Texture {
        kind: TextureKind::Noise,
        strength: 30,
        period: 4,
    });
    s.plate = Some(Plate {
        color: Some(Rgb(6, 6, 6)),
        border: 2,
        border_color: Some(Rgb(7, 7, 7)),
        cut_corners: true,
    });
    let font = test_font();
    let direct = render(&s, &font).unwrap().to_svg();

    let text = toml::to_string(&s).unwrap();
    let back: Spec = toml::from_str(&text).unwrap();
    assert_eq!(back, s);
    assert_eq!(toml::to_string(&back).unwrap(), text);
    assert_eq!(render(&back, &font).unwrap().to_svg(), direct);
}

#[test]
fn defaults_leave_no_trace_in_the_serialized_form() {
    let s = spec("A");
    let text = toml::to_string(&s).unwrap();
    for key in [
        "seed",
        "padding",
        "align",
        "line_gap",
        "spacing",
        "outline",
        "extrude",
        "bevel",
        "texture",
        "plate",
        "dither",
        "per_letter",
        "letter_colors",
    ] {
        assert!(!text.contains(key), "{key} in:\n{text}");
    }
}

#[test]
fn svg_of_a_busy_spec_stays_within_a_small_color_budget() {
    let mut s = spec("AI");
    s.fill.color = None;
    s.outline = outline(1, Connectivity::Eight);
    s.extrude = extrude(6, 1, 1);
    s.bevel = Some(Bevel::default());
    s.texture = Some(Texture::default());
    s.plate = Some(Plate::default());
    let svg = render(&s, &test_font()).unwrap().to_svg();
    let paths = svg.matches("<path").count();
    assert!(paths <= 7 * 2 * 3 + 6 + 3 + 2, "{paths} paths");
}
