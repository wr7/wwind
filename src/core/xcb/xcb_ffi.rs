use libloading::os::unix::Library;

use std::{
    marker::{PhantomData, PhantomPinned},
    ffi::{self, CStr}, ops::BitOr, mem, fmt::Debug,
};

#[derive(Copy, Clone, Debug)]
#[repr(u16)]
pub enum XCBWindowClass {
    CopyFromParent = 0,
    InputOutput = 1,
    Input = 2,
}

#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub enum XCBEventMaskEnum {
    KeyPress = 1,
    KeyRelease = 2,
    ButtonPress = 4,
    ButtonRelease = 8,
    EnterWindow = 16,
    LeaveWindow = 32,
    PointerMotion = 64,
    PointerMotionHint = 128,
    Button1Motion = 256,
    Button2Motion = 512,
    Button3Motion = 1024,
    Button4Motion = 2048,
    Button5Motion = 4096,
    ButtonMotion = 8192,
    KeymapState = 16384,
    Exposure = 32768,
    VisibilityChange = 65536,
    StructureNotify = 131072,
    ResizeRedirect = 262144,
    SubstructureNotify = 524288,
    SubstructureRedirect = 1048576,
    FocusChange = 2097152,
    PropertyChange = 4194304,
    ColorMapChange = 8388608,
    OwnerGrabButton = 16777216,
}

impl BitOr<XCBEventMaskEnum> for XCBEventMaskEnum {
    type Output = u32;

    fn bitor(self, rhs: XCBEventMaskEnum) -> u32 {
        self as u32 | rhs as u32
    }
}

impl BitOr<XCBEventMaskEnum> for u32 {
    type Output = u32;

    fn bitor(self, rhs: XCBEventMaskEnum) -> u32 {
        self | rhs as u32
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct XCBWindowMask{
    data: u32,
}


#[derive(Copy, Clone, Debug)]
#[repr(u32)]
pub enum XCBWindowMaskEnum {
    BackPixmap = 1,
    BackPixel = 2,
    BorderPixmap = 4,
    BorderPixel = 8,
    BitGravity = 16,
    WindowGravity = 32, 
    BackingStore = 64,
    BackingPlanes = 128,
    BackingPixel = 256,
    SaveUnder = 1024,
    EventMask = 2048,
    DoNotPropogate = 4096,
    Colormap = 8192,
    Cursor = 16384,
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum XCBPropertyMode {
    Replace = 0,
    Prepend = 1,
    Append = 2,
}


impl BitOr<XCBWindowMaskEnum> for XCBWindowMaskEnum {
    type Output = XCBWindowMask;

    fn bitor(self, rhs: XCBWindowMaskEnum) -> Self::Output {
        let data = self as u32 | self as u32;
        XCBWindowMask {data}
    }
}

impl BitOr<XCBWindowMask> for XCBWindowMaskEnum {
    type Output = XCBWindowMask;

    fn bitor(self, rhs: XCBWindowMask) -> Self::Output {
        let data = self as u32 | rhs.data;
        XCBWindowMask {data}
    }
}

impl BitOr<XCBWindowMaskEnum> for XCBWindowMask {
    type Output = XCBWindowMask;

    fn bitor(self, rhs: XCBWindowMaskEnum) -> Self::Output {
        let data = self.data | rhs as u32;
        XCBWindowMask {data}
    }
}

pub struct XCBFunctions {
    library: Library,
    xcb_connect: unsafe extern "C" fn(*const u8, *mut ffi::c_int) -> *mut xcb_connection_t,
    xcb_flush: unsafe extern "C" fn(*mut xcb_connection_t) -> ffi::c_int,
    xcb_connection_has_error: unsafe extern "C" fn(*mut xcb_connection_t) -> ffi::c_int,
    xcb_get_setup: unsafe extern "C" fn(*mut xcb_connection_t) -> *const xcb_setup_t,
    xcb_setup_roots_iterator: unsafe extern "C" fn (*const xcb_setup_t) -> xcb_screen_iterator_t,
    xcb_generate_id: unsafe extern "C" fn (*mut xcb_connection_t) -> u32,
    xcb_create_window: unsafe extern "C" fn(*mut xcb_connection_t, u8, xcb_window_t, xcb_window_t, i16, i16, u16, u16, u16, XCBWindowClass, xcb_visualid_t, XCBWindowMask, *const ffi::c_void) -> xcb_void_cookie_t,
    xcb_map_window: unsafe extern "C" fn (*mut xcb_connection_t, xcb_window_t) -> xcb_void_cookie_t,
    xcb_intern_atom: unsafe extern "C" fn (*mut xcb_connection_t, u8, u16, *const u8) -> xcb_intern_atom_cookie_t,
    xcb_intern_atom_reply: unsafe extern "C" fn(*mut xcb_connection_t, xcb_intern_atom_cookie_t, *mut *mut xcb_generic_error_t) -> *mut xcb_intern_atom_reply_t,
    xcb_disconnect: unsafe extern "C" fn(*mut xcb_connection_t),
    xcb_change_property: unsafe extern "C" fn(*mut xcb_connection_t, XCBPropertyMode, xcb_window_t, xcb_atom_t, xcb_atom_t, u8, u32, *const ffi::c_void) -> xcb_void_cookie_t,
    xcb_wait_for_event: unsafe extern "C" fn(*mut xcb_connection_t) -> *mut xcb_generic_event_t,
    xcb_send_event: unsafe extern "C" fn(*mut xcb_connection_t, u8, xcb_window_t, u32, *const u8) -> xcb_void_cookie_t,
    xcb_get_atom_name: unsafe extern "C" fn(*mut xcb_connection_t, xcb_atom_t) -> xcb_get_atom_name_cookie_t,
    xcb_get_atom_name_name: unsafe extern "C" fn (*const xcb_get_atom_name_reply_t) -> *mut u8,
    xcb_get_atom_name_reply: unsafe extern "C" fn(*mut xcb_connection_t, xcb_get_atom_name_cookie_t, *mut *mut xcb_generic_error_t) -> *mut xcb_get_atom_name_reply_t,
    xcb_destroy_window: unsafe extern "C" fn(*mut xcb_connection_t, xcb_window_t) -> xcb_void_cookie_t,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct xcb_generic_event_t {
    pub response_type: u8,
    pad0: u8,
    pub sequence: u16,
    pad: [u32; 7],
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct xcb_get_atom_name_cookie_t {
    sequence: ffi::c_uint,
}

#[repr(C)]
pub struct xcb_get_atom_name_reply_t {
    response_type: u8,
    pad0: u8,
    sequence: u16,
    length: u32,
    name_len: u16,
    pad1: [u8; 22],
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct xcb_client_message_event_t {
    pub response_type: u8,
    pub format: u8,
    pub sequence: u16,
    pub window: xcb_window_t,
    pub message_type: xcb_atom_t,
    pub data: xcb_client_message_data_t,
}

impl xcb_client_message_event_t {
    /// Unsafe: `event` must be a client message event
    pub unsafe fn from_generic(event: xcb_generic_event_t) -> xcb_client_message_event_t {
        mem::transmute(event)
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub union xcb_client_message_data_t {
    pub data8: [u8; 20],
    pub data16: [u16; 10],
    pub data32: [u32; 5],
}

impl XCBFunctions {
    pub unsafe fn xcb_connect(&self, display_name: Option<&CStr>, screen_pointer: *mut ffi::c_int) -> *mut xcb_connection_t {
        (self.xcb_connect)(display_name.map_or(std::ptr::null(), |name| name.as_ptr()) as *const u8, screen_pointer)
    }
    pub unsafe fn xcb_flush(&self, connection: *mut xcb_connection_t) -> ffi::c_int {
        (self.xcb_flush)(connection)
    }
    pub unsafe fn xcb_connection_has_error(&self, connection: *mut xcb_connection_t) -> bool {
        (self.xcb_connection_has_error)(connection) != 0
    }
    pub unsafe fn xcb_get_setup(&self, connection: *mut xcb_connection_t) -> *const xcb_setup_t {
        (self.xcb_get_setup)(connection)
    }
    pub unsafe fn xcb_setup_roots_iterator(&self, setup: *const xcb_setup_t) -> xcb_screen_iterator_t {
        (self.xcb_setup_roots_iterator)(setup)
    }
    pub unsafe fn xcb_generate_id(&self, connection: *mut xcb_connection_t) -> u32 {
        (self.xcb_generate_id)(connection)
    }
    pub unsafe fn xcb_create_window(&self, connection: *mut xcb_connection_t, depth: Option<u8>, window_id: xcb_window_t, parent: xcb_window_t, x: i16, y: i16, width: u16, height: u16, border_width: u16, class: XCBWindowClass, visual: xcb_visualid_t, value_mask: XCBWindowMask, value_list: *const ffi::c_void) -> xcb_void_cookie_t {
        let depth = depth.unwrap_or(0);
        (self.xcb_create_window)(connection, depth, window_id, parent, x, y, width, height, border_width, class, visual, value_mask, value_list)
    }
    pub unsafe fn xcb_map_window(&self, connection: *mut xcb_connection_t, window_id: xcb_window_t) -> xcb_void_cookie_t {
        (self.xcb_map_window)(connection, window_id)
    }
    pub unsafe fn xcb_intern_atom(&self, connection: *mut xcb_connection_t, only_if_exists: bool, name: &str) -> xcb_intern_atom_cookie_t {
        assert!(name.len() <= 32, "Atoms cannot be more than 32 bytes long");
        (self.xcb_intern_atom)(connection, only_if_exists as u8, name.len() as u16, name.as_ptr())
    }
    pub unsafe fn xcb_intern_atom_reply(&self, connection: *mut xcb_connection_t, cookie: xcb_intern_atom_cookie_t, error: *mut *mut xcb_generic_error_t) -> *mut xcb_intern_atom_reply_t {
        (self.xcb_intern_atom_reply)(connection, cookie, error)
    }
    pub unsafe fn xcb_disconnect(&self, connection: *mut xcb_connection_t) {
        (self.xcb_disconnect)(connection)
    }
    pub unsafe fn xcb_change_property(&self, connection: *mut xcb_connection_t, mode: XCBPropertyMode, window: xcb_window_t, property: xcb_atom_t, data_type: xcb_atom_t, element_size: u8, data_len: u32, data: *const ffi::c_void) -> xcb_void_cookie_t {
        (self.xcb_change_property)(connection, mode, window, property, data_type, element_size, data_len, data)
    }
    pub unsafe fn xcb_wait_for_event(&self, connection: *mut xcb_connection_t) -> *mut xcb_generic_event_t {
        (self.xcb_wait_for_event)(connection)
    }
    pub unsafe fn xcb_send_event(&self, connection: *mut xcb_connection_t, propogate: bool, destination: xcb_window_t, event_mask: u32, event: *const u8) -> xcb_void_cookie_t {
        (self.xcb_send_event)(connection, propogate as u8, destination, event_mask, event)
    }
    pub unsafe fn xcb_get_atom_name(&self, connection: *mut xcb_connection_t, atom_name: xcb_atom_t) -> xcb_get_atom_name_cookie_t {
        (self.xcb_get_atom_name)(connection, atom_name)
    }
    pub unsafe fn xcb_get_atom_name_name(&self, reply: *const xcb_get_atom_name_reply_t) -> *mut u8 {
        (self.xcb_get_atom_name_name)(reply)
    }
    pub unsafe fn xcb_get_atom_name_reply(&self, connection: *mut xcb_connection_t, cookie: xcb_get_atom_name_cookie_t, error: *mut *mut xcb_generic_error_t) -> *mut xcb_get_atom_name_reply_t {
        (self.xcb_get_atom_name_reply)(connection, cookie, error)
    }
    pub unsafe fn xcb_destroy_window(&self, connection: *mut xcb_connection_t, window: xcb_window_t) -> xcb_void_cookie_t {
        (self.xcb_destroy_window)(connection, window)
    }
}

macro_rules! load_functions {
    {$library: ident, $($function_name: ident);+} => {
        $(
            let mut lib_string: Vec<u8> = stringify!($function_name).as_bytes().to_owned();
            lib_string.push(0);

            let $function_name = *$library.get(&*lib_string)?;
        )+
    };
}

impl XCBFunctions {
    pub fn new(library: Library) -> Result<Self, libloading::Error> {
        unsafe {
            load_functions!{library,
                xcb_connect; 
                xcb_flush; 
                xcb_connection_has_error; 
                xcb_get_setup; 
                xcb_setup_roots_iterator; 
                xcb_generate_id; 
                xcb_create_window; 
                xcb_map_window;
                xcb_intern_atom;
                xcb_intern_atom_reply;
                xcb_disconnect;
                xcb_change_property;
                xcb_wait_for_event;
                xcb_send_event;
                xcb_get_atom_name;
                xcb_get_atom_name_name;
                xcb_get_atom_name_reply;
                xcb_destroy_window
            }

            Ok(XCBFunctions {
                library,
                xcb_connect, 
                xcb_flush, 
                xcb_connection_has_error, 
                xcb_get_setup, 
                xcb_setup_roots_iterator, 
                xcb_generate_id, 
                xcb_create_window, 
                xcb_map_window,
                xcb_intern_atom,
                xcb_intern_atom_reply,
                xcb_disconnect,
                xcb_change_property,
                xcb_wait_for_event,
                xcb_send_event,
                xcb_get_atom_name,
                xcb_get_atom_name_name,
                xcb_get_atom_name_reply,
                xcb_destroy_window,
            })
        }
    }
}

pub struct xcb_connection_t {
    _data: [u8; 0],
    _phantom_pinned: PhantomData<(*mut u8, PhantomPinned)>,
}

#[repr(C)]
pub struct xcb_void_cookie_t{
    sequence: ffi::c_int,
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
#[repr(C)]
pub struct xcb_window_t(pub u32);

#[repr(C)]
pub struct xcb_colormap_t(u32);

#[derive(Clone, Copy)]
#[repr(C)]
pub struct xcb_visualid_t(u32);

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct xcb_atom_t(u32);

#[repr(C)]
pub struct xcb_screen_t {
    pub root: xcb_window_t,
    pub default_colormap: xcb_colormap_t,
    pub white_pixel: u32,
    pub black_pixel: u32,
    pub current_input_masks: u32,
    pub width_in_pixels: u16,
    pub height_in_pixels: u16,
    pub width_in_millimeters: u16,
    pub height_in_millimeters: u16,
    pub min_installed_maps: u16,
    pub max_installed_maps: u16,
    pub root_visual: xcb_visualid_t,
    pub backing_stores: u8,
    pub save_unders: u8,
    pub root_depth: u8,
    pub allowed_depths_len: u8,
}

#[repr(C)]
pub struct xcb_screen_iterator_t {
    pub data: *mut xcb_screen_t,
    pub rem: ffi::c_int,
    pub index: ffi::c_int,
}

pub struct xcb_setup_t {
    _data: [u8; 0],
    _phantom_pinned: PhantomData<(*mut u8, PhantomPinned)>,
}

#[repr(C)]
pub struct xcb_intern_atom_cookie_t {
    sequence: ffi::c_int,
}

#[repr(C)]
pub struct xcb_intern_atom_reply_t {
    response_type: u8,
    pad0: u8,
    sequence: u16,
    length: u32,
    pub atom: xcb_atom_t,
}

#[repr(C)]
pub struct xcb_generic_error_t {
    response_type: u8,
    error_code: u8,
    sequence: u16,
    resource_id: u32,
    minor_code: u16,
    major_code: u8,
    pad0: u8,
    pad: [u32; 5],
    full_sequence: u32,
}


impl Debug for xcb_atom_t {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("xcb_atom_t").field(&self.0).finish()
    }
}