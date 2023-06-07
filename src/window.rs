use std::marker::PhantomData;

use crate::{
    core::{CoreDrawingContext, CoreStateImplementation, CoreWindowRef},
    state::CoreStateData,
    util::PhantomUnsend,
    DrawingContext, RectRegion, WWindState,
};

pub struct WindowData {
    pub on_close: Option<Box<dyn for<'a> FnMut(&'a mut WWindState, &'a mut Window<'a>)>>,
    pub redraw: Option<Box<dyn for<'a> FnMut(&'a mut WWindState, &'a mut Window<'a>, RectRegion)>>,
}

impl WindowData {
    pub fn new(_width: u16, _height: u16) -> Self {
        Self {
            on_close: None,
            redraw: None,
        }
    }
}

pub struct Window<'a> {
    pub(crate) window_ref: CoreWindowRef,
    pub(crate) data: *mut CoreStateData,
    pub(crate) _unsend: PhantomUnsend,
    pub(crate) _phantom_data: PhantomData<&'a ()>,
}

impl Window<'_> {
    pub(crate) fn get_core_data_mut(&mut self) -> &mut CoreStateData {
        unsafe { &mut *self.data }
    }
    pub(crate) fn get_core_data(&self) -> &CoreStateData {
        unsafe { &*self.data }
    }
}

impl Window<'_> {
    pub fn schedule_window_destruction(&mut self) {
        let window_to_schedule = self.window_ref;
        let windows_to_destroy = &mut self.get_core_data_mut().windows_to_destroy;

        if !windows_to_destroy.contains(&window_to_schedule) {
            unsafe { windows_to_destroy.push(window_to_schedule) };
        }
    }

    pub fn on_window_close<F: for<'a> FnMut(&'a mut WWindState, &'a mut Window) + 'static>(
        &mut self,
        closure: F,
    ) {
        let window_ref = self.window_ref;

        self.get_core_data_mut()
            .windows
            .get_mut(&window_ref)
            .map(|data| data.on_close = Some(Box::new(closure)));
    }
    pub fn on_redraw<F: for<'a> FnMut(&'a mut WWindState, &'a mut Window, RectRegion) + 'static>(
        &mut self,
        closure: F,
    ) {
        let window_ref = self.window_ref;

        self.get_core_data_mut()
            .windows
            .get_mut(&window_ref)
            .map(|data| data.redraw = Some(Box::new(closure)));
    }

    pub fn get_drawing_context(&mut self) -> DrawingContext<'_> {
        let window_ref = self.window_ref;

        let context = unsafe { self.get_core_data_mut().core_state.get_context(window_ref) };
        DrawingContext {
            context: CoreDrawingContext {
                context,
                core_state_data: self.data,
            },
            _unsend: Default::default(),
            _phantom_data: PhantomData,
        }
    }

    pub fn get_size(&self) -> (u16, u16) {
        let window_ref = self.window_ref;

        self.get_core_data().core_state.get_size(window_ref)
    }
}
