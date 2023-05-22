use std::convert::Infallible;
use std::ffi::CString;
use std::mem::MaybeUninit;
use std::ptr::addr_of;
use std::{ptr, mem, iter};

use crate::core::core_state_implementation::CoreWindowRef;

use super::CoreStateImplementation;
use winapi::shared::windef::HWND;
use winapi::shared::minwindef::HMODULE;
use winapi::um::libloaderapi::GetModuleHandleA;
use winapi::um::winuser::{WNDCLASSA, RegisterClassA, CreateWindowExA, WS_OVERLAPPEDWINDOW, MSG, GetMessageA, TranslateMessage, WM_CLOSE, DefWindowProcA, DestroyWindow, SetWindowTextA};

pub struct Win32State {
    hinst: HMODULE,
}

impl CoreStateImplementation for Win32State {
    type Error = Infallible;

    type Window = HWND;

    unsafe fn new() -> Result<Self, Self::Error> {
        let hinst = GetModuleHandleA(ptr::null());
        Ok(Win32State {hinst})
    }

    fn add_window(&mut self, x: i16, y: i16, height: u16, width: u16, title: &str) -> Result<Self::Window, Self::Error> {
        const CLASS_NAME: *const i8 = b"WWIND Window\0".as_ptr() as *const i8;

        static mut WINDOW_CLASS_REGISTERED: bool = false;
        unsafe {
            if !WINDOW_CLASS_REGISTERED {
                let mut class: WNDCLASSA = mem::zeroed();
                class.lpfnWndProc = None;
                class.hInstance = self.hinst;
                class.lpszClassName = CLASS_NAME;

                RegisterClassA(addr_of!(class));

                WINDOW_CLASS_REGISTERED = true;
            }
        }

        let window_name = title.as_ptr() as *const i8;

        let window = unsafe {CreateWindowExA(
            0, 
            CLASS_NAME, 
            window_name, 
            WS_OVERLAPPEDWINDOW, 
            x as i32, y as i32, 
            width as i32, height as i32, 
            ptr::null_mut(), 
            ptr::null_mut(), 
            self.hinst, 
            ptr::null_mut()
        )};

        Ok(window)
    }

    fn set_window_title(&mut self, window: Self::Window, title: &str) {
        // C-String moment
        let title_vec: Vec<i8> = title.as_bytes().iter().copied().filter(|&b| b != 0).chain(iter::once(0)).map(|n| n as i8).collect();

        unsafe {SetWindowTextA(window, title_vec.as_ptr())};
    }

    unsafe fn destroy_window(&mut self, window: Self::Window) {
        DestroyWindow(window);
    }

    unsafe fn wait_for_events(state: &mut super::CoreState) -> bool {
        // TODO: make GetMessageA close the window when it returns false
        let mut msg = MaybeUninit::<MSG>::uninit();

        if GetMessageA(msg.as_mut_ptr(), ptr::null_mut(), 0, 0) == 0 {
            return false;
        }
        TranslateMessage(msg.as_ptr());

        let msg = msg.assume_init();

        match msg.message {
            WM_CLOSE => {
                let window_ref = CoreWindowRef::from_win32(msg.hwnd);
                super::on_window_close(state, window_ref);

                return true;
            },
            _ => (),
        }

        DefWindowProcA(msg.hwnd, msg.message, msg.wParam, msg.lParam);
        true
    }
}