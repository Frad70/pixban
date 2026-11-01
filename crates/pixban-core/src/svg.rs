use std::collections::HashMap;
use std::fmt::Write;

use crate::color::Rgb;
use crate::render::Grid;

struct Rect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

fn merge_rows(runs: &[(u32, u32, u32)]) -> Vec<Rect> {
    let mut rects: Vec<Rect> = Vec::new();
    let mut open: HashMap<(u32, u32), usize> = HashMap::new();
    let mut last_y: Option<u32> = None;
    let mut i = 0;
    while i < runs.len() {
        let y = runs[i].0;
        if last_y != Some(y.wrapping_sub(1)) {
            open.clear();
        }
        let mut next_open = HashMap::new();
        while i < runs.len() && runs[i].0 == y {
            let (_, x, w) = runs[i];
            let idx = match open.get(&(x, w)) {
                Some(&idx) => {
                    rects[idx].h += 1;
                    idx
                }
                None => {
                    rects.push(Rect { x, y, w, h: 1 });
                    rects.len() - 1
                }
            };
            next_open.insert((x, w), idx);
            i += 1;
        }
        open = next_open;
        last_y = Some(y);
    }
    rects
}

fn escape_title(text: &str, out: &mut String) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\t' | '\n' | '\r' => out.push(' '),
            c if c < ' ' => {}
            '\u{fffe}' | '\u{ffff}' => {}
            c => out.push(c),
        }
    }
}

pub fn to_svg(grid: &Grid, scale: u32, title: &str) -> String {
    let (w, h) = (grid.width(), grid.height());
    let background = grid.background();

    let mut order: Vec<Rgb> = Vec::new();
    let mut runs: HashMap<Rgb, Vec<(u32, u32, u32)>> = HashMap::new();
    for y in 0..h {
        let row = grid.row(y);
        let mut x = 0;
        while x < row.len() {
            let cell = row[x];
            let start = x;
            while x < row.len() && row[x] == cell {
                x += 1;
            }
            if cell.is_transparent() || background == Some(cell.rgb()) {
                continue;
            }
            let color = cell.rgb();
            runs.entry(color)
                .or_insert_with(|| {
                    order.push(color);
                    Vec::new()
                })
                .push((y, start as u32, (x - start) as u32));
        }
    }

    let mut out = String::new();
    let _ = write!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {w} {h}\" shape-rendering=\"crispEdges\" role=\"img\">\n<title>",
        w * scale,
        h * scale
    );
    escape_title(title, &mut out);
    out.push_str("</title>\n");
    if let Some(bg) = background {
        let _ = writeln!(out, "<path fill=\"{bg}\" d=\"M0 0h{w}v{h}h-{w}z\"/>");
    }
    for color in order {
        let _ = write!(out, "<path fill=\"{color}\" d=\"");
        for r in merge_rows(&runs[&color]) {
            let _ = write!(out, "M{} {}h{}v{}h-{}z", r.x, r.y, r.w, r.h, r.w);
        }
        out.push_str("\"/>\n");
    }
    out.push_str("</svg>\n");
    out
}
