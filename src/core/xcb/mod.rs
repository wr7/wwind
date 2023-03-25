mod xcb_ffi;
mod atoms;

use libloading::os::unix::Library;

use std::{mem::{self, MaybeUninit}, ptr::{self, addr_of}};

use xcb_ffi::{XCBFunctions, xcb_connection_t, xcb_screen_t};

pub use xcb_ffi::xcb_window_t;

use crate::{core::{xcb::xcb_ffi::{xcb_client_message_event_t, xcb_atom_t}, CoreState}, WWindState};

use super::{CoreWindow, CoreStateData};

use self::{atoms::Atoms, xcb_ffi::{XCBWindowClass, xcb_visualid_t, XCBWindowMaskEnum, XCBEventMaskEnum, XCBPropertyMode}};

pub struct XCBState {
    functions: XCBFunctions,
    atoms: Atoms,
    connection: *mut xcb_connection_t,
    screen: *mut xcb_screen_t,
}


impl XCBState {
    // Two XCB instances cannot exist at the same time. 
    pub unsafe fn new() -> Result<Self, libloading::Error> {
        let xcb_library = Library::new("libxcb.so")?;
        let functions = XCBFunctions::new(xcb_library)?;

        let connection = functions.xcb_connect(None, std::ptr::null_mut());

        if functions.xcb_connection_has_error(connection) {
            panic!("X connection could not be created");
        }

        let screen = functions.xcb_setup_roots_iterator(functions.xcb_get_setup(connection)).data;

        if screen.is_null() {
            panic!("Could not find X Screen");
        }

        let atoms = Atoms::new(connection, &functions);

        Ok(Self {functions, atoms, connection, screen})
    }
    pub fn set_window_title(&mut self, window: CoreWindow, title: &str) {
        unsafe {
            let window = window.xcb_window;

            self.functions.xcb_change_property(self.connection, XCBPropertyMode::Replace, window, self.atoms.net_wm_name, self.atoms.utf8_string, 8, title.len() as u32, title.as_bytes().as_ptr() as *const _);
        }
    }

    pub fn add_window(&mut self, x: i16, y: i16, height: u16, width: u16, title: &str) -> CoreWindow {
        unsafe {
            let window = xcb_window_t(self.functions.xcb_generate_id(self.connection));

            let event_mask = XCBEventMaskEnum::Exposure as u32;
            let value_mask = XCBWindowMaskEnum::BackPixel | XCBWindowMaskEnum::EventMask;
            let values: [u32; 2] = [self.screen().black_pixel, event_mask];

            let root = self.screen().root;
            let root_visual = self.screen().root_visual;

            self.functions.xcb_create_window(self.connection, None, window, root, x, y, width, height, 1, XCBWindowClass::CopyFromParent, root_visual, value_mask, values.as_ptr() as *const _);

            self.functions.xcb_map_window(self.connection, window);

            let protocols = [self.atoms.wm_delete_window, self.atoms.net_wm_ping];

            self.functions.xcb_change_property(self.connection, XCBPropertyMode::Replace, window, self.atoms.wm_protocols, self.atoms.atom, 32, protocols.len() as u32, protocols.as_ptr() as *const _);

            self.set_window_title(CoreWindow {xcb_window: window}, title);

            self.functions.xcb_flush(self.connection);

            super::CoreWindow{xcb_window: window}
        }
    }

    pub unsafe fn destroy_window(&mut self, window: xcb_window_t) {
        self.functions.xcb_destroy_window(self.connection, window);
    }

    /*pub unsafe fn rename_window(&mut self, window: xcb_window_t, window_name: &str) {
        self.functions.xcb_change_property(self.connection, mode, window, property, data_type, element_size, data_len, data)
    }*/

    #[inline]
    unsafe fn screen<'a>(&'a mut self) -> &'a mut xcb_screen_t {
        self.screen.as_mut().unwrap()
    }
    
}

pub unsafe fn wait_for_events(state: &mut CoreState) -> bool {
    let xcb_state = state.get_xcb();

    let event = xcb_state.functions.xcb_wait_for_event(xcb_state.connection);

    if event.is_null() {
        return false;
    }

    let event = *Box::from_raw(event);

    let response_type = event.response_type & !0x80;

    match response_type {
        12 => { // XCB_EXPOSE
            // expose
        },
        33 => { // XCB_CLIENT_MESSAGE
            let client_event = xcb_client_message_event_t::from_generic(event);

            if client_event.message_type == xcb_state.atoms.wm_protocols {
                let protocol = client_event.data.data32[0];

                if protocol == 0 {
                    return true;
                }

                let protocol: xcb_atom_t = mem::transmute(protocol);

                if protocol == xcb_state.atoms.wm_delete_window {
                    let window = CoreWindow { xcb_window: client_event.window };

                    super::on_window_close(state, window);
        
                } else if protocol == xcb_state.atoms.net_wm_ping {
                    let mut reply = client_event;
                    
                    reply.window = (*xcb_state.screen).root;

                    xcb_state.functions.xcb_send_event(xcb_state.connection, false, (*xcb_state.screen).root, XCBEventMaskEnum::SubstructureNotify | XCBEventMaskEnum::ResizeRedirect, addr_of!(reply) as *const _);

                    xcb_state.functions.xcb_flush(xcb_state.connection);
                } else {
                    println!("unknown client event type {:?}", client_event.message_type);
                }

            }
        }
        _ => {
            println!("Unknown event: {response_type:?}");
        }
    }

    true
}

impl Drop for XCBState {
    fn drop(&mut self) {
        unsafe {self.functions.xcb_disconnect(self.connection)};
    }
}