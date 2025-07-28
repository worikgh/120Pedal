//! Front end for Qzn3t pedal
//! Designed to run on a touch screen
//! PLANNED: Allow editing the volume of effects
extern crate simple;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use pedal_state::read_state;
use pedal_state::write_state;
use pedal_state::PedalState;
use send_osc::OscSender;
use simple::{Event, Rect};
use std::cell::RefCell;
use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::fs::metadata;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::exit;
use std::rc::Rc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::{channel, Sender};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::System;
mod send_osc;

const COLOUR_BLUE: [u8; 4] = [0, 0, 0xff, 0xff];
const COLOUR_GREEN: [u8; 4] = [0, 0xff, 0, 0xff];
const COLOUR_RED: [u8; 4] = [0xff, 0, 0, 0xff];
const COLOUR_BLACK: [u8; 4] = [0x0, 0, 0, 0xff];

/// Colours for the sliders.
const COLOUR_SELECTED: [u8; 4] = [0xf0, 0x0f, 0xff, 0x88];
const COLOUR_UNSELECTED: [u8; 4] = [0x0f, 0xf0, 0xff, 0x88];
const COLOUR_THUMB: [u8; 4] = COLOUR_RED;

/// The background of the slider
const COLOUR_BACKGROUND: [u8; 4] = [0xf8, 0xf0, 0xf0, 255];

struct App {
    window: simple::Window,
    width: u16,
    height: u16,
    invert: bool,
}
impl App {
    fn new(name: &str, width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            window: simple::Window::new(name, width, height),
            invert: false,
        }
    }
    fn set_colour(&mut self, colour: &[u8; 4]) {
        self.window
            .set_color(colour[0], colour[1], colour[2], colour[3]);
    }
    fn fill_rect(&mut self, r: Rect) {
        // If the window is inverted adjust rect
        let r = if self.invert {
            let y = self.height as i32 - r.y;
            let y = y - r.height() as i32;
            Rect::new(r.x, y, r.width(), r.height())
        } else {
            r
        };
        self.window.fill_rect(r);
    }
    #[allow(dead_code)]
    fn invert(&mut self, f: bool) {
        self.invert = f;
    }
}

trait TouchRectFn {
    fn event(&mut self, is_down: bool, x: f64, y: f64);
    fn point_inside(&self, x: f64, y: f64) -> bool;
    fn paint(&mut self, app: &mut App);
    fn tick(&mut self, _app: &mut App) {}
}

/// The command modes for MainCommandRect
#[derive(Debug, Clone)]
enum CommandMode {
    EditMode,
    LiveMode,
}

// Button to nute the mixer
#[derive(Debug)]
struct MuteButton {
    corners: [f64; 4],
    colour_muted: [u8; 4],
    colour_unmuted: [u8; 4],
    colour_pressed: [u8; 4],
    pressed: bool,
    muted: bool,
    osc: Rc<OscSender>, // Shared OSC transmitter
}
impl MuteButton {
    fn new(osc: Rc<OscSender>, x: f64, y: f64, w: f64, h: f64) -> Self {
        Self {
            corners: [x, y, w, h],
            colour_muted: COLOUR_RED,
            colour_unmuted: COLOUR_GREEN,
            colour_pressed: COLOUR_BLUE,
            pressed: false,
            muted: false,
            osc,
        }
    }
}
impl TouchRectFn for MuteButton {
    fn event(&mut self, is_down: bool, _x: f64, _y: f64) {
        if self.pressed && !is_down {
            // Take action

            self.muted = !self.muted;
            let value = if self.muted { 0.0 } else { 1.0 };
            let osc_msg = "/M/{}".to_string();
            if let Err(err) = self.osc.send(osc_msg.as_str(), value) {
                eprintln!("Error qzn3t_gui: Sending OSC: {osc_msg}  Value: {value}  Error: {err}");
            }
        }
        self.pressed = is_down;
    }
    fn paint(&mut self, app: &mut App) {
        let colour = if !self.pressed {
            if self.muted {
                self.colour_muted
            } else {
                self.colour_unmuted
            }
        } else {
            self.colour_pressed
        };
        app.set_colour(&colour);
        let x = self.corners[0];
        let y = self.corners[1];
        let w = self.corners[2];
        let h = self.corners[3];
        let x = (x * app.width as f64) as i32;
        let y = (y * app.height as f64) as i32;
        let w = (w * app.width as f64) as u32;
        let h = (h * app.height as f64) as u32;
        let fill_rect = Rect::new(x, y, w, h);
        app.fill_rect(fill_rect);
    }
    fn point_inside(&self, x: f64, y: f64) -> bool {
        point_inside_rect(x, y, self.corners)
    }
}
// Button that is highlighted while pressed, and is used to add (or
// subtract) a value
#[derive(Debug)]
struct AdjButton {
    // Add (or subtract) this value
    value: ButtonIncrement,

    corners: [f64; 4],
    colour: [u8; 4],
    colour_pressed: [u8; 4],
    pressed: bool,
    target: Rc<SliderValue>,
}

#[derive(PartialEq, Eq, Debug)]
/// The values that can be added to the slider value
enum ButtonIncrement {
    NegativeSmall,
    NegativeBig,
    PositiveSmall,
    PositiveBig,
}
impl ButtonIncrement {
    fn value(&self) -> isize {
        match self {
            ButtonIncrement::NegativeSmall => -1,
            ButtonIncrement::PositiveSmall => 1,
            ButtonIncrement::NegativeBig => -10,
            ButtonIncrement::PositiveBig => 10,
        }
    }
}
impl AdjButton {
    fn new(
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        value: ButtonIncrement,
        target: Rc<SliderValue>,
    ) -> Self {
        let colour: [u8; 4] = match value {
            ButtonIncrement::NegativeSmall | ButtonIncrement::PositiveSmall => COLOUR_BLUE,
            ButtonIncrement::NegativeBig | ButtonIncrement::PositiveBig => COLOUR_GREEN,
        };
        Self {
            corners: [x, y, w, h],
            value,
            colour,
            colour_pressed: COLOUR_RED,
            pressed: false,
            target,
        }
    }
}
impl TouchRectFn for AdjButton {
    fn event(&mut self, is_down: bool, _x: f64, _y: f64) {
        if self.pressed && !is_down {
            let new_value = *self.target.value.borrow() + (self.value.value() as f32 / 127.0);
            let new_value = new_value.clamp(0.0, 1.0);
            *self.target.value.borrow_mut() = new_value;
            let osc_msg = format!("/v/{}", self.target.idx);
            if let Err(err) = self.target.osc.send(osc_msg.as_str(), new_value) {
                eprintln!(
                    "Error qzn3t_gui: Sending OSC: {osc_msg}  Value: {new_value}  Error: {err}"
                );
            }
        }
        self.pressed = is_down;
    }

    fn point_inside(&self, x: f64, y: f64) -> bool {
        point_inside_rect(x, y, self.corners)
    }

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
            app.set_colour(&self.colour_pressed);
        } else {
            app.set_colour(&self.colour);
        }
        app.fill_rect(fill_rect);

        // Draw a "-" or "+", thin or thick....
        {
            let thickness: usize = if self.value == ButtonIncrement::NegativeBig
                || self.value == ButtonIncrement::PositiveBig
            {
                h as usize / 4
            } else {
                h as usize / 8
            };

            let y1 = y + (h / 2) as i32 - (thickness as i32 / 2);
            let x1 = x + (w / 2) as i32 - (thickness as i32 / 2);

            let plus = matches!(
                self.value,
                ButtonIncrement::PositiveSmall | ButtonIncrement::PositiveBig
            );

            app.set_colour(&COLOUR_BLACK);
            if plus {
                // Vertical
                let rect = Rect::new(x1, y, thickness as u32, h);
                app.fill_rect(rect);
            }

            // horizontal
            let diff_h = w.saturating_sub(h); // Make horizontal same as vertical
            let rect = Rect::new(x + diff_h as i32 / 2, y1, w - diff_h, thickness as u32);
            app.fill_rect(rect);
        }
    }
}

#[derive(Debug)]
struct SliderValue {
    value: RefCell<f32>, // Value of slider
    osc: Rc<OscSender>,  // Shared OSC transmitter
    idx: u8,             // Identifier
}
impl SliderValue {
    pub fn new(osc: Rc<OscSender>, initial_value: f32, idx: u8) -> Self {
        SliderValue {
            value: RefCell::new(initial_value),
            osc,
            idx,
        }
    }
}

/// Adjusting one parameter.
#[derive(Debug)]
struct Slider {
    // Graphical widgets
    add_one: AdjButton,
    add_ten: AdjButton,
    sub_one: AdjButton,
    sub_ten: AdjButton,

    // Value displayed
    slider_value: Rc<SliderValue>,

    // The index of the slider that identifies it and the flag toset
    // when selected.  This is shared with `main` so sliders can be
    // selected externally
    idx_selected: Rc<RefCell<IdxSelected>>,

    corners: [f64; 4],
    // `w_f` is width factor.  If it is 1.0 there is no space
    // between sliders
    w_f: f64,
}
#[derive(Debug)]
struct IdxSelected {
    idx: u8,
    selected: bool,
}
#[allow(clippy::too_many_arguments)]
impl Slider {
    fn new(
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        margin: f64,
        w_f: f64,
        value: f32,
        idx: u8,
        osc: Rc<OscSender>,
    ) -> Self {
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

        // Initialise the mixer settings
        let osc_msg = format!("/v/{}", idx);
        if let Err(err) = osc.send(osc_msg.as_str(), value) {
            eprintln!(
                "Error qzn3t_gui: Sending OSC initialising EffectMixer: {osc_msg}  Value: {value}  Error: {err}"
            );
        }

        let slider_value = SliderValue::new(osc, value, idx);
        let slider_value = Rc::new(slider_value);
        let add_one = AdjButton::new(
            but_one_x,
            but_add_y,
            but_w,
            but_h,
            ButtonIncrement::PositiveSmall,
            Rc::clone(&slider_value),
        );
        let add_ten = AdjButton::new(
            but_ten_x,
            but_add_y,
            but_w,
            but_h,
            ButtonIncrement::PositiveBig,
            Rc::clone(&slider_value),
        );
        let sub_one = AdjButton::new(
            but_one_x,
            but_sub_y,
            but_w,
            but_h,
            ButtonIncrement::NegativeSmall,
            Rc::clone(&slider_value),
        );
        let sub_ten = AdjButton::new(
            but_ten_x,
            but_sub_y,
            but_w,
            but_h,
            ButtonIncrement::NegativeBig,
            Rc::clone(&slider_value),
        );

        Self {
            corners: [x, y, w, h],
            slider_value,
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
}
impl TouchRectFn for Slider {
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
        point_inside_rect(x, y, self.corners)
            || self.add_one.point_inside(x, y)
            || self.add_ten.point_inside(x, y)
            || self.sub_one.point_inside(x, y)
            || self.sub_ten.point_inside(x, y)
    }

    fn paint(&mut self, app: &mut App) {
        // Paint the background
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
            let selected: bool = self.idx_selected.borrow().selected;
            if selected {
                app.set_colour(&COLOUR_SELECTED);
            } else {
                app.set_colour(&COLOUR_UNSELECTED);
            }
            app.fill_rect(fill_rect);
        }

        // Paint the buttons
        self.add_one.paint(app);
        self.add_ten.paint(app);
        self.sub_one.paint(app);
        self.sub_ten.paint(app);

        // Paint the value indicator
        {
            let x = self.corners[0] - self.corners[2] / 2.0;
            let y = self.corners[1]
                + self.corners[3] * (1.0 - *self.slider_value.value.borrow() as f64);
            // let y = 1.0 - y;
            let w = self.corners[2];

            let x = (x * app.width as f64) as i32;
            let y = (y * app.height as f64) as i32;
            let w = (w * app.width as f64) as u32;
            let h = 2;
            let rect = Rect::new(x, y, w, h);
            app.set_colour(&COLOUR_THUMB);
            app.fill_rect(rect);
        }
    }
}

#[derive(Debug)]
struct EffectMixer {
    sliders: Vec<Slider>,
    corners: [f64; 4],
    state_rx: Receiver<PedalState>,
    pedal_state: PedalState,
}
impl EffectMixer {
    fn new(osc: Rc<OscSender>, pedal_state: PedalState, x: f64, y: f64, w: f64, h: f64) -> Self {
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
                sliders.push(Slider::new(
                    x,
                    y,
                    x_step,
                    h,
                    margin,
                    w_f,
                    c.1,
                    c.0,
                    osc.clone(),
                ));
                idx += 1;
            }
        }
        let (state_tx, state_rx) = channel();
        let _jh = monitor_pedal_state(state_tx.clone(), pedal_state.clone());
        Self {
            pedal_state,
            sliders,
            corners: [x, y, w, h],
            state_rx,
            //state_tx,
        }
    }
    fn init(&self) {}
}
impl TouchRectFn for EffectMixer {
    fn event(&mut self, is_down: bool, x: f64, y: f64) {
        // Pass to sliders
        let mut dirty = false;
        for s in self.sliders.iter_mut() {
            if s.point_inside(x, y) {
                s.event(is_down, x, y);
                dirty = true;
                break;
            }
        }
        if dirty {}
    }

    fn point_inside(&self, x: f64, y: f64) -> bool {
        point_inside_rect(x, y, self.corners)
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
        app.set_colour(&COLOUR_BACKGROUND);
        app.fill_rect(fill_area);

        for s in self.sliders.iter_mut() {
            s.paint(app);
        }
    }

    fn tick(&mut self, app: &mut App) {
        // Do two things every tick
        // 1. Check if the selected channel has changed.
        // 2. Check if the volume on any slider has changed
        let mut selected_slider: Option<u8> = self.pedal_state.selected;
        {
            let mut dirty = false;
            if let Ok(r_state) = self.state_rx.try_recv() {
                let new_selected_slider = r_state.selected;
                if new_selected_slider != selected_slider {
                    selected_slider = new_selected_slider;
                    dirty = true;
                }
                for s in self.sliders.iter_mut() {
                    let idx = s.idx_selected.borrow().idx;

                    let old_selected = s.idx_selected.borrow().selected;
                    let new_selected = if r_state.selected.is_some() {
                        let res = r_state.selected.as_ref().unwrap() == &idx;
                        // eprintln!("DBG qzn3t_gui: EffectMixer.tick idx: {idx} new_selected: {res}",);
                        res
                    } else {
                        false
                    };
                    s.select(new_selected);
                    if old_selected != new_selected {
                        // eprintln!(
                        //     "DBG qzn3t_gui: EffectMixer.tick idx: {idx} old: {old_selected} -> {}  r_state: {r_state:?}",
                        //     s.idx_selected.borrow().selected
                        // );
                        dirty = true;
                    }
                }
            }
            if dirty {
                self.pedal_state.selected = selected_slider;
                self.paint(app);
            }
        }
        {
            let mut dirty = false;
            for s in self.sliders.iter() {
                let idx = s.idx_selected.borrow().idx;
                let value = *s.slider_value.value.borrow();
                if let Some(choice) = self.pedal_state.choices.iter_mut().find(|x| x.0 == idx) {
                    if choice.1 != value {
                        choice.1 = value;
                        dirty = true;
                        break;
                    }
                }
            }
            if dirty {
                write_state(
                    &self.pedal_state,
                    pedals_dir().to_str().expect("Pedals path to write"),
                )
                .expect("Writing state");
            }
        }
    }
}

/// The "button" that switches between `EditMode` where the pedal has
/// no effect and the web interface is offered for adjusting pedal
/// parameters adn `LiveMode` where the pedlal does have an efect and
/// the web interface is turned off
struct MainCommandRect {
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
    mode: CommandMode,
    /// If there are errors `valid` is false
    valid: bool,
    // jh: JoinHandle<()>,
    qzn3t_beacon: Arc<AtomicBool>,
}

impl MainCommandRect {
    /// This runs the command from MainTouchRect.  The command takes
    /// one `bool` argument.  If `true` it will run `qzn3t` otherwise
    /// it runs `mod-ui`.  It returns `true` if the comand succeeded,
    /// `false` otherwise
    fn run_command(&mut self) -> bool {
        let command = self.command.as_str();
        let argument = match self.mode {
            CommandMode::EditMode => "false",
            CommandMode::LiveMode => "true",
        };
        self.valid = match std::process::Command::new(command).arg(argument).status() {
            Ok(s) => {
                eprintln!(
                    "DBG qzn3t_gui: Run command: {command} Ok.  Arg: {argument}: Success: {}",
                    s.success(),
                );
                s.success()
            }
            Err(err) => {
                eprintln!("DBG qzn3t_gui: Run command Err {argument}: {err:?}");
                false
            }
        };
        self.valid
    }

    fn new(width: f64, height: f64, command: String) -> Self {
        // Set up thread to monitor Qzn3t health
        let qzn3t_beacon_read = Arc::new(AtomicBool::new(false));
        let qzn3t_beacon_write = Arc::clone(&qzn3t_beacon_read);
        let _jh = std::thread::spawn(move || loop {
            let qz3t_beacon_value = qzn3t_running();
            qzn3t_beacon_write.store(qz3t_beacon_value, Ordering::Relaxed);
            thread::sleep(Duration::from_millis(100));
        });

        Self {
            corners: [0.0, 0.0, width, height],
            down: false,
            state_colour: [255, 0, 0, 255],
            not_state_colour: [0, 0, 255, 255],
            command,
            mode: CommandMode::LiveMode,
            valid: true,
            //jh,
            qzn3t_beacon: qzn3t_beacon_read,
        }
    }
}

impl TouchRectFn for MainCommandRect {
    /// Touch events toggle between `mod-ui` and `qzn3t`
    fn event(&mut self, is_down: bool, _x: f64, _y: f64) {
        if self.down != is_down {
            if !is_down {
                // Released. Take action
                let dbg_state = self.mode.clone();
                self.mode = match self.mode {
                    CommandMode::EditMode => CommandMode::LiveMode,
                    CommandMode::LiveMode => CommandMode::EditMode,
                };
                self.run_command();
                eprintln!(
                    "DBG gui qzn3t_gui: BoolCommandRect State change: {dbg_state:?} -> {:?}",
                    self.mode
                );
            }
            self.down = is_down;
        }
    }
    fn point_inside(&self, x: f64, y: f64) -> bool {
        point_inside_rect(x, y, self.corners)
    }
    fn paint(&mut self, app: &mut App) {
        let valid = match self.mode {
            CommandMode::LiveMode => self.qzn3t_beacon.load(Ordering::Relaxed),
            CommandMode::EditMode => true,
        };

        let colour: [u8; 4] = if self.down {
            COLOUR_BLACK
        } else {
            match self.mode {
                CommandMode::LiveMode => self.state_colour,
                CommandMode::EditMode => self.not_state_colour,
            }
        };

        // The dimensions of the button
        let x = self.corners[0];
        let y = self.corners[1];
        let w = self.corners[2];
        let h = self.corners[3];
        let x = (x * app.width as f64) as i32;
        let y = (y * app.height as f64) as i32;
        let w = (w * app.width as f64) as u32;
        let h = (h * app.height as f64) as u32;

        let fill_area = simple::Rect::new(x, y, w, h);
        if valid {
            app.set_colour(&colour);
            app.fill_rect(fill_area);
        } else {
            // Invalid state. A cross of colour
            app.set_colour(&COLOUR_BLACK);
            app.fill_rect(fill_area);

            // Make the cross
            app.set_colour(&colour);

            // Adjustment factor.  Increase this to make the cross
            // (that indicates invalid) skinnier. Too skinney and the
            // cross will disappear
            let adj: usize = 4;

            // Horizontal
            // `x` and `w` constant
            // adjust `y` and `h`
            {
                let y = y + (h as i32 / 2) - h as i32 / (adj as i32 * 2);
                let h = h / adj as u32;
                let fill_area = simple::Rect::new(x, y, w, h);
                app.fill_rect(fill_area);
            }

            // Vertical
            // `y` and `h` constant
            // Adjust `x` and `w`
            {
                let x = x + (w as i32 / 2) - w as i32 / (adj as i32 * 2);
                let w = w / adj as u32;
                let fill_area = simple::Rect::new(x, y, w, h);
                app.fill_rect(fill_area);
            }
        };
    }
    fn tick(&mut self, app: &mut App) {
        let valid = self.qzn3t_beacon.load(Ordering::Relaxed);
        if valid != self.valid {
            eprintln!(
                "DBG qzn3t_gui: MainCommandRect.tick self.valid: {} -> {valid}",
                self.valid
            );
            self.valid = valid;
        }
        self.paint(app);
    }
}

/// `TouchScreenCtl` Control surface for the device.  The main screen
struct TouchScreenCtl {
    /// The control areas
    rects: Vec<Box<dyn TouchRectFn>>,

    /// Over all size
    width: u16,
    height: u16,

    /// If inverted
    inverted: bool,
}

impl TouchScreenCtl {
    /// A window event.  Typically a touch/mouse event
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
                let y = if self.inverted { 1.0 - y } else { y };

                if i.point_inside(x, y) {
                    i.event(is_down, x, y);
                }
            }
        }
    }

    /// Called regularly to maintain state
    fn tick(&mut self, app: &mut App) {
        for r in self.rects.iter_mut() {
            r.tick(app);
        }
    }
}

/// Global access to the PedalState directory
fn pedals_dir() -> PathBuf {
    // Set up display of pedals and volume
    let mut pedals_dir = env::current_dir().expect("Failed to get current dir");
    pedals_dir.push("../PEDALS");
    pedals_dir.canonicalize().expect("Failed to resolve path")
}

fn inner_main() -> Result<(), Box<dyn Error>> {
    let _ = qzn3t_running();
    // The first argument is the command that starts the qzn3t pedals
    // or mod-ui, second is 0 for do not invert, 1 for invert, the
    // third and fourth are width and height
    let mut args = env::args().skip(1);
    eprintln!("DBG qzn3t_gui: args: {args:?}  args.len(): {}", args.len());
    let command = match args.next() {
        Some(arg) => arg,
        None => panic!("Pass the control script as an argument"),
    };
    let inverted: bool = match args.next() {
        Some(arg) => {
            if &arg == "0" {
                false
            } else if &arg == "1" {
                true
            } else {
                panic!("Error: Second argument: {arg} is invalid")
            }
        }
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
        eprintln!("Error qzn3t_gui: {command} is not executable");
        exit(1);
    }

    // If there are two arguments left they are the width and height
    // of the window.  Other wise default to screen for 3.5 inch
    // Raspberry Pi screen
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
        width = simple::Window::get_max_width()? as u16;
        height = simple::Window::get_max_height()? as u16;
    }

    // Read in the PedalState object to set the initial state of the
    // pedals.  The state file must exist.
    let pedals_path = pedals_dir();
    let pedal_state = match read_state(pedals_path.to_str().expect("Cannot convert path to string"))
        .expect("gui: Failed reading PedalState from: {pedals_path}")
    {
        Some(p) => p,
        None => PedalState {
            selected: None,
            choices: Vec::new(),
        },
    };

    // Main window.
    let mut app = App::new("Qzn3t", width, height);
    app.invert(inverted);
    // The button that switches between `qzn3t` and `mod-ui`.  Width
    // and height are normalised.
    const MAIN_WIDTH: f64 = 0.15; // 15%
    const MAIN_HEIGHT: f64 = 0.25;
    let mut main_button = MainCommandRect::new(MAIN_WIDTH, MAIN_HEIGHT, command);

    // Run the command once to initialise Pi in mod-ui
    if !main_button.run_command() {
        eprintln!("Error qzn3t_gui: Failed main_button.run_command");
        exit(1);
    }

    // Communicate with mixer
    let osc_port = 5020;
    let osc_addr = format!("127.0.0.1:{}", osc_port);
    let osc = match OscSender::new("127.0.0.1:5200", &osc_addr) {
        Ok(o) => o,
        Err(err) => panic!("{err:?}: Failed to create OSC: {osc_addr:?}"),
    };
    let osc = Rc::new(osc);

    // Mute button
    let mute_button = MuteButton::new(osc.clone(), 1.0 - MAIN_WIDTH, 0.0, MAIN_WIDTH, MAIN_HEIGHT);

    // The mixer that controls the volumes of the effects
    let effects_mixer = EffectMixer::new(
        osc.clone(),
        pedal_state,
        0.0,
        MAIN_HEIGHT,
        1.0,
        1.0 - MAIN_HEIGHT,
    );
    effects_mixer.init();

    // Main screen
    let mut tsc = TouchScreenCtl {
        rects: vec![
            Box::new(main_button),
            Box::new(effects_mixer),
            Box::new(mute_button),
        ],
        width,
        height,
        inverted,
    };

    let paint_screen = |app: &mut App, tsc: &mut TouchScreenCtl| {
        app.window.clear();
        for i in tsc.rects.iter_mut() {
            i.paint(app);
        }
    };
    paint_screen(&mut app, &mut tsc);

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
    Ok(())
}

fn main() {
    if let Err(err) = inner_main() {
        eprintln!("Error qzn3t_gui: inner_main: {err}");
    }
}

/// Monitor the PedalState file to see if the selected effect has been
/// changed.  In which case send a message to the main thread to
/// change the selected slider
pub fn monitor_pedal_state(
    tx: Sender<PedalState>,
    pedal_state: PedalState,
) -> std::thread::JoinHandle<()> {
    let mut stored_state = pedal_state;
    let file_path = pedals_dir();
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

        eprintln!(
            "DBG qzn3t_gui: Monitoring pedal state at: {}",
            path.display()
        );
        for event in watcher_rx.into_iter().flatten() {
            if let EventKind::Modify(_modify_kind) = event.kind {
                // State file changed
                // Check selected slider has changed
                let new_state = match read_state(path.to_str().expect("Statefile path invalid")) {
                    Ok(state) => state,
                    Err(e) => {
                        // Is this an error?
                        eprintln!("DBG qzn3t_gui: Failed to read initial state: {}", e);
                        continue;
                    }
                };
                if new_state.is_none() {
                    eprintln!("Error qzn3t_gui: Failed to read pedal state in file monitor");
                    continue;
                }
                let new_state = new_state.unwrap();
                let last_selected: Option<u8> = stored_state.selected;
                let new_selected: Option<u8> = new_state.selected;
                if last_selected != new_selected {
                    stored_state = new_state;
                    let _ = tx.send(stored_state.clone());
                }
            }
        }
        println!("Finished monitoring pedal state at: {}", path.display());
    })
}

/// Helper function for detecting when the mouse/pointer is inside a rectangle/window
fn point_inside_rect(x: f64, y: f64, corners: [f64; 4]) -> bool {
    x > corners[0] && x <= corners[2] + corners[0] && y > corners[1] && y < corners[3] + corners[1]
}

/// Check if the Qzn3t pedal  simulator is running
fn qzn3t_running() -> bool {
    let mut sys = System::new_all();
    sys.refresh_all();
    let mut c: HashSet<String> = HashSet::new();
    for process in sys.processes().values() {
        if let Some(process_name) = process.name().to_str() {
            if process_name == "read_midi" {
                c.insert("read_midi".to_string());
            } else if process_name == "jack_midi" {
                c.insert("jack_midi".to_string());
            } else if process_name == "translate_midi" {
                c.insert("translate_midi".to_string());
            }
        }
    }
    c.contains("jack_midi") && c.contains("translate_midi") && c.contains("read_midi")
}
