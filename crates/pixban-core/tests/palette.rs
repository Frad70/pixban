use pixban_core::palette::{self, parse_custom, resolve};
use pixban_core::{Error, Rgb};

fn hex(c: &Rgb) -> String {
    c.to_string()
}

#[test]
fn extended_group_is_separate_from_the_base_eight() {
    assert_eq!(palette::names().count(), 8);
    let ext: Vec<&str> = palette::extended_names().collect();
    assert!((12..=14).contains(&ext.len()), "{ext:?}");
    for name in &ext {
        assert!(!palette::names().any(|b| b == *name), "{name} is in both");
        assert!(palette::find(name).is_some(), "{name}");
    }
    for want in [
        "pico8-warm",
        "pico8-cool",
        "gameboy",
        "nes",
        "c64",
        "cga",
        "lava",
        "ocean",
        "forest",
        "sunset",
        "pastel",
        "vaporwave",
        "ember",
    ] {
        assert!(ext.contains(&want), "{want}");
    }
}

#[test]
fn extended_ramps_have_four_to_eight_distinct_stops() {
    for p in palette::EXTENDED {
        assert!(
            (4..=8).contains(&p.stops.len()),
            "{} {}",
            p.name,
            p.stops.len()
        );
        let mut sorted: Vec<Rgb> = p.stops.to_vec();
        sorted.sort_by_key(|c| (c.0, c.1, c.2));
        sorted.dedup();
        assert_eq!(sorted.len(), p.stops.len(), "{} repeats a stop", p.name);
        assert!(!p.stops.contains(&p.shadow), "{} shadow is a stop", p.name);
    }
}

#[test]
fn hardware_ramps_use_published_values() {
    let stops =
        |n: &str| -> Vec<String> { palette::find(n).unwrap().stops.iter().map(hex).collect() };
    assert_eq!(
        stops("gameboy"),
        ["#9bbc0f", "#8bac0f", "#306230", "#0f380f"]
    );
    let pico_all = [
        "#000000", "#1d2b53", "#7e2553", "#008751", "#ab5236", "#5f574f", "#c2c3c7", "#fff1e8",
        "#ff004d", "#ffa300", "#ffec27", "#00e436", "#29adff", "#83769c", "#ff77a8", "#ffccaa",
    ];
    for name in ["pico8-warm", "pico8-cool"] {
        for c in stops(name) {
            assert!(pico_all.contains(&c.as_str()), "{name} {c}");
        }
    }
    let c64_all = [
        "#000000", "#ffffff", "#68372b", "#70a4b2", "#6f3d86", "#588d43", "#352879", "#b8c76f",
        "#6f4f25", "#433900", "#9a6759", "#444444", "#6c6c6c", "#9ad284", "#6c5eb5", "#959595",
    ];
    for c in stops("c64") {
        assert!(c64_all.contains(&c.as_str()), "c64 {c}");
    }
    let cga_all = [
        "#000000", "#0000aa", "#00aa00", "#00aaaa", "#aa0000", "#aa00aa", "#aa5500", "#aaaaaa",
        "#555555", "#5555ff", "#55ff55", "#55ffff", "#ff5555", "#ff55ff", "#ffff55", "#ffffff",
    ];
    for c in stops("cga") {
        assert!(cga_all.contains(&c.as_str()), "cga {c}");
    }
}

#[test]
fn names_resolve_case_insensitively_to_owned_stops() {
    let r = resolve("Lava").unwrap();
    assert_eq!(r.stops.len(), 7);
    assert_eq!(r.shadow, palette::find("lava").unwrap().shadow);
    assert_eq!(
        resolve("nope").unwrap_err(),
        Error::UnknownPalette("nope".into())
    );
}

#[test]
fn custom_palette_parses_colors_and_an_optional_shadow() {
    let r = parse_custom("#ff0000, 00ff00,#00f").unwrap();
    assert_eq!(r.stops, [Rgb(255, 0, 0), Rgb(0, 255, 0), Rgb(0, 0, 255)]);
    assert_eq!(r.shadow, Rgb(0, 0, 255).darken(30));
    let r = parse_custom("#ff0000,#ffff00/#330000").unwrap();
    assert_eq!(r.shadow, Rgb(0x33, 0, 0));
    assert_eq!(resolve("#ff0000,#ffff00/#330000").unwrap(), r);
    let sixteen = vec!["#123456"; 16].join(",");
    assert_eq!(parse_custom(&sixteen).unwrap().stops.len(), 16);
}

#[test]
fn custom_palette_errors_name_the_bad_item() {
    let msg = |s: &str| parse_custom(s).unwrap_err().to_string();
    assert!(
        msg("#ff0000,#zz0000,#00ff00").contains("item 2 '#zz0000'"),
        "{}",
        msg("#ff0000,#zz0000,#00ff00")
    );
    assert!(msg("#ff0000,,#00ff00").contains("item 2 ''"));
    assert!(msg("#ff0000,#12345").contains("item 2 '#12345'"));
    assert!(msg("#ff0000,#00ff00/xyz").contains("shadow color 'xyz'"));
    assert!(msg("#ff0000").contains("1 colors given, use 2 to 16"));
    let seventeen = vec!["#123456"; 17].join(",");
    assert!(msg(&seventeen).contains("17 colors given"));
    assert!(matches!(
        resolve("#ff0000").unwrap_err(),
        Error::InvalidPalette(_)
    ));
}
