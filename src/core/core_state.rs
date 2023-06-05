use std::{
    cell::UnsafeCell,
    collections::HashMap,
    mem::MaybeUninit,
    rc::Rc,
    sync::atomic::{self, AtomicBool},
};

use crate::{
    core::{core_state_implementation::WWindCoreEvent, CoreStateImplementation},
    util::ForgetGuard,
    WWindState, Window, WindowData,
};

use super::{
    core_state_implementation::{CoreStateEnum, CoreWindowRef},
    CoreDrawingContext, CoreWindow, DrawingContextEnum,
};

#[derive(Clone)]
pub struct CoreState {
    data: Rc<UnsafeCell<CoreStateData>>,
}

pub struct CoreStateData {
    pub(super) core_state: CoreStateEnum,
    pub(super) windows: HashMap<CoreWindowRef, WindowData>,
    pub(super) windows_to_destroy: Vec<CoreWindowRef>,
}

impl CoreState {
    fn get_data_mut(&mut self) -> &mut CoreStateData {
        unsafe { self.data.get().as_mut().unwrap_unchecked() }
    }

    unsafe fn get_data(&self) -> &mut CoreStateData {
        self.data.get().as_mut().unwrap_unchecked()
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
    fn get_calling_details(
        &mut self,
        window_ref: CoreWindowRef,
    ) -> (ForgetGuard<'_, WWindState>, Window) {
        let core_window = self.get_window_from_ref(window_ref);

        let window = Window::from_core_window(core_window);
        let wwind_state = WWindState::from_core_state(self);

        (wwind_state, window)
    }

    pub fn get_window_from_ref(&self, core_window_ref: CoreWindowRef) -> CoreWindow {
        let core_state_data = self.data.clone();

        CoreWindow {
            core_window_ref,
            core_state_data,
        }
    }

    pub fn get_core_context(&self, context: DrawingContextEnum) -> CoreDrawingContext {
        CoreDrawingContext {
            context,
            core_state_data: self.data.clone(),
        }
    }

    pub unsafe fn wait_for_events(&mut self) {
        static mut CORE_STATE: MaybeUninit<CoreState> = MaybeUninit::uninit();
        CORE_STATE = MaybeUninit::new(self.clone());

        let core_state = &mut self.get_data_mut().core_state;

        core_state.wait_for_events(&mut (on_event as unsafe fn(WWindCoreEvent)));

        unsafe fn on_event(event: WWindCoreEvent) {
            let core_state = CORE_STATE.assume_init_mut();

            match event {
                WWindCoreEvent::CloseWindow(window_ref) => {
                    if let Some(window_data) =
                        core_state.get_data_mut().windows.get_mut(&window_ref)
                    {
                        let closure = window_data.on_close.take();
                        let mut closure = if let Some(closure) = closure {
                            closure
                        } else {
                            let mut window = core_state.get_window_from_ref(window_ref);
                            window.schedule_window_destruction();
                            return;
                        };

                        let (mut wwind_state, mut window) =
                            core_state.get_calling_details(window_ref);

                        closure(&mut wwind_state, &mut window);

                        core_state
                            .get_data_mut()
                            .windows
                            .get_mut(&window_ref)
                            .map(|data| data.on_close.insert(closure));
                    } else {
                        println!("Exposed non-existant window");
                    }
                }
                WWindCoreEvent::Expose(window_ref, region) => {
                    if let Some(window_data) =
                        core_state.get_data_mut().windows.get_mut(&window_ref)
                    {
                        let closure = window_data.redraw.take();
                        let mut closure = if let Some(closure) = closure {
                            closure
                        } else {
                            return;
                        };

                        let (mut wwind_state, mut window) =
                            core_state.get_calling_details(window_ref);

                        closure(&mut wwind_state, &mut window, region);

                        core_state
                            .get_data_mut()
                            .windows
                            .get_mut(&window_ref)
                            .map(|data| data.redraw.insert(closure));
                    } else {
                        println!("Exposed non-existant window");
                    }
                }
            }
        }
    }
    pub unsafe fn add_window(
        &mut self,
        x: i16,
        y: i16,
        height: u16,
        width: u16,
        title: &str,
    ) -> CoreWindow {
        let core_window_ref = self
            .get_data_mut()
            .core_state
            .add_window(x, y, height, width, title)
            .unwrap();

        let windows = &mut self.get_data_mut().windows;
        windows.insert(core_window_ref, WindowData::new(width, height));

        let core_state = self.data.clone();

        CoreWindow {
            core_state_data: core_state,
            core_window_ref,
        }
    }

    pub fn do_windows_exist(&self) -> bool {
        !unsafe { self.get_data() }.windows.is_empty()
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
    #[cfg(windows)]
    Win32,
}

pub(super) static STATE_CREATED: AtomicBool = AtomicBool::new(false);
pub(super) static mut CORE_STATE_TYPE: MaybeUninit<CoreStateType> = MaybeUninit::uninit();

impl CoreState {
    pub fn new() -> Option<Self> {
        if !STATE_CREATED.fetch_or(true, atomic::Ordering::Acquire) {
            let core_state = unsafe { CoreStateEnum::new() }.unwrap();
            let windows = HashMap::new();
            let windows_to_destroy = Vec::new();

            let data = CoreStateData {
                core_state,
                windows,
                windows_to_destroy,
            };

            Some(CoreState {
                data: Rc::new(UnsafeCell::new(data)),
            })
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
