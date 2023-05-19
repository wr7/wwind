use super::{CoreWindowRef, CoreState};

pub trait CoreStateImplementation: Sized {
    /// The error that can occur when initializing the state
    type Error;
    /// The type internally used to represent a window
    type Window: Sized + Copy;

    /// ## Safety
    /// Should not be called while another CoreStateImplementation exists
    unsafe fn new() -> Result<Self, Self::Error>;
    fn add_window(&mut self, x: i16, y: i16, height: u16, width: u16, title: &str) -> CoreWindowRef;
    fn set_window_title(&mut self, window: CoreWindowRef, title: &str);
    /// ## Safety
    /// The same window should not be destroyed twice
    unsafe fn destroy_window(&mut self, window: CoreWindowRef);
    unsafe fn wait_for_events(state: &mut CoreState) -> bool;
}