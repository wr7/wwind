use std::{sync::atomic, mem::MaybeUninit, collections::HashMap, cmp::Ordering, hash::Hash};
use super::{
    CoreState, 
    STATE_CREATED, 
    CoreStateType, 
    CORE_STATE_TYPE, 
    CoreStateData
};

#[cfg(xcb)]
use super::xcb::XCBState;

#[cfg(x11)]
use super::x11rb::{RbError, X11RbState};


pub trait CoreStateImplementation: Sized {
    /// The error that can occur when initializing the state
    type Error;
    /// The type internally used to represent a window
    type Window: Sized + Copy;

    /// ## Safety
    /// Should not be called while another CoreStateImplementation exists
    unsafe fn new() -> Result<Self, Self::Error>;
    fn add_window(&mut self, x: i16, y: i16, height: u16, width: u16, title: &str) -> Result<Self::Window, Self::Error>;
    fn set_window_title(&mut self, window: Self::Window, title: &str);
    /// ## Safety
    /// The same window should not be destroyed twice
    unsafe fn destroy_window(&mut self, window: Self::Window);
    unsafe fn wait_for_events(state: &mut CoreState) -> bool;
}

/// An enumeration over all of the [CoreStateImplementation]s.
pub enum CoreStateEnum {
    #[cfg(xcb)]
    XCB(XCBState),
    #[cfg(x11)]
    X11(X11RbState),
}

/// Represents a reference to a window from any [CoreStateImplementation].
#[derive(Clone, Copy)]
pub union CoreWindowRef {
    #[cfg(xcb)]
    xcb: <XCBState as CoreStateImplementation>::Window,
    #[cfg(x11)]
    x11: <X11RbState as CoreStateImplementation>::Window,
}

impl CoreWindowRef {
    #[cfg(x11)]
    pub unsafe fn from_x11(x11: <X11RbState as CoreStateImplementation>::Window) -> Self {
        Self {x11}
    }
    #[cfg(xcb)]
    pub unsafe fn from_xcb(xcb: <XCBState as CoreStateImplementation>::Window) -> Self {
        Self {xcb}
    }
    #[cfg(xcb)]
    pub unsafe fn xcb(self) -> <XCBState as CoreStateImplementation>::Window {
        self.xcb
    }
    #[cfg(x11)]
    pub unsafe fn x11(self) -> <X11RbState as CoreStateImplementation>::Window {
        self.x11
    }
}

impl PartialEq for CoreWindowRef {
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            if !STATE_CREATED.load(atomic::Ordering::Acquire) {
                panic!("PartialEq called with no loaded state");
            }
            let state_type = CORE_STATE_TYPE.assume_init_ref();

            match state_type {
                #[cfg(xcb)]
                CoreStateType::XCB => self.xcb() == other.xcb(),
                #[cfg(x11)]
                CoreStateType::X11 => self.x11() == other.x11(),
            }
        }
    }
}

impl Eq for CoreWindowRef {}

impl Hash for CoreWindowRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        unsafe {
            if !STATE_CREATED.load(atomic::Ordering::Acquire) {
                panic!("Hash called with no loaded state");
            }

            let state_type = CORE_STATE_TYPE.assume_init_ref();

            match state_type {
                #[cfg(xcb)]
                CoreStateType::XCB => self.xcb().hash(state),
                #[cfg(x11)]
                CoreStateType::X11 => self.x11().hash(state),
            }
        }
    }
}

#[derive(Debug)]
pub enum CoreError {
    #[cfg(x11)]
    RbError(<X11RbState as CoreStateImplementation>::Error),
    #[cfg(xcb)]
    XCBError(<XCBState as CoreStateImplementation>::Error),
    StateExists(),
}

#[cfg(x11)]
impl From<<X11RbState as CoreStateImplementation>::Error> for CoreError {
    fn from(value: <X11RbState as CoreStateImplementation>::Error) -> Self {
        CoreError::RbError(value)
    }
}

#[cfg(xcb)]
impl From<<XCBState as CoreStateImplementation>::Error> for CoreError {
    fn from(value: <XCBState as CoreStateImplementation>::Error) -> Self {
        CoreError::XCBError(value)
    }
}

impl CoreStateImplementation for CoreStateEnum {
    type Error = CoreError;

    type Window = CoreWindowRef;

    unsafe fn new() -> Result<Self, Self::Error> {
        unsafe {MaybeUninit::write(&mut CORE_STATE_TYPE, CoreStateType::X11)};

        let x11_state = unsafe {X11RbState::new().unwrap()};

        let core_state = CoreStateEnum::X11(x11_state);

        Ok(core_state)
    }

    fn add_window(&mut self, x: i16, y: i16, height: u16, width: u16, title: &str) -> Result<Self::Window, Self::Error> {
        unsafe {
            let window = match self {
                #[cfg(xcb)]
                CoreStateEnum::XCB(xcb_state) => Self::Window::from_xcb(xcb_state.add_window(x, y, height, width, title)?),
                #[cfg(x11)]
                CoreStateEnum::X11(x11_state) => Self::Window::from_x11(x11_state.add_window(x, y, height, width, title)?),
            };
            Ok(window)
        }
    }

    fn set_window_title(&mut self, window: Self::Window, title: &str) {
        unsafe {match self {
            #[cfg(xcb)]
            CoreStateEnum::XCB(xcb_state) => xcb_state.set_window_title(Self::Window::xcb(window), title),
            #[cfg(x11)]
            CoreStateEnum::X11(x11_state) => x11_state.set_window_title(Self::Window::x11(window), title),
        }}
    }

    unsafe fn destroy_window(&mut self, window: Self::Window) {
        match self {
            #[cfg(xcb)]
            CoreStateEnum::XCB(s) => s.destroy_window(CoreWindowRef::xcb(window)),
            #[cfg(x11)]
            CoreStateEnum::X11(s) => s.destroy_window(CoreWindowRef::x11(window)),
        }
    }

    unsafe fn wait_for_events(state: &mut CoreState) -> bool {
        let core_state_type = CORE_STATE_TYPE.assume_init_ref();

        match core_state_type {
            #[cfg(xcb)]
            CoreStateType::XCB => XCBState::wait_for_events(state),
            #[cfg(x11)]
            CoreStateType::X11 => X11RbState::wait_for_events(state),
        }
    }
}