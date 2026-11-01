use pixban_core::{font, Font};

const BUILTIN: &[(&str, &str)] = &[("block", include_str!("../fonts/block.pxf"))];

pub fn names() -> impl Iterator<Item = &'static str> {
    BUILTIN.iter().map(|(name, _)| *name)
}

pub fn source(name: &str) -> Option<&'static str> {
    BUILTIN
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, src)| *src)
}

pub fn get(name: &str) -> Option<Font> {
    source(name).map(|src| font::parse(src).expect("built-in font is valid"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_builtin_font_parses() {
        for name in names() {
            let font = get(name).unwrap();
            assert_eq!(font.name().to_lowercase(), name);
        }
    }

    #[test]
    fn block_has_the_logo_letters() {
        let font = get("Block").unwrap();
        assert_eq!(font.height(), 7);
        assert_eq!(font.spacing(), 2);
        for c in "BRAZIE".chars() {
            assert_eq!(font.glyph(c).unwrap().width(), 6, "glyph {c}");
        }
        assert!(font.glyph('Q').is_none());
    }

    #[test]
    fn unknown_font_is_none() {
        assert!(get("nope").is_none());
    }
}
