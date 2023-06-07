use crate::{
    state::{CoreStateType, CORE_STATE_TYPE, STATE_CREATED},
    Color, RectRegion,
};
use std::{convert::Infallible, hash::Hash, sync::atomic};

#[cfg(x11)]
use super::x11rb::X11RbState;

#[cfg(windows)]
use super::win32::Win32State;

pub trait CoreStateImplementation: Sized {
    /// The error that can occur when initializing the state
    type Error;
    /// The type internally used to represent a window
    type Window: Sized + Copy;
    /// Something that you can draw on
    type DrawingContext: Sized + Copy;

    /// ## Safety
    /// Should not be called while another CoreStateImplementation exists
    unsafe fn new() -> Result<Self, Self::Error>;
    fn add_window(
        &mut self,
        x: i16,
        y: i16,
        height: u16,
        width: u16,
        title: &str,
    ) -> Result<Self::Window, Self::Error>;
    fn set_window_title(&mut self, window: Self::Window, title: &str);
    fn flush(&mut self) -> Result<(), Self::Error>;
    /// ## Safety
    /// The same window should not be destroyed twice
    unsafe fn destroy_window(&mut self, window: Self::Window);
    unsafe fn wait_for_events(&mut self, on_event: &mut unsafe fn(WWindCoreEvent));

    fn get_size(&self, window: Self::Window) -> (u16, u16);

    // Drawing
    unsafe fn get_context(&mut self, window: Self::Window) -> Self::DrawingContext;
    fn draw_line(
        &mut self,
        drawing_context: Self::DrawingContext,
        x1: u16,
        y1: u16,
        x2: u16,
        y2: u16,
    ) -> Result<(), Self::Error>;

    fn draw_rectangle(
        &mut self,
        drawing_context: Self::DrawingContext,
        rectangle: RectRegion,
    ) -> Result<(), Self::Error>;

    fn set_draw_color(
        &mut self,
        context: Self::DrawingContext,
        color: Color,
    ) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy)]
pub enum WWindCoreEvent {
    CloseWindow(CoreWindowRef),
    Expose(CoreWindowRef, RectRegion),
}

/// An enumeration over all of the [CoreStateImplementation]s.
pub(crate) enum CoreStateEnum {
    #[cfg(x11)]
    X11(X11RbState),
    #[cfg(windows)]
    Win32(Win32State),
}

/// Represents a reference to a window from any [CoreStateImplementation].
#[derive(Clone, Copy)]
pub union CoreWindowRef {
    #[cfg(x11)]
    x11: <X11RbState as CoreStateImplementation>::Window,
    #[cfg(windows)]
    win32: <Win32State as CoreStateImplementation>::Window,
}

#[cfg(windows)]
impl From<<Win32State as CoreStateImplementation>::Window> for CoreWindowRef {
    fn from(win32: <Win32State as CoreStateImplementation>::Window) -> Self {
        CoreWindowRef { win32 }
    }
}

#[cfg(x11)]
impl From<<X11RbState as CoreStateImplementation>::Window> for CoreWindowRef {
    fn from(x11: <X11RbState as CoreStateImplementation>::Window) -> Self {
        CoreWindowRef { x11 }
    }
}

impl CoreWindowRef {
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
                #[cfg(x11)]
                CoreStateType::X11 => self.x11() == other.x11(),
                #[cfg(windows)]
                CoreStateType::Win32 => self.win32 == other.win32(),
            }
        }
    }
}

#[derive(Clone, Copy)]
pub enum DrawingContextEnum {
    #[cfg(x11)]
    X11(<X11RbState as CoreStateImplementation>::DrawingContext),
    #[cfg(windows)]
    Win32(<Win32State as CoreStateImplementation>::DrawingContext),
}

impl DrawingContextEnum {
    #[cfg(x11)]
    unsafe fn x11(self) -> <X11RbState as CoreStateImplementation>::DrawingContext {
        if let Self::X11(context) = self {
            context
        } else {
            panic!()
        }
    }
    #[cfg(windows)]
    unsafe fn win32(self) -> <Win32State as CoreStateImplementation>::DrawingContext {
        if let Self::Win32(context) = self {
            context
        } else {
            panic!()
        }
    }
}

#[cfg(windows)]
impl From<<Win32State as CoreStateImplementation>::DrawingContext> for DrawingContextEnum {
    fn from(value: <Win32State as CoreStateImplementation>::DrawingContext) -> Self {
        Self::Win32(value)
    }
}

#[cfg(x11)]
impl From<<X11RbState as CoreStateImplementation>::DrawingContext> for DrawingContextEnum {
    fn from(value: <X11RbState as CoreStateImplementation>::DrawingContext) -> Self {
        Self::X11(value)
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

// Boilerplate that I intend to eventually automate with a macro

impl CoreStateImplementation for CoreStateEnum {
    type Error = CoreError;

    type Window = CoreWindowRef;

    type DrawingContext = DrawingContextEnum;

    unsafe fn new() -> Result<Self, Self::Error> {
        #[cfg(windows)]
        {
            CORE_STATE_TYPE.write(CoreStateType::Win32);

            let win32_state = unsafe { Win32State::new().unwrap() };

            let core_state = CoreStateEnum::Win32(win32_state);

            Ok(core_state)
        }

        #[cfg(x11)]
        {
            let err = {
                match X11RbState::new() {
                    Ok(state) => {
                        CORE_STATE_TYPE.write(CoreStateType::X11);

                        let state = CoreStateEnum::X11(state);

                        return Ok(state);
                    }
                    Err(err) => err,
                }
            };

            Err(err.into())
        }
    }

    fn add_window(
        &mut self,
        x: i16,
        y: i16,
        height: u16,
        width: u16,
        title: &str,
    ) -> Result<Self::Window, Self::Error> {
        let window = match self {
            #[cfg(x11)]
            CoreStateEnum::X11(x11_state) => {
                x11_state.add_window(x, y, height, width, title)?.into()
            }
            #[cfg(windows)]
            CoreStateEnum::Win32(win32_state) => {
                win32_state.add_window(x, y, height, width, title)?.into()
            }
        };
        Ok(window)
    }

    fn set_window_title(&mut self, window: Self::Window, title: &str) {
        unsafe {
            match self {
                #[cfg(x11)]
                CoreStateEnum::X11(x11_state) => {
                    x11_state.set_window_title(Self::Window::x11(window), title)
                }
                #[cfg(windows)]
                CoreStateEnum::Win32(win32_state) => {
                    win32_state.set_window_title(Self::Window::win32(window), title)
                }
            }
        }
    }

    fn draw_line(
        &mut self,
        drawing_context: Self::DrawingContext,
        x1: u16,
        y1: u16,
        x2: u16,
        y2: u16,
    ) -> Result<(), Self::Error> {
        unsafe {
            match self {
                #[cfg(x11)]
                CoreStateEnum::X11(s) => Ok(s.draw_line(drawing_context.x11(), x1, y1, x2, y2)?),
                #[cfg(windows)]
                CoreStateEnum::Win32(s) => {
                    Ok(s.draw_line(drawing_context.win32(), x1, y1, x2, y2)?)
                }
            }
        }
    }

    fn draw_rectangle(
        &mut self,
        drawing_context: Self::DrawingContext,
        rectangle: RectRegion,
    ) -> Result<(), Self::Error> {
        unsafe {
            match self {
                #[cfg(x11)]
                CoreStateEnum::X11(s) => Ok(s.draw_rectangle(drawing_context.x11(), rectangle)?),
                #[cfg(windows)]
                CoreStateEnum::Win32(s) => {
                    Ok(s.draw_rectangle(drawing_context.win32(), rectangle)?)
                }
            }
        }
    }

    unsafe fn destroy_window(&mut self, window: Self::Window) {
        match self {
            #[cfg(x11)]
            CoreStateEnum::X11(s) => s.destroy_window(CoreWindowRef::x11(window)),
            #[cfg(windows)]
            CoreStateEnum::Win32(s) => s.destroy_window(CoreWindowRef::win32(window)),
        }
    }

    unsafe fn wait_for_events(&mut self, on_event: &mut unsafe fn(WWindCoreEvent)) {
        match self {
            #[cfg(x11)]
            CoreStateEnum::X11(s) => s.wait_for_events(on_event),
            #[cfg(windows)]
            CoreStateEnum::Win32(s) => s.wait_for_events(on_event),
        }
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        match self {
            #[cfg(x11)]
            CoreStateEnum::X11(s) => s.flush()?,
            #[cfg(windows)]
            CoreStateEnum::Win32(s) => s.flush()?,
        }
        Ok(())
    }

    fn set_draw_color(
        &mut self,
        drawing_context: Self::DrawingContext,
        color: Color,
    ) -> Result<(), Self::Error> {
        unsafe {
            match self {
                #[cfg(windows)]
                CoreStateEnum::Win32(s) => s.set_draw_color(drawing_context.win32(), color)?,
                #[cfg(x11)]
                CoreStateEnum::X11(s) => s.set_draw_color(drawing_context.x11(), color)?,
            }
        }
        Ok(())
    }

    unsafe fn get_context(&mut self, window: Self::Window) -> Self::DrawingContext {
        match self {
            #[cfg(windows)]
            CoreStateEnum::Win32(s) => s.get_context(window.win32()).into(),
            #[cfg(x11)]
            CoreStateEnum::X11(s) => s.get_context(window.x11()).into(),
        }
    }

    fn get_size(&self, window: Self::Window) -> (u16, u16) {
        unsafe {
            match self {
                #[cfg(windows)]
                CoreStateEnum::Win32(s) => s.get_size(window.win32()),
                #[cfg(x11)]
                CoreStateEnum::X11(s) => s.get_size(window.x11()),
            }
        }
    }
}
