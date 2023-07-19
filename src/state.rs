use std::{
    collections::HashMap,
    mem::MaybeUninit,
    sync::atomic::{self, AtomicBool},
};

use crate::{
    core::{CoreStateEnum, CoreStateImplementation, CoreWindowRef, WWindCoreEvent},
    util::PhantomUnsend,
    window::WindowData,
    Window, SHOULD_EXIT,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoreStateType {
    #[cfg(x11)]
    X11,
    #[cfg(windows)]
    Win32,
}

pub(super) static STATE_CREATED: AtomicBool = AtomicBool::new(false);
pub(super) static mut CORE_STATE_TYPE: MaybeUninit<CoreStateType> = MaybeUninit::uninit();

#[repr(C)]
pub struct WWindState {
    data: *mut CoreStateData,
    _unsend: PhantomUnsend,
}

pub struct CoreStateData {
    pub(crate) core_state: CoreStateEnum,
    pub(crate) windows: HashMap<CoreWindowRef, WindowData>,
    pub(crate) windows_to_destroy: Vec<CoreWindowRef>,
}

impl WWindState {
    pub fn schedule_exit(&mut self) {
        unsafe { SHOULD_EXIT = true };
    }

    pub fn add_window<'a>(
        &'a mut self,
        x: i16,
        y: i16,
        height: u16,
        width: u16,
        title: &str,
    ) -> Window<'a> {
        let window_ref = self
            .get_core_data_mut()
            .core_state
            .add_window(x, y, height, width, title)
            .unwrap();

        self.get_core_data_mut()
            .windows
            .insert(window_ref, WindowData::new(width, height));

        self.get_window_from_ref(window_ref)
    }

    pub fn do_windows_exist(&self) -> bool {
        !self.get_core_data().windows.is_empty()
    }
}

impl WWindState {
    pub(crate) unsafe fn clone(&self) -> Self {
        Self {
            data: self.data,
            _unsend: Default::default(),
        }
    }

    pub(crate) fn new() -> Option<Self> {
        if !STATE_CREATED.fetch_or(true, atomic::Ordering::Acquire) {
            let core_state = unsafe { CoreStateEnum::new() }.unwrap();
            let windows = HashMap::new();
            let windows_to_destroy = Vec::new();

            let data = CoreStateData {
                core_state,
                windows,
                windows_to_destroy,
            };

            let state = Box::new(data);

            Some(Self {
                data: Box::into_raw(state),
                _unsend: PhantomUnsend::default(),
            })
        } else {
            None
        }
    }

    pub(crate) fn get_core_data_mut(&mut self) -> &mut CoreStateData {
        unsafe { &mut *self.data }
    }

    pub(crate) fn get_core_data(&self) -> &CoreStateData {
        unsafe { &*self.data }
    }

    /// Destroys the windows that were scheduled for deletion
    /// ## Safety
    /// This function is unsafe if a CoreWindow exists for a window that's scheduled for deletion
    pub(crate) unsafe fn destroy_pending_windows(&mut self) {
        while let Some(window_ref) = self.get_core_data_mut().windows_to_destroy.pop() {
            self.destroy_window(window_ref)
        }
    }

    /// Directly destroys the underlying Window
    /// ## Safety
    /// This function is unsafe if a CoreWindow for this window exists after this function is called
    unsafe fn destroy_window(&mut self, window: CoreWindowRef) {
        self.get_core_data_mut().core_state.destroy_window(window);

        self.get_core_data_mut().windows.remove(&window);
    }

    pub(crate) fn get_window_from_ref(&mut self, window_ref: CoreWindowRef) -> Window {
        Window::from_parts(window_ref, self.data)
    }

    pub(crate) unsafe fn destroy(self) {
        CORE_STATE_TYPE.assume_init_drop();
        drop(Box::from_raw(self.data));

        STATE_CREATED.store(false, atomic::Ordering::Release);
    }

    pub(crate) fn flush(&mut self) {
        self.get_core_data_mut().core_state.flush().unwrap()
    }

    pub(crate) unsafe fn wait_for_events(&mut self) {
        static mut STATE: MaybeUninit<WWindState> = MaybeUninit::uninit();
        STATE = MaybeUninit::new(self.clone());

        let core_state = &mut self.get_core_data_mut().core_state;

        core_state.wait_for_events(&mut (on_event as unsafe fn(WWindCoreEvent)));

        unsafe fn on_event(event: WWindCoreEvent) {
            let state = STATE.assume_init_mut();

            match event {
                WWindCoreEvent::CloseWindow(window_ref) => {
                    if let Some(window_data) =
                        state.get_core_data_mut().windows.get_mut(&window_ref)
                    {
                        let closure = window_data.on_close.take();
                        let mut closure = if let Some(closure) = closure {
                            closure
                        } else {
                            let mut window = state.get_window_from_ref(window_ref);
                            window.schedule_window_destruction();
                            return;
                        };

                        let mut state_clone = state.clone();
                        let mut window = state.get_window_from_ref(window_ref);

                        closure(&mut state_clone, &mut window);

                        state
                            .get_core_data_mut()
                            .windows
                            .get_mut(&window_ref)
                            .map(|data| data.on_close.insert(closure));
                    } else {
                        println!("CloseWindow called on non-existant window");
                    }
                }
                WWindCoreEvent::Expose(window_ref, region) => {
                    if let Some(window_data) =
                        state.get_core_data_mut().windows.get_mut(&window_ref)
                    {
                        let closure = window_data.redraw.take();
                        let mut closure = if let Some(closure) = closure {
                            closure
                        } else {
                            return;
                        };

                        let mut state_clone = state.clone();
                        let mut window = state.get_window_from_ref(window_ref);

                        closure(&mut state_clone, &mut window, region);

                        state
                            .get_core_data_mut()
                            .windows
                            .get_mut(&window_ref)
                            .map(|data| data.redraw.insert(closure));

                        state.flush();
                    } else {
                        println!("Exposed non-existant window");
                    }
                }
                WWindCoreEvent::Keydown(window_ref, keycode) => {
                    if let Some(window_data) =
                        state.get_core_data_mut().windows.get_mut(&window_ref)
                    {
                        let closure = window_data.keydown.take();
                        let mut closure = if let Some(closure) = closure {
                            closure
                        } else {
                            return;
                        };

                        let mut state_clone = state.clone();
                        let mut window = state.get_window_from_ref(window_ref);

                        closure(&mut state_clone, &mut window, keycode);

                        state
                            .get_core_data_mut()
                            .windows
                            .get_mut(&window_ref)
                            .map(|data| data.keydown.insert(closure));
                    } else {
                        println!("Keydown called on non-existant window");
                    }
                }
            }
        }
    }
}
