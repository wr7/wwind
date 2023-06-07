use crate::core::CoreDrawingContext;

#[allow(non_camel_case_types)]
#[allow(non_upper_case_globals)]
#[allow(non_snake_case)]
use std::marker::PhantomData;

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
mod state;
mod util;
mod window;

pub use state::WWindState;
pub use window::Window;

static mut SHOULD_EXIT: bool = false;

#[derive(Clone, Copy, Debug)]
pub struct WindowPositionData {
    pub width: u16,
    pub height: u16,
    pub x: i16,
    pub y: i16,
}

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

pub struct DrawingContext<'a> {
    context: CoreDrawingContext,
    _unsend: PhantomUnsend,
    _phantom_data: PhantomData<&'a ()>,
}

impl<'a> DrawingContext<'a> {
    pub fn draw_line(&mut self, x1: u16, y1: u16, x2: u16, y2: u16) {
        self.context.draw_line(x1, y1, x2, y2);
    }

    pub fn draw_rectangle(&mut self, rectangle: RectRegion) {
        self.context.draw_rectangle(rectangle);
    }

    pub fn set_draw_color(&mut self, color: Color) {
        self.context.set_draw_color(color);
    }

    pub(crate) fn from_core_context(context: CoreDrawingContext) -> Self {
        Self {
            context,
            _unsend: Default::default(),
            _phantom_data: Default::default(),
        }
    }
}

pub struct WWindInstance<OnInit: FnOnce(&mut WWindState)> {
    state: WWindState,
    on_init: OnInit,
    _unsend: PhantomUnsend,
}

impl<OnInit: FnOnce(&mut WWindState)> WWindInstance<OnInit> {
    pub fn new(on_init: OnInit) -> Option<Self> {
        let state = WWindState::new()?;
        let _unsend = Default::default();

        Some(Self {
            on_init,
            state,
            _unsend,
        })
    }

    pub fn run(mut self) {
        (self.on_init)(&mut self.state);

        unsafe {
            while !SHOULD_EXIT && self.state.do_windows_exist() {
                self.state.wait_for_events();
                self.state.destroy_pending_windows();
            }

            self.state.destroy();
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {}
}
