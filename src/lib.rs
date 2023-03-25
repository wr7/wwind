#[allow(non_camel_case_types)]
#[allow(non_upper_case_globals)]
#[allow(non_snake_case)]


use crate::core::{CoreWindow, CoreState, CoreStateData};
use std::{mem, sync::atomic::AtomicBool, collections::HashMap, ptr::{addr_of, addr_of_mut}, marker::PhantomData};

use util::PhantomUnsend;

mod core;
mod util;

static mut SHOULD_EXIT: bool = false;

struct WindowData {
    on_close: Option<Box<dyn for<'a> FnMut(&'a mut WWindState, Window<'a>)>>,
}

impl Default for WindowData {
    fn default() -> Self {
        let on_close = None;
        Self { on_close }
    }
}

pub struct Window<'a> {
    window: core::CoreWindow,
    _unsend: PhantomUnsend,
    _phantom_data: PhantomData<&'a()>,
}

impl Window<'_> {
    pub fn schedule_window_destruction(&mut self) {
        unsafe {self.window.schedule_window_destruction()};
    }
    pub fn on_window_close<F: for<'a> FnMut(&'a mut WWindState, Window<'a>)+'static>(&mut self, closure: F) {
        self.window.on_window_close_attempt(closure);
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
        

        while !unsafe {SHOULD_EXIT} && self.state.do_windows_exist() {
            unsafe {self.state.wait_for_events()};
            self.state.destroy_pending_windows();
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