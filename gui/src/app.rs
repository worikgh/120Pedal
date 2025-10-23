use crate::Rect;
use std::io::Read;
use std::{collections::HashMap, fs::File};
/// The outer structure
pub struct App {
    pub window: simple::Window,
    pub width: u16,
    pub height: u16,
    pub fonts: HashMap<String, Vec<u8>>,
}
impl App {
    pub fn new(name: &str, width: u16, height: u16) -> Self {
        Self::new_inner(name, Some((width, height)))
    }
    pub fn new_fullscreen(name: &str) -> Self {
        Self::new_inner(name, None)
    }
    fn new_inner(name: &str, dim: Option<(u16, u16)>) -> Self {
        let width: u16;
        let height: u16;
        let window = if let Some((w, h)) = dim {
            width = w;
            height = h;
            simple::Window::new(name, width, height)
        } else {
            let window = simple::Window::new_fullscreen(name);
            let (w, h) = window.drawable_size();
            width = w as u16;
            height = h as u16;
            window
        };
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
        Self {
            width,
            height,
            window,
            fonts,
        }
    }
    pub fn set_colour(&mut self, colour: &[u8; 4]) {
        self.window
            .set_color(colour[0], colour[1], colour[2], colour[3]);
    }

    /// Wrapper around `simple.window.fill_rect`
    pub fn fill_rect(&mut self, r: Rect) {
        self.window.fill_rect(r);
    }
}
