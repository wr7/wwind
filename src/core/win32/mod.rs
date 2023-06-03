use std::convert::Infallible;

use std::mem::MaybeUninit;
use std::ptr::{addr_of};

use std::{iter, mem, ptr};


use winapi::um::errhandlingapi::GetLastError;
use winapi::um::wingdi::{
    GdiFlush, GetStockObject, LineTo, MoveToEx, SelectObject,
    SetDCPenColor, DC_PEN,
};


use crate::RectRegion;

use super::core_state_implementation::WWindCoreEvent;
use super::CoreStateImplementation;
use winapi::shared::minwindef::{HMODULE, LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::{HDC, HPEN, HWND};
use winapi::um::libloaderapi::GetModuleHandleA;
use winapi::um::winuser::{
    CreateWindowExA, DefWindowProcA, DestroyWindow, DispatchMessageA, GetDC,
    GetMessageA, GetUpdateRect, RegisterClassA, SetWindowTextA,
    ShowWindow, TranslateMessage, ValidateRect, CS_OWNDC, SW_NORMAL, WM_CLOSE, WM_PAINT, WNDCLASSA, WS_OVERLAPPEDWINDOW,
};

static mut ON_EVENT: Option<unsafe fn(WWindCoreEvent)> = None;

pub struct Win32State {
    hinst: HMODULE,
    pen: HPEN,
}

#[derive(Debug)]
enum WindowsSendMessage {
    CloseWindow(HWND),
    Paint(HWND, RectRegion),
}

unsafe impl Sync for WindowsSendMessage {}
unsafe impl Send for WindowsSendMessage {}

static mut SEND_MESSAGE_QUEUE: Vec<WindowsSendMessage> = Vec::new();

/// Neccesary because of "SendMessage" messages. Ugh
unsafe extern "system" fn window_proc(
    window: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CLOSE => {
            if let Some(on_event) = ON_EVENT {
                on_event(WWindCoreEvent::CloseWindow(window.into()));
            }

            return 0;
        }
        WM_PAINT => {
            if let Some(on_event) = ON_EVENT {
                let mut rect = MaybeUninit::uninit();
                let res = GetUpdateRect(window, rect.as_mut_ptr(), 0);
                let rect = rect.assume_init();

                ValidateRect(window, ptr::null());

                if res == 0 {
                    return 0;
                }

                let rect_region = RectRegion {
                    x: rect.left as u16,
                    y: rect.bottom as u16,
                    width: rect.right as u16,
                    height: rect.top as u16,
                };

                on_event(WWindCoreEvent::Expose(window.into(), rect_region));
            } else {
                ValidateRect(window, ptr::null());
            }
        }
        _ => (),
    }

    DefWindowProcA(window, msg, wparam, lparam)
}

#[derive(Clone, Copy)]
pub struct WindowsDrawingContext {
    pub context: HDC,
}

impl CoreStateImplementation for Win32State {
    type Error = Infallible;

    type Window = HWND;

    type DrawingContext = WindowsDrawingContext;

    unsafe fn new() -> Result<Self, Self::Error> {
        let hinst = GetModuleHandleA(ptr::null());
        let pen = GetStockObject(DC_PEN as i32) as *mut _;

        Ok(Win32State { hinst, pen })
    }

    fn add_window(
        &mut self,
        x: i16,
        y: i16,
        height: u16,
        width: u16,
        title: &str,
    ) -> Result<Self::Window, Self::Error> {
        const CLASS_NAME: &[u8] = b"WWIND Window\0";

        static mut WINDOW_CLASS_REGISTERED: bool = false;
        unsafe {
            if !WINDOW_CLASS_REGISTERED {
                let mut class: WNDCLASSA = mem::zeroed();
                class.lpfnWndProc = Some(window_proc);
                class.hInstance = self.hinst;
                class.lpszClassName = CLASS_NAME.as_ptr() as *const i8;
                class.style = CS_OWNDC;

                RegisterClassA(addr_of!(class));

                WINDOW_CLASS_REGISTERED = true;
            }
        }

        println!("b");

        // C String moment
        let title: Vec<i8> = title
            .as_bytes()
            .iter()
            .copied()
            .filter(|&b| b != 0)
            .chain(iter::once(0))
            .map(|n| n as i8)
            .collect();

        let window = unsafe {
            CreateWindowExA(
                0,
                dbg!(CLASS_NAME.as_ptr() as *const i8),
                dbg!(title.as_ptr()),
                WS_OVERLAPPEDWINDOW,
                x as i32,
                y as i32,
                width as i32,
                height as i32,
                ptr::null_mut(),
                ptr::null_mut(),
                self.hinst,
                ptr::null_mut(),
            )
        };

        println!("{}", unsafe { GetLastError() });

        unsafe {
            let dc = GetDC(window);
            SelectObject(dc, self.pen as *mut _);
        }

        println!("c");
        dbg!(window);

        unsafe { ShowWindow(dbg!(window), SW_NORMAL) };

        Ok(window)
    }

    fn set_window_title(&mut self, window: Self::Window, title: &str) {
        // C-String moment
        let title_vec: Vec<i8> = title
            .as_bytes()
            .iter()
            .copied()
            .filter(|&b| b != 0)
            .chain(iter::once(0))
            .map(|n| n as i8)
            .collect();

        unsafe { SetWindowTextA(window, title_vec.as_ptr()) };
    }

    unsafe fn destroy_window(&mut self, window: Self::Window) {
        DestroyWindow(window);
    }

    unsafe fn wait_for_events(&mut self, on_event: &mut unsafe fn(WWindCoreEvent)) {
        // TODO: make GetMessageA close the window when it returns false

        ON_EVENT = Some(*on_event);

        let mut msg = MaybeUninit::uninit();

        if GetMessageA(msg.as_mut_ptr(), ptr::null_mut(), 0, 0) == 0 {
            return;
        }

        TranslateMessage(msg.as_ptr());
        DispatchMessageA(msg.as_ptr());
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        unsafe {
            GdiFlush();
        }
        Ok(())
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
            // BeginPath(drawing_context.context);

            MoveToEx(
                drawing_context.context,
                x1 as i32,
                y1 as i32,
                ptr::null_mut(),
            );

            LineTo(drawing_context.context, x2 as i32, y2 as i32);

            // StrokeAndFillPath(drawing_context.context);

            // EndPath(drawing_context.context);
        }

        Ok(())
    }

    unsafe fn get_context(&mut self, window: Self::Window) -> Self::DrawingContext {
        let context = GetDC(window);

        Self::DrawingContext { context }
    }

    fn set_draw_color(
        &mut self,
        context: Self::DrawingContext,
        color: crate::Color,
    ) -> Result<(), Self::Error> {
        unsafe { SetDCPenColor(context.context, color.into()) };
        Ok(())
    }
}
