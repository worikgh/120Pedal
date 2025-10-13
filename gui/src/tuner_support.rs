//! Code to support the tuner

use std::{fs::File, io::Read};

use ab_glyph::{Font, FontRef, Point, PxScale};

/// Convert a Unicode character into a bit map width and height `w` and `h`
pub fn char_to_bitmap(c: char, w: usize, h: usize) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Get font first
    let font_fn = "assets/DejaVuSerif-Bold.ttf";
    let font_data = match File::open(font_fn) {
        Ok(mut f) => {
            let mut v = Vec::new();
            f.read_to_end(&mut v).expect("Reading djv font from file");
            v
        }
        Err(err) => panic!("Error gui: Font {font_fn} could not be loaded. {err}"),
    };
    let font_ref = FontRef::try_from_slice(font_data.as_slice())?;

    // Scale the font to fit the desired dimensions
    let scale = PxScale::from((h as f32) * 0.8); // 80% of height to allow for descent

    // Create a glyph for the character
    let id = font_ref.glyph_id(c);
    let mut glyph = id.with_scale(scale);
    glyph.position = Point {
        x: 0.0,
        y: h as f32,
    };

    // Create output buffer
    let mut bitmap = vec![0u8; w * h];

    // Rasterize the glyph
    if let Some(outline) = font_ref.outline_glyph(glyph) {
        outline.draw(|x, y, v| {
            let x = x as usize;
            let y = y as usize;
            if x < w && y < h {
                // Convert alpha to binary (you can adjust the threshold)
                bitmap[y * w + x] = if v > 0.5 { 255 } else { 0 };
            }
        });
    }

    Ok(bitmap)
}
