use crate::error::Error;
use crate::render::Grid;

pub fn to_png(grid: &Grid, scale: u32) -> Result<Vec<u8>, Error> {
    let enc_err = |e: png::EncodingError| Error::Encode(e.to_string());
    let (w, h) = (grid.width() * scale, grid.height() * scale);
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, w, h);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(enc_err)?;
    {
        let mut stream = writer.stream_writer().map_err(enc_err)?;
        let mut line = Vec::with_capacity(w as usize * 4);
        for y in 0..grid.height() {
            line.clear();
            for cell in grid.row(y) {
                for _ in 0..scale {
                    line.extend_from_slice(&[cell.0, cell.1, cell.2, cell.3]);
                }
            }
            for _ in 0..scale {
                std::io::Write::write_all(&mut stream, &line)
                    .map_err(|e| Error::Encode(e.to_string()))?;
            }
        }
        stream.finish().map_err(enc_err)?;
    }
    writer.finish().map_err(enc_err)?;
    Ok(out)
}
