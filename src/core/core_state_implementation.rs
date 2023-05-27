use std::{sync::atomic, mem::MaybeUninit, collections::HashMap, cmp::Ordering, hash::Hash, convert::Infallible};
use crate::WindowPositionData;

use super::{
    CoreState, 
    STATE_CREATED, 
    CoreStateType, 
    CORE_STATE_TYPE, 
    CoreStateData, CoreWindow
};

#[cfg(xcb)]
use super::xcb::XCBState;

#[cfg(x11)]
use super::x11rb::{RbError, X11RbState};

#[cfg(windows)]
use super::win32::Win32State;

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
    fn get_position_data(&self, window: Self::Window) -> WindowPositionData;
    /// ## Safety
    /// The same window should not be destroyed twice
    unsafe fn destroy_window(&mut self, window: Self::Window);
    unsafe fn wait_for_events(&mut self) -> Option<WWindCoreEvent>;
    fn draw_line(&mut self, window: Self::Window, x1: i16, y1: i16, x2: i16, y2: i16) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy)]
pub enum WWindCoreEvent {
    CloseWindow(CoreWindowRef),
    Expose{window: CoreWindowRef, x: u16, y: u16, width: u16, height: u16},
}

/// An enumeration over all of the [CoreStateImplementation]s.
pub enum CoreStateEnum {
    #[cfg(xcb)]
    XCB(XCBState),
    #[cfg(x11)]
    X11(X11RbState),
    #[cfg(windows)]
    Win32(Win32State),
}

/// Represents a reference to a window from any [CoreStateImplementation].
#[derive(Clone, Copy)]
pub union CoreWindowRef {
    #[cfg(xcb)]
    xcb: <XCBState as CoreStateImplementation>::Window,
    #[cfg(x11)]
    x11: <X11RbState as CoreStateImplementation>::Window,
    #[cfg(windows)]
    win32: <Win32State as CoreStateImplementation>::Window,
}

#[cfg(windows)]
impl From<<Win32State as CoreStateImplementation>::Window> for CoreWindowRef {
    fn from(win32: <Win32State as CoreStateImplementation>::Window) -> Self {
        CoreWindowRef {win32}
    }
}

#[cfg(xcb)]
impl From<<XCBState as CoreStateImplementation>::Window> for CoreWindowRef {
    fn from(xcb: <XCBState as CoreStateImplementation>::Window) -> Self {
        CoreWindowRef {xcb}
    }
}

#[cfg(x11)]
impl From<<X11RbState as CoreStateImplementation>::Window> for CoreWindowRef {
    fn from(x11: <X11RbState as CoreStateImplementation>::Window) -> Self {
        CoreWindowRef {x11}
    }
}

impl CoreWindowRef {
    #[cfg(xcb)]
    pub unsafe fn xcb(self) -> <XCBState as CoreStateImplementation>::Window {
        self.xcb
    }
    #[cfg(x11)]
    pub unsafe fn x11(self) -> <X11RbState as CoreStateImplementation>::Window {
        self.x11
    }
    #[cfg(windows)]
    pub unsafe fn win32(self) -> <Win32State as CoreStateImplementation>::Window {
        self.win32
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
                #[cfg(windows)]
                CoreStateType::Win32 => self.win32 == other.win32(),
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
                #[cfg(windows)]
                CoreStateType::Win32 => self.win32().hash(state),
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

impl From<Infallible> for CoreError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
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
        #[cfg(windows)] {
            CORE_STATE_TYPE.write(CoreStateType::Win32);

            let win32_state = unsafe {Win32State::new().unwrap()};

            let core_state = CoreStateEnum::Win32(win32_state);

            return Ok(core_state);
        }

        #[cfg(x11)]
        let err = {
            match X11RbState::new() {
                Ok(state) => {
                    CORE_STATE_TYPE.write(CoreStateType::X11);

                    let state = CoreStateEnum::X11(state);

                    return Ok(state)
                },
                Err(err) => err,
            }
        };

        #[cfg(xcb)]
        let err = {
            match XCBState::new() {
                Ok(state) => {
                    CORE_STATE_TYPE.write(CoreStateType::XCB);

                    let state = CoreStateEnum::XCB(state);

                    return Ok(state)
                },
                Err(err) => err,
            }
        };

        panic!("{err:?}");
    }

    fn add_window(&mut self, x: i16, y: i16, height: u16, width: u16, title: &str) -> Result<Self::Window, Self::Error> {
        unsafe {
            let window = match self {
                #[cfg(xcb)]
                CoreStateEnum::XCB(xcb_state) => xcb_state.add_window(x, y, height, width, title)?.into(),
                #[cfg(x11)]
                CoreStateEnum::X11(x11_state) => x11_state.add_window(x, y, height, width, title)?.into(),
                #[cfg(windows)]
                CoreStateEnum::Win32(win32_state) => win32_state.add_window(x, y, height, width, title)?.into(),
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
            #[cfg(windows)]
            CoreStateEnum::Win32(win32_state) => win32_state.set_window_title(Self::Window::win32(window), title)
        }}
    }

    fn draw_line(&mut self, window: Self::Window, x1: i16, y1: i16, x2: i16, y2: i16) -> Result<(), Self::Error> {
        unsafe {
            match self {
                #[cfg(xcb)]
                CoreStateEnum::XCB(s) => Ok(s.draw_line(CoreWindowRef::xcb(window), x1, y1, x2, y2)?),
                #[cfg(x11)]
                CoreStateEnum::X11(s) => Ok(s.draw_line(CoreWindowRef::x11(window), x1, y1, x2, y2)?),
                #[cfg(windows)]
                CoreStateEnum::Win32(s) => Ok(s.draw_line(CoreWindowRef::win32(window), x1, y1, x2, y2)?),
            }
        }
    }

    unsafe fn destroy_window(&mut self, window: Self::Window) {
        match self {
            #[cfg(xcb)]
            CoreStateEnum::XCB(s) => s.destroy_window(CoreWindowRef::xcb(window)),
            #[cfg(x11)]
            CoreStateEnum::X11(s) => s.destroy_window(CoreWindowRef::x11(window)),
            #[cfg(windows)]
            CoreStateEnum::Win32(s) => s.destroy_window(CoreWindowRef::win32(window)),
        }
    }

    unsafe fn wait_for_events(&mut self) -> Option<WWindCoreEvent> {
        match self {
            #[cfg(xcb)]
            CoreStateEnum::XCB(s) => s.wait_for_events(),
            #[cfg(x11)]
            CoreStateEnum::X11(s) => s.wait_for_events(),
            #[cfg(windows)]
            CoreStateEnum::Win32(s) => s.wait_for_events(),
        }
    }

    fn get_position_data(&self, window: Self::Window) -> WindowPositionData {
        unsafe {
            match self {
                #[cfg(xcb)]
                CoreStateEnum::XCB(s) => s.get_position_data(window.xcb()),
                #[cfg(x11)]
                CoreStateEnum::X11(s) => s.get_position_data(window.x11()),
                #[cfg(windows)]
                CoreStateEnum::Win32(s) => s.get_position_data(window.win32()),
            }
        }
    }
}