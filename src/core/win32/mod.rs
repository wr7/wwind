use std::convert::Infallible;

use std::mem::MaybeUninit;
use std::ptr::addr_of;

use std::{iter, mem, ptr};

use winapi::um::errhandlingapi::GetLastError;
use winapi::um::wingdi::{
    ExtTextOutA, GdiFlush, GetStockObject, LineTo, MoveToEx, SelectClipRgn, SelectObject,
    SetDCBrushColor, SetDCPenColor, DC_BRUSH, DC_PEN,
};

use crate::RectRegion;

use super::core_state_implementation::WWindCoreEvent;
use super::CoreStateImplementation;
use winapi::shared::minwindef::{HIWORD, HMODULE, LOWORD, LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::{HBRUSH, HDC, HPEN, HWND, RECT};
use winapi::um::libloaderapi::GetModuleHandleA;
use winapi::um::winuser::{
    CreateWindowExA, DefWindowProcA, DestroyWindow, DispatchMessageA, FillRect, GetClientRect,
    GetDC, GetMessageA, GetUpdateRect, GetWindowLongPtrA, GetWindowRect, RedrawWindow,
    RegisterClassA, SetWindowLongPtrA, SetWindowTextA, ShowWindow, TranslateMessage, ValidateRect,
    CS_OWNDC, GWLP_USERDATA, RDW_INTERNALPAINT, RDW_NOINTERNALPAINT, SW_NORMAL, WM_CLOSE, WM_PAINT,
    WM_SIZE, WNDCLASSA, WS_OVERLAPPEDWINDOW,
};

static mut ON_EVENT: Option<unsafe fn(WWindCoreEvent)> = None;

pub struct Win32State {
    hinst: HMODULE,
    pen: HPEN,
    brush: HBRUSH,
}

struct Win32WindowData {
    width: u16,
    height: u16,
}

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
        WM_SIZE => {
            let width = LOWORD(lparam as u32);
            let height = HIWORD(lparam as u32);

            let window_data =
                &mut *(GetWindowLongPtrA(window, GWLP_USERDATA) as *mut Win32WindowData);

            if width <= window_data.width && height <= window_data.height {
                let rect = RECT {
                    left: 0,
                    bottom: 0,
                    top: height as i32,
                    right: width as i32,
                };

                RedrawWindow(window, addr_of!(rect), ptr::null_mut(), RDW_INTERNALPAINT);
            }

            window_data.width = width;
            window_data.height = height;
        }
        WM_PAINT => {
            if let Some(on_event) = ON_EVENT {
                SelectClipRgn(GetDC(window), ptr::null_mut());

                let mut rect = MaybeUninit::uninit();
                GetUpdateRect(window, rect.as_mut_ptr(), 0);
                let rect = rect.assume_init();

                ValidateRect(window, ptr::null());

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
        let brush = GetStockObject(DC_BRUSH as i32) as *mut _;

        Ok(Win32State { hinst, pen, brush })
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

        let window_data = Box::into_raw(Box::new(Win32WindowData { width, height }));

        unsafe {
            let dc = GetDC(window);

            SelectObject(dc, self.pen as *mut _);
            SelectObject(dc, self.brush as *mut _);

            SetWindowLongPtrA(window, GWLP_USERDATA, window_data as isize);
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

    fn draw_rectangle(
        &mut self,
        drawing_context: Self::DrawingContext,
        rectangle: RectRegion,
    ) -> Result<(), Self::Error> {
        let left = rectangle.x as i32;
        let bottom = rectangle.y as i32;
        let right = (rectangle.x + rectangle.width) as i32;
        let top = (rectangle.y + rectangle.height) as i32;

        let rect = RECT {
            left,
            top,
            right,
            bottom,
        };

        unsafe {
            FillRect(drawing_context.context, addr_of!(rect), self.brush);
        }

        Ok(())
    }

    fn draw_text(
        &mut self,
        drawing_context: Self::DrawingContext,
        x: u16,
        y: u16,
        text: &str,
    ) -> Result<(), Self::Error> {
        unsafe {
            ExtTextOutA(
                drawing_context.context,
                x as i32,
                y as i32,
                0,
                ptr::null(),
                text.as_ptr() as *const i8,
                text.len() as u32,
                ptr::null(),
            )
        };

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
        unsafe {
            SetDCPenColor(context.context, color.into());
            SetDCBrushColor(context.context, color.into());
        }
        Ok(())
    }

    fn get_size(&self, window: Self::Window) -> (u16, u16) {
        let window_data =
            unsafe { &*(GetWindowLongPtrA(window, GWLP_USERDATA) as *const Win32WindowData) };

        (window_data.width, window_data.height)
    }
}
