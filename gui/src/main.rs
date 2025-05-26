extern crate simple;
use simple::{Event, Window};

trait TouchRectFn {
    fn event(&mut self, is_down: bool, x: f64, y: f64);
    fn point_inside(&self, x: f64, y: f64) -> bool;
    fn paint(&self, app: &mut Window);
}

/// The "button" that switches metween `mod-ui` and `qzn3t` MIDI pedal
struct MainTouchRect {
    /// RGBA
    state_colour: [u8; 4],
    not_state_colour: [u8; 4],
    /// Name of an external function that one argument: `state`.
    /// Starts `mod-ui` or `qzn3t`
    command: String,
    /// x,y,w,h in 0..1
    corners: [f64; 4],
    /// Was the last event a 'mouse_down'
    down: bool,
    /// This is effectively a toggle
    state: bool,
    /// If there are errors `valid` is false
    valid: bool,
    /// The size of the rectangular area in native pixels
    width: u16,
    height: u16,
}
impl TouchRectFn for MainTouchRect {
    fn event(&mut self, is_down: bool, _x: f64, _y: f64) {
        if self.down != is_down {
            if !is_down {
                // Released. Take action
                let argument = self.state;
                match std::process::Command::new(self.command.as_str())
                    .arg(argument.to_string())
                    .status()
                {
                    Ok(s) => {
                        eprintln!("DBG Run command Ok {argument}: Success: {}", s.success(),);
                        self.valid = s.success();
                    }
                    Err(err) => eprintln!("DBG Run command Err {argument}: {err:?}"),
                };
                self.state = !self.state;
            }
            self.down = is_down;
        }
    }
    fn point_inside(&self, x: f64, y: f64) -> bool {
        if x > self.corners[0]
            && x <= self.corners[2] + self.corners[0]
            && y > self.corners[1]
            && y < self.corners[3] + self.corners[1]
        {
            true
        } else {
            false
        }
    }
    fn paint(&self, app: &mut Window) {
        let colour: [u8; 4];
        let fill_area = if self.valid {
            simple::Rect::new(
                (self.corners[0] * self.width as f64) as i32,
                (self.corners[1] * self.height as f64) as i32,
                (self.corners[2] * self.width as f64) as u32,
                (self.corners[3] * self.height as f64) as u32,
            )
        } else {
            let x0 = 0;
            let x1 = self.width as i32;
            let y0 = (0.4 * self.height as f64) as i32;
            let y1 = (0.6 * self.height as f64) as i32;
            simple::Rect::new(x0, y0, (x1 - x0) as u32, (y1 - y0) as u32)
        };
        if self.down {
            colour = [0, 0, 0, 0];
        } else if self.state {
            colour = [
                self.state_colour[0],
                self.state_colour[1],
                self.state_colour[2],
                self.state_colour[3],
            ];
        } else {
            colour = [
                self.not_state_colour[0],
                self.not_state_colour[1],
                self.not_state_colour[2],
                self.not_state_colour[3],
            ];
        }
        app.set_color(colour[0], colour[1], colour[2], colour[3]);
        app.fill_rect(fill_area);
    }
}

/// `TouchScreenCtl` Control surface for the device
struct TouchScreenCtl {
    /// The control areas
    rects: Vec<Box<dyn TouchRectFn>>,

    /// Over all size
    width: u16,
    height: u16,
}

impl TouchScreenCtl {
    fn event(&mut self, e: &Event) {
        if let Event::Mouse {
            mouse_x,
            mouse_y,
            is_down,
            ..
        } = *e
        {
            for i in self.rects.iter_mut() {
                let x = mouse_x as f64 / self.width as f64;
                let y = mouse_y as f64 / self.height as f64;
                if i.point_inside(x, y) {
                    i.event(is_down, x, y);
                }
            }
        }
    }
}
fn main() {
    let argv = match std::env::args().nth(1) {
        Some(arg) => arg,
        None => panic!("Pass the control script as an argument"),
    };
    // TODO: Check `argv` is executable
    eprintln!("DBG argv: {argv}");

    let width: u16 = 475;
    let height: u16 = 250;
    let mut app = simple::Window::new("Qzn3t", width as u16, height as u16);

    // The button that switches between `qzn3t` and `mod-ui`
    let main_button = MainTouchRect {
        corners: [0.0, 0.0, 0.75, 1.0],
        down: false,
        state_colour: [0, 0, 255, 255],
        not_state_colour: [255, 0, 0, 255],
        command: argv,
        state: false,
        valid: true,
        width,
        height,
    };

    let mut tsc = TouchScreenCtl {
        rects: vec![Box::new(main_button)],
        width,
        height,
    };

    let paint_screen = |app: &mut Window, tsc: &mut TouchScreenCtl| {
        app.clear();
        for i in tsc.rects.iter() {
            i.paint(app);
        }
    };
    paint_screen(&mut app, &mut tsc);
    while app.next_frame() {
        while app.has_event() {
            let e = app.next_event();
            tsc.event(&e);
            paint_screen(&mut app, &mut tsc);
        }
    }
}
