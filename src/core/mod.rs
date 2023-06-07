use crate::{state::CoreStateData, Color, RectRegion, WWindState, Window};

mod core_state_implementation;

pub(crate) use self::core_state_implementation::CoreWindowRef;
pub(crate) use core_state_implementation::CoreStateEnum;
pub use core_state_implementation::CoreStateImplementation;

#[cfg(windows)]
mod win32;
#[cfg(x11)]
mod x11rb;

pub use core_state_implementation::DrawingContextEnum;
pub use core_state_implementation::WWindCoreEvent;

#[derive(Clone)]
pub struct CoreDrawingContext {
    pub context: DrawingContextEnum,
    pub core_state_data: *mut CoreStateData,
}

impl CoreDrawingContext {
    fn get_core_state_data_mut(&mut self) -> &mut CoreStateData {
        unsafe { &mut *self.core_state_data }
    }

    pub fn set_draw_color(&mut self, color: Color) {
        let context = self.context;

        self.get_core_state_data_mut()
            .core_state
            .set_draw_color(context, color)
            .unwrap();
    }

    pub fn draw_line(&mut self, x1: u16, y1: u16, x2: u16, y2: u16) {
        let context = self.context;

        self.get_core_state_data_mut()
            .core_state
            .draw_line(context, x1, y1, x2, y2)
            .unwrap()
    }

    pub fn draw_rectangle(&mut self, rectangle: RectRegion) {
        let context = self.context;

        self.get_core_state_data_mut()
            .core_state
            .draw_rectangle(context, rectangle)
            .unwrap()
    }
}

/// Represents a window.
/// Note: this doesn't destroy the window upon drop
#[derive(Clone)]
pub struct CoreWindow {
    pub core_window_ref: CoreWindowRef,
    pub core_state_data: *mut CoreStateData,
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
        unsafe { &mut *self.core_state_data }
    }

    pub fn get_core_state_data(&self) -> &CoreStateData {
        unsafe { &*self.core_state_data }
    }

    pub fn on_window_close_attempt<
        F: for<'a> FnMut(&'a mut WWindState, &'a mut Window) + 'static,
    >(
        &mut self,
        closure: F,
    ) {
        let window_ref = self.core_window_ref;

        self.get_core_state_data_mut()
            .windows
            .get_mut(&window_ref)
            .map(|data| data.on_close = Some(Box::new(closure)));
    }

    pub fn on_redraw<F: for<'a> FnMut(&'a mut WWindState, &'a mut Window, RectRegion) + 'static>(
        &mut self,
        closure: F,
    ) {
        let window_ref = self.core_window_ref;

        self.get_core_state_data_mut()
            .windows
            .get_mut(&window_ref)
            .map(|data| data.redraw = Some(Box::new(closure)));
    }

    pub fn get_drawing_context(&mut self) -> CoreDrawingContext {
        let window_ref = self.core_window_ref;
        let core_state = &mut self.get_core_state_data_mut().core_state;

        unsafe {
            CoreDrawingContext {
                context: core_state.get_context(window_ref),
                core_state_data: self.core_state_data,
            }
        }
    }

    pub fn get_size(&self) -> (u16, u16) {
        self.get_core_state_data()
            .core_state
            .get_size(self.core_window_ref)
    }
}
