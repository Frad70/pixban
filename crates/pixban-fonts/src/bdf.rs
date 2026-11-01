use crate::import::{RawFont, RawGlyph};

fn hex_byte(s: &[u8]) -> Option<u8> {
    let digit = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    Some(digit(*s.first()?)? << 4 | digit(*s.get(1)?)?)
}

fn int(word: Option<&str>, line: usize, what: &str) -> Result<i32, String> {
    word.and_then(|w| w.parse().ok())
        .ok_or_else(|| format!("line {line}: {what} needs a number"))
}

pub(crate) fn parse(src: &str) -> Result<RawFont, String> {
    let lines: Vec<&str> = src.lines().map(str::trim).collect();
    let mut font = RawFont::default();
    let mut started = false;
    let mut i = 0;
    while i < lines.len() {
        let ln = i + 1;
        let line = lines[i];
        i += 1;
        let mut words = line.split_whitespace();
        let key = words.next().unwrap_or("");
        if key.is_empty() || key == "COMMENT" {
            continue;
        }
        if !started {
            if key != "STARTFONT" {
                return Err(format!("line {ln}: expected STARTFONT"));
            }
            started = true;
            continue;
        }
        match key {
            "FONTBOUNDINGBOX" => {
                let w = int(words.next(), ln, "FONTBOUNDINGBOX width")?;
                let h = int(words.next(), ln, "FONTBOUNDINGBOX height")?;
                let _x = int(words.next(), ln, "FONTBOUNDINGBOX x")?;
                let y = int(words.next(), ln, "FONTBOUNDINGBOX y")?;
                if w < 0 || h < 0 {
                    return Err(format!("line {ln}: FONTBOUNDINGBOX has a negative size"));
                }
                font.bbox_width = Some(w as u32);
                font.bbox_height = Some(h);
                font.bbox_y = Some(y);
            }
            "STARTPROPERTIES" => loop {
                let Some(raw) = lines.get(i) else {
                    return Err(format!("line {ln}: STARTPROPERTIES without ENDPROPERTIES"));
                };
                let pln = i + 1;
                i += 1;
                let mut parts = raw.split_whitespace();
                match parts.next() {
                    Some("ENDPROPERTIES") => break,
                    Some("FONT_ASCENT") => {
                        font.ascent = Some(int(parts.next(), pln, "FONT_ASCENT")?)
                    }
                    Some("FONT_DESCENT") => {
                        font.descent = Some(int(parts.next(), pln, "FONT_DESCENT")?)
                    }
                    Some("DEFAULT_CHAR") => {
                        let v = int(parts.next(), pln, "DEFAULT_CHAR")?;
                        font.default_char = u32::try_from(v).ok();
                    }
                    _ => {}
                }
            },
            "STARTCHAR" => {
                let mut code: i64 = -1;
                let mut advance = 0;
                let mut bbx = None;
                let mut bits = Vec::new();
                loop {
                    let Some(raw) = lines.get(i) else {
                        return Err(format!("line {ln}: STARTCHAR without ENDCHAR"));
                    };
                    let cln = i + 1;
                    i += 1;
                    let mut parts = raw.split_whitespace();
                    match parts.next() {
                        Some("ENDCHAR") => break,
                        Some("ENCODING") => code = i64::from(int(parts.next(), cln, "ENCODING")?),
                        Some("DWIDTH") => advance = int(parts.next(), cln, "DWIDTH")?,
                        Some("BBX") => {
                            let w = int(parts.next(), cln, "BBX width")?;
                            let h = int(parts.next(), cln, "BBX height")?;
                            let x = int(parts.next(), cln, "BBX x")?;
                            let y = int(parts.next(), cln, "BBX y")?;
                            if w < 0 || h < 0 {
                                return Err(format!("line {cln}: BBX has a negative size"));
                            }
                            bbx = Some((w as u32, h as u32, x, y));
                        }
                        Some("BITMAP") => {
                            let Some((w, h, _, _)) = bbx else {
                                return Err(format!("line {cln}: BITMAP before BBX"));
                            };
                            let per_row = (w as usize).div_ceil(8);
                            for _ in 0..h {
                                let Some(row) = lines.get(i) else {
                                    return Err(format!("line {cln}: BITMAP ends early"));
                                };
                                let rln = i + 1;
                                i += 1;
                                let hex = row.as_bytes();
                                if hex.len() != per_row * 2 {
                                    return Err(format!(
                                        "line {rln}: bitmap row has {} hex digits, expected {}",
                                        hex.len(),
                                        per_row * 2
                                    ));
                                }
                                for b in 0..per_row {
                                    let byte = hex_byte(&hex[2 * b..]).ok_or_else(|| {
                                        format!("line {rln}: bitmap row is not hexadecimal")
                                    })?;
                                    for x in 0..(w as usize).min(8 * (b + 1)) - 8 * b {
                                        bits.push(byte >> (7 - x) & 1 == 1);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                let Some((width, height, x, y)) = bbx else {
                    return Err(format!("line {ln}: glyph has no BBX"));
                };
                if code >= 0 {
                    font.glyphs.push(RawGlyph {
                        code: code as u32,
                        advance,
                        width,
                        height,
                        x,
                        y,
                        bits,
                    });
                }
            }
            "ENDFONT" => break,
            _ => {}
        }
    }
    if !started {
        return Err("empty file".to_string());
    }
    Ok(font)
}
