use std::collections::HashMap;
use std::fmt::Write;

use crate::anim::{Animated, Plan};
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

type Runs = HashMap<Rgb, Vec<(u32, u32, u32)>>;

/// Horizontal runs of every color, with the colors in order of first appearance. Cells that
/// are transparent or equal to the grid's background color are left out.
fn color_runs(grid: &Grid) -> (Vec<Rgb>, Runs) {
    let (h, background) = (grid.height(), grid.background());
    let mut order: Vec<Rgb> = Vec::new();
    let mut runs: Runs = HashMap::new();
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
    (order, runs)
}

fn path_data(runs: &[(u32, u32, u32)]) -> String {
    let mut d = String::new();
    for r in merge_rows(runs) {
        let _ = write!(d, "M{} {}h{}v{}h-{}z", r.x, r.y, r.w, r.h, r.w);
    }
    d
}

/// The background path, if any, and one path per color.
fn write_grid(grid: &Grid, out: &mut String) {
    let (w, h) = (grid.width(), grid.height());
    if let Some(bg) = grid.background() {
        let _ = writeln!(out, "<path fill=\"{bg}\" d=\"M0 0h{w}v{h}h-{w}z\"/>");
    }
    let (order, runs) = color_runs(grid);
    for color in order {
        let _ = writeln!(
            out,
            "<path fill=\"{color}\" d=\"{}\"/>",
            path_data(&runs[&color])
        );
    }
}

fn open_svg(grid_w: u32, grid_h: u32, scale: u32, title: &str, out: &mut String) {
    let _ = write!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {grid_w} {grid_h}\" shape-rendering=\"crispEdges\" role=\"img\">\n<title>",
        grid_w * scale,
        grid_h * scale
    );
    escape_title(title, out);
    out.push_str("</title>\n");
}

pub fn to_svg(grid: &Grid, scale: u32, title: &str) -> String {
    let mut out = String::new();
    open_svg(grid.width(), grid.height(), scale, title, &mut out);
    write_grid(grid, &mut out);
    out.push_str("</svg>\n");
    out
}

/// The cells of `frame` that differ from `base`, as a transparent grid holding only those
/// cells. `None` when a cell turns transparent, which painting over `base` cannot undo.
pub(crate) fn added_cells(base: &Grid, frame: &Grid) -> Option<Grid> {
    let mut out = frame.blank_like();
    for y in 0..frame.height() {
        for x in 0..frame.width() {
            let (was, now) = (base.get(x, y), frame.get(x, y));
            if was == now {
                continue;
            }
            if now.is_transparent() {
                return None;
            }
            out.set_cell(x, y, now);
        }
    }
    Some(out)
}

fn is_blank(grid: &Grid) -> bool {
    (0..grid.height()).all(|y| grid.row(y).iter().all(|c| c.is_transparent()))
}

fn percent(k: usize, n: usize) -> String {
    let milli = k * 100_000 / n;
    let (whole, frac) = (milli / 1000, milli % 1000);
    if frac == 0 {
        format!("{whole}%")
    } else {
        let digits = format!("{frac:03}");
        format!("{whole}.{}%", digits.trim_end_matches('0'))
    }
}

/// Keyframes that hold one declaration per frame, merging runs of equal ones.
fn keyframes(values: &[String]) -> String {
    let n = values.len();
    let mut out = String::new();
    let mut prev: Option<&String> = None;
    for (k, v) in values.iter().enumerate() {
        if prev == Some(v) {
            continue;
        }
        let _ = write!(out, "{}{{{v}}}", percent(k, n));
        prev = Some(v);
    }
    out
}

/// Keyframe rules by body, so layers that move alike share a rule.
#[derive(Default)]
struct Rules {
    names: HashMap<String, String>,
    css: String,
}

impl Rules {
    fn name(&mut self, values: &[String]) -> String {
        let body = keyframes(values);
        if let Some(n) = self.names.get(&body) {
            return n.clone();
        }
        let name = format!("k{}", self.names.len());
        let _ = write!(self.css, "@keyframes {name}{{{body}}}");
        self.names.insert(body, name.clone());
        name
    }
}

fn visibility(shown: &[bool]) -> Vec<String> {
    shown
        .iter()
        .map(|&s| format!("visibility:{}", if s { "visible" } else { "hidden" }))
        .collect()
}

fn open_group(classes: &str, rule: &str, out: &mut String) {
    let _ = writeln!(
        out,
        "<g class=\"{classes}\" style=\"animation-name:{rule}\">"
    );
}

fn opacity(percent: u32) -> String {
    match percent {
        100 => "1".to_string(),
        p if p % 10 == 0 => format!(".{}", p / 10),
        p => format!(".{p:02}"),
    }
}

pub(crate) fn animated_svg(a: &Animated) -> String {
    let mut rules = Rules::default();
    let mut body = String::new();
    let mut extra_css = String::new();
    match &a.plan {
        Plan::Frames => frames_body(a, &mut rules, &mut body),
        Plan::Shimmer {
            band,
            width,
            percent: pct,
            ink,
        } => {
            let h = a.rest.height();
            write_grid(&a.rest, &mut body);
            let (_, runs) = color_runs(ink);
            let mut clip: Vec<(u32, u32, u32)> = runs.values().flatten().copied().collect();
            clip.sort_unstable();
            let _ = writeln!(
                body,
                "<clipPath id=\"c\"><path d=\"{}\"/></clipPath>",
                path_data(&clip)
            );
            let mut d = String::new();
            for y in 0..h {
                let _ = write!(d, "M{} {y}h{width}v1h-{width}z", -(y as i64));
            }
            let moves: Vec<String> = band
                .iter()
                .map(|b| format!("transform:translate({}px,0)", b.unwrap_or(-1000)))
                .collect();
            let rule = rules.name(&moves);
            let _ = writeln!(body, "<g clip-path=\"url(#c)\">");
            let _ = writeln!(
                body,
                "<path class=\"a b\" style=\"animation-name:{rule}\" fill=\"#fff\" fill-opacity=\"{}\" d=\"{d}\"/>",
                opacity(*pct)
            );
            body.push_str("</g>\n");
            extra_css.push_str(".b{transform:translate(-1000px,0)}");
        }
        Plan::Wave {
            base,
            letters,
            offsets,
        } => {
            write_grid(base, &mut body);
            for (layer, moves) in letters.iter().zip(offsets) {
                let values: Vec<String> = moves
                    .iter()
                    .map(|(dx, dy)| format!("transform:translate({dx}px,{dy}px)"))
                    .collect();
                let rule = rules.name(&values);
                open_group("a", &rule, &mut body);
                write_grid(layer, &mut body);
                body.push_str("</g>\n");
            }
        }
        Plan::Type {
            base,
            reveal,
            counts,
            cursor,
            cursor_size,
            cursor_color,
        } => {
            write_grid(base, &mut body);
            for (c, layer) in reveal.iter().enumerate() {
                let shown: Vec<bool> = counts.iter().map(|&k| k as usize > c).collect();
                let rule = rules.name(&visibility(&shown));
                open_group("a", &rule, &mut body);
                write_grid(layer, &mut body);
                body.push_str("</g>\n");
            }
            let values: Vec<String> = cursor
                .iter()
                .map(|&(x, y, lit)| {
                    format!(
                        "visibility:{};transform:translate({x}px,{y}px)",
                        if lit { "visible" } else { "hidden" }
                    )
                })
                .collect();
            let rule = rules.name(&values);
            let (cw, ch) = cursor_size;
            let _ = writeln!(
                body,
                "<path class=\"a h\" style=\"animation-name:{rule}\" fill=\"{cursor_color}\" d=\"M0 0h{cw}v{ch}h-{cw}z\"/>"
            );
        }
    }
    let mut out = String::new();
    open_svg(a.rest.width(), a.rest.height(), a.scale, &a.title, &mut out);
    let total = a.frame_ms as usize * a.frames.len();
    let _ = writeln!(
        out,
        "<style>.a{{animation-duration:{total}ms;animation-timing-function:step-end;animation-iteration-count:infinite}}.h{{visibility:hidden}}{extra_css}{}@media (prefers-reduced-motion:reduce){{.a{{animation:none!important}}}}</style>",
        rules.css
    );
    out.push_str(&body);
    out.push_str("</svg>\n");
    out
}

/// Frames rebuilt by painting: the still logo stays, and each distinct frame brings the cells
/// that differ from it. A frame that has to hide parts of the still logo brings itself whole
/// and hides the still logo while it is up.
fn frames_body(a: &Animated, rules: &mut Rules, out: &mut String) {
    let n = a.frames.len();
    struct Layer {
        grid: Grid,
        shown: Vec<bool>,
    }
    let mut layers: Vec<Layer> = Vec::new();
    let mut base_shown = vec![true; n];
    for (k, frame) in a.frames.iter().enumerate() {
        let grid = match added_cells(&a.rest, frame) {
            Some(diff) if is_blank(&diff) => continue,
            Some(diff) => diff,
            None => {
                base_shown[k] = false;
                frame.clone()
            }
        };
        match layers.iter_mut().find(|l| l.grid == grid) {
            Some(l) => l.shown[k] = true,
            None => {
                let mut shown = vec![false; n];
                shown[k] = true;
                layers.push(Layer { grid, shown });
            }
        }
    }
    if base_shown.iter().all(|&s| s) {
        write_grid(&a.rest, out);
    } else {
        let rule = rules.name(&visibility(&base_shown));
        open_group("a", &rule, out);
        write_grid(&a.rest, out);
        out.push_str("</g>\n");
    }
    for layer in &layers {
        let rule = rules.name(&visibility(&layer.shown));
        open_group("a h", &rule, out);
        write_grid(&layer.grid, out);
        out.push_str("</g>\n");
    }
}
