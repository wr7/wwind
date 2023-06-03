use crate::core::CoreDrawingContext;
use crate::core::DrawingContextEnum;
#[allow(non_camel_case_types)]
#[allow(non_upper_case_globals)]
#[allow(non_snake_case)]
use crate::core::{CoreState, CoreStateData, CoreWindow};
use std::{
    collections::HashMap,
    marker::PhantomData,
    mem::{self, transmute_copy},
    ptr::{addr_of, addr_of_mut},
    sync::atomic::AtomicBool,
};

//  TODO:
// -  remove RC in CoreWindowState
//   * RC is leaked when CoreState is dropped
// -  add support for color modes besides TrueColor
// - Fix modal message loops on window
//   * Why are these a thing? Can't poorly written programs just run poorly?
//     Why do we need to force modal message loops on everyone in a poor attempt to fix this issue?
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
            blue: (hex & 0xFF) as u8,
            green: ((hex & 0xFF00) >> 8) as u8,
            red: (hex & 0xFF0000 >> 16) as u8,
        }
    }
    pub const fn from_rgb(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }
}

use util::{ForgetGuard, PhantomUnsend};

mod core;
mod util;

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

struct WindowData {
    on_close: Option<Box<dyn for<'a> FnMut(&'a mut WWindState, Window<'a>)>>,
    redraw: Option<Box<dyn for<'a> FnMut(&'a mut WWindState, Window<'a>, RectRegion)>>,
}

impl WindowData {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            on_close: None,
            redraw: None,
        }
    }
}

pub struct Window<'a> {
    window: core::CoreWindow,
    _unsend: PhantomUnsend,
    _phantom_data: PhantomData<&'a ()>,
}

impl Window<'_> {
    pub(crate) fn from_core_window(window: CoreWindow) -> Self {
        Self {
            window,
            _unsend: Default::default(),
            _phantom_data: PhantomData,
        }
    }

    pub fn schedule_window_destruction(&mut self) {
        unsafe { self.window.schedule_window_destruction() };
    }
    pub fn on_window_close<F: for<'a> FnMut(&'a mut WWindState, Window<'a>) + 'static>(
        &mut self,
        closure: F,
    ) {
        self.window.on_window_close_attempt(closure);
    }
    pub fn on_redraw<F: for<'a> FnMut(&'a mut WWindState, Window<'a>, RectRegion) + 'static>(
        &mut self,
        closure: F,
    ) {
        self.window.on_redraw(closure);
    }
    pub fn position_data(&mut self) -> WindowPositionData {
        self.window.get_position_data()
    }

    pub fn get_drawing_context<'a>(&'a mut self) -> DrawingContext<'a> {
        DrawingContext::from_core_context(self.window.get_drawing_context())
    }
}

pub struct WWindInstance<OnInit: FnOnce(&mut WWindState)> {
    state: CoreState,
    on_init: OnInit,
    _unsend: PhantomUnsend,
}

impl<OnInit: FnOnce(&mut WWindState)> WWindInstance<OnInit> {
    pub fn new(on_init: OnInit) -> Option<Self> {
        let state = CoreState::new()?;
        let _unsend = Default::default();

        Some(Self {
            on_init,
            state,
            _unsend,
        })
    }

    pub fn run(mut self) {
        let mut state = WWindState::from_core_state(&mut self.state);
        (self.on_init)(&mut state);

        unsafe {
            while !SHOULD_EXIT && self.state.do_windows_exist() {
                self.state.wait_for_events();
                self.state.destroy_pending_windows();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}

#[repr(C)]
pub struct WWindState {
    state: CoreState,
    _unsend: PhantomUnsend,
}

impl WWindState {
    pub(crate) fn from_core_state<'a>(state: &'a mut CoreState) -> ForgetGuard<'a, Self> {
        let state = state.clone();

        ForgetGuard::new(Self {
            state,
            _unsend: Default::default(),
        })
    }

    pub fn schedule_exit(&mut self) {
        unsafe { SHOULD_EXIT = true };
    }

    pub fn add_window<'a>(
        &'a mut self,
        x: i16,
        y: i16,
        height: u16,
        width: u16,
        title: &str,
    ) -> Window<'a> {
        let window = unsafe { self.state.add_window(x, y, height, width, title) };
        let _unsend = Default::default();
        let _phantom_data = Default::default();

        Window {
            window,
            _unsend,
            _phantom_data,
        }
    }
}
