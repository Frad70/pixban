mod common;

use pixban_core::{layout, Align, Error, LayoutOptions};

fn width_of(text: &str) -> u32 {
    let f = common::test_font();
    layout(&f, text, &LayoutOptions::default()).unwrap().width
}

#[test]
fn widths_add_glyphs_and_spacing() {
    assert_eq!(width_of("A"), 6);
    assert_eq!(width_of("II"), 6 + 2 + 6);
    assert_eq!(width_of("I I"), 6 + 2 + 3 + 2 + 6);
}

#[test]
fn kerning_pulls_pairs_together_in_one_direction_only() {
    assert_eq!(width_of("AI"), 6 + 2 - 1 + 6);
    assert_eq!(width_of("IA"), 6 + 2 + 6);
}

#[test]
fn positions_follow_the_pen() {
    let f = common::test_font();
    let l = layout(&f, "AIA", &LayoutOptions::default()).unwrap();
    let xs: Vec<u32> = l.lines[0].glyphs.iter().map(|g| g.x).collect();
    assert_eq!(xs, vec![0, 7, 15]);
    assert_eq!((l.width, l.height), (21, 7));
}

#[test]
fn case_upper_maps_lowercase_including_cyrillic() {
    assert_eq!(width_of("ai"), width_of("AI"));
    let f = common::test_font();
    let l = layout(&f, "д", &LayoutOptions::default()).unwrap();
    assert!(l.missing.is_empty());
    assert_eq!(l.lines[0].glyphs[0].ch, 'Д');
}

#[test]
fn unknown_characters_use_the_fallback_and_are_reported_once() {
    let f = common::test_font();
    let l = layout(&f, "A?😀?A", &LayoutOptions::default()).unwrap();
    assert_eq!(l.missing, vec!['?', '😀']);
    assert_eq!(l.width, 6 + 2 + 5 + 2 + 5 + 2 + 5 + 2 + 6);
    let flags: Vec<bool> = l.lines[0].glyphs.iter().map(|g| g.is_fallback).collect();
    assert_eq!(flags, vec![false, true, true, true, false]);
}

#[test]
fn rejects_empty_multiline_and_huge_text() {
    let f = common::test_font();
    let o = LayoutOptions::default();
    assert_eq!(layout(&f, "", &o).unwrap_err(), Error::EmptyText);
    assert_eq!(layout(&f, "\n\n", &o).unwrap_err(), Error::EmptyText);
    let long = "A".repeat(pixban_core::limits::MAX_TEXT_CHARS + 1);
    assert!(matches!(
        layout(&f, &long, &o).unwrap_err(),
        Error::TextTooLong { .. }
    ));
    let max = "A".repeat(pixban_core::limits::MAX_TEXT_CHARS);
    assert!(layout(&f, &max, &o).is_ok());
}

#[test]
fn text_wider_than_the_canvas_limit_is_an_error() {
    let src = format!(
        "name: W\nheight: 1\nbaseline: 1\nspacing: 32\ncase: mixed\n---\nchar: A\n{}\n",
        "#".repeat(64)
    );
    let f = pixban_core::font::parse(&src).unwrap();
    let text = "A".repeat(300);
    assert!(matches!(
        layout(&f, &text, &LayoutOptions::default()).unwrap_err(),
        Error::CanvasTooLarge { .. }
    ));
}

#[test]
fn line_height_is_the_union_of_extra_rows_actually_used() {
    let src = "name: X\nheight: 3\nbaseline: 3\nspacing: 1\ncase: mixed\n---\n\
               char: A\n##\n##\n##\n\
               char: E\ntop: 1\n##\n##\n##\n##\n\
               char: g\nbottom: 2\n##\n##\n##\n##\n##\n";
    let f = pixban_core::font::parse(src).unwrap();
    let plain = layout(&f, "AA", &LayoutOptions::default()).unwrap();
    assert_eq!(plain.height, 3);
    let raised = layout(&f, "AE", &LayoutOptions::default()).unwrap();
    assert_eq!(raised.height, 4);
    assert_eq!(
        raised.lines[0].glyphs[0].y, 1,
        "band glyph starts one row lower"
    );
    assert_eq!(
        raised.lines[0].glyphs[1].y, 0,
        "raised glyph starts at the top"
    );
    let both = layout(&f, "EgA", &LayoutOptions::default()).unwrap();
    assert_eq!(both.height, 6);
    assert_eq!(both.lines[0].glyphs[2].y, 1);
}

fn opts(align: Align, line_gap: u32, spacing: Option<i32>) -> LayoutOptions {
    let mut o = LayoutOptions::default();
    o.align = align;
    o.line_gap = line_gap;
    o.spacing = spacing;
    o
}

#[test]
fn lines_stack_with_the_gap_and_the_widest_line_sets_the_width() {
    let f = common::test_font();
    let l = layout(&f, "AA\nI", &LayoutOptions::default()).unwrap();
    assert_eq!(l.lines.len(), 2);
    assert_eq!((l.width, l.height), (14, 7 + 1 + 7));
    assert_eq!((l.lines[0].y, l.lines[0].width), (0, 14));
    assert_eq!((l.lines[1].y, l.lines[1].width), (8, 6));
    assert_eq!(l.lines[1].glyphs[0].y, 8);
    for gap in [0, 3] {
        let l = layout(&f, "A\nI\nA", &opts(Align::Left, gap, None)).unwrap();
        assert_eq!(l.height, 21 + 2 * gap);
        assert_eq!(l.lines[2].y, 14 + 2 * gap);
    }
}

#[test]
fn alignment_offsets_shorter_lines() {
    let f = common::test_font();
    let xs = |align| {
        let l = layout(&f, "AIA\nI", &opts(align, 1, None)).unwrap();
        (l.lines[1].x, l.lines[1].glyphs[0].x, l.width)
    };
    assert_eq!(xs(Align::Left), (0, 0, 21));
    assert_eq!(xs(Align::Center), (7, 7, 21));
    assert_eq!(xs(Align::Right), (15, 15, 21));
}

#[test]
fn blank_lines_keep_their_height_and_trailing_newlines_are_dropped() {
    let f = common::test_font();
    let l = layout(&f, "A\n\nI", &opts(Align::Left, 1, None)).unwrap();
    assert_eq!(l.lines.len(), 3);
    assert_eq!(l.height, 7 * 3 + 2);
    assert!(l.lines[1].glyphs.is_empty());
    let l = layout(&f, "A\n\n", &LayoutOptions::default()).unwrap();
    assert_eq!(l.lines.len(), 1);
    let l = layout(&f, "A\r\nI", &LayoutOptions::default()).unwrap();
    assert_eq!(l.lines.len(), 2);
}

#[test]
fn spacing_override_replaces_the_font_spacing_and_may_be_negative() {
    let f = common::test_font();
    for (sp, expect) in [(0, 12), (5, 17), (-2, 10)] {
        let l = layout(&f, "II", &opts(Align::Left, 1, Some(sp))).unwrap();
        assert_eq!(l.width, expect, "spacing {sp}");
    }
    let l = layout(&f, "II", &opts(Align::Left, 1, Some(-2))).unwrap();
    assert_eq!(l.lines[0].glyphs[1].x, 4);
}

#[test]
fn line_and_spacing_limits_are_errors() {
    let f = common::test_font();
    let sixteen = vec!["A"; 16].join("\n");
    assert_eq!(
        layout(&f, &sixteen, &LayoutOptions::default())
            .unwrap()
            .lines
            .len(),
        16
    );
    let seventeen = vec!["A"; 17].join("\n");
    assert_eq!(
        layout(&f, &seventeen, &LayoutOptions::default()).unwrap_err(),
        Error::TooManyLines { lines: 17, max: 16 }
    );
    for sp in [-3, 33] {
        let e = layout(&f, "A", &opts(Align::Left, 1, Some(sp))).unwrap_err();
        assert!(e.to_string().contains("letter spacing"), "{e}");
    }
    let e = layout(&f, "A", &opts(Align::Left, 33, None)).unwrap_err();
    assert!(e.to_string().contains("line gap"), "{e}");
}
