//! Animated PNG writer: full alpha, frames stored as the rectangle that changed since the
//! frame before, infinite loop. The default image of the file is frame 0.

use crate::anim::Animated;
use crate::error::Error;
use crate::render::Grid;

fn enc(e: png::EncodingError) -> Error {
    Error::Encode(e.to_string())
}

fn changed(grid: &Grid, before: &Grid) -> (u32, u32, u32, u32) {
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if grid.get(x, y) != before.get(x, y) {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x);
                y1 = y1.max(y);
            }
        }
    }
    if x0 == u32::MAX {
        (0, 0, 1, 1)
    } else {
        (x0, y0, x1 - x0 + 1, y1 - y0 + 1)
    }
}

pub(crate) fn to_apng(a: &Animated) -> Result<Vec<u8>, Error> {
    let frames = a.frames();
    let scale = a.scale();
    let (w, h) = a.pixel_size();
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, w, h);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_animated(frames.len() as u32, 0).map_err(enc)?;
    encoder.set_blend_op(png::BlendOp::Source).map_err(enc)?;
    encoder.set_dispose_op(png::DisposeOp::None).map_err(enc)?;
    let mut writer = encoder.write_header().map_err(enc)?;
    for (k, grid) in frames.iter().enumerate() {
        let (x, y, cw, ch) = if k == 0 {
            (0, 0, grid.width(), grid.height())
        } else {
            changed(grid, &frames[k - 1])
        };
        writer
            .set_frame_delay(a.frame_ms() as u16, 1000)
            .map_err(enc)?;
        writer.set_frame_position(0, 0).map_err(enc)?;
        writer
            .set_frame_dimension(cw * scale, ch * scale)
            .map_err(enc)?;
        writer
            .set_frame_position(x * scale, y * scale)
            .map_err(enc)?;
        let mut data = Vec::with_capacity((cw * ch * scale * scale * 4) as usize);
        let mut line = Vec::with_capacity((cw * scale * 4) as usize);
        for yy in y..y + ch {
            line.clear();
            for xx in x..x + cw {
                let c = grid.get(xx, yy);
                for _ in 0..scale {
                    line.extend_from_slice(&[c.0, c.1, c.2, c.3]);
                }
            }
            for _ in 0..scale {
                data.extend_from_slice(&line);
            }
        }
        writer.write_image_data(&data).map_err(enc)?;
    }
    writer.finish().map_err(enc)?;
    Ok(out)
}
