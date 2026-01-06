//! Front end for Qzn3t pedal
//! Designed to run on a touch screen
//! PLANNED: Allow editing the volume of effects
extern crate simple;
use crate::app::App;
use crate::tuner_support::draw_char;
use clap::Parser;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use pedal_state::PedalState;
use pedal_state::read_state;
use pedal_state::write_state;
use rand::random;
use send_osc::OscSender;
use simple::{Event, Rect, event::MouseEventType, hide_mouse};
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::fs::metadata;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::exit;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::{Sender, channel};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use sysinfo::System;
use tuner::TunerArgs;
use tuner::TunerData;
use tuner::TunerNote;
use tuner::get_results;
mod app;
mod send_osc;
mod tuner_support;
const COLOUR_BLUE: [u8; 4] = [0, 0, 0xff, 0xff];
const COLOUR_GREEN: [u8; 4] = [0, 0xff, 0, 0xff];
const COLOUR_RED: [u8; 4] = [0xff, 0, 0, 0xff];
const COLOUR_BLACK: [u8; 4] = [0, 0, 0, 0xff];
const COLOUR_WHITE: [u8; 4] = [0xff, 0xff, 0xff, 0xff];
/// Sliders.
const COLOUR_SELECTED: [u8; 4] = [0xf0, 0x0f, 0xff, 0x88];
const COLOUR_UNSELECTED: [u8; 4] = [0x0f, 0xf0, 0xff, 0x88];
const COLOUR_THUMB: [u8; 4] = COLOUR_RED;
/// Background of the slider
const COLOUR_BACKGROUND: [u8; 4] = [0xf8, 0xf0, 0xf0, 255];

/// The number of "cents" above or below the actual tuned frequency
/// that is defined a s"in tune"
const CENTS_TOLERANCE: f32 = 10.0;
/// The limit.  If the absolute value of `cents` is above this the display is "100% out of tune"
const CENTS_LIMIT: f32 = 20.0;
/// The display limit: When `cents` crosses the line from "out of
/// tune" to "in tune" this is the proportion of the display that it
/// occupies.  Not zero, so it is still visible just before it
/// disappears
const CENTS_MIN_DISPLAY: f32 = 1.0 / 3.0;

trait TouchRectFn {
    fn event(&mut self, event_type: simple::event::MouseEventType, x: f32, y: f32);
    fn point_inside(&self, x: f32, y: f32) -> bool;
    fn paint(&mut self, app: &mut App);
    fn tick(&mut self, _app: &mut App) {}
    // Buttons need to know if the mouse has been released out of the
    // objects area, so they can note it is no longer "pressed".
    fn mouse_released(&mut self, _x: f32, _y: f32) {}
    // Buttons, and TouchRects that contain buttons, implement this so
    // than while "pressed" if mouse moves out, they can turn off the
    // "colour_pressed" and turn that colour on if it moves in
    fn mouse_at(&mut self, _x: f32, _y: f32) {}
}

/// The tuner display
type TunerDataMutex = Arc<Mutex<Option<TunerData>>>;
#[derive(Debug)]
struct TunerDisplay {
    kill_flag: Arc<AtomicBool>,
    corners: [f32; 4],
    this_handle: Option<JoinHandle<()>>,
    get_result_handle: Option<JoinHandle<()>>,
    tuner_data: TunerDataMutex,
}
impl TunerDisplay {
    fn start_tuner() -> (
        JoinHandle<()>,
        JoinHandle<()>,
        Arc<AtomicBool>,
        TunerDataMutex,
    ) {
        let tuner_args = TunerArgs {
            connect_port: Some("system:capture_1".to_string()),
        };
        let (tx, rx) = mpsc::channel::<TunerData>();
        let kill_flag = Arc::new(AtomicBool::new(false));
        let kill_flag_ret = kill_flag.clone();
        let tuner_data = Arc::new(Mutex::new(None));
        let tuner_data_ret = tuner_data.clone();
        let gr_handle = get_results(&tuner_args, tx, kill_flag.clone());
        let tuner_data_arc = tuner_data.clone();

        let handle = thread::spawn(move || {
            let mut last_updated = Instant::now();
            loop {
                match rx.try_recv() {
                    Ok(td) => {
                        let mut t = tuner_data_arc.lock().unwrap();
                        *t = Some(td);
                        last_updated = Instant::now();
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        eprintln!("Error tuner: Tuner channel disconnected");
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => {
                        let elapsed = last_updated.elapsed();
                        // TODO: Make this an argument not constant 2_000
                        if elapsed.as_millis() > 2_000 {
                            let mut t = tuner_data_arc.lock().unwrap();
                            *t = None;
                        }
                    }
                };
                thread::sleep(Duration::from_millis(100));
            }
        });
        (handle, gr_handle, kill_flag_ret, tuner_data_ret)
    }
    fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        let (handle, gr_handle, kill_flag, tuner_data) = Self::start_tuner();
        Self {
            corners: [x, y, w, h],
            this_handle: Some(handle),
            get_result_handle: Some(gr_handle),
            tuner_data,
            kill_flag,
        }
    }
}
impl TouchRectFn for TunerDisplay {
    fn tick(&mut self, app: &mut App) {
        self.paint(app);
    }
    fn event(&mut self, event_type: MouseEventType, _x: f32, _y: f32) {
        if event_type == MouseEventType::Up {
            let state = self.kill_flag.load(Ordering::SeqCst);
            let kill_flag_state = !state;
            self.kill_flag.store(kill_flag_state, Ordering::SeqCst);
            if kill_flag_state {
                if let Some(h) = self.this_handle.take() {
                    _ = h.join();
                }
                if let Some(h) = self.get_result_handle.take() {
                    _ = h.join();
                }
            } else {
                let r = Self::start_tuner();
                self.this_handle = Some(r.0);
                self.get_result_handle = Some(r.1);
                self.kill_flag = r.2;
                self.tuner_data = r.3;
            }
        }
    }
    fn point_inside(&self, x: f32, y: f32) -> bool {
        point_inside_corners(x, y, self.corners)
    }
    // TunerDisplay
    fn paint(&mut self, app: &mut App) {
        let (x, y, w, h) = pixel_boundary(self.corners, app);
        let killed = self.kill_flag.load(Ordering::SeqCst);
        if killed {
            // app.set_colour(&COLOUR_BLACK);
            // let fr = Rect::new(x, y, w, h);
            // app.fill_rect(fr);
            let x = (self.corners[0] * app.width as f32) as i32;
            let y = (self.corners[1] * app.height as f32) as i32;
            let w = (self.corners[2] * app.width as f32) as u32;
            let h = (self.corners[3] * app.height as f32) as u32;
            for _ in 0..100 {
                // Pick a random colour...
                let c: f32 = random();
                let c = (c * 4.0).trunc() as usize;
                let colour = match c {
                    0 => COLOUR_BLUE,
                    1 => COLOUR_GREEN,
                    2 => COLOUR_RED,
                    3 => COLOUR_BLACK,
                    _ => panic!("{c}"),
                };
                app.set_colour(&colour);

                // A random pixel
                let tx = (random::<f32>() * w as f32) as i32;
                let ty = (random::<f32>() * h as f32) as i32;

                // Fill it
                let fr = Rect::new(tx + x, ty + y, 1, 1);
                app.fill_rect(fr);
            }
        } else {
            match &*self.tuner_data.lock().unwrap() {
                None => {
                    // No data to display yet.
                    app.set_colour(&COLOUR_BLACK);
                    let fr = Rect::new(x, y, w, h);
                    app.fill_rect(fr);
                }
                Some(data) => {
                    // Got some data to display.

                    app.set_colour(&COLOUR_WHITE);
                    app.fill_rect(Rect::new(x, y, w, h));

                    // The bounding boxes
                    // Rectangle for note and rect for modifier
                    let ww = 2 * w / 3;
                    let hh = 2 * h / 3;
                    let margin = 5;
                    // The note letter
                    let note_rect = {
                        let xx = x + margin;
                        let yy = y + h as i32 / 3;
                        Rect::new(xx, yy, ww, hh)
                    };
                    // The sharp symbol
                    let mod_rect = {
                        let xx = x + w as i32 / 3;
                        let yy = margin + y + h as i32 / 3 - hh as i32 / 2;
                        Rect::new(xx, yy, ww, hh)
                    };
                    // The octave
                    let oct_rect = {
                        let ww = w / 3;
                        let hh = h / 3;
                        Rect::new(x + margin, y + margin, ww, hh)
                    };
                    // The cents scale
                    let cents_rect = {
                        let xx = x + w as i32 / 4;
                        let yy = y;
                        let hh = h;
                        let ww = 3 * w / 4;
                        Rect::new(xx, yy, ww, hh)
                    };
                    let tuner_note = data.note.clone();
                    let note: char = match tuner_note {
                        TunerNote::A | TunerNote::ASharp => 'A',
                        TunerNote::B => 'B',
                        TunerNote::C | TunerNote::CSharp => 'C',
                        TunerNote::D | TunerNote::DSharp => 'D',
                        TunerNote::E => 'E',
                        TunerNote::F | TunerNote::FSharp => 'F',
                        TunerNote::G | TunerNote::GSharp => 'G',
                    };
                    let modifier = if tuner_note == TunerNote::A
                        || tuner_note == TunerNote::B
                        || tuner_note == TunerNote::C
                        || tuner_note == TunerNote::D
                        || tuner_note == TunerNote::E
                        || tuner_note == TunerNote::F
                        || tuner_note == TunerNote::G
                    {
                        None
                    } else {
                        Some('#')
                    };
                    let octave = data.octave;
                    let octave = if (0..=9).contains(&octave) {
                        (data.octave as u8 + 0x0030) as char
                    } else {
                        '?'
                    };

                    let cents = data.cents_offset.round().clamp(-100.0, 100.0);

                    // The size of the bar that indicates if below or
                    // above tuned.
                    let cents_display_min = CENTS_MIN_DISPLAY * h as f32;
                    let cents_display_max = h as f32;
                    let hh = if cents.abs() > CENTS_LIMIT {
                        // If cents.abs() > CENTS_LIMIT then it is 100% There is
                        // no point distinguishing levels if worse than
                        // that
                        cents_display_max
                    } else if cents.abs() < CENTS_TOLERANCE {
                        // The minimum that is displayed before it is "in tune"
                        // Use about a third of the display
                        cents_display_min
                    } else {
                        // Linearly interpolate

                        // Proportion of visible area occupied
                        let numerator = cents - CENTS_TOLERANCE;
                        let denominator = CENTS_LIMIT - CENTS_TOLERANCE;
                        let proportion: f32 = numerator / denominator;

                        // Calculate how much of the area available to fill with colour
                        cents_display_min + proportion * (cents_display_max - cents_display_min)
                    } / 2.0; // It is two halves, so half the calculated size

                    if cents < -CENTS_TOLERANCE {
                        // Flat
                        let x = cents_rect.x;
                        let y = cents_rect.h / 2;
                        let h = hh as u32;
                        let w = cents_rect.w as u32;
                        let fill_rect = Rect::new(x, y, w, h);
                        app.set_colour(&COLOUR_RED);
                        app.fill_rect(fill_rect);
                    } else if cents > 10.0 {
                        // Sharp
                        let x = cents_rect.x;
                        let y = cents_rect.height() as i32 / 2 - hh as i32;
                        let h = hh as u32;
                        let w = cents_rect.w as u32;
                        let fill_rect = Rect::new(x, y, w, h);
                        app.set_colour(&COLOUR_BLUE);
                        app.fill_rect(fill_rect);
                    } else {
                        // In tune
                        let fill_rect = Rect::new(x, y, w, h);
                        app.set_colour(&COLOUR_GREEN);
                        app.fill_rect(fill_rect);
                    }

                    // Draw the octave
                    draw_char(&oct_rect, octave, app, &COLOUR_BLACK);

                    // Draw the note
                    draw_char(&note_rect, note, app, &COLOUR_BLACK);

                    if let Some(m) = modifier {
                        // Draw the modifier
                        draw_char(&mod_rect, m, app, &COLOUR_BLACK);
                    }
                }
            }
        }
    }
}

/// Button to mute the mixer
#[derive(Debug)]
struct MuteButton {
    corners: [f32; 4],

    colour_muted: [u8; 4],
    colour_pressed: [u8; 4],
    colour_unmuted: [u8; 4],
    muted: bool,
    osc: Rc<OscSender>, // Shared OSC transmitter
    pressed: bool,
    mouse_in: bool,
}
impl MuteButton {
    fn new(osc: Rc<OscSender>, x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            corners: [x, y, w, h],
            colour_muted: COLOUR_RED,
            colour_unmuted: COLOUR_GREEN,
            colour_pressed: COLOUR_BLUE,
            pressed: false,
            muted: false,
            osc,
            mouse_in: false,
        }
    }
}
impl TouchRectFn for MuteButton {
    fn event(&mut self, event_type: MouseEventType, _x: f32, _y: f32) {
        self.mouse_in = true;
        if event_type == MouseEventType::Up {
            if self.pressed {
                // Take action
                self.muted = !self.muted;
                let value = if self.muted { 0.0 } else { 1.0 };
                let osc_msg = "/M/{}";
                send_f32_osc(&self.osc, osc_msg, value);
            }
            self.pressed = false;
        } else if event_type == MouseEventType::Down {
            self.pressed = true;
        }
    }

    // Mute button
    fn paint(&mut self, app: &mut App) {
        let colour = if self.pressed && self.mouse_in {
            self.colour_pressed
        } else if self.muted {
            self.colour_muted
        } else {
            self.colour_unmuted
        };

        app.set_colour(&colour);
        let x = self.corners[0];
        let y = self.corners[1];
        let w = self.corners[2];
        let h = self.corners[3];
        let x = (x * app.width as f32) as i32;
        let y = (y * app.height as f32) as i32;
        let w = (w * app.width as f32) as u32;
        let h = (h * app.height as f32) as u32;
        let fill_rect = Rect::new(x, y, w, h);
        app.fill_rect(fill_rect);
    }
    fn point_inside(&self, x: f32, y: f32) -> bool {
        point_inside_corners(x, y, self.corners)
    }

    fn mouse_at(&mut self, x: f32, y: f32) {
        self.mouse_in = self.point_inside(x, y);
    }
    fn mouse_released(&mut self, x: f32, y: f32) {
        if self.pressed && !self.point_inside(x, y) {
            self.pressed = false;
        }
    }
}

/// Button that is highlighted while pressed, and is used to add (or
/// subtract) a value
#[derive(Debug)]
struct AdjButton {
    corners: [f32; 4],

    colour: [u8; 4],
    colour_pressed: [u8; 4],
    pressed: bool,
    mouse_in: bool,
    target: Rc<SliderState>,
    value: ButtonIncrement, // Add (or subtract) this value
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
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        value: ButtonIncrement,
        target: Rc<SliderState>,
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
            mouse_in: false,
        }
    }
}
impl TouchRectFn for AdjButton {
    fn event(&mut self, event_type: MouseEventType, _x: f32, _y: f32) {
        self.mouse_in = true;
        if self.pressed && event_type == MouseEventType::Up {
            let old_value = *self.target.value.borrow();
            let new_value = old_value + (self.value.value() as f32 / 127.0);
            let new_value = new_value.clamp(0.0, 1.0);
            *self.target.value.borrow_mut() = new_value;
            let osc_msg = format!("/v/{}", self.target.idx);
            send_f32_osc(&self.target.osc, osc_msg.as_str(), new_value);
            self.pressed = false;
        } else if event_type == MouseEventType::Down {
            self.pressed = true;
        }
    }

    fn point_inside(&self, x: f32, y: f32) -> bool {
        point_inside_corners(x, y, self.corners)
    }

    // AdjButton
    fn paint(&mut self, app: &mut App) {
        let w = self.corners[2];
        let h = self.corners[3];
        let x = self.corners[0];
        let y = self.corners[1];
        let x = (x * app.width as f32) as i32;
        let y = (y * app.height as f32) as i32;
        let w = (w * app.width as f32) as u32;
        let h = (h * app.height as f32) as u32;
        let fill_rect = Rect::new(x, y, w, h);
        // For now plus/sub one is blue and plus/sub ten is green
        if self.pressed && self.mouse_in {
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

    fn mouse_at(&mut self, x: f32, y: f32) {
        self.mouse_in = self.point_inside(x, y);
    }
    fn mouse_released(&mut self, x: f32, y: f32) {
        if self.pressed && !self.point_inside(x, y) {
            self.pressed = false;
        }
    }
}

/// The state of a [Slider]
#[derive(Debug)]
struct SliderState {
    value: RefCell<f32>, // Value of slider
    osc: Rc<OscSender>,  // Shared OSC transmitter
    idx: u8,             // Identifier
}
impl SliderState {
    pub fn new(osc: Rc<OscSender>, initial_value: f32, idx: u8) -> Self {
        SliderState {
            value: RefCell::new(initial_value),
            osc,
            idx,
        }
    }
}

/// Adjusting one parameter.  Herein this controls the volume of one
/// effect One slider per effect.
#[derive(Debug)]
struct Slider {
    corners: [f32; 4],

    // Graphical widgets
    add_one: AdjButton,
    add_ten: AdjButton,
    sub_one: AdjButton,
    sub_ten: AdjButton,

    // Value displayed
    slider_state: Rc<SliderState>,

    // The index of the slider that identifies it and the flag to set
    // when selected.  This is shared with `main` so sliders can be
    // selected externally
    idx_selected: Rc<RefCell<IdxSelected>>,

    // `w_f` is width factor.  If it is 1.0 there is no space
    // between sliders
    w_f: f32,
}
/// Hold the selected state of [Slider].  TODO: Conceptually only one
/// [Slider] can be selected at a time - that is the "active effect".
/// This is a poor representation of that concept, as any number of
/// sliders can be selected.
#[derive(Debug)]
struct IdxSelected {
    idx: u8,
    selected: bool,
}
#[allow(clippy::too_many_arguments)]
impl Slider {
    fn new(
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        margin: f32,
        w_f: f32,
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
        let but_one_x: f32 = x - w / 2.0;
        let but_ten_x: f32 = x;
        let but_h = (h + 2.0 * margin) * margin;
        let but_w = w / 2.0;
        let but_add_y = y - but_h;
        let but_sub_y = y + h;

        // Initialise the mixer settings
        let osc_msg = format!("/v/{}", idx);
        if let Err(err) = osc.send(osc_msg.as_str(), value) {
            eprintln!(
                "Error qzn3t_gui: Sending OSC initialising EffectMixer: {osc_msg}  Value: {value}  err: {err}"
            );
        }

        let slider_value = SliderState::new(osc, value, idx);
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
            slider_state: slider_value,
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

    fn point_inside_slider(&self, x: f32, y: f32) -> bool {
        let w = self.corners[2] * self.w_f;
        let h = self.corners[3];
        let l = self.corners[0] - w / 2.0;
        let t = self.corners[1];
        x > l && x <= l + w && y > t && y <= t + h
    }

    fn handle_click(&mut self, _x: f32, y: f32) {
        // 127 states for `value`.  MIDI
        let value = *self.slider_state.value.borrow();
        let height = self.corners[3];
        let cnr_y = self.corners[1];
        let mouse_v = 1.0 - (y - cnr_y) / height;
        assert!(mouse_v > 0.0);
        let delta_v = mouse_v - value;
        let new_v = value + delta_v / 2.0;
        let osc_msg = format!("/v/{}", self.slider_state.idx);
        send_f32_osc(&self.slider_state.osc, &osc_msg, new_v);
        *self.slider_state.value.borrow_mut() = new_v;
    }
}
impl TouchRectFn for Slider {
    fn event(&mut self, event_type: MouseEventType, x: f32, y: f32) {
        // Set this if a button handles this, so the slider itself
        // does not move the thumb towards the mouse event
        let mut handled = false;

        // send to buttons
        for b in [
            &mut self.add_one,
            &mut self.add_ten,
            &mut self.sub_one,
            &mut self.sub_ten,
        ] {
            if b.point_inside(x, y) {
                b.event(event_type, x, y);
                handled = true;
            }
        }

        if !handled && event_type == MouseEventType::Up && self.point_inside_slider(x, y) {
            self.handle_click(x, y);
        }
    }

    /// Check slider and buttons
    fn point_inside(&self, x: f32, y: f32) -> bool {
        let r1 = self.point_inside_slider(x, y);
        let r2 = self.add_one.point_inside(x, y);
        let r3 = self.add_ten.point_inside(x, y);
        let r4 = self.sub_one.point_inside(x, y);
        let r5 = self.sub_ten.point_inside(x, y);
        r1 || r2 || r3 || r4 || r5
    }

    fn paint(&mut self, app: &mut App) {
        // Paint the background
        {
            let w = self.corners[2] * self.w_f;
            let h = self.corners[3];
            let x = self.corners[0] - w / 2.0;
            let y = self.corners[1];

            let x = (x * app.width as f32) as i32;
            let y = (y * app.height as f32) as i32;
            let w = (w * app.width as f32) as u32;
            let h = (h * app.height as f32) as u32;
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
            let y = self.corners[1] + self.corners[3] * (1.0 - *self.slider_state.value.borrow());
            // let y = 1.0 - y;
            let w = self.corners[2];

            let x = (x * app.width as f32) as i32;
            let y = (y * app.height as f32) as i32;
            let w = (w * app.width as f32) as u32;
            let h = 2;
            let rect = Rect::new(x, y, w, h);
            app.set_colour(&COLOUR_THUMB);
            app.fill_rect(rect);
        }
    }

    fn mouse_at(&mut self, x: f32, y: f32) {
        self.add_one.mouse_at(x, y);
        self.add_ten.mouse_at(x, y);
        self.sub_one.mouse_at(x, y);
        self.sub_ten.mouse_at(x, y);
    }
    fn mouse_released(&mut self, x: f32, y: f32) {
        self.add_one.mouse_released(x, y);
        self.add_ten.mouse_released(x, y);
        self.sub_one.mouse_released(x, y);
        self.sub_ten.mouse_released(x, y);
    }
}

#[derive(Debug)]
/// Holder for the sliders that represent the volume (and selected
/// state) of each effect.
struct EffectContainer {
    corners: [f32; 4],

    pedal_state: PedalState,
    sliders: Vec<Slider>,
    state_rx: Receiver<PedalState>,
}
impl EffectContainer {
    fn new(pedal_state: PedalState, sliders: Vec<Slider>, x: f32, y: f32, w: f32, h: f32) -> Self {
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
impl TouchRectFn for EffectContainer {
    fn event(&mut self, event_type: MouseEventType, x: f32, y: f32) {
        // Pass to sliders
        let mut dirty = false;
        for s in self.sliders.iter_mut() {
            if s.point_inside(x, y) {
                s.event(event_type, x, y);
                dirty = true;
                break;
            }
        }
        if dirty {}
    }

    fn point_inside(&self, x: f32, y: f32) -> bool {
        point_inside_corners(x, y, self.corners)
    }

    fn paint(&mut self, app: &mut App) {
        // Paint the background
        let x = self.corners[0];
        let y = self.corners[1];
        let w = self.corners[2];
        let h = self.corners[3];
        let x = (x * app.width as f32) as i32;
        let y = (y * app.height as f32) as i32;
        let w = (w * app.width as f32) as u32;
        let h = (h * app.height as f32) as u32;
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
            // Thing 1: selected channel
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
                        r_state.selected.as_ref().unwrap() == &idx
                    } else {
                        false
                    };
                    s.select(new_selected);
                    if old_selected != new_selected {
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
            // Thing 2: volume
            let mut dirty = false;
            for s in self.sliders.iter() {
                let idx = s.idx_selected.borrow().idx;
                let value = *s.slider_state.value.borrow();
                if let Some(choice) = self.pedal_state.choices.iter_mut().find(|x| x.0 == idx) {
                    const EPSILON: f32 = 0.00000001;
                    if (choice.1 - value).abs() > EPSILON {
                        choice.1 = value;
                        dirty = true;
                        break;
                    }
                } else {
                    // The pedal cannot be found
                    panic!(
                        "Error gui: In EffectMixer.tick slider {idx} could not be found in the PedalState: {:?}",
                        self.pedal_state
                    );
                }
            }
            if dirty && let Err(err) = write_state(&self.pedal_state, &pedals_dir()) {
                eprintln!("Error gui:  Cannot write state: {err}");
            }
        }
    }

    fn mouse_at(&mut self, x: f32, y: f32) {
        for s in self.sliders.iter_mut() {
            s.mouse_at(x, y);
        }
    }
    fn mouse_released(&mut self, x: f32, y: f32) {
        for s in self.sliders.iter_mut() {
            s.mouse_released(x, y);
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
    corners: [f32; 4],
    /// True when mouse is pressed
    pressed: bool,
    /// This is effectively a toggle
    mode: CommandMode,
    /// If there are errors `valid` is false
    valid: bool,
    // jh: JoinHandle<()>,
    qzn3t_beacon: Arc<AtomicBool>,
    // True when the mouse pointer is in the button
    mouse_in: bool,
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
                eprintln!("Error qzn3t_gui: Run command Err {argument}: {err:?}");
                false
            }
        };
        self.valid
    }

    fn new(width: f32, height: f32, command: String) -> Self {
        // Set up thread to monitor Qzn3t health
        let qzn3t_beacon_read = Arc::new(AtomicBool::new(false));
        let qzn3t_beacon_write = Arc::clone(&qzn3t_beacon_read);
        let _jh = std::thread::spawn(move || {
            loop {
                let qz3t_beacon_value = qzn3t_running();
                qzn3t_beacon_write.store(qz3t_beacon_value, Ordering::Relaxed);
                thread::sleep(Duration::from_millis(100));
            }
        });

        Self {
            corners: [0.0, 0.0, width, height],
            pressed: false,
            state_colour: [255, 0, 0, 255],
            not_state_colour: [0, 0, 255, 255],
            command,
            mode: CommandMode::LiveMode,
            valid: true,
            //jh,
            qzn3t_beacon: qzn3t_beacon_read,
            mouse_in: false,
        }
    }
}
impl TouchRectFn for MainCommandRect {
    /// Touch events toggle between `mod-ui` and `qzn3t`
    fn event(&mut self, event_type: MouseEventType, x: f32, y: f32) {
        match event_type {
            MouseEventType::Up => {
                // Check: Is point inside tested up the stack?
                if self.point_inside(x, y) && self.pressed {
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

                self.pressed = false;
            }
            MouseEventType::Down => self.pressed = true,
            MouseEventType::Move => (),
        }
    }
    fn point_inside(&self, x: f32, y: f32) -> bool {
        point_inside_corners(x, y, self.corners)
    }

    // MainCommandRect
    fn paint(&mut self, app: &mut App) {
        let valid = match self.mode {
            CommandMode::LiveMode => self.qzn3t_beacon.load(Ordering::Relaxed),
            CommandMode::EditMode => true,
        };

        let colour: [u8; 4] = if self.mouse_in && self.pressed {
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
        let x = (x * app.width as f32) as i32;
        let y = (y * app.height as f32) as i32;
        let w = (w * app.width as f32) as u32;
        let h = (h * app.height as f32) as u32;

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
            let adj: usize = 3;

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
            self.valid = valid;
        }
        self.paint(app);
    }

    fn mouse_at(&mut self, x: f32, y: f32) {
        self.mouse_in = self.point_inside(x, y);
    }
    fn mouse_released(&mut self, x: f32, y: f32) {
        if self.pressed && !self.point_inside(x, y) {
            self.pressed = false;
        }
    }
}

/// The command modes for MainCommandRect
#[derive(Debug, Clone)]
enum CommandMode {
    EditMode,
    LiveMode,
}

/// `TouchScreenCtl` Control surface for the device.  The main screen
struct TouchScreenCtl {
    /// The control areas
    rects: Vec<Box<dyn TouchRectFn>>,

    /// Over all size
    width: u16,
    height: u16,
}
impl TouchScreenCtl {
    /// A window event.  Typically a touch/mouse event
    fn event(&mut self, e: &Event) {
        if let Event::Mouse {
            mouse_x,
            mouse_y,
            event_type,
            ..
        } = *e
        {
            for i in 0..self.rects.len() {
                let touch_rect = &mut self.rects[i];
                let (x, y) = {
                    let x = mouse_x as f32 / self.width as f32;
                    let y = mouse_y as f32 / self.height as f32;
                    (x, y)
                };
                if touch_rect.point_inside(x, y) {
                    touch_rect.event(event_type, x, y);
                }

                if event_type == MouseEventType::Move {
                    touch_rect.mouse_at(x, y);
                } else if event_type == MouseEventType::Up {
                    touch_rect.mouse_released(x, y);
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

/// Structure to implement command line arguments with `clap`
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct CmdArgs {
    #[arg(short, long, default_value_t = 0.3)]
    pub max_vol: f32,

    #[arg(
        short = 't',
        long,
        help = "Turn on debugging messages for tuner",
        default_value_t = false
    )]
    pub tuner_verbose: bool,

    #[arg(
        short = 'l',
        long,
        help = "The interval in MS between tuner samples",
        default_value_t = 200
    )]
    pub tuner_interval: u64,

    // Command that starts the qzn3t pedals
    // or mod-ui
    #[arg(
        short = 'c',
        long,
        help = "Command that starts the qzn3t pedals or mod-ui"
    )]
    pub cmd: String,

    // If None then full screen, else the width and height of the main
    // window
    #[arg(short = 'x')]
    width: Option<u16>,

    #[arg(short = 'y')]
    height: Option<u16>,
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
            if let EventKind::Modify(modify_kind) = event.kind {
                // State file changed
                // Check selected slider has changed
                let new_state = match read_state(path) {
                    Ok(state) => state,
                    Err(e) => {
                        eprintln!(
                            "DBG qzn3t_gui: State file event: {modify_kind:?}. Read error:  state: {}",
                            e
                        );
                        continue;
                    }
                };
                let new_state = match new_state {
                    Some(s) => s,
                    None => {
                        eprintln!("Error qzn3t_gui: Failed to read pedal state in file monitor");
                        continue;
                    }
                };
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

/// Helper function for detecting when the mouse/pointer is inside a
/// rectangle/window.
fn point_inside_corners(x: f32, y: f32, corners: [f32; 4]) -> bool {
    let l = corners[0];
    let t = corners[1];
    let w = corners[2];
    let h = corners[3];
    x > l && x <= l + w && y > t && y <= t + h
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

/// Turn the `corners` array or 0..1 coordinates to pixel coordinates
/// TODO: Could the output be all unsigned because the input is on the
/// unit rectangle.
fn pixel_boundary(corners: [f32; 4], app: &App) -> (i32, i32, u32, u32) {
    let x = corners[0];
    let y = corners[1];
    let w = corners[2];
    let h = corners[3];
    let x = (x * app.width as f32) as i32;
    let y = (y * app.height as f32) as i32;
    let w = (w * app.width as f32) as u32;
    let h = (h * app.height as f32) as u32;
    (x, y, w, h)
}

/// Global access to the PedalState directory
fn pedals_dir() -> PathBuf {
    // Set up display of pedals and volume
    let mut pedals_dir = env::current_dir().expect("Failed to get current dir");
    pedals_dir.push("../PEDALS");
    pedals_dir.canonicalize().expect("Failed to resolve path")
}

/// Send an OSC message
fn send_f32_osc(osc: &Rc<OscSender>, msg: &str, value: f32) {
    if let Err(err) = osc.send(msg, value) {
        eprintln!("Error gui: OSC send failed: msg: {msg} value: {value}.  Error: {err}");
    }
}

/// The starting point
fn inner_main() -> Result<(), Box<dyn Error>> {
    let _ = qzn3t_running();
    let args = CmdArgs::parse();

    // The  command that starts the qzn3t pedals
    let command = args.cmd;

    // Check `command` is executable
    #[cfg(unix)]
    if metadata(&command)
        .unwrap_or_else(|e| panic!("Error gui: {e}: Cannot get metadata for {command}"))
        .permissions()
        .mode()
        & 0o111
        == 0
    {
        eprintln!("Error qzn3t_gui: {command} is not executable");
        exit(1);
    }

    // The width and height of the window, if supplied, otherwise
    // default to full screen
    let dim: Option<(u16, u16)> = if args.width.is_some() && args.height.is_some() {
        // Passed width and height as arguments
        Some((args.width.unwrap(), args.height.unwrap()))
    } else {
        None
    };

    // Read in the PedalState object to set the initial state of the
    // pedals.  The state file must exist.
    let pedals_path = pedals_dir();
    let pedal_state = match read_state(&pedals_path)
        .expect("Error gui: Failed reading PedalState from: {pedals_path}")
    {
        Some(p) => p,
        None => PedalState {
            selected: None,
            choices: Vec::new(),
        },
    };

    // Main window.
    let mut app = if let Some((width, height)) = dim {
        App::new("Qzn3t", width, height)
    } else {
        App::new_fullscreen("Qzn3t-fs")
    };
    hide_mouse();
    // The button that switches between `qzn3t` and `mod-ui`.  Width
    // and height are normalised.
    const BUTTON_WIDTH: f32 = 0.15; // 15%
    const BUTTON_HEIGHT: f32 = 0.25;
    let mut main_button = MainCommandRect::new(BUTTON_WIDTH, BUTTON_HEIGHT, command);

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
        Err(err) => panic!("Error gui: {err:?}: Failed to create OSC: {osc_addr:?}"),
    };
    let osc = Rc::new(osc);

    let tuner_display =
        TunerDisplay::new(0.5 - BUTTON_WIDTH / 2.0, 0.0, BUTTON_WIDTH, BUTTON_HEIGHT);

    // Mute button
    let mute_button = MuteButton::new(
        osc.clone(),
        1.0 - BUTTON_WIDTH,
        0.0,
        BUTTON_WIDTH,
        BUTTON_HEIGHT,
    );

    let x = 0.0;
    let y = BUTTON_HEIGHT;
    let w = 1.0;
    let h = 1.0 - BUTTON_HEIGHT;
    // Sliders
    let channels = pedal_state.choices.clone();

    let mut sliders: Vec<Slider> = Vec::new();
    {
        // For the sliders the `y`, `h`, `w`, `w_f`, `margin` and
        // `x_step` are constant

        // Scale for width of the drawn slider
        let w_f = 0.2;
        {
            // Top and bottom
            let margin = 0.1;
            let x_step = 1.0 / (1.0 + channels.len() as f32);
            let y = y + h * margin;
            let h = h - 2.0 * h * margin;
            let mut idx: u8 = 1;
            for c in channels.iter() {
                let x = x + idx as f32 * x_step;
                let slider = Slider::new(x, y, x_step, h, margin, w_f, c.1, idx, osc.clone());
                sliders.push(slider);
                idx += 1;
            }
        }
    }
    // The mixer that controls the volumes of the effects
    let effects_mixer = EffectContainer::new(pedal_state, sliders, x, y, w, h);
    effects_mixer.init();

    // Main screen
    let rects: Vec<Box<dyn TouchRectFn>> = vec![
        Box::new(main_button),
        Box::new(effects_mixer),
        Box::new(mute_button),
        Box::new(tuner_display),
    ];
    let mut inside = HashMap::new();
    for i in 0..rects.len() {
        inside.insert(i, false);
    }

    let mut tsc = TouchScreenCtl {
        rects,
        width: app.width,
        height: app.height,
    };

    let paint_screen = |app: &mut App, tsc: &mut TouchScreenCtl| {
        app.window.clear();
        for i in tsc.rects.iter_mut() {
            i.paint(app);
        }
    };

    paint_screen(&mut app, &mut tsc);

    while app.window.next_frame() {
        while app.window.has_event() {
            let e = app.window.next_event();

            if let Event::Mouse {
                mouse_x,
                mouse_y,
                event_type,
                button,
            } = e
            {
                // Transform x/y from mouse event
                let width = app.window.drawable_size().0;
                let height = app.window.drawable_size().1;
                let x_p = height as f32 * mouse_x as f32 / width as f32;
                let y_p = width as f32 * mouse_y as f32 / height as f32;
                let x = (width as f32 - y_p).round() as i32;
                let y = x_p.round() as i32;
                let me = Event::Mouse {
                    event_type,
                    button,
                    mouse_x: x,
                    mouse_y: y,
                };
                eprintln!("GUI Transform event: {mouse_x}x{mouse_y} -> {x}x{y}");
                tsc.event(&me);
                paint_screen(&mut app, &mut tsc);
                {
                    app.window.set_color(255, 0, 0, 255);
                    let r = Rect::new(x - 20, y - 2, 40, 4);
                    app.window.fill_rect(r);
                    let r = Rect::new(x - 2, y - 20, 4, 40);
                    app.window.fill_rect(r);
                }
            } else {
                tsc.event(&e);
                paint_screen(&mut app, &mut tsc);
            }
        }
        tsc.tick(&mut app);
    }
    Ok(())
}
fn main() {
    eprintln!("DBG gui: PID {}", std::process::id());
    if let Err(err) = inner_main() {
        eprintln!("Error qzn3t_gui: inner_main: {err}");
    }
}
