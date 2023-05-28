#[allow(non_camel_case_types)]
#[allow(non_upper_case_globals)]
#[allow(non_snake_case)]

//  TODO: 
// -  remove RC in CoreWindowState
// -  test library



use crate::core::{CoreWindow, CoreState, CoreStateData};
use std::{mem, sync::atomic::AtomicBool, collections::HashMap, ptr::{addr_of, addr_of_mut}, marker::PhantomData};

use util::{PhantomUnsend, ForgetGuard};

mod core;
mod util;

static mut SHOULD_EXIT: bool = false;

#[derive(Clone, Copy, Debug)]
pub struct WindowPositionData {
    pub width: u16, pub height: u16,
    pub x: i16, pub y: i16,
}

#[derive(Clone, Copy, Debug)]
pub struct RectRegion {
    pub x: u16, pub y: u16,
    pub width: u16, pub height: u16,
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
    _phantom_data: PhantomData<&'a()>,
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
        unsafe {self.window.schedule_window_destruction()};
    }
    pub fn on_window_close<F: for<'a> FnMut(&'a mut WWindState, Window<'a>)+'static>(&mut self, closure: F) {
        self.window.on_window_close_attempt(closure);
    }
    pub fn on_redraw<F: for<'a> FnMut(&'a mut WWindState, Window<'a>, RectRegion)+'static>(&mut self, closure: F) {
        self.window.on_redraw(closure);
    }
    pub fn draw_line(&mut self, x1: i16, y1: i16, x2: i16, y2: i16) {
        self.window.draw_line(x1, y1, x2, y2)
    }
    pub fn position_data(&mut self) -> WindowPositionData {
        self.window.get_position_data()
    }
}

pub struct WWindInstance<OnInit: FnOnce(&mut WWindState)> {
    state: CoreState,
    on_init: OnInit,
    _unsend: PhantomUnsend,
}

impl<OnInit: FnOnce(&mut WWindState)> WWindInstance<OnInit> {
    pub fn new(on_init: OnInit) -> Option<Self>{
        let state = CoreState::new()?;
        let _unsend = Default::default();

        Some(Self {on_init, state, _unsend})
    }
    
    pub fn run(mut self){
        
        let mut state = WWindState {state: &mut self.state as *mut CoreState, _unsend: Default::default()};
        (self.on_init)(&mut state);
        mem::forget(state);
        

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
    fn it_works() {

    }
}

#[repr(C)]
pub struct WWindState{
    state: *mut CoreState,
    _unsend: PhantomUnsend,
}

impl WWindState {
    pub(crate) fn from_core_state<'a>(state: &'a mut CoreState) -> ForgetGuard<'a, Self> {
        ForgetGuard::new(Self {state: state as *mut CoreState, _unsend: Default::default()})
    }

    pub fn schedule_exit(&mut self) {
        unsafe {SHOULD_EXIT = true};
    }
    pub fn add_window<'a>(&'a mut self, x: i16, y: i16, height: u16, width: u16, title: &str) -> Window<'a> {
        let core_state = unsafe {self.state.as_mut().unwrap()};
        let window = unsafe{core_state.add_window(x, y, height, width, title)};
        let _unsend = Default::default();
        let _phantom_data = Default::default();
        
        Window { window, _unsend, _phantom_data }
    }
}