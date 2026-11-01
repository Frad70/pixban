use std::path::PathBuf;
use std::process::{Command, Output};

fn pixban(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pixban"))
        .args(args)
        .output()
        .unwrap()
}

fn ok_stdout(args: &[&str]) -> String {
    let out = pixban(args);
    assert!(
        out.status.success(),
        "pixban {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

fn tmp(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

type Cells = Vec<Option<String>>;

fn num(s: &str) -> usize {
    s.parse().unwrap()
}

fn paint(cells: &mut Cells, w: usize, x: usize, y: usize, rw: usize, rh: usize, color: &str) {
    for yy in y..y + rh {
        for xx in x..x + rw {
            cells[yy * w + xx] = Some(color.to_string());
        }
    }
}

fn dims(svg: &str) -> (usize, usize) {
    let doc = roxmltree::Document::parse(svg).unwrap();
    let vb: Vec<usize> = doc
        .root_element()
        .attribute("viewBox")
        .unwrap()
        .split(' ')
        .map(num)
        .collect();
    (vb[2], vb[3])
}

fn cells_from_rects(svg: &str) -> Cells {
    let (w, h) = dims(svg);
    let doc = roxmltree::Document::parse(svg).unwrap();
    let mut cells = vec![None; w * h];
    for n in doc.descendants().filter(|n| n.has_tag_name("rect")) {
        let a = |k: &str| num(n.attribute(k).unwrap());
        paint(
            &mut cells,
            w,
            a("x"),
            a("y"),
            a("width"),
            a("height"),
            n.attribute("fill").unwrap(),
        );
    }
    cells
}

fn cells_from_paths(svg: &str) -> Cells {
    let (w, h) = dims(svg);
    let doc = roxmltree::Document::parse(svg).unwrap();
    let mut cells = vec![None; w * h];
    for n in doc.descendants().filter(|n| n.has_tag_name("path")) {
        let fill = n.attribute("fill").unwrap();
        for part in n
            .attribute("d")
            .unwrap()
            .split('M')
            .filter(|p| !p.is_empty())
        {
            let (x, rest) = part.split_once(' ').unwrap();
            let (y, rest) = rest.split_once('h').unwrap();
            let (rw, rest) = rest.split_once('v').unwrap();
            let (rh, _) = rest.split_once('h').unwrap();
            paint(&mut cells, w, num(x), num(y), num(rw), num(rh), fill);
        }
    }
    cells
}

#[test]
fn anchor_matches_the_brazier_logo_cell_for_cell() {
    let ours = ok_stdout(&["brazier", "-f", "block", "-p", "fire", "--shadow", "1,1"]);
    let original = include_str!("fixtures/brazier-logo.svg");
    assert_eq!(dims(&ours), (55, 8));
    assert_eq!(dims(original), (55, 8));
    let root = |s: &str| {
        let d = roxmltree::Document::parse(s).unwrap();
        let r = d.root_element();
        (
            r.attribute("width").unwrap().to_string(),
            r.attribute("height").unwrap().to_string(),
        )
    };
    assert_eq!(root(&ours), root(original));
    assert_eq!(cells_from_paths(&ours), cells_from_rects(original));
    assert!(ours.len() < 3000, "{} bytes", ours.len());
}

#[test]
fn flags_dump_and_reload_give_identical_svg() {
    let args = [
        "Brazier",
        "-f",
        "block",
        "-p",
        "synthwave",
        "--gradient",
        "horizontal",
        "--shadow",
        "-2,1",
        "--shadow-color",
        "112233",
        "--scale",
        "5",
        "--bg",
        "#0d1117",
    ];
    let direct = ok_stdout(&args);

    let mut dump_args = args.to_vec();
    dump_args.push("--dump-spec");
    let dump = ok_stdout(&dump_args);
    let file = tmp("roundtrip.toml");
    std::fs::write(&file, &dump).unwrap();
    let path = file.to_str().unwrap();
    assert_eq!(ok_stdout(&["--spec", path]), direct);

    assert_eq!(ok_stdout(&["--spec", path, "--dump-spec"]), dump);

    let overridden = ok_stdout(&["--spec", path, "--scale", "2"]);
    assert_ne!(overridden, direct);
    let mut flags2 = args.to_vec();
    let at = flags2.iter().position(|a| *a == "5").unwrap();
    flags2[at] = "2";
    assert_eq!(overridden, ok_stdout(&flags2));
}

#[test]
fn text_flag_overrides_text_from_the_file() {
    let file = tmp("text.toml");
    std::fs::write(&file, "text = \"zz\"\n[fill]\npalette = \"ice\"\n").unwrap();
    let path = file.to_str().unwrap();
    assert_eq!(
        ok_stdout(&["--spec", path]),
        ok_stdout(&["zz", "-p", "ice"])
    );
    assert_eq!(
        ok_stdout(&["--spec", path, "ai"]),
        ok_stdout(&["ai", "-p", "ice"])
    );
}

#[test]
fn unknown_characters_warn_once_and_still_render() {
    let out = pixban(&["Привет 😀"]);
    assert!(out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();
    assert_eq!(err.lines().count(), 1, "{err}");
    assert!(err.starts_with("warning:"), "{err}");
    let svg = String::from_utf8(out.stdout).unwrap();
    roxmltree::Document::parse(&svg).unwrap();
}

#[test]
fn png_is_chosen_by_extension() {
    let file = tmp("out.PNG");
    let out = pixban(&["brazier", "--scale", "2", "-o", file.to_str().unwrap()]);
    assert!(out.status.success());
    let bytes = std::fs::read(&file).unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    let r = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    assert_eq!((r.info().width, r.info().height), (54 * 2, 7 * 2));
}

#[test]
fn format_flag_beats_extension_and_svg_goes_to_a_file() {
    let file = tmp("forced.dat");
    let out = pixban(&["ab", "--format", "svg", "-o", file.to_str().unwrap()]);
    assert!(out.status.success());
    assert!(std::fs::read_to_string(&file).unwrap().starts_with("<svg"));
    let out = pixban(&["ab", "-o", tmp("no-extension").to_str().unwrap()]);
    assert!(!out.status.success());
}

#[test]
fn bad_input_is_a_clean_error() {
    for (args, needle) in [
        (vec![], "no text"),
        (vec!["a", "-f", "nope"], "unknown font"),
        (vec!["a", "-p", "nope"], "unknown palette"),
        (vec!["a", "--scale", "0"], "scale"),
        (vec!["a", "--shadow", "99,0"], "shadow offset"),
        (vec!["a", "--shadow-color", "fff"], "needs a shadow"),
        (vec!["a\nb"], "multi-line"),
    ] {
        let out = pixban(&args);
        assert_eq!(out.status.code(), Some(1), "{args:?}");
        let err = String::from_utf8(out.stderr).unwrap();
        assert!(
            err.starts_with("error:") && err.contains(needle),
            "{args:?}: {err}"
        );
    }
}

#[test]
fn huge_input_is_refused_with_a_clear_message() {
    let long = "a".repeat(5000);
    let out = pixban(&[&long]);
    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("limit"), "{err}");
    let out = pixban(&["brazier", "--scale", "64", "--shadow", "32,32"]);
    assert!(out.status.success());
    let out = pixban(&[&"a".repeat(1000), "--scale", "64"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8(out.stderr).unwrap().contains("limit"));
}

#[test]
fn scale_must_be_an_integer() {
    let out = pixban(&["a", "--scale", "1.5"]);
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn listing_subcommands() {
    assert_eq!(ok_stdout(&["fonts"]).lines().collect::<Vec<_>>(), ["block"]);
    let palettes = ok_stdout(&["palettes"]);
    let names: Vec<&str> = palettes.lines().collect();
    assert_eq!(names.len(), 8);
    assert_eq!(names[0], "fire");
}

#[test]
fn text_that_looks_like_a_subcommand_goes_after_double_dash() {
    let out = pixban(&["--", "fonts"]);
    assert!(out.status.success());
    assert!(String::from_utf8(out.stdout).unwrap().starts_with("<svg"));
}
