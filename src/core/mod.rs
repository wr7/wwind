use crate::{Window, WWindState};

use self::xcb::XCBState;

use super::WindowData;
use std::{ffi::c_void, collections::HashMap, mem::{MaybeUninit, self}, ptr::{addr_of_mut, addr_of}, sync::atomic::{self, AtomicBool, Ordering}, hash::Hash, cell::UnsafeCell, rc::Rc};

#[cfg(unix)]
mod xcb;

pub struct CoreState {
    data: Rc<UnsafeCell<CoreStateData>>,
}

pub struct CoreStateData {
    core_state: CoreStateEnum,
    windows: HashMap<CoreWindowRef, WindowData>,
    windows_to_destroy: Vec<CoreWindowRef>,
}

pub enum CoreStateEnum {
    #[cfg(unix)]
    XCB(XCBState)
}

static mut WINDOWS_TO_DESTROY: MaybeUninit<Vec<CoreWindow>> = MaybeUninit::uninit();

impl CoreState {
    unsafe fn get_xcb(&mut self) -> &mut XCBState {
        let data = self.data.get().as_mut().unwrap_unchecked();
        
        if let CoreStateEnum::XCB(state) = &mut self.data {
            state
        } else {
            panic!("get_xcb called with non-xcb state")
        }
    }
}

impl CoreState {
    pub unsafe fn wait_for_events(&mut self) -> bool {
        match self.core_state {
            CoreStateEnum::XCB(_) => xcb::wait_for_events(self),
        }
    }
    pub unsafe fn add_window(&mut self, x: i16, y: i16, height: u16, width: u16, title: &str) -> CoreWindow {
        let core_window = match &mut self.core_state {
            #[cfg(unix)]
            CoreStateEnum::XCB(xcb_state) => {xcb_state.add_window(x, y, height, width, title)},
        };
        

        let windows = &mut self.windows;
        windows.insert(core_window, Default::default());

        core_window;

        todo!()
    }

    pub fn do_windows_exist(&self) -> bool{
        !self.windows.is_empty()
    }

    pub fn destroy_pending_windows(&mut self) {
        let windows_to_destroy = unsafe{WINDOWS_TO_DESTROY.assume_init_mut()};

        windows_to_destroy.drain(..).map(|window| {
            self.destroy_window(window);
        }).count();

    }
    fn destroy_window(&mut self, window: CoreWindow) {
        match &mut self.core_state {
            CoreStateEnum::XCB(xcb_state) => unsafe {xcb_state.destroy_window(window.core_window.xcb_window)},
        }
        self.windows.remove(&window.core_window);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoreStateType {
    #[cfg(unix)]
    XCB
}

static STATE_CREATED: AtomicBool = AtomicBool::new(false);
static mut CORE_STATE_TYPE: MaybeUninit<CoreStateType> = MaybeUninit::uninit();

unsafe fn on_window_close<'a>(state: &'a mut CoreState, core_window: CoreWindowRef) {
    // if let Some(window_data) = state.windows.get_mut(&core_window) {
    //     let on_close = window_data.on_close.take();
    //     mem::drop(window_data);

    //     if let Some(mut on_close) = on_close {

    //         let window: Window<'a> = Window {window: core_window, _unsend: Default::default(), _phantom_data: Default::default()};
    //         let mut wwind_state = WWindState {state: state as *mut CoreState, _unsend: Default::default()};

    //         on_close(&mut wwind_state, window);

    //         state.windows.get_mut(&core_window).map(|window_data| window_data.on_close = Some(on_close));
    //         mem::forget(wwind_state);

    //         // window_data.on_close = Some(on_close); // UNDEFINED BEHAVIOR
    //     } else {
    //         println!("No on close for window");
    //         core_window.schedule_window_destruction();
    //     }
    // } else {
    //     println!("on_window_close called on non-existant window");
    // }

    todo!()
}

impl CoreState {
    pub fn new() -> Option<Self> {
        if !STATE_CREATED.fetch_or(true, atomic::Ordering::Acquire) {
            unsafe {WINDOWS_TO_DESTROY.write(Vec::new())};

            unsafe {MaybeUninit::write(&mut CORE_STATE_TYPE, CoreStateType::XCB)};

            let xcb_state = unsafe {XCBState::new().unwrap()};

            let core_state = CoreStateEnum::XCB(xcb_state);
            let windows = HashMap::new();
            let windows_to_destroy = Vec::new();

            Some(CoreState {core_state, windows, windows_to_destroy})
        } else {
            None
        }
    }
}

impl Drop for CoreState {
    fn drop(&mut self) {
        unsafe {
            CORE_STATE_TYPE.assume_init_drop();
            WINDOWS_TO_DESTROY.assume_init_drop();

            STATE_CREATED.store(false, atomic::Ordering::Release);
        }
    }
}

/// Represents a window.
/// Note: this doesn't destroy the window upon drop
#[derive(Clone)]
pub struct CoreWindow {
    core_window: CoreWindowRef,
    core_state: Rc<UnsafeCell<CoreState>>,
}

// Represents a reference to a window
#[derive(Clone, Copy)]
union CoreWindowRef {
    xcb_window: self::xcb::xcb_window_t,
}

impl PartialEq for CoreWindow {
    fn eq(&self, other: &Self) -> bool {
        self.core_window == other.core_window
    }
}

impl PartialEq for CoreWindowRef {
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
impl Eq for CoreWindowRef {}
impl Hash for CoreWindowRef {
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
    pub unsafe fn schedule_window_destruction(&mut self) {
        let windows_to_destroy = WINDOWS_TO_DESTROY.assume_init_mut();
        if !windows_to_destroy.contains(&self) {
            windows_to_destroy.push(todo!());
        }
    }
    pub fn on_window_close_attempt<F: for<'a> FnMut(&'a mut WWindState, Window<'a>) + 'static>(&mut self, closure: F) {
        let core_state = unsafe {
            self.core_state.get().as_mut().unwrap()
        };

        core_state.windows.get_mut(&self.core_window).map(|data| data.on_close = Some(Box::new(closure)));
    }
}