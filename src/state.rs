use std::{
    collections::HashMap,
    ffi::c_void,
    marker::PhantomData,
    mem::{self, MaybeUninit},
    ops::{Deref, DerefMut},
    ptr,
    sync::atomic::{self, AtomicBool},
};

use crate::{
    core::{CoreStateEnum, CoreStateImplementation, CoreWindowRef, WWindCoreEvent},
    util::PhantomUnsend,
    window::{self, OnClose, OnKeydown, OnRedraw, WindowData},
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

pub(super) static mut USERDATA: *mut c_void = ptr::null_mut();

#[repr(transparent)]
pub struct WWindState<UserData = ()>(#[doc(hidden)] WWindInitState<UserData>);

impl<UserData> WWindState<UserData> {
    pub fn userdata(&self) -> &UserData {
        unsafe { &*(USERDATA as *const UserData) }
    }
    pub fn userdata_mut(&mut self) -> &mut UserData {
        unsafe { &mut *(USERDATA as *mut UserData) }
    }
}

impl<UserData> WWindState<UserData> {
    pub(crate) unsafe fn from_init(state: WWindInitState<UserData>) -> Self {
        Self(state)
    }
}

impl<UserData> Deref for WWindState<UserData> {
    type Target = WWindInitState<UserData>;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<UserData> DerefMut for WWindState<UserData> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

impl<UserData> AsRef<WWindInitState<UserData>> for WWindState<UserData> {
    fn as_ref(&self) -> &WWindInitState<UserData> {
        &self.0
    }
}

impl<UserData> AsMut<WWindInitState<UserData>> for WWindState<UserData> {
    fn as_mut(&mut self) -> &mut WWindInitState<UserData> {
        &mut self.0
    }
}

impl<UserData> From<WWindState<UserData>> for WWindInitState<UserData> {
    fn from(value: WWindState<UserData>) -> Self {
        value.0
    }
}

#[repr(C)]
pub struct WWindInitState<UserData = ()> {
    data: *mut CoreStateData,
    _phantomdata: PhantomData<UserData>,
    _unsend: PhantomUnsend,
}

pub struct CoreStateData {
    pub(crate) core_state: CoreStateEnum,
    pub(crate) windows: HashMap<CoreWindowRef, WindowData>,
    pub(crate) windows_to_destroy: Vec<CoreWindowRef>,
}

impl<UserData> WWindInitState<UserData> {
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
    ) -> Window<'a, UserData> {
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

impl<U> WWindInitState<U> {
    pub(crate) unsafe fn with_data<V>(self) -> WWindInitState<V> {
        mem::transmute(self)
    }

    pub(crate) unsafe fn clone(&self) -> Self {
        Self {
            data: self.data,
            _unsend: Default::default(),
            _phantomdata: PhantomData,
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
                _phantomdata: PhantomData,
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

    pub(crate) fn get_window_from_ref(&mut self, window_ref: CoreWindowRef) -> Window<U> {
        Window::from_parts(window_ref, self.data)
    }

    pub(crate) unsafe fn destroy(self) {
        CORE_STATE_TYPE.assume_init_drop();
        drop(Box::from_raw(self.data));

        if !USERDATA.is_null() {
            let mut userdata: Box<MaybeUninit<U>> = Box::from_raw(USERDATA as *mut _);
            userdata.assume_init_drop();

            USERDATA = ptr::null_mut();
        }

        STATE_CREATED.store(false, atomic::Ordering::Release);
    }

    pub(crate) fn flush(&mut self) {
        self.get_core_data_mut().core_state.flush().unwrap()
    }

    pub(crate) unsafe fn wait_for_events(&mut self) {
        static mut STATE: MaybeUninit<WWindInitState> = MaybeUninit::uninit();

        STATE = MaybeUninit::new(std::mem::transmute::<WWindInitState<U>, WWindInitState>(
            self.clone(),
        ));

        let core_state = &mut self.get_core_data_mut().core_state;

        core_state.wait_for_events(&mut (on_event::<U> as unsafe fn(WWindCoreEvent)));

        unsafe fn on_event<U>(event: WWindCoreEvent) {
            let mut state = STATE.assume_init_mut().clone().with_data();

            match event {
                WWindCoreEvent::CloseWindow(window_ref) => {
                    if let Some(window_data) =
                        state.get_core_data_mut().windows.get_mut(&window_ref)
                    {
                        let mut closure = if let Some(closure) = window_data.on_close.take() {
                            mem::transmute::<[usize; 2], Box<OnClose<U>>>(closure)
                        } else {
                            let mut window = state.get_window_from_ref(window_ref);
                            window.schedule_window_destruction();
                            return;
                        };

                        let mut state_clone = WWindState::from_init(state.clone());
                        let mut window = state.get_window_from_ref(window_ref);

                        closure(&mut state_clone, &mut window);

                        state
                            .get_core_data_mut()
                            .windows
                            .get_mut(&window_ref)
                            .map(|data| data.on_close.insert(mem::transmute(closure)));
                    } else {
                        println!("CloseWindow called on non-existant window");
                    }
                }
                WWindCoreEvent::Expose(window_ref, region) => {
                    if let Some(window_data) =
                        state.get_core_data_mut().windows.get_mut(&window_ref)
                    {
                        let mut closure = if let Some(closure) = window_data.redraw.take() {
                            mem::transmute::<[usize; 2], Box<OnRedraw<U>>>(closure)
                        } else {
                            return;
                        };

                        let mut state_clone = WWindState::from_init(state.clone());
                        let mut window = state.get_window_from_ref(window_ref);

                        closure(&mut state_clone, &mut window, region);

                        state
                            .get_core_data_mut()
                            .windows
                            .get_mut(&window_ref)
                            .map(|data| data.redraw.insert(mem::transmute(closure)));
                    } else {
                        println!("CloseWindow called on non-existant window");
                    }
                }
                WWindCoreEvent::Keydown(window_ref, keycode) => {
                    if let Some(window_data) =
                        state.get_core_data_mut().windows.get_mut(&window_ref)
                    {
                        let mut closure = if let Some(closure) = window_data.keydown.take() {
                            mem::transmute::<[usize; 2], Box<OnKeydown<U>>>(closure)
                        } else {
                            return;
                        };

                        let mut state_clone = WWindState::from_init(state.clone());
                        let mut window = state.get_window_from_ref(window_ref);

                        closure(&mut state_clone, &mut window, keycode);

                        state
                            .get_core_data_mut()
                            .windows
                            .get_mut(&window_ref)
                            .map(|data| data.keydown.insert(mem::transmute(closure)));
                    } else {
                        println!("CloseWindow called on non-existant window");
                    }
                }
            }

            state.flush();
        }
    }
}
