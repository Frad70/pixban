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

const ALL_FLAGS: &[&str] = &[
    "PI\\nXB",
    "-f",
    "block",
    "-p",
    "#ff3c00,#ffcc00/#220a00",
    "--gradient",
    "diagonal",
    "--dither",
    "bayer4",
    "--color",
    "#112233",
    "--per-letter",
    "--letter-colors",
    "#ff0000,#00ff00,#0000ff",
    "--shadow",
    "-1,2",
    "--shadow-color",
    "#101010",
    "--outline",
    "2",
    "--outline-color",
    "#202020",
    "--outline-conn",
    "4",
    "--extrude",
    "5",
    "--extrude-dir",
    "-1,1",
    "--extrude-color",
    "#303030",
    "--extrude-step",
    "20",
    "--bevel",
    "40",
    "--texture",
    "noise",
    "--texture-strength",
    "30",
    "--texture-period",
    "4",
    "--seed",
    "77",
    "--plate",
    "--plate-color",
    "#404040",
    "--plate-border",
    "2",
    "--plate-border-color",
    "#505050",
    "--cut-corners",
    "--padding",
    "2",
    "--bg",
    "#0d1117",
    "--align",
    "center",
    "--line-gap",
    "2",
    "--spacing",
    "-1",
    "--scale",
    "3",
];

#[test]
fn every_option_survives_flags_dump_and_reload() {
    let direct = ok_stdout(ALL_FLAGS);
    roxmltree::Document::parse(&direct).unwrap();

    let mut dump_args = ALL_FLAGS.to_vec();
    dump_args.push("--dump-spec");
    let dump = ok_stdout(&dump_args);
    for key in [
        "text",
        "seed",
        "padding",
        "align",
        "line_gap",
        "spacing",
        "dither",
        "per_letter",
        "letter_colors",
        "[shadow]",
        "[outline]",
        "[extrude]",
        "[bevel]",
        "[texture]",
        "[plate]",
        "connectivity = \"4\"",
        "cut_corners = true",
    ] {
        assert!(dump.contains(key), "{key} missing from:\n{dump}");
    }
    assert!(dump.starts_with("text = \"\"\"\nPI\nXB\"\"\"\n"), "{dump}");

    let file = tmp("all-options.toml");
    std::fs::write(&file, &dump).unwrap();
    let path = file.to_str().unwrap();
    assert_eq!(ok_stdout(&["--spec", path]), direct);
    assert_eq!(ok_stdout(&["--spec", path, "--dump-spec"]), dump);
}

#[test]
fn flags_override_single_fields_of_a_loaded_table() {
    let file = tmp("outline.toml");
    std::fs::write(
        &file,
        "text = \"ab\"\n[outline]\nwidth = 1\nconnectivity = \"4\"\ncolor = \"#102030\"\n",
    )
    .unwrap();
    let path = file.to_str().unwrap();
    let dump = ok_stdout(&["--spec", path, "--outline", "2", "--dump-spec"]);
    assert!(
        dump.contains("width = 2") && dump.contains("connectivity = \"4\""),
        "{dump}"
    );
    assert!(dump.contains("#102030"), "{dump}");
    let dump = ok_stdout(&["--spec", path, "--outline-conn", "8", "--dump-spec"]);
    assert!(
        dump.contains("connectivity = \"8\"") && dump.contains("width = 1"),
        "{dump}"
    );
}

#[test]
fn a_literal_backslash_n_breaks_the_line() {
    let two = ok_stdout(&["ab\\nab", "--line-gap", "1"]);
    let real = ok_stdout(&["ab\nab", "--line-gap", "1"]);
    assert_eq!(two, real);
    let doc = roxmltree::Document::parse(&two).unwrap();
    assert_eq!(doc.root_element().attribute("viewBox"), Some("0 0 14 15"));
    let one = ok_stdout(&["ab"]);
    assert_ne!(one, two);
}

#[test]
fn custom_palettes_work_and_bad_items_are_named() {
    let svg = ok_stdout(&["hi", "-p", "#ff0000,#0000ff"]);
    assert!(svg.contains("#ff0000") && svg.contains("#0000ff"));
    let out = pixban(&["hi", "-p", "#ff0000,#00ff0,#0000ff"]);
    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(
        err.starts_with("error:") && err.contains("item 2 '#00ff0'"),
        "{err}"
    );
    let out = pixban(&["hi", "-p", "#ff0000"]);
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("1 colors given, use 2 to 16"), "{err}");
}

#[test]
fn dependent_flags_need_their_effect() {
    for (args, needle) in [
        (vec!["a", "--outline-color", "fff"], "--outline"),
        (vec!["a", "--outline-conn", "4"], "--outline"),
        (vec!["a", "--extrude-dir", "1,0"], "--extrude"),
        (vec!["a", "--extrude-step", "10"], "--extrude"),
        (vec!["a", "--texture-strength", "10"], "--texture"),
        (vec!["a", "--plate-border", "2"], "--plate"),
        (vec!["a", "--cut-corners"], "--plate"),
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
fn effect_limits_are_clean_errors() {
    for (args, needle) in [
        (
            vec!["a", "--outline", "5"],
            "outline width 5 is out of range, use 1 to 4",
        ),
        (vec!["a", "--extrude", "33"], "extrusion length 33"),
        (
            vec!["a", "--extrude", "3", "--extrude-dir", "0,0"],
            "points nowhere",
        ),
        (
            vec!["a", "--extrude", "3", "--extrude-dir", "2,0"],
            "extrusion dx 2",
        ),
        (vec!["a", "--padding", "65"], "padding 65"),
        (vec!["a", "--spacing", "-3"], "letter spacing -3"),
        (vec!["a", "--bevel", "0"], "bevel amount"),
        (
            vec!["a", "--texture", "noise", "--texture-strength", "101"],
            "texture strength",
        ),
        (vec!["a", "--plate", "--plate-border", "9"], "plate border"),
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
fn help_lists_every_group_and_flag() {
    let help = ok_stdout(&["--help"]);
    for needle in [
        "Fill:",
        "Effects:",
        "Plate and background:",
        "Layout:",
        "Output:",
        "Animation:",
        "--anim",
        "--frames",
        "--frame-ms",
        "--flame",
        "--amplitude",
        "--band",
        "--count",
        "--hues",
        "--frames-dir",
        "--outline",
        "--outline-color",
        "--extrude",
        "--extrude-dir",
        "--bevel",
        "--dither",
        "--texture",
        "--plate",
        "--padding",
        "--align",
        "--line-gap",
        "--letter-colors",
        "--seed",
        "--spacing",
    ] {
        assert!(help.contains(needle), "{needle} missing from --help");
    }
    assert!(help.lines().count() < 100, "{} lines", help.lines().count());
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
        (vec![&"a\n".repeat(17)], "lines"),
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
    assert_eq!(
        ok_stdout(&["fonts"]).lines().collect::<Vec<_>>(),
        ["block", "slim", "tiny", "mega"]
    );
    let palettes = ok_stdout(&["palettes"]);
    let names: Vec<&str> = palettes.lines().collect();
    assert_eq!(names.len(), 8);
    assert_eq!(names[0], "fire");

    let all = ok_stdout(&["palettes", "--all"]);
    let lines: Vec<&str> = all.lines().collect();
    assert_eq!(&lines[..8], &names[..]);
    assert_eq!(&lines[8..10], ["", "extended:"]);
    let extended = &lines[10..];
    assert!((12..=14).contains(&extended.len()), "{extended:?}");
    assert!(extended.contains(&"gameboy") && extended.contains(&"pico8-warm"));
    assert_eq!(
        ok_stdout(&["Hi", "-p", "gameboy"]),
        ok_stdout(&["Hi", "-p", "GameBoy"])
    );
}

#[test]
fn text_that_looks_like_a_subcommand_goes_after_double_dash() {
    let out = pixban(&["--", "fonts"]);
    assert!(out.status.success());
    assert!(String::from_utf8(out.stdout).unwrap().starts_with("<svg"));
}

fn dir(name: &str) -> PathBuf {
    let d = tmp(name);
    let _ = std::fs::remove_dir_all(&d);
    d
}

#[test]
fn animation_files_are_chosen_by_extension() {
    let base = [
        "HI", "-f", "block", "--anim", "wave", "--frames", "4", "--scale", "2",
    ];
    let with = |file: &PathBuf| -> Vec<String> {
        base.iter()
            .map(|s| s.to_string())
            .chain(["-o".to_string(), file.to_str().unwrap().to_string()])
            .collect()
    };
    let run = |file: &PathBuf| {
        let args = with(file);
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let out = pixban(&args);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        std::fs::read(file).unwrap()
    };
    let gif = run(&tmp("wave.gif"));
    assert_eq!(&gif[..6], b"GIF89a");
    for name in ["wave.apng", "wave.png"] {
        let bytes = run(&tmp(name));
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "{name}");
        let reader = png::Decoder::new(std::io::Cursor::new(bytes))
            .read_info()
            .unwrap();
        assert_eq!(
            reader.info().animation_control.unwrap().num_frames,
            4,
            "{name}"
        );
    }
    let svg = String::from_utf8(run(&tmp("wave.svg"))).unwrap();
    assert!(svg.contains("@keyframes") && svg.contains("prefers-reduced-motion"));
    assert!(!svg.contains("<script"));
}

#[test]
fn format_flag_writes_gif_and_apng_to_stdout() {
    for (format, magic) in [("gif", &b"GIF89a"[..]), ("apng", &b"\x89PNG"[..])] {
        let out = pixban(&["HI", "--anim", "burn", "--frames", "3", "--format", format]);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(out.stdout.starts_with(magic), "{format}");
    }
}

#[test]
fn gif_and_apng_need_an_animation() {
    for format in ["gif", "apng"] {
        let out = pixban(&["HI", "--format", format]);
        assert_eq!(out.status.code(), Some(1));
        let err = String::from_utf8(out.stderr).unwrap();
        assert!(err.contains("need an animation"), "{err}");
    }
}

#[test]
fn frames_dir_writes_numbered_pngs() {
    let d = dir("frames-out");
    let out = pixban(&[
        "HI",
        "--anim",
        "shimmer",
        "--frames",
        "5",
        "--scale",
        "2",
        "--frames-dir",
        d.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stdout.starts_with(d.to_str().unwrap().as_bytes()));
    let mut names: Vec<String> = std::fs::read_dir(&d)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "frame-000.png",
            "frame-001.png",
            "frame-002.png",
            "frame-003.png",
            "frame-004.png"
        ]
    );
    let sizes: std::collections::BTreeSet<(u32, u32)> = names
        .iter()
        .map(|n| {
            let r = png::Decoder::new(std::io::BufReader::new(
                std::fs::File::open(d.join(n)).unwrap(),
            ))
            .read_info()
            .unwrap();
            (r.info().width, r.info().height)
        })
        .collect();
    assert_eq!(sizes.len(), 1);
}

#[test]
fn animation_settings_need_an_animation_and_the_right_one() {
    for (args, needle) in [
        (vec!["a", "--frames", "4"], "--frames needs an animation"),
        (vec!["a", "--flame", "4"], "--flame needs an animation"),
        (
            vec!["a", "--frames-dir", "x"],
            "--frames-dir needs an animation",
        ),
        (
            vec!["a", "--anim", "wave", "--flame", "4"],
            "flame only applies to burn",
        ),
        (
            vec!["a", "--anim", "burn", "--frames", "1"],
            "frames 1 is out of range",
        ),
        (
            vec!["a", "--anim", "burn", "--frames", "121"],
            "frames 121 is out of range",
        ),
        (
            vec!["a", "--anim", "burn", "--frame-ms", "10"],
            "frame time in ms 10",
        ),
        (vec!["a", "--anim", "melt"], "invalid value"),
    ] {
        let out = pixban(&args);
        assert!(!out.status.success(), "{args:?}");
        let err = String::from_utf8(out.stderr).unwrap();
        assert!(err.contains(needle), "{args:?}: {err}");
    }
    let out = pixban(&[
        &"a".repeat(400),
        "--anim",
        "burn",
        "--frames",
        "120",
        "--scale",
        "1",
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8(out.stderr)
        .unwrap()
        .contains("animation limits"));
}

#[test]
fn gif_delay_that_cannot_be_stored_warns_once() {
    let file = tmp("slow.gif");
    let out = pixban(&[
        "HI",
        "--anim",
        "wave",
        "--frames",
        "3",
        "--frame-ms",
        "75",
        "-o",
        file.to_str().unwrap(),
    ]);
    assert!(out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();
    assert_eq!(err.lines().count(), 1, "{err}");
    assert!(
        err.starts_with("warning: frame time 75 ms is stored as 80 ms"),
        "{err}"
    );
    let out = pixban(&[
        "HI",
        "--anim",
        "wave",
        "--frames",
        "3",
        "--frame-ms",
        "80",
        "-o",
        file.to_str().unwrap(),
    ]);
    assert!(out.stderr.is_empty());
}

#[test]
fn animation_flags_dump_and_reload_give_identical_files() {
    let args = [
        "BURN",
        "-f",
        "block",
        "-p",
        "ice",
        "--shadow",
        "1,1",
        "--seed",
        "4",
        "--anim",
        "burn",
        "--frames",
        "6",
        "--frame-ms",
        "70",
        "--flame",
        "5",
        "--scale",
        "3",
    ];
    let dumped = ok_stdout(&[&args[..], &["--dump-spec"]].concat());
    assert!(
        dumped.contains("[anim]") && dumped.contains("kind = \"burn\""),
        "{dumped}"
    );
    let file = tmp("burn.toml");
    std::fs::write(&file, dumped).unwrap();
    let from_flags = ok_stdout(&args);
    let from_file = ok_stdout(&["--spec", file.to_str().unwrap()]);
    assert_eq!(from_flags, from_file);
    for ext in ["gif", "apng"] {
        let (a, b) = (tmp(&format!("a.{ext}")), tmp(&format!("b.{ext}")));
        let mut one: Vec<&str> = args.to_vec();
        one.extend(["-o", a.to_str().unwrap()]);
        assert!(pixban(&one).status.success());
        assert!(
            pixban(&["--spec", file.to_str().unwrap(), "-o", b.to_str().unwrap()])
                .status
                .success()
        );
        assert_eq!(
            std::fs::read(&a).unwrap(),
            std::fs::read(&b).unwrap(),
            "{ext}"
        );
    }
}

#[test]
fn static_output_ignores_the_animation_options_being_absent() {
    let plain = ok_stdout(&["brazier", "-f", "block", "-p", "fire", "--shadow", "1,1"]);
    assert!(!plain.contains("<style") && !plain.contains("@keyframes"));
}

#[test]
fn every_animation_flag_survives_dump_and_reload() {
    let cases: [(&str, &[&str]); 7] = [
        ("burn", &["--flame", "4"]),
        ("shimmer", &["--band", "2"]),
        ("wave", &["--amplitude", "1"]),
        ("type", &[]),
        ("glitch", &["--amplitude", "2", "--count", "2"]),
        ("rainbow", &["--hues", "6"]),
        ("sparkle", &["--count", "3"]),
    ];
    for (kind, knobs) in cases {
        let mut args = vec![
            "AI",
            "--seed",
            "9",
            "--scale",
            "2",
            "--shadow",
            "1,1",
            "--anim",
            kind,
            "--frames",
            "6",
            "--frame-ms",
            "90",
        ];
        args.extend_from_slice(knobs);
        let direct = ok_stdout(&args);
        let mut dump_args = args.clone();
        dump_args.push("--dump-spec");
        let dump = ok_stdout(&dump_args);
        assert!(dump.contains(&format!("kind = \"{kind}\"")), "{dump}");
        assert!(
            dump.contains("frames = 6") && dump.contains("frame_ms = 90"),
            "{dump}"
        );
        let file = tmp(&format!("anim-{kind}.toml"));
        std::fs::write(&file, &dump).unwrap();
        let path = file.to_str().unwrap();
        assert_eq!(ok_stdout(&["--spec", path]), direct, "{kind}");
        assert_eq!(ok_stdout(&["--spec", path, "--dump-spec"]), dump, "{kind}");
    }
}
