//! Front end for Qzn3t pedal
//! Designed to run on a touch screen
//! PLANNED: Allow editing the volume of effects
extern crate simple;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use pedal_state::read_state;
use pedal_state::PedalState;
use simple::{Event, Rect};
use std::cell::RefCell;
use std::env;
use std::fs::metadata;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;
use std::process::exit;
use std::rc::Rc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::{channel, Sender};
use std::time::{Duration, Instant};
mod send_osc;

struct App {
    window: simple::Window,
    width: u16,
    height: u16,
}
impl App {
    fn new(name: &str, width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            window: simple::Window::new(name, width, height),
        }
    }
}

trait TouchRectFn {
    fn event(&mut self, is_down: bool, x: f64, y: f64);
    fn point_inside(&self, x: f64, y: f64) -> bool;
    fn paint(&mut self, app: &mut App);
    fn tick(&mut self, app: &mut App);
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
}

/// The "button" that executes a system command, and passes its
/// `state` as a tri-state argument
#[allow(dead_code)]
enum TriState {
    A,
    B,
    C,
}

// Button that is highlighted while pressed, and is used to add (or
// subtract) a value
struct PushButton {
    // Add (or subtract) this value
    value: isize,

    corners: [f64; 4],
    colour: [u8; 4],
    colour_pressed: [u8; 4],
    pressed: bool,
    target: Rc<RefCell<f64>>,
}
impl PushButton {
    fn new(x: f64, y: f64, w: f64, h: f64, value: isize, target: Rc<RefCell<f64>>) -> Self {
        let colour: [u8; 4] = if value.abs() == 1 {
            [0, 0, 0xff, 0xff]
        } else {
            [0, 0xff, 0, 0xff]
        };
        Self {
            corners: [x, y, w, h],
            value,
            colour,
            colour_pressed: [0xff, 0, 0, 0xff],
            pressed: false,
            target,
        }
    }
}
impl TouchRectFn for PushButton {
    #[allow(dead_code, unused_variables)]
    fn event(&mut self, is_down: bool, x: f64, y: f64) {
        if self.pressed && !is_down {
            *self.target.borrow_mut() += self.value as f64 / 127.0;
            if *self.target.borrow() < 0.0 {
                *self.target.borrow_mut() = 0.0;
            } else if *self.target.borrow() > 127.0 {
                *self.target.borrow_mut() = 127.0;
            }
        }
        self.pressed = is_down;
    }

    #[allow(dead_code, unused_variables)]
    fn point_inside(&self, x: f64, y: f64) -> bool {
        x > self.corners[0]
            && x <= self.corners[2] + self.corners[0]
            && y > self.corners[1]
            && y < self.corners[3] + self.corners[1]
    }

    #[allow(dead_code, unused_variables)]
    fn paint(&mut self, app: &mut App) {
        let w = self.corners[2];
        let h = self.corners[3];
        let x = self.corners[0];
        let y = self.corners[1];
        let x = (x * app.width as f64) as i32;
        let y = (y * app.height as f64) as i32;
        let w = (w * app.width as f64) as u32;
        let h = (h * app.height as f64) as u32;
        let fill_rect = Rect::new(x, y, w, h);
        // For now plus/sub one is blue and plus/sub ten is green
        if self.pressed {
            app.window.set_color(
                self.colour_pressed[0],
                self.colour_pressed[1],
                self.colour_pressed[2],
                self.colour_pressed[3],
            );
        } else {
            app.window.set_color(
                self.colour[0],
                self.colour[1],
                self.colour[2],
                self.colour[3],
            );
        }
        app.window.fill_rect(fill_rect);
    }

    fn tick(&mut self, _app: &mut App) {}
}

/// Adjusting one parameter.
struct Slider {
    // Graphical widgets
    add_one: PushButton,
    add_ten: PushButton,
    sub_one: PushButton,
    sub_ten: PushButton,

    // Value displayed
    value: Rc<RefCell<f64>>,

    // The index of the slider that identifies it and the flag toset
    // when selected.  This is shared with `main` so sliders can be
    // selected externally
    idx_selected: Rc<RefCell<IdxSelected>>,

    corners: [f64; 4],
    // `w_f` is width factor.  If it is 1.0 there is no space
    // between sliders
    w_f: f64,
}
struct IdxSelected {
    idx: u8,
    selected: bool,
}
#[allow(clippy::too_many_arguments)]
impl Slider {
    fn new(x: f64, y: f64, w: f64, h: f64, margin: f64, w_f: f64, value: f64, idx: u8) -> Self {
        // The `value` of the slider is shared by the `PushButton`s so it can be changed
        let value = Rc::new(RefCell::new(value));

        // default to not selected
        let idx_selected = Rc::new(RefCell::new(IdxSelected {
            idx,
            selected: false,
        }));

        // Calculate the positions of the buttons.  The buttons for
        // adding go on top, the buttons for subtracting at the
        // bottom.  The buttons for one on left, ten on right
        let but_one_x: f64 = x - w / 2.0;
        let but_ten_x: f64 = x;
        let but_h = (h + 2.0 * margin) * margin;
        let but_w = w / 2.0;
        let but_add_y = y - but_h;
        let but_sub_y = y + h;
        let add_one = PushButton::new(but_one_x, but_add_y, but_w, but_h, 1, value.clone());
        let add_ten = PushButton::new(but_ten_x, but_add_y, but_w, but_h, 10, value.clone());
        let sub_one = PushButton::new(but_one_x, but_sub_y, but_w, but_h, -1, value.clone());
        let sub_ten = PushButton::new(but_ten_x, but_sub_y, but_w, but_h, -10, value.clone());
        Self {
            corners: [x, y, w, h],
            value,
            idx_selected,
            w_f,
            add_one,
            add_ten,
            sub_one,
            sub_ten,
        }
    }

    fn select(&self, selected: bool) {
        self.idx_selected.borrow_mut().selected = selected;
    }

    /// The value must be between 0..1
    #[allow(dead_code)]
    fn set_value(&mut self, value: f64) {
        assert!((0.0..=1.0).contains(&value));
        *self.value.borrow_mut() = value;
    }
}
impl TouchRectFn for Slider {
    #[allow(dead_code, unused_variables)]
    fn event(&mut self, is_down: bool, x: f64, y: f64) {
        // send to buttons
        for b in [
            &mut self.add_one,
            &mut self.add_ten,
            &mut self.sub_one,
            &mut self.sub_ten,
        ] {
            if b.point_inside(x, y) {
                b.event(is_down, x, y);
            }
        }
    }

    /// Check slider and buttons
    fn point_inside(&self, x: f64, y: f64) -> bool {
        x > self.corners[0]
            && x <= self.corners[2] + self.corners[0]
            && y > self.corners[1]
            && y < self.corners[3] + self.corners[1]
            || self.add_one.point_inside(x, y)
            || self.add_ten.point_inside(x, y)
            || self.sub_one.point_inside(x, y)
            || self.sub_ten.point_inside(x, y)
    }

    fn paint(&mut self, app: &mut App) {
        // Paint the white background
        {
            let w = self.corners[2] * self.w_f;
            let h = self.corners[3];
            let x = self.corners[0] - w / 2.0;
            let y = self.corners[1];

            let x = (x * app.width as f64) as i32;
            let y = (y * app.height as f64) as i32;
            let w = (w * app.width as f64) as u32;
            let h = (h * app.height as f64) as u32;
            let fill_rect = Rect::new(x, y, w, h);
            if self.idx_selected.borrow().selected {
                app.window.set_color(0xf0, 0x0f, 0xff, 0x88);
            } else {
                app.window.set_color(0x0f, 0xf0, 0xff, 0x88);
            }
            app.window.fill_rect(fill_rect);
            // app.set_color(0x0, 0x0, 0x0, 0xff);
            // app.draw_rect(fill_rect);
        }

        // Paint the buttons
        self.add_one.paint(app);
        self.add_ten.paint(app);
        self.sub_one.paint(app);
        self.sub_ten.paint(app);

        // Paint the value indicator
        {
            let x = self.corners[0] - self.corners[2] / 2.0;
            let y = self.corners[1] + self.corners[3] * (1.0 - *self.value.borrow());
            // let y = 1.0 - y;
            let w = self.corners[2];

            let x = (x * app.width as f64) as i32;
            let y = (y * app.height as f64) as i32;
            let w = (w * app.width as f64) as u32;
            let h = 2;
            let rect = Rect::new(x, y, w, h);
            app.window.set_color(0xff, 0, 0, 0xff);
            app.window.fill_rect(rect);
        }
    }

    fn tick(&mut self, _app: &mut App) {}
}

struct EffectMixer {
    _selected: Option<u8>,
    _channels: Vec<(u8, f64)>,
    sliders: Vec<Slider>,
    corners: [f64; 4],
    state_tx: Sender<Option<u8>>,
    state_rx: Receiver<Option<u8>>,
}
impl EffectMixer {
    fn new(pedal_state: &PedalState, x: f64, y: f64, w: f64, h: f64) -> Self {
        let channels = pedal_state.choices.clone();
        // Sliders
        let mut sliders: Vec<Slider> = Vec::new();
        {
            // For the sliders the `y`, `h`, `w`, `w_f`, `margin` and
            // `x_step` are constant

            // Scale for width of the drawn slider
            let w_f = 0.2;

            // Top and bottom
            let margin = 0.1;

            let x_step = 1.0 / (1.0 + channels.len() as f64);
            let y = y + h * margin;
            let h = h - 2.0 * h * margin;
            let mut idx: usize = 1;
            for c in channels.iter() {
                let x = x + idx as f64 * x_step;
                sliders.push(Slider::new(x, y, x_step, h, margin, w_f, c.1, c.0));
                idx += 1;
            }
        }
        let (state_tx, state_rx) = channel();
        Self {
            _selected: pedal_state.selected,
            _channels: channels,
            sliders,
            corners: [x, y, w, h],
            state_rx,
            state_tx,
        }
    }
    fn init(&self, pedals_path: PathBuf) {
        let _jh = monitor_pedal_state(pedals_path, self.state_tx.clone());
    }
}
impl TouchRectFn for EffectMixer {
    fn event(&mut self, is_down: bool, x: f64, y: f64) {
        // Pass to sliders
        for s in self.sliders.iter_mut() {
            if s.point_inside(x, y) {
                s.event(is_down, x, y);
            }
        }
    }

    fn point_inside(&self, x: f64, y: f64) -> bool {
        x > self.corners[0]
            && x <= self.corners[2] + self.corners[0]
            && y > self.corners[1]
            && y < self.corners[3] + self.corners[1]
    }

    fn paint(&mut self, app: &mut App) {
        // Paint the background
        let x = self.corners[0];
        let y = self.corners[1];
        let w = self.corners[2];
        let h = self.corners[3];
        let x = (x * app.width as f64) as i32;
        let y = (y * app.height as f64) as i32;
        let w = (w * app.width as f64) as u32;
        let h = (h * app.height as f64) as u32;
        let fill_area = simple::Rect::new(x, y, w, h);
        app.window.set_color(0xf0, 0xf0, 0xf0, 255);
        app.window.fill_rect(fill_area);

        for s in self.sliders.iter_mut() {
            s.paint(app);
        }
    }

    fn tick(&mut self, app: &mut App) {
        let mut dirty = false;
        if let Ok(selected) = self.state_rx.try_recv() {
            eprintln!("DBG State notification change: {selected:?}");
            if let Some(idx) = selected {
                for s in self.sliders.iter_mut() {
                    let mut idx_select = s.idx_selected.borrow_mut();
                    let old_selected = idx_select.selected;
                    idx_select.selected = idx_select.idx == idx;
                    if old_selected != idx_select.selected {
                        dirty = true;
                    }
                }
            } else {
                for s in self.sliders.iter_mut() {
                    if s.idx_selected.borrow().selected {
                        dirty = true;
                    }
                    s.select(false);
                }
            }
        }
        if dirty {
            self.paint(app);
        }
    }
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
    fn paint(&mut self, app: &mut App) {
        let colour: [u8; 4];
        let fill_area = if self.valid {
            let x = self.corners[0];
            let y = self.corners[1];
            let w = self.corners[2];
            let h = self.corners[3];
            let x = (x * app.width as f64) as i32;
            let y = (y * app.height as f64) as i32;
            let w = (w * app.width as f64) as u32;
            let h = (h * app.height as f64) as u32;
            simple::Rect::new(x, y, w, h)
        } else {
            let x0 = 0;
            let x1 = (self.corners[3] * app.width as f64) as i32;
            let y0 = (0.4 * app.height as f64) as i32;
            let y1 = (0.6 * app.height as f64) as i32;
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
        app.window
            .set_color(colour[0], colour[1], colour[2], colour[3]);
        app.window.fill_rect(fill_area);
    }

    fn tick(&mut self, _app: &mut App) {}
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

    fn tick(&mut self, app: &mut App) {
        for r in self.rects.iter_mut() {
            r.tick(app);
        }
    }
}
fn main() {
    let mut args = env::args().skip(1);
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

    // Set up display of pedals and volume
    let mut pedals_dir = env::current_dir().expect("Failed to get current dir");
    pedals_dir.push("../PEDALS");
    eprintln!("DBG Pedals path: {pedals_dir:?}");
    let pedals_path = pedals_dir.canonicalize().expect("Failed to resolve path");
    let pedal_state = match read_state(pedals_path.to_str().expect("Cannot convert path to string"))
        .expect("Failed reading PedalState")
    {
        Some(p) => p,
        None => PedalState {
            selected: None,
            choices: Vec::new(),
        },
    };

    if !run_command(command.as_str(), false, true) {
        eprintln!("Failed to run `{command} false`");
        exit(1);
    }
    let mut app = App::new("Qzn3t", width, height);

    // The button that switches between `qzn3t` and `mod-ui`
    const MAIN_WIDTH: f64 = 0.15;
    const MAIN_HEIGHT: f64 = 0.25;
    let main_button = BoolCommandRect {
        corners: [0.0, 0.0, MAIN_WIDTH, MAIN_HEIGHT],
        down: false,
        state_colour: [0, 0, 255, 255],
        not_state_colour: [255, 0, 0, 255],
        command,
        state: true,
        valid: true,
    };

    let effects_mixer = EffectMixer::new(&pedal_state, 0.0, MAIN_HEIGHT, 1.0, 1.0 - MAIN_HEIGHT);
    effects_mixer.init(pedals_path);

    let mut tsc = TouchScreenCtl {
        rects: vec![Box::new(main_button), Box::new(effects_mixer)],
        width,
        height,
    };

    let paint_screen = |app: &mut App, tsc: &mut TouchScreenCtl| {
        app.window.clear();
        for i in tsc.rects.iter_mut() {
            i.paint(app);
        }
    };
    paint_screen(&mut app, &mut tsc);

    // Channels for monitoring the file
    eprintln!("DBG gui main");
    // Doing about 60 frames a second.  Arrange a `tick()` every 100ms
    let tick_interval = Duration::from_millis(100);
    let mut last_tick_time = Instant::now() - tick_interval;

    while app.window.next_frame() {
        while app.window.has_event() {
            let e = app.window.next_event();
            tsc.event(&e);
            paint_screen(&mut app, &mut tsc);
        }
        if last_tick_time.elapsed() >= tick_interval {
            tsc.tick(&mut app);
            last_tick_time = Instant::now();
        }
    }
}

pub fn monitor_pedal_state(
    file_path: impl AsRef<Path> + Send + 'static,
    tx: Sender<Option<u8>>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let path = file_path.as_ref();
        let (watcher_tx, watcher_rx) = channel();

        // Create watcher with proper config
        let mut watcher: RecommendedWatcher = Watcher::new(
            watcher_tx,
            Config::default().with_poll_interval(Duration::from_millis(100)),
        )
        .expect("Failed to create file watcher");

        watcher
            .watch(path, RecursiveMode::NonRecursive)
            .expect("Failed to watch file");

        println!("Monitoring pedal state at: {}", path.display());

        let last_state = match read_state(path.to_str().expect("Statefile path invalid")) {
            Ok(state) => state,
            Err(e) => {
                eprintln!("Failed to read initial state: {}", e);
                return;
            }
        };
        let mut last_selected: Option<u8> = if let Some(state) = last_state {
            state.selected
        } else {
            None
        };

        // Send initial state
        let _ = tx.send(last_selected);

        for event in watcher_rx.into_iter().flatten() {
            if let EventKind::Modify(_modify_kind) = event.kind {
                // State file changed
                // Check selected slider has changed
                let new_state = match read_state(path.to_str().expect("Statefile path invalid")) {
                    Ok(state) => state,
                    Err(e) => {
                        eprintln!("Failed to read initial state: {}", e);
                        return;
                    }
                };
                if let Some(new_state) = new_state {
                    eprintln!(
                            "DBG monitor_pedal_state last_selected: {last_selected:?} new_state.selected: {:?}",
                            new_state.selected
                        );
                    if last_selected != new_state.selected {
                        last_selected = new_state.selected;
                        let _ = tx.send(last_selected);
                    }
                }
            }
        }
    })
}
