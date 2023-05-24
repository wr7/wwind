use std::convert::Infallible;
use std::ffi::CString;
use std::mem::MaybeUninit;
use std::ptr::addr_of;
use std::sync::RwLock;
use std::{ptr, mem, iter};

use winapi::um::errhandlingapi::GetLastError;

use crate::core::core_state_implementation::CoreWindowRef;

use super::CoreStateImplementation;
use super::core_state_implementation::WWindCoreEvent;
use winapi::shared::windef::HWND;
use winapi::shared::minwindef::{HMODULE, WPARAM, LPARAM, LRESULT, UINT};
use winapi::um::libloaderapi::GetModuleHandleA;
use winapi::um::winuser::{WNDCLASSA, RegisterClassA, CreateWindowExA, WS_OVERLAPPEDWINDOW, MSG, GetMessageA, TranslateMessage, WM_CLOSE, DefWindowProcA, DestroyWindow, SetWindowTextA, ShowWindow, SW_NORMAL};

pub struct Win32State {
    hinst: HMODULE,
}

#[derive(Debug)]
enum SendMessage {
    CloseWindow(HWND),
}

unsafe impl Sync for SendMessage {}
unsafe impl Send for SendMessage {}

static SEND_MESSAGE_QUEUE: RwLock<Vec<SendMessage>> = RwLock::new(Vec::new());

/// Used for "SendMessage" messages. Ugh
unsafe extern "system" fn window_proc(window: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CLOSE => {
            dbg!(SEND_MESSAGE_QUEUE.write()).unwrap().push(SendMessage::CloseWindow(window));
            0
        },
        _ => DefWindowProcA(window, msg, wparam, lparam),
    }
} 

impl CoreStateImplementation for Win32State {
    type Error = Infallible;

    type Window = HWND;

    unsafe fn new() -> Result<Self, Self::Error> {
        let hinst = GetModuleHandleA(ptr::null());
        Ok(Win32State {hinst})
    }

    fn add_window(&mut self, x: i16, y: i16, height: u16, width: u16, title: &str) -> Result<Self::Window, Self::Error> {
        const CLASS_NAME: &[u8] = b"WWIND Window\0";

        static mut WINDOW_CLASS_REGISTERED: bool = false;
        unsafe {
            if !WINDOW_CLASS_REGISTERED {
                let mut class: WNDCLASSA = mem::zeroed();
                class.lpfnWndProc = Some(window_proc);
                class.hInstance = self.hinst;
                class.lpszClassName = CLASS_NAME.as_ptr() as *const i8;

                RegisterClassA(addr_of!(class));

                WINDOW_CLASS_REGISTERED = true;
            }
        }

        println!("b");

        // C String moment
        let title: Vec<i8> = title.as_bytes().iter().copied().filter(|&b| b != 0).chain(iter::once(0)).map(|n| n as i8).collect();

        let window = unsafe {CreateWindowExA(
            0, 
            dbg!(CLASS_NAME.as_ptr() as *const i8), 
            dbg!(title.as_ptr()), 
            WS_OVERLAPPEDWINDOW, 
            x as i32, y as i32, 
            width as i32, height as i32, 
            ptr::null_mut(), 
            ptr::null_mut(), 
            self.hinst, 
            ptr::null_mut()
        )};

        println!("{}", unsafe {GetLastError()});

        println!("c");
        dbg!(window);

        unsafe {ShowWindow(dbg!(window), SW_NORMAL)};

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

    unsafe fn wait_for_events(&mut self) -> Option<WWindCoreEvent> {
        // TODO: make GetMessageA close the window when it returns false
        let mut queue = SEND_MESSAGE_QUEUE.write().unwrap();
        if let Some(msg) = queue.pop() {
            let SendMessage::CloseWindow(window) = msg;

            return Some(WWindCoreEvent::CloseWindow(window.into()));

            // let window_ref = CoreWindowRef::from_win32(window);

            // super::on_window_close(state, window_ref);
            // return true;
        }

        drop(queue);

        let mut msg = MaybeUninit::<MSG>::uninit();

        if GetMessageA(msg.as_mut_ptr(), ptr::null_mut(), 0, 0) == 0 {
            return None;
        }
        TranslateMessage(msg.as_ptr());

        let msg = msg.assume_init();
        match msg.message {
            WM_CLOSE => {
                return Some(WWindCoreEvent::CloseWindow(msg.hwnd.into()));
            },
            e => (),
        }

        DefWindowProcA(msg.hwnd, msg.message, msg.wParam, msg.lParam);
        
        None
    }
}