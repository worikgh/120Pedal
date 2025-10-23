//! Code to support the tuner

use ab_glyph::{Font, FontRef, Point, PxScale};
use simple::Rect;

use crate::app::App;

/// Convert a Unicode character into a bit map width and height `w` and `h`
fn char_to_bitmap(
    app: &App,
    c: char,
    w: usize,
    h: usize,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Get font first
    let font_fn = "assets/DejaVuSerif-Bold.ttf";
    let font = app.fonts.get(font_fn).expect("Failed to load font");
    let font_ref = FontRef::try_from_slice(font.as_slice())?;

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

/// Draw a character on the screen.
pub fn draw_char(r: &Rect, c: char, app: &mut App, colour: &[u8; 4]) {
    draw_char_xywh(
        r.x(),
        r.y(),
        r.width() as i32,
        r.height() as i32,
        c,
        app,
        colour,
    );
}
fn draw_char_xywh(x: i32, y: i32, w: i32, h: i32, c: char, app: &mut App, colour: &[u8; 4]) {
    let bitmap = char_to_bitmap(app, c, w as usize, h as usize)
        .expect("Get bitmap for note_rect: {note_rect:?}");
    app.set_colour(colour);
    for xx in 0..w {
        for yy in 0..h {
            let idx = (w * yy + xx) as usize;
            match bitmap.get(idx) {
                Some(0) => (),
                Some(_) => {
                    let fr = Rect::new(x + xx, y + yy, 1, 1);
                    app.fill_rect(fr);
                }
                None => panic!("Error gui: draw_char idx: {idx}  c {c}"),
            }
        }
    }
}
