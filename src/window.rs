use std::{ffi::c_void, marker::PhantomData, mem};

use crate::{
    core::{CoreStateImplementation, CoreWindowRef},
    state::CoreStateData,
    util::PhantomUnsend,
    DrawingContext, RectRegion, WWindInitState, WWindState,
};

pub type OnClose<UserData> = dyn FnMut(&mut WWindState<UserData>, &mut Window<UserData>) + 'static;
pub type OnRedraw<UserData> =
    dyn FnMut(&mut WWindState<UserData>, &mut Window<UserData>, RectRegion) + 'static;
pub type OnKeydown<UserData> =
    dyn FnMut(&mut WWindState<UserData>, &mut Window<UserData>, u32) + 'static;

pub struct WindowData {
    pub on_close: Option<[usize; 2]>,
    pub redraw: Option<[usize; 2]>,
    pub keydown: Option<[usize; 2]>,
}

impl WindowData {
    pub fn new(_width: u16, _height: u16) -> Self {
        Self {
            on_close: None,
            redraw: None,
            keydown: None,
        }
    }
}

#[repr(C)]
pub struct Window<'a, UserData = ()> {
    window_ref: CoreWindowRef,
    data: *mut CoreStateData,
    _unsend: PhantomUnsend,
    _phantom_data: PhantomData<(&'a (), UserData)>,
}

impl<'a, UserData> Window<'a, UserData> {
    pub(crate) unsafe fn with_data<V>(self) -> Window<'a, V> {
        mem::transmute(self)
    }

    pub fn schedule_window_destruction(&mut self) {
        let window_to_schedule = self.window_ref;
        let windows_to_destroy = &mut self.get_core_data_mut().windows_to_destroy;

        if !windows_to_destroy.contains(&window_to_schedule) {
            windows_to_destroy.push(window_to_schedule);
        }
    }

    pub fn on_window_close<F: FnMut(&mut WWindState<UserData>, &mut Window<UserData>) + 'static>(
        &mut self,
        closure: F,
    ) {
        let window_ref = self.window_ref;

        if let Some(window_data) = self.get_core_data_mut().windows.get_mut(&window_ref) {
            if let Some(old_binding) = window_data.on_close {
                drop(unsafe { mem::transmute::<[usize; 2], Box<OnClose<UserData>>>(old_binding) })
            }

            window_data.on_close =
                Some(unsafe { mem::transmute(Box::new(closure) as Box<OnClose<UserData>>) });
        }
    }

    pub fn on_redraw<
        F: FnMut(&mut WWindState<UserData>, &mut Window<UserData>, RectRegion) + 'static,
    >(
        &mut self,
        closure: F,
    ) {
        let window_ref = self.window_ref;

        if let Some(window_data) = self.get_core_data_mut().windows.get_mut(&window_ref) {
            if let Some(old_binding) = window_data.redraw {
                drop(unsafe { mem::transmute::<[usize; 2], Box<OnRedraw<UserData>>>(old_binding) })
            }

            window_data.redraw =
                Some(unsafe { mem::transmute(Box::new(closure) as Box<OnRedraw<UserData>>) });
        }
    }

    pub fn on_keydown<F: FnMut(&mut WWindState<UserData>, &mut Window<UserData>, u32) + 'static>(
        &mut self,
        closure: F,
    ) {
        let window_ref = self.window_ref;

        if let Some(window_data) = self.get_core_data_mut().windows.get_mut(&window_ref) {
            if let Some(old_binding) = window_data.keydown {
                drop(unsafe { mem::transmute::<[usize; 2], Box<OnKeydown<UserData>>>(old_binding) })
            }

            window_data.keydown =
                Some(unsafe { mem::transmute(Box::new(closure) as Box<OnKeydown<UserData>>) });
        }
    }
    pub fn get_drawing_context(&mut self) -> DrawingContext<'_> {
        let window_ref = self.window_ref;

        let context = unsafe { self.get_core_data_mut().core_state.get_context(window_ref) };

        DrawingContext::from_parts(context, self.data)
    }

    pub fn get_size(&self) -> (u16, u16) {
        let window_ref = self.window_ref;

        self.get_core_data().core_state.get_size(window_ref)
    }
}

impl<UserData> Window<'_, UserData> {
    pub(crate) fn from_parts(window_ref: CoreWindowRef, data: *mut CoreStateData) -> Self {
        Self {
            window_ref,
            data,
            _unsend: Default::default(),
            _phantom_data: PhantomData,
        }
    }
    pub(crate) fn get_core_data_mut(&mut self) -> &mut CoreStateData {
        unsafe { &mut *self.data }
    }
    pub(crate) fn get_core_data(&self) -> &CoreStateData {
        unsafe { &*self.data }
    }
}
