//! wwind is a cross-platform Rust UI library that is meant to provide a
//! nice programming experience and significantly reduce runtime errors and
//! undefined/unexpected behavior. It is meant to do all of this while still
//! providing reasonable performance and platform consistency.
//! # Example
//! ```rust,no_run
//! use wwind::{WWindInstance, WWindState, Window, Color, RectRegion};
//!
//! fn main() {
//!     let instance = WWindInstance::new(|state: &mut WWindState| {
//!         let mut window = state.add_window(100, 100, 500, 500, "test title");
//!
//!         window.on_redraw(|_, window, _| {
//!             let (w, h) = window.get_size();
//!             let window_rect = RectRegion { x: 0, y: 0, width: w, height: h };
//!
//!             let mut context = window.get_drawing_context();
//!
//!             // Draw background //
//!
//!             context.set_draw_color(Color::from_hex(0xffffff));
//!             context.draw_rectangle(window_rect);
//!             
//!             // Draw lines //
//!
//!             context.set_draw_color(Color::from_rgb(0, 255, 0));
//!             context.draw_line(0, 0, w, h);
//!             
//!             context.set_draw_color(Color::from_rgb(255,0,0));
//!             context.draw_line(w, 0, 0, h);
//!         });
//!     });
//!
//!     instance.unwrap().run();
//! }
//! ```
//! ![Expected Output](https://github.com/wr7/wwind/raw/master/docs/example.png)

use std::{marker::PhantomData, mem::MaybeUninit};

//  TODO:
// -  fix Win32Data leak
// -  add support for color modes besides TrueColor

#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl From<Color> for u32 {
    fn from(value: Color) -> Self {
        value.red as u32 | ((value.green as u32) << 8) | ((value.blue as u32) << 16)
    }
}

impl From<u32> for Color {
    fn from(value: u32) -> Self {
        Self::from_hex(value)
    }
}

impl Color {
    pub const fn from_hex(hex: u32) -> Self {
        Self {
            blue: (hex & 0x000000FF) as u8,
            green: ((hex & 0x0000FF00) >> 8) as u8,
            red: ((hex & 0x00FF0000) >> 16) as u8,
        }
    }
    pub const fn from_rgb(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }
}

use util::PhantomUnsend;

mod core;
mod drawing_context;
mod state;
mod util;
mod window;

pub use drawing_context::DrawingContext;
pub use state::WWindInitState;
pub use state::WWindState;
pub use window::Window;

static mut SHOULD_EXIT: bool = false;

#[derive(Clone, Copy, Debug)]
pub struct RectRegion {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl RectRegion {
    pub fn get_bottom_y(&self) -> u16 {
        self.height + self.y
    }
    pub fn get_right_x(&self) -> u16 {
        self.x + self.width
    }
}

pub struct WWindInstance<OnInit: FnOnce(&mut WWindInitState<UserData>) -> UserData, UserData = ()> {
    state: WWindInitState<UserData>,
    on_init: OnInit,
    _unsend: PhantomUnsend,
}

impl<OnInit: FnOnce(&mut WWindInitState<UserData>) -> UserData, UserData>
    WWindInstance<OnInit, UserData>
{
    pub fn new(on_init: OnInit) -> Option<Self> {
        let state = WWindInitState::new()?;
        let _unsend = Default::default();

        Some(Self {
            on_init,
            state,
            _unsend,
        })
    }

    pub fn run(mut self) {
        let userdata = (self.on_init)(&mut self.state);
        let userdata = Box::into_raw(Box::new(userdata));

        unsafe { state::USERDATA = userdata as *mut _ };

        unsafe {
            while !SHOULD_EXIT && self.state.do_windows_exist() {
                self.state.wait_for_events();
                self.state.destroy_pending_windows();
            }

            self.state.destroy();
        }
    }
}
