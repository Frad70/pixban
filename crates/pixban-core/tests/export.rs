mod common;

use pixban_core::{render, Rgb, Rgba, Shadow, Spec};

fn spec(text: &str) -> Spec {
    Spec {
        text: text.to_string(),
        ..Spec::default()
    }
}

fn svg_of(s: &Spec) -> String {
    render(s, &common::test_font()).unwrap().to_svg()
}

fn assert_clean_svg(svg: &str, title: &str) {
    let doc = roxmltree::Document::parse(svg).unwrap_or_else(|e| panic!("{e}\n{svg}"));
    let root = doc.root_element();
    assert_eq!(root.tag_name().name(), "svg");
    assert_eq!(
        root.tag_name().namespace(),
        Some("http://www.w3.org/2000/svg")
    );
    assert_eq!(root.attribute("shape-rendering"), Some("crispEdges"));
    for node in doc.descendants() {
        assert!(!node.is_comment(), "comment in output");
        if node.is_element() {
            let n = node.tag_name().name();
            assert!(["svg", "title", "path"].contains(&n), "unexpected <{n}>");
        }
    }
    let t = doc
        .descendants()
        .find(|n| n.has_tag_name("title"))
        .and_then(|n| n.text())
        .unwrap_or("");
    assert_eq!(t, title);
    assert!(!svg.contains("<text") && !svg.contains("<rect") && !svg.contains("<!--"));
}

#[test]
fn svg_has_cell_viewbox_and_pixel_size() {
    let mut s = spec("AI");
    s.scale = Some(5);
    s.shadow = Some(Shadow::default());
    let svg = svg_of(&s);
    assert_clean_svg(&svg, "AI");
    let doc = roxmltree::Document::parse(&svg).unwrap();
    let root = doc.root_element();
    assert_eq!(root.attribute("viewBox"), Some("0 0 14 8"));
    assert_eq!(root.attribute("width"), Some("70"));
    assert_eq!(root.attribute("height"), Some("40"));
}

#[test]
fn svg_uses_one_path_per_color() {
    let mut s = spec("AI");
    s.fill.palette = Some("fire".into());
    s.shadow = Some(Shadow::default());
    let svg = svg_of(&s);
    let doc = roxmltree::Document::parse(&svg).unwrap();
    let fills: Vec<&str> = doc
        .descendants()
        .filter(|n| n.has_tag_name("path"))
        .map(|n| n.attribute("fill").unwrap())
        .collect();
    let mut unique = fills.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(fills.len(), unique.len());
    assert_eq!(fills.len(), 8);
}

#[test]
fn svg_merges_equal_runs_on_consecutive_rows() {
    let mut s = spec("I");
    s.fill.color = Some(Rgb(255, 0, 0));
    let svg = svg_of(&s);
    assert!(svg.contains("M2 1h2v5h-2z"), "{svg}");
}

#[test]
fn svg_background_is_a_single_path() {
    let mut s = spec("I");
    s.bg = Some(Rgb(13, 17, 23));
    let svg = svg_of(&s);
    assert_clean_svg(&svg, "I");
    assert!(svg.contains("fill=\"#0d1117\" d=\"M0 0h6v7h-6z\""), "{svg}");
}

#[test]
fn svg_title_is_escaped_and_stays_well_formed() {
    for text in [
        "a<b>&c",
        "tab\there",
        "bell\u{7}x",
        "\u{fffe}ok",
        "Привет",
        "😀",
        "I\"I'",
    ] {
        assert_clean_svg(
            &svg_of(&spec(text)),
            &text.replace(['\t'], " ").replace(['\u{7}', '\u{fffe}'], ""),
        );
    }
}

#[test]
fn png_round_trips_the_grid() {
    let font = common::test_font();
    let mut s = spec("AI");
    s.scale = Some(3);
    s.shadow = Some(Shadow {
        dx: -1,
        dy: 2,
        color: None,
    });
    s.bg = Some(Rgb(1, 2, 3));
    let out = render(&s, &font).unwrap();
    let bytes = out.to_png().unwrap();

    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgba);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
    assert_eq!((info.width, info.height), out.pixel_size());
    for py in 0..info.height {
        for px in 0..info.width {
            let i = (py * info.width + px) as usize * 4;
            let got = Rgba(buf[i], buf[i + 1], buf[i + 2], buf[i + 3]);
            assert_eq!(got, out.grid.get(px / 3, py / 3), "pixel {px},{py}");
        }
    }
}

#[test]
fn png_keeps_transparency() {
    let out = render(&spec("I"), &common::test_font()).unwrap();
    let bytes = out.to_png().unwrap();
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    reader.next_frame(&mut buf).unwrap();
    assert_eq!(buf[3], 255);
    let row3 = 3 * 8 * 48 * 4;
    assert_eq!(buf[row3 + 3], 0);
}
