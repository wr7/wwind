use crate::{Window, WWindState};

use self::xcb::XCBState;

use super::WindowData;
use std::{ffi::c_void, collections::HashMap, mem::{MaybeUninit, self}, ptr::{addr_of_mut, addr_of}, sync::atomic::{self, AtomicBool, Ordering}, hash::Hash};

#[cfg(unix)]
mod xcb;

pub enum CoreState {
    #[cfg(unix)]
    XCB(XCBState),
}

static mut WINDOWS: MaybeUninit<HashMap<CoreWindow, WindowData>> = MaybeUninit::uninit();
static mut WINDOWS_TO_DESTROY: MaybeUninit<Vec<CoreWindow>> = MaybeUninit::uninit();

impl CoreState {
    fn get_xcb(&mut self) -> &mut XCBState {
        if let CoreState::XCB(state) = self {
            state
        } else {
            panic!("get_xcb called with non-xcb state")
        }
    }
}

impl CoreState {
    pub unsafe fn wait_for_events(&mut self) -> bool {
        match self {
            CoreState::XCB(_) => xcb::wait_for_events(self),
        }
    }
    pub unsafe fn add_window(&mut self, x: i16, y: i16, height: u16, width: u16, title: &str) -> CoreWindow {
        let core_window = match self {
            #[cfg(unix)]
            CoreState::XCB(xcb_state) => {xcb_state.add_window(x, y, height, width, title)},
        };

        let windows = WINDOWS.assume_init_mut();
        windows.insert(core_window, Default::default());
        core_window
    }
    pub fn do_windows_exist(&self) -> bool{
        !unsafe {WINDOWS.assume_init_ref().is_empty()}
    }

    pub fn destroy_pending_windows(&mut self) {
        let windows_to_destroy = unsafe{WINDOWS_TO_DESTROY.assume_init_mut()};

        windows_to_destroy.drain(..).map(|window| {
            self.destroy_window(window);
        }).count();

    }
    fn destroy_window(&mut self, window: CoreWindow) {
        match self {
            CoreState::XCB(xcb_state) => unsafe {xcb_state.destroy_window(window.xcb_window)},
        }
        unsafe {WINDOWS.assume_init_mut()}.remove(&window);
    }
}

pub struct CoreStateData {
    windows: HashMap<CoreWindow, WindowData>
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoreStateType {
    #[cfg(unix)]
    XCB
}

static STATE_CREATED: AtomicBool = AtomicBool::new(false);
static mut CORE_STATE_TYPE: MaybeUninit<CoreStateType> = MaybeUninit::uninit();

unsafe fn on_window_close<'a>(state: &'a mut CoreState, core_window: CoreWindow) {
    if let Some(window_data) = WINDOWS.assume_init_mut().get_mut(&core_window) {
        let on_close = window_data.on_close.take();
        mem::drop(window_data);

        if let Some(mut on_close) = on_close {

            let window: Window<'a> = Window {window: core_window, _unsend: Default::default(), _phantom_data: Default::default()};
            let mut wwind_state = WWindState {state: state as *mut CoreState, _unsend: Default::default()};

            on_close(&mut wwind_state, window);

            WINDOWS.assume_init_mut().get_mut(&core_window).map(|window_data| window_data.on_close = Some(on_close));
            mem::forget(wwind_state);

            // window_data.on_close = Some(on_close); // UNDEFINED BEHAVIOR
        } else {
            println!("No on close for window");
            core_window.schedule_window_destruction();
        }
    } else {
        println!("on_window_close called on non-existant window");
    }
}

impl CoreState {
    pub fn new() -> Option<Self> {
        if !STATE_CREATED.fetch_or(true, atomic::Ordering::Acquire) {
            unsafe {WINDOWS.write(HashMap::new())};
            unsafe {WINDOWS_TO_DESTROY.write(Vec::new())};

            unsafe {MaybeUninit::write(&mut CORE_STATE_TYPE, CoreStateType::XCB)};

            let xcb_state = unsafe {XCBState::new().unwrap()};

            Some(CoreState::XCB(xcb_state))
        } else {
            None
        }
    }
}

impl Drop for CoreState {
    fn drop(&mut self) {
        unsafe {
            CORE_STATE_TYPE.assume_init_drop();
            WINDOWS.assume_init_drop();
            WINDOWS_TO_DESTROY.assume_init_drop();

            STATE_CREATED.store(false, atomic::Ordering::Release);
        }
    }
}

/// Represents a reference to a window.
/// Note: this doesn't destroy the window upon drop
#[derive(Clone, Copy)]
pub union CoreWindow {
    xcb_window: self::xcb::xcb_window_t,
}

impl CoreWindow {

}

impl PartialEq for CoreWindow {
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            if !STATE_CREATED.load(Ordering::Acquire) {
                panic!("PartialEq called with no loaded state");
            }
            let state_type = CORE_STATE_TYPE.assume_init_ref();

            match state_type {
                CoreStateType::XCB => {self.xcb_window == other.xcb_window}
            }
        }
    }
}
impl Eq for CoreWindow {}
impl Hash for CoreWindow {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        unsafe {
            if !STATE_CREATED.load(Ordering::Acquire) {
                panic!("Hash called with no loaded state");
            }

            let state_type = CORE_STATE_TYPE.assume_init_ref();

            match state_type {
                CoreStateType::XCB => {self.xcb_window.hash(state)}
            }
        }
    }
}

impl CoreWindow {
    /// Unsafe: function can only be called while the CoreState exists and on the thread where it was created
    pub unsafe fn schedule_window_destruction(self) {
        let windows_to_destroy = WINDOWS_TO_DESTROY.assume_init_mut();
        if !windows_to_destroy.contains(&self) {
            windows_to_destroy.push(self);
        }
    }
    pub fn on_window_close_attempt<F: for<'a> FnMut(&'a mut WWindState, Window<'a>) + 'static>(self, closure: F) {
        unsafe{WINDOWS.assume_init_mut()}.get_mut(&self).map(|data| data.on_close = Some(Box::new(closure)));
    }
}