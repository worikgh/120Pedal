//! The outer structure
use simple::Window;

use crate::Rect;
use std::io::Read;
use std::{collections::HashMap, fs::File};

pub struct App {
    pub window: simple::Window,
    pub model_width: u16,
    pub model_height: u16,
    pub fonts: HashMap<String, Vec<u8>>,
}
impl App {
    /// Sized
    pub fn new(name: &str, view_width: u16, view_height: u16) -> Self {
        Self::new_inner(name, Some((view_width, view_height)))
    }

    /// Full screen
    pub fn new_fullscreen(name: &str) -> Self {
        Self::new_inner(name, None)
    }
    fn new_inner(name: &str, dim: Option<(u16, u16)>) -> Self {
        let view_width: u16;
        let view_height: u16;
        let window = if let Some((w, h)) = dim {
            view_width = w;
            view_height = h;
            simple::Window::new(name, view_width, view_height)
        } else {
            let window = simple::Window::new_fullscreen(name);
            let (w, h) = window.drawable_size();
            view_width = w as u16;
            view_height = h as u16;
            window
        };
        eprintln!("DBG gui: WxH {view_width}x{view_height}");
        let font_fn = "assets/DejaVuSerif-Bold.ttf";
        let font = match File::open(font_fn) {
            Ok(mut f) => {
                let mut v: Vec<u8> = Vec::new();
                f.read_to_end(&mut v).expect("Reading djv font from file");
                v
            }
            Err(err) => panic!("Error gui: Font {font_fn} could not be loaded. {err}"),
        };
        let mut fonts = HashMap::new();
        fonts.insert(font_fn.to_string(), font);

        // Rotation
        let model_width = view_height;
        let model_height = view_height;
        Self {
            model_width,
            model_height,
            window,
            fonts,
        }
    }

    /// The colour the next drawing operation will use
    pub fn set_colour(&mut self, colour: &[u8; 4]) {
        self.window
            .set_color(colour[0], colour[1], colour[2], colour[3]);
    }

    /// Wrapper around `simple.window.fill_rect`.  Fills in the
    /// current colour.
    pub fn fill_rect(&mut self, r: Rect) {
        let ds = self.window.drawable_size();
        let view_w = ds.0;
        let view_h = ds.1;
        let (x, y) = Window::translate_model_90c(r.x(), r.y(), view_w, view_h);
        let rect = Rect::new(x, y, r.height(), r.width());
        self.window.fill_rect(rect);
        eprintln!("fill_rect: {r:?} -> {rect:?} {view_w}x{view_h}");
        // self.window.fill_rect(r);
    }
}
