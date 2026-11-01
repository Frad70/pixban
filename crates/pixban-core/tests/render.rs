mod common;

use pixban_core::limits::{MAX_SCALE, MAX_SHADOW_OFFSET};
use pixban_core::palette::{self, stop_index};
use pixban_core::{render, Direction, Error, Rgb, Rgba, Shadow, Spec};

fn spec(text: &str) -> Spec {
    Spec {
        text: text.to_string(),
        ..Spec::default()
    }
}

fn shadow(dx: i32, dy: i32) -> Option<Shadow> {
    Some(Shadow {
        dx,
        dy,
        color: None,
    })
}

#[test]
fn stop_mapping_is_exact_when_stops_match_rows() {
    for r in 0..7 {
        assert_eq!(stop_index(r, 7, 7), r as usize);
    }
}

#[test]
fn stop_mapping_rounds_to_the_nearest_stop() {
    let got: Vec<usize> = (0..7).map(|r| stop_index(r, 7, 3)).collect();
    assert_eq!(got, vec![0, 0, 1, 1, 1, 2, 2]);
    let got: Vec<usize> = (0..7).map(|r| stop_index(r, 7, 2)).collect();
    assert_eq!(got, vec![0, 0, 0, 1, 1, 1, 1]);
}

#[test]
fn stop_mapping_handles_degenerate_sizes() {
    assert_eq!(stop_index(0, 1, 7), 0);
    assert_eq!(stop_index(0, 0, 7), 0);
    assert_eq!(stop_index(5, 7, 1), 0);
    assert_eq!(stop_index(99, 7, 7), 6);
    assert_eq!(stop_index(0, 2, 5), 0);
    assert_eq!(stop_index(1, 2, 5), 4);
}

#[test]
fn fire_palette_is_the_brazier_one() {
    let fire = palette::find("fire").unwrap();
    let hex: Vec<String> = fire.stops.iter().map(Rgb::to_string).collect();
    assert_eq!(
        hex,
        ["#df4f07", "#df5707", "#d75f07", "#cf6f0f", "#cf7f0f", "#c78f17", "#bf9f1f"]
    );
    assert_eq!(fire.shadow.to_string(), "#470f07");
    assert_eq!(palette::find("FIRE").unwrap().name, "fire");
    let names: Vec<&str> = palette::names().collect();
    for n in [
        "fire",
        "ice",
        "toxic",
        "neon",
        "gold",
        "synthwave",
        "mono",
        "rainbow",
    ] {
        assert!(names.contains(&n), "{n}");
    }
}

#[test]
fn vertical_gradient_colors_each_row_with_its_stop() {
    let font = common::test_font();
    let mut s = spec("I");
    s.fill.palette = Some("fire".into());
    let out = render(&s, &font).unwrap();
    let fire = palette::find("fire").unwrap();
    assert_eq!((out.grid.width(), out.grid.height()), (6, 7));
    for y in 0..7 {
        assert_eq!(out.grid.get(0, y).is_transparent(), y != 0 && y != 6);
    }
    assert_eq!(out.grid.get(0, 0), fire.stops[0].into());
    assert_eq!(out.grid.get(5, 6), fire.stops[6].into());
    assert_eq!(out.grid.get(2, 3), fire.stops[3].into());
}

#[test]
fn horizontal_gradient_maps_columns() {
    let font = common::test_font();
    let mut s = spec("I");
    s.fill.palette = Some("rainbow".into());
    s.fill.direction = Direction::Horizontal;
    let out = render(&s, &font).unwrap();
    let rb = palette::find("rainbow").unwrap();
    assert_eq!(out.grid.get(0, 0), rb.stops[0].into());
    assert_eq!(out.grid.get(5, 0), rb.stops[6].into());
    assert_eq!(out.grid.get(0, 6), out.grid.get(0, 0));
}

#[test]
fn solid_color_overrides_the_palette() {
    let font = common::test_font();
    let mut s = spec("I");
    s.fill.color = Some(Rgb(1, 2, 3));
    let out = render(&s, &font).unwrap();
    for y in [0, 6] {
        for x in 0..6 {
            assert_eq!(out.grid.get(x, y), Rgba(1, 2, 3, 255));
        }
    }
}

#[test]
fn shadow_grows_the_canvas_by_the_offset() {
    let font = common::test_font();
    for (dx, dy) in [(0, 0), (1, 1), (3, 0), (0, 2), (-1, -1), (-2, 3), (4, -5)] {
        let mut s = spec("AI");
        s.shadow = shadow(dx, dy);
        let out = render(&s, &font).unwrap();
        assert_eq!(out.grid.width(), 13 + dx.unsigned_abs(), "dx {dx}");
        assert_eq!(out.grid.height(), 7 + dy.unsigned_abs(), "dy {dy}");
    }
    let plain = render(&spec("AI"), &font).unwrap();
    assert_eq!((plain.grid.width(), plain.grid.height()), (13, 7));
}

#[test]
fn shadow_is_drawn_under_the_fill_at_the_offset() {
    let font = common::test_font();
    let fire = palette::find("fire").unwrap();
    let mut s = spec("I");
    s.shadow = shadow(-2, 3);
    let out = render(&s, &font).unwrap();
    assert_eq!((out.grid.width(), out.grid.height()), (8, 10));
    assert_eq!(out.grid.get(2, 0), fire.stops[0].into());
    assert_eq!(out.grid.get(0, 3), fire.shadow.into());
    assert!(out.grid.get(0, 0).is_transparent());
    assert_eq!(out.grid.get(4, 3), fire.stops[3].into());
    assert_eq!(out.grid.get(2, 3), fire.shadow.into());
}

#[test]
fn shadow_default_color_follows_palette_and_can_be_overridden() {
    let font = common::test_font();
    let mut s = spec("I");
    s.shadow = shadow(1, 1);
    let corner = |s: &Spec| render(s, &font).unwrap().grid.get(6, 7);

    assert_eq!(corner(&s), Rgb::hex(0x470f07).into());
    s.fill.palette = Some("ice".into());
    assert_eq!(corner(&s), palette::find("ice").unwrap().shadow.into());
    s.fill.palette = None;
    s.fill.color = Some(Rgb(100, 200, 50));
    assert_eq!(corner(&s), Rgb(30, 60, 15).into());
    s.shadow = Some(Shadow {
        dx: 1,
        dy: 1,
        color: Some(Rgb(9, 9, 9)),
    });
    assert_eq!(corner(&s), Rgb(9, 9, 9).into());
}

#[test]
fn background_is_transparent_unless_set() {
    let font = common::test_font();
    let out = render(&spec("I"), &font).unwrap();
    assert!(out.grid.get(0, 3).is_transparent());
    assert_eq!(out.grid.background(), None);

    let mut s = spec("I");
    s.bg = Some(Rgb(13, 17, 23));
    let out = render(&s, &font).unwrap();
    assert_eq!(out.grid.get(0, 3), Rgba(13, 17, 23, 255));
    assert_eq!(out.grid.background(), Some(Rgb(13, 17, 23)));
}

#[test]
fn scale_defaults_to_the_font_and_is_validated() {
    let font = common::test_font();
    assert_eq!(render(&spec("I"), &font).unwrap().scale, 8);
    let mut s = spec("I");
    s.scale = Some(3);
    let out = render(&s, &font).unwrap();
    assert_eq!(out.pixel_size(), (18, 21));
    s.scale = Some(0);
    assert!(matches!(render(&s, &font), Err(Error::InvalidScale { .. })));
    s.scale = Some(MAX_SCALE + 1);
    assert!(matches!(render(&s, &font), Err(Error::InvalidScale { .. })));
}

#[test]
fn limits_give_errors_not_panics() {
    let font = common::test_font();
    let mut s = spec("I");
    s.shadow = shadow(MAX_SHADOW_OFFSET + 1, 0);
    assert!(matches!(
        render(&s, &font),
        Err(Error::InvalidShadowOffset { .. })
    ));
    s.shadow = shadow(i32::MIN, i32::MAX);
    assert!(render(&s, &font).is_err());

    let mut s = spec(&"A".repeat(1000));
    s.scale = Some(MAX_SCALE);
    assert!(matches!(
        render(&s, &font),
        Err(Error::OutputTooLarge { .. })
    ));

    let mut s = spec("I");
    s.fill.palette = Some("nope".into());
    assert_eq!(
        render(&s, &font).unwrap_err(),
        Error::UnknownPalette("nope".into())
    );
}

#[test]
fn missing_characters_are_reported() {
    let font = common::test_font();
    let out = render(&spec("A?я"), &font).unwrap();
    assert_eq!(out.missing, vec!['?', 'Я']);
}
