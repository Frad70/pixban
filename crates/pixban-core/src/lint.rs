use crate::font::{CaseMode, Font};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Script {
    LatinUpper,
    LatinLower,
    Digits,
    CyrillicUpper,
    CyrillicLower,
    Punctuation,
}

impl Script {
    pub const ALL: [Script; 6] = [
        Script::LatinUpper,
        Script::LatinLower,
        Script::Digits,
        Script::CyrillicUpper,
        Script::CyrillicLower,
        Script::Punctuation,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Script::LatinUpper => "Latin upper",
            Script::LatinLower => "Latin lower",
            Script::Digits => "digits",
            Script::CyrillicUpper => "Cyrillic upper",
            Script::CyrillicLower => "Cyrillic lower",
            Script::Punctuation => "punctuation",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Script::LatinUpper => "latin-upper",
            Script::LatinLower => "latin-lower",
            Script::Digits => "digits",
            Script::CyrillicUpper => "cyrillic-upper",
            Script::CyrillicLower => "cyrillic-lower",
            Script::Punctuation => "punctuation",
        }
    }

    /// Characters the script needs in a font of this case mode. Caps fonts map
    /// lowercase input to uppercase, so they need no lowercase glyphs.
    pub fn expected(self, case: CaseMode) -> Vec<char> {
        let mixed = case == CaseMode::Mixed;
        match self {
            Script::LatinUpper => ('A'..='Z').collect(),
            Script::LatinLower if mixed => ('a'..='z').collect(),
            Script::LatinLower => Vec::new(),
            Script::Digits => ('0'..='9').collect(),
            Script::CyrillicUpper => ('\u{410}'..='\u{42F}').chain(['\u{401}']).collect(),
            Script::CyrillicLower if mixed => ('\u{430}'..='\u{44F}').chain(['\u{451}']).collect(),
            Script::CyrillicLower => Vec::new(),
            Script::Punctuation => {
                let mut v: Vec<char> = (' '..='/')
                    .chain(':'..='@')
                    .chain('['..='`')
                    .chain('{'..='~')
                    .collect();
                v.extend(['\u{AB}', '\u{BB}', '\u{2014}', '\u{2026}', '\u{2116}']);
                v
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Coverage {
    pub script: Script,
    pub covered: usize,
    pub expected: usize,
    pub missing: Vec<char>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub coverage: Vec<Coverage>,
}

/// Structural checks of a font plus glyph coverage per script.
pub fn lint(font: &Font) -> Report {
    let mut report = Report::default();
    let band = font.height();

    for c in font.chars() {
        let Some(g) = font.glyph(c) else { continue };
        let label = format!("{c:?}");
        if g.height() != band + g.top() + g.bottom() {
            report.errors.push(format!(
                "glyph {label} has {} rows, expected band {band} plus {} above and {} below",
                g.height(),
                g.top(),
                g.bottom()
            ));
        }
        if c != ' ' && !g.has_ink() {
            report.errors.push(format!("glyph {label} has no ink"));
        }
        if g.width() > font.max_width() {
            report.errors.push(format!(
                "glyph {label} is {} wide, max_width is {}",
                g.width(),
                font.max_width()
            ));
        }
        if g.top() > 0 && !(0..g.top()).any(|y| (0..g.width()).any(|x| g.get(x, y))) {
            report.warnings.push(format!(
                "glyph {label} declares {} top rows with no ink",
                g.top()
            ));
        }
        if g.bottom() > 0 {
            let first = g.top() + band;
            let empty = (first..g.height()).all(|y| (0..g.width()).all(|x| !g.get(x, y)));
            if empty {
                report.warnings.push(format!(
                    "glyph {label} declares {} bottom rows with no ink",
                    g.bottom()
                ));
            }
        }
    }

    let fb = font.fallback();
    if fb.height() != band + fb.top() + fb.bottom() || !fb.has_ink() {
        report
            .errors
            .push("fallback glyph is empty or has the wrong height".to_string());
    }

    for ((a, b), _) in font.kerning_pairs() {
        if font.glyph(a).is_none() || font.glyph(b).is_none() {
            report.warnings.push(format!(
                "kerning pair {a}{b} uses a character without a glyph"
            ));
        }
    }

    for script in Script::ALL {
        let expected = script.expected(font.case());
        let missing: Vec<char> = expected
            .iter()
            .copied()
            .filter(|c| font.glyph(*c).is_none())
            .collect();
        report.coverage.push(Coverage {
            script,
            covered: expected.len() - missing.len(),
            expected: expected.len(),
            missing,
        });
    }
    report
}
