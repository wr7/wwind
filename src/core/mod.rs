use crate::{Window, WWindState};

use self::{core_state_implementation::{CoreWindowRef, CoreStateEnum}};

#[cfg(xcb)]
use self::xcb::XCBState;

#[cfg(x11)]
use self::x11rb::X11RbState;

use super::WindowData;
use std::{ffi::c_void, collections::HashMap, mem::{MaybeUninit, self}, ptr::{addr_of_mut, addr_of}, sync::atomic::{self, AtomicBool, Ordering}, hash::Hash, cell::UnsafeCell, rc::Rc};

mod core_state_implementation;

pub use core_state_implementation::CoreStateImplementation;

#[cfg(xcb)]
mod xcb;
#[cfg(x11)]
mod x11rb;

#[derive(Clone)]
pub struct CoreState {
    data: Rc<UnsafeCell<CoreStateData>>,
}

pub struct CoreStateData {
    core_state: CoreStateEnum,
    windows: HashMap<CoreWindowRef, WindowData>,
    windows_to_destroy: Vec<CoreWindowRef>,
}

impl CoreState {
    fn get_data_mut(&mut self) -> &mut CoreStateData {
        unsafe {self.data.get().as_mut().unwrap_unchecked()}
    }

    unsafe fn get_data(&self) -> &mut CoreStateData {
        self.data.get().as_mut().unwrap_unchecked()
    }

    #[cfg(xcb)]
    unsafe fn get_xcb(&mut self) -> &mut XCBState {
        if let CoreStateEnum::XCB(state) = &mut self.get_data_mut().core_state {
            state
        } else {
            panic!("get_xcb called with non-xcb state")
        }
    }

    #[cfg(x11)]
    unsafe fn get_x11(&mut self) -> &mut X11RbState {
        if let CoreStateEnum::X11(state) = &mut self.get_data_mut().core_state {
            state
        } else {
            panic!("get_x11 called with non-x11 state")
        }
    }
}

impl CoreState {
    pub fn get_window_from_ref(&self, core_window_ref: CoreWindowRef) -> CoreWindow {
        let core_state_data = self.data.clone();

        CoreWindow { core_window_ref, core_state_data }
    }

    pub unsafe fn wait_for_events(&mut self) -> bool {
        CoreStateEnum::wait_for_events(self)
    }
    pub unsafe fn add_window(&mut self, x: i16, y: i16, height: u16, width: u16, title: &str) -> CoreWindow {
        let core_window_ref = self.get_data_mut().core_state.add_window(x, y, height, width, title).unwrap();

        let windows = &mut self.get_data_mut().windows;
        windows.insert(core_window_ref, Default::default());

        let core_state = self.data.clone();

        CoreWindow {core_state_data: core_state, core_window_ref}
    }

    pub fn do_windows_exist(&self) -> bool{
        !unsafe{self.get_data()}.windows.is_empty()
    }

    /// Destroys the windows that were scheduled for deletion
    /// ## Safety
    /// This function is unsafe if a CoreWindow exists for a window that's scheduled for deletion
    pub unsafe fn destroy_pending_windows(&mut self) {
        while let Some(window_ref) = self.get_data_mut().windows_to_destroy.pop() {
            self.destroy_window(window_ref)
        }
    }

    /// Directly destroys the underlying Window
    /// ## Safety
    /// This function is unsafe if a CoreWindow for this window exists after this function is called
    unsafe fn destroy_window(&mut self, window: CoreWindowRef) {
        self.get_data_mut().core_state.destroy_window(window);

        self.get_data_mut().windows.remove(&window);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoreStateType {
    #[cfg(xcb)]
    XCB,
    #[cfg(x11)]
    X11,
}

static STATE_CREATED: AtomicBool = AtomicBool::new(false);
static mut CORE_STATE_TYPE: MaybeUninit<CoreStateType> = MaybeUninit::uninit();

unsafe fn on_window_close<'a>(state: &'a mut CoreState, core_window_ref: CoreWindowRef) {
    if let Some(window_data) = state.get_data().windows.get_mut(&core_window_ref) {
        let on_close = window_data.on_close.take();
        mem::drop(window_data);

        if let Some(mut on_close) = on_close {

            let core_window = state.get_window_from_ref(core_window_ref);

            let window: Window<'a> = Window {window: core_window, _unsend: Default::default(), _phantom_data: Default::default()};
            let mut wwind_state = WWindState {state: state as *mut CoreState, _unsend: Default::default()};

            on_close(&mut wwind_state, window);

            state.get_data().windows.get_mut(&core_window_ref).map(|window_data| window_data.on_close = Some(on_close));
            mem::forget(wwind_state);

            // window_data.on_close = Some(on_close); // UNDEFINED BEHAVIOR
        } else {
            println!("No on close for window");
            state.get_window_from_ref(core_window_ref).schedule_window_destruction();
        }
    } else {
        println!("on_window_close called on non-existant window");
    }
}

impl CoreState {
    pub fn new() -> Option<Self> {
        if !STATE_CREATED.fetch_or(true, atomic::Ordering::Acquire) {

            let core_state = unsafe{CoreStateEnum::new()}.unwrap();
            let windows = HashMap::new();
            let windows_to_destroy = Vec::new();

            let data = CoreStateData {core_state, windows, windows_to_destroy};

            Some(CoreState {data: Rc::new(UnsafeCell::new(data))})
        } else {
            None
        }
    }
}

impl Drop for CoreState {
    fn drop(&mut self) {
        unsafe {
            CORE_STATE_TYPE.assume_init_drop();

            STATE_CREATED.store(false, atomic::Ordering::Release);
        }
    }
}

/// Represents a window.
/// Note: this doesn't destroy the window upon drop
#[derive(Clone)]
pub struct CoreWindow {
    core_window_ref: CoreWindowRef,
    core_state_data: Rc<UnsafeCell<CoreStateData>>,
}

impl PartialEq for CoreWindow {
    fn eq(&self, other: &Self) -> bool {
        self.core_window_ref == other.core_window_ref
    }
}

impl CoreWindow {
    /// Unsafe: function can only be called while the CoreState exists and on the thread where it was created
    pub unsafe fn schedule_window_destruction(&mut self) {
        let core_window_ref = self.core_window_ref;
        let windows_to_destroy = &mut self.get_core_state_data_mut().windows_to_destroy;

        if !windows_to_destroy.contains(&core_window_ref) {
            windows_to_destroy.push(core_window_ref);
        }
    }
    pub fn get_core_state_data_mut(&mut self) -> &mut CoreStateData {
        unsafe {self.core_state_data.get().as_mut().unwrap()}
    }

    pub fn on_window_close_attempt<F: for<'a> FnMut(&'a mut WWindState, Window<'a>) + 'static>(&mut self, closure: F) {
        let window_ref = self.core_window_ref;

        self.get_core_state_data_mut().windows.get_mut(&window_ref).map(|data| data.on_close = Some(Box::new(closure)));
    }
}