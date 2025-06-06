extern crate simple;
use simple::{Event, Window};
use std::fs::metadata;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::process::exit;
mod send_osc;
trait TouchRectFn {
    fn event(&mut self, is_down: bool, x: f64, y: f64);
    fn point_inside(&self, x: f64, y: f64) -> bool;
    fn paint(&self, app: &mut Window);
}

/// The "button" that executes a system command, and passes its
/// `state` as a booleen argument
struct BoolCommandRect {
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

/// The "button" that executes a system command, and passes its
/// `state` as a tri-state argument
#[allow(dead_code)]
enum TriState {
    A,
    B,
    C,
}

#[allow(dead_code)]
struct TriCommandRect {
    /// RGBA
    state_a_colour: [u8; 4],
    state_b_colour: [u8; 4],
    state_c_colour: [u8; 4],

    /// Name of an external function that takes one argument: `state`.
    /// Starts `mod-ui` or `qzn3t`
    command: String,
    /// x,y,w,h in 0..1
    corners: [f64; 4],
    /// Was the last event a 'mouse_down'
    down: bool,
    /// This is effectively a toggle
    state: TriState,

    /// If there are errors `valid` is false
    valid: bool,

    /// The size of the rectangular area in native pixels
    width: u16,
    height: u16,
}

/// This runs the command from MainTouchRect.  The command takes one
/// `bool` argument.  If `tru` it will run `qzn3t` otherwise it rns
/// `mod-ui`.  It returns `true` if the comand succeeded, `false`
/// otherwise
fn run_command(command: &str, argument: bool, _network: bool) -> bool {
    match std::process::Command::new(command)
        .arg(argument.to_string())
        .status()
    {
        Ok(s) => {
            eprintln!(
                "DBG Run command: {command} Ok.  Arg: {argument}: Success: {}",
                s.success(),
            );
            s.success()
        }
        Err(err) => {
            eprintln!("DBG Run command Err {argument}: {err:?}");
            false
        }
    }
}

impl TouchRectFn for TriCommandRect {
    fn event(&mut self, is_down: bool, _x: f64, _y: f64) {
        if self.down != is_down {
            if !is_down {
                // Released. Rotate state
                self.state = match self.state {
                    TriState::A => TriState::B,
                    TriState::B => TriState::C,
                    TriState::C => TriState::A,
                };
            }
            self.down = is_down;
        }
    }
    fn point_inside(&self, x: f64, y: f64) -> bool {
        x > self.corners[0]
            && x <= self.corners[2] + self.corners[0]
            && y > self.corners[1]
            && y < self.corners[3] + self.corners[1]
    }
    fn paint(&self, app: &mut Window) {
        let fill_area = simple::Rect::new(
            (self.corners[0] * self.width as f64) as i32,
            (self.corners[1] * self.height as f64) as i32,
            (self.corners[2] * self.width as f64) as u32,
            (self.corners[3] * self.height as f64) as u32,
        );
        let colour = if self.down {
            [0, 0, 0, 0]
        } else {
            match self.state {
                TriState::A => self.state_a_colour,
                TriState::B => self.state_b_colour,
                TriState::C => self.state_c_colour,
            }
        };
        app.set_color(colour[0], colour[1], colour[2], colour[3]);
        app.fill_rect(fill_area);
    }
}

impl BoolCommandRect {
    fn run_command(&mut self) {
        self.valid = run_command(self.command.as_str(), self.state, true);
    }
}

impl TouchRectFn for BoolCommandRect {
    fn event(&mut self, is_down: bool, _x: f64, _y: f64) {
        if self.down != is_down {
            if !is_down {
                // Released. Take action
                self.run_command();
                self.state = !self.state;
            }
            self.down = is_down;
        }
    }
    fn point_inside(&self, x: f64, y: f64) -> bool {
        x > self.corners[0]
            && x <= self.corners[2] + self.corners[0]
            && y > self.corners[1]
            && y < self.corners[3] + self.corners[1]
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
    let mut args = std::env::args().skip(1);
    eprintln!("args: {args:?}  args.len(): {}", args.len());
    let command = match args.next() {
        Some(arg) => arg,
        None => panic!("Pass the control script as an argument"),
    };
    // Check `command` is executable
    #[cfg(unix)]
    if metadata(&command)
        .unwrap_or_else(|e| panic!("{e}: Cannot get metadata for {command}"))
        .permissions()
        .mode()
        & 0o111
        == 0
    {
        eprintln!("{command} is not executable");
        exit(1);
    }
    let width: u16;
    let height: u16;
    if args.len() == 2 {
        // Passed width and height as arguments
        width = args
            .next()
            .unwrap() // Checked this argument is here
            .parse::<u16>()
            .expect("Width argument not parsed as u16");
        height = args
            .next()
            .unwrap() // Checked this argument is here
            .parse::<u16>()
            .expect("Height argument not parsed as u16");
    } else {
        width = 475;
        height = 250;
    }
    if !run_command(command.as_str(), false, true) {
        eprintln!("Failed to run `{command} false`");
        exit(1);
    }
    let mut app = simple::Window::new("Qzn3t", width, height);

    // The button that switches between `qzn3t` and `mod-ui`
    const MAIN_WIDTH: f64 = 0.15;
    const MAIN_HEIGHT: f64 = 0.15;
    let main_button = BoolCommandRect {
        corners: [0.0, 0.0, MAIN_WIDTH, MAIN_HEIGHT],
        down: false,
        state_colour: [0, 0, 255, 255],
        not_state_colour: [255, 0, 0, 255],
        command,
        state: true,
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
    eprintln!("DBG gui main");
    while app.next_frame() {
        while app.has_event() {
            let e = app.next_event();
            tsc.event(&e);
            paint_screen(&mut app, &mut tsc);
        }
    }
}
