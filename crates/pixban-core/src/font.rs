use std::collections::HashMap;

use crate::error::ParseError;
use crate::limits::{MAX_FONT_HEIGHT, MAX_GLYPH_WIDTH, MAX_KERN, MAX_SPACING};

pub const DEFAULT_SCALE: u32 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaseMode {
    Upper,
    Mixed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Glyph {
    width: u32,
    height: u32,
    bits: Vec<bool>,
}

impl Glyph {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn get(&self, x: u32, y: u32) -> bool {
        x < self.width && y < self.height && self.bits[(y * self.width + x) as usize]
    }

    fn hollow_box(height: u32) -> Glyph {
        let width = ((height * 5 + 3) / 7).max(3);
        let mut bits = vec![false; (width * height) as usize];
        for y in 0..height {
            for x in 0..width {
                let edge = x == 0 || y == 0 || x == width - 1 || y == height - 1;
                bits[(y * width + x) as usize] = edge;
            }
        }
        Glyph {
            width,
            height,
            bits,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Font {
    name: String,
    height: u32,
    baseline: u32,
    spacing: u32,
    case: CaseMode,
    default_scale: u32,
    glyphs: HashMap<char, Glyph>,
    fallback: Glyph,
    kerning: HashMap<(char, char), i32>,
}

impl Font {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn baseline(&self) -> u32 {
        self.baseline
    }

    pub fn spacing(&self) -> u32 {
        self.spacing
    }

    pub fn case(&self) -> CaseMode {
        self.case
    }

    pub fn default_scale(&self) -> u32 {
        self.default_scale
    }

    pub fn glyph(&self, c: char) -> Option<&Glyph> {
        self.glyphs.get(&c)
    }

    pub fn fallback(&self) -> &Glyph {
        &self.fallback
    }

    pub fn kerning(&self, left: char, right: char) -> i32 {
        self.kerning.get(&(left, right)).copied().unwrap_or(0)
    }

    pub fn chars(&self) -> Vec<char> {
        let mut v: Vec<char> = self.glyphs.keys().copied().collect();
        v.sort_unstable();
        v
    }
}

struct Located<T> {
    value: T,
    line: usize,
}

fn err(line: usize, message: impl Into<String>) -> ParseError {
    ParseError {
        line,
        message: message.into(),
    }
}

fn parse_number<T: std::str::FromStr>(
    line: usize,
    key: &str,
    value: &str,
) -> Result<Located<T>, ParseError> {
    value
        .parse()
        .map(|value| Located { value, line })
        .map_err(|_| err(line, format!("'{key}' needs a number, found '{value}'")))
}

fn set_once<T>(slot: &mut Option<Located<T>>, key: &str, v: Located<T>) -> Result<(), ParseError> {
    if slot.is_some() {
        return Err(err(v.line, format!("duplicate '{key}'")));
    }
    *slot = Some(v);
    Ok(())
}

fn require<T>(slot: Option<Located<T>>, key: &str, line: usize) -> Result<Located<T>, ParseError> {
    slot.ok_or_else(|| err(line, format!("missing header '{key}'")))
}

fn parse_glyph_name(line: usize, name: &str) -> Result<Option<char>, ParseError> {
    match name {
        "fallback" => Ok(None),
        "space" => Ok(Some(' ')),
        _ => {
            if let Some(hex) = name.strip_prefix("U+") {
                return u32::from_str_radix(hex, 16)
                    .ok()
                    .and_then(char::from_u32)
                    .map(Some)
                    .ok_or_else(|| err(line, format!("invalid code point '{name}'")));
            }
            let mut it = name.chars();
            match (it.next(), it.next()) {
                (Some(c), None) => Ok(Some(c)),
                _ => Err(err(
                    line,
                    format!("invalid glyph name '{name}', use one character, 'space', 'fallback' or U+XXXX"),
                )),
            }
        }
    }
}

pub fn parse(src: &str) -> Result<Font, ParseError> {
    let src = src.strip_prefix('\u{feff}').unwrap_or(src);
    let lines: Vec<&str> = src.lines().collect();

    let mut name = None;
    let mut height = None;
    let mut baseline = None;
    let mut spacing = None;
    let mut case = None;
    let mut scale = None;
    let mut kerning: HashMap<(char, char), i32> = HashMap::new();

    let mut idx = 0;
    let separator_line = loop {
        let Some(raw) = lines.get(idx) else {
            return Err(err(
                lines.len() + 1,
                "missing '---' separator before glyphs",
            ));
        };
        idx += 1;
        let ln = idx;
        let text = raw.trim();
        if text.is_empty() || text.starts_with('#') {
            continue;
        }
        if text == "---" {
            break ln;
        }
        let Some((key, value)) = text.split_once(':') else {
            return Err(err(ln, "expected 'key: value' or '---'"));
        };
        let (key, value) = (key.trim(), value.trim());
        match key {
            "name" => {
                if value.is_empty() {
                    return Err(err(ln, "'name' is empty"));
                }
                set_once(
                    &mut name,
                    key,
                    Located {
                        value: value.to_string(),
                        line: ln,
                    },
                )?;
            }
            "height" => set_once(&mut height, key, parse_number::<u32>(ln, key, value)?)?,
            "baseline" => set_once(&mut baseline, key, parse_number::<u32>(ln, key, value)?)?,
            "spacing" => set_once(&mut spacing, key, parse_number::<u32>(ln, key, value)?)?,
            "scale" => set_once(&mut scale, key, parse_number::<u32>(ln, key, value)?)?,
            "case" => {
                let mode = match value {
                    "upper" => CaseMode::Upper,
                    "mixed" => CaseMode::Mixed,
                    _ => {
                        return Err(err(
                            ln,
                            format!("'case' must be 'upper' or 'mixed', found '{value}'"),
                        ))
                    }
                };
                set_once(
                    &mut case,
                    key,
                    Located {
                        value: mode,
                        line: ln,
                    },
                )?;
            }
            "kern" => {
                let mut parts = value.split_whitespace();
                let (Some(pair), Some(amount), None) = (parts.next(), parts.next(), parts.next())
                else {
                    return Err(err(ln, "expected 'kern: AB amount'"));
                };
                let mut pc = pair.chars();
                let (Some(a), Some(b), None) = (pc.next(), pc.next(), pc.next()) else {
                    return Err(err(
                        ln,
                        format!("kerning pair '{pair}' must be exactly two characters"),
                    ));
                };
                let amount: i32 = amount
                    .parse()
                    .map_err(|_| err(ln, format!("kerning amount '{amount}' is not a number")))?;
                if !(-MAX_KERN..=MAX_KERN).contains(&amount) {
                    return Err(err(
                        ln,
                        format!("kerning amount must be between -{MAX_KERN} and {MAX_KERN}"),
                    ));
                }
                if kerning.insert((a, b), amount).is_some() {
                    return Err(err(ln, format!("duplicate kerning pair '{pair}'")));
                }
            }
            _ => return Err(err(ln, format!("unknown header '{key}'"))),
        }
    };

    let name = require(name, "name", separator_line)?;
    let height = require(height, "height", separator_line)?;
    let baseline = require(baseline, "baseline", separator_line)?;
    let spacing = require(spacing, "spacing", separator_line)?;
    let case = require(case, "case", separator_line)?;

    if !(1..=MAX_FONT_HEIGHT).contains(&height.value) {
        return Err(err(
            height.line,
            format!("'height' must be between 1 and {MAX_FONT_HEIGHT}"),
        ));
    }
    if !(1..=height.value).contains(&baseline.value) {
        return Err(err(
            baseline.line,
            "'baseline' must be between 1 and 'height'",
        ));
    }
    if spacing.value > MAX_SPACING {
        return Err(err(
            spacing.line,
            format!("'spacing' must be at most {MAX_SPACING}"),
        ));
    }
    let default_scale = match scale {
        Some(s) if !(1..=crate::limits::MAX_SCALE).contains(&s.value) => {
            return Err(err(
                s.line,
                format!("'scale' must be between 1 and {}", crate::limits::MAX_SCALE),
            ));
        }
        Some(s) => s.value,
        None => DEFAULT_SCALE,
    };
    let h = height.value;

    let mut glyphs: HashMap<char, Glyph> = HashMap::new();
    let mut fallback: Option<Glyph> = None;

    while idx < lines.len() {
        let ln = idx + 1;
        let text = lines[idx].trim();
        idx += 1;
        if text.is_empty() {
            continue;
        }
        let Some(rest) = text.strip_prefix("char:") else {
            if text.chars().all(|c| c == '#' || c == '.') {
                return Err(err(ln, format!("glyph has more than {h} rows")));
            }
            return Err(err(ln, "expected 'char: X'"));
        };
        let key = parse_glyph_name(ln, rest.trim())?;
        let label = rest.trim().to_string();

        let mut rows: Vec<&str> = Vec::with_capacity(h as usize);
        for k in 0..h {
            let row_ln = idx + 1;
            let row = lines.get(idx).map(|r| r.trim());
            match row {
                None | Some("") => {
                    return Err(err(
                        row_ln,
                        format!("glyph '{label}' has {k} rows, expected {h}"),
                    ));
                }
                Some(r) if r.starts_with("char:") => {
                    return Err(err(
                        row_ln,
                        format!("glyph '{label}' has {k} rows, expected {h}"),
                    ));
                }
                Some(r) => {
                    if let Some(bad) = r.chars().find(|c| *c != '#' && *c != '.') {
                        return Err(err(
                            row_ln,
                            format!("invalid character '{bad}' in glyph '{label}', rows use '#' and '.'"),
                        ));
                    }
                    rows.push(r);
                }
            }
            idx += 1;
        }
        let width = rows[0].chars().count() as u32;
        if width > MAX_GLYPH_WIDTH {
            return Err(err(
                ln + 1,
                format!("glyph '{label}' is {width} wide, the limit is {MAX_GLYPH_WIDTH}"),
            ));
        }
        for (k, r) in rows.iter().enumerate() {
            let w = r.chars().count() as u32;
            if w != width {
                return Err(err(
                    ln + 1 + k,
                    format!("row is {w} wide but the first row of glyph '{label}' is {width}"),
                ));
            }
        }
        let bits: Vec<bool> = rows
            .iter()
            .flat_map(|r| r.chars().map(|c| c == '#'))
            .collect();
        let glyph = Glyph {
            width,
            height: h,
            bits,
        };
        match key {
            None => {
                if fallback.replace(glyph).is_some() {
                    return Err(err(ln, "duplicate glyph 'fallback'"));
                }
            }
            Some(c) => {
                if glyphs.insert(c, glyph).is_some() {
                    return Err(err(ln, format!("duplicate glyph '{label}'")));
                }
            }
        }
    }

    Ok(Font {
        name: name.value,
        height: h,
        baseline: baseline.value,
        spacing: spacing.value,
        case: case.value,
        default_scale,
        glyphs,
        fallback: fallback.unwrap_or_else(|| Glyph::hollow_box(h)),
        kerning,
    })
}
