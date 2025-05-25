extern crate simple;
use simple::{Event, Window};

/// For a button
enum Action {
    Execute(String),
}

/// A buton
#[derive(PartialEq)]
enum PedalState {
    ModUi,
    Qzn3t,
}
struct TouchRect {
    /// x,y,w,h in 0..1
    corners: [f64; 4],
    action: Action,
    down: bool,
    state: PedalState,
    colour: [u8; 4],
    valid: bool,
}

impl TouchRect {
    fn _new(x: f64, y: f64, w: f64, h: f64, action: Action, colour: [u8; 4]) -> Self {
        Self {
            corners: [x, y, w, h],
            action,
            down: false,
            colour,
            state: PedalState::Qzn3t,
            valid: true,
        }
    }
    fn event(&mut self, e: &Event) {
        if let Event::Mouse { is_down, .. } = e {
            // eprintln!("DBG TouchRect event: {is_down}");
            if self.down != *is_down {
                self.down = *is_down;
                if !self.down {
                    self.state = match self.state {
                        PedalState::ModUi => PedalState::Qzn3t,
                        PedalState::Qzn3t => PedalState::ModUi,
                    };
                    match &self.action {
                        Action::Execute(command) => {
                            let argument = self.state == PedalState::Qzn3t;
                            match std::process::Command::new(command)
                                .arg(argument.to_string())
                                .status()
                            {
                                Ok(s) => {
                                    eprintln!(
                                        "DBG Run command Ok {argument}: Success: {}",
                                        s.success(),
                                    );
                                    self.valid = s.success();
                                }
                                Err(err) => eprintln!("DBG Run command Err {argument}: {err:?}"),
                            };
                        }
                    };
                }
            }
        }
    }
}

/// `TouchScreenCtl` Control surface for the device
struct TouchScreenCtl {
    rects: Vec<TouchRect>,
    width: usize,
    height: usize,
}

impl TouchScreenCtl {
    fn event(&mut self, e: &Event) {
        for i in self.rects.iter_mut() {
            if let Event::Mouse {
                mouse_x, mouse_y, ..
            } = *e
            {
                let x = mouse_x as f64 / self.width as f64;
                let y = mouse_y as f64 / self.height as f64;
                if x > i.corners[0]
                    && x <= i.corners[0] + i.corners[2]
                    && y > i.corners[1]
                    && y <= i.corners[1] + i.corners[3]
                {
                    i.event(e);
                }
            }
            i.event(e);
        }
    }
}
fn main() {
    let argv = match std::env::args().nth(1) {
        Some(arg) => arg,
        None => panic!("Pass the control script as an argument"),
    };
    eprintln!("DBG argv: {argv}");

    let width: usize = 475;
    let height: usize = 250;
    let mut app = simple::Window::new("Qzn3t", width as u16, height as u16);

    // One rect for the whole screen
    let main_button = TouchRect {
        corners: [0.0, 0.0, 1.0, 1.0],
        down: false,
        colour: [255, 0, 0, 127],
        action: Action::Execute(argv),
        state: PedalState::Qzn3t,
        valid: true,
    };
    let mut _tsc = TouchScreenCtl {
        rects: vec![main_button],
        width,
        height,
    };

    let paint_screen = |app: &mut Window, _tsc: &mut TouchScreenCtl| {
        for i in _tsc.rects.iter() {
            let colour: [u8; 4];
            if i.down {
                colour = [0, 0, 0, 0];
            } else if i.state == PedalState::Qzn3t {
                colour = [i.colour[0], i.colour[1], i.colour[2], i.colour[3]];
            } else {
                let r = ((i.colour[0] as u16 + 127) % 255) as u8;
                let g = ((i.colour[1] as u16 + 127) % 255) as u8;
                let b = ((i.colour[2] as u16 + 127) % 255) as u8;
                let a = ((i.colour[3] as u16 + 127) % 255) as u8;
                colour = [r, g, b, a];
            }

            app.set_color(colour[0], colour[1], colour[2], colour[3]);
            app.clear();
            let fill_area: simple::Rect;
            if i.valid {
                fill_area = simple::Rect::new(
                    i.corners[0] as i32 * _tsc.width as i32,
                    i.corners[1] as i32 * _tsc.height as i32,
                    i.corners[2] as u32 * _tsc.width as u32,
                    i.corners[3] as u32 * _tsc.height as u32,
                );
            }else{
                let x0 = 0;
                let x1 = width as i32;
                let y0 = (0.4 * height as f64) as i32;
                let y1 = (0.6 * height as f64) as i32;
                fill_area = simple::Rect::new(x0, y0, (x1-x0) as u32, (y1-y0) as u32);
            }
            app.fill_rect(fill_area);
            eprintln!("DBG Paint button rect: {fill_area:?} colour: {colour:?} Valid:{}", i.valid);
        }
    };
    paint_screen(&mut app, &mut _tsc);
    while app.next_frame() {
        while app.has_event() {
            let e = app.next_event();
            _tsc.event(&e);
            paint_screen(&mut app, &mut _tsc);
        }
    }
}
