use crate::core::CoreState;

use super::{xcb_ffi::{xcb_atom_t, xcb_intern_atom_cookie_t, XCBFunctions, xcb_connection_t}, XCBState};

macro_rules! create_atoms {
    ($connection:ident, $functions:ident; $($atom_var_name:ident= $atom_name:ident);+) => {
        $(let $atom_var_name;)+
        unsafe {
            let mut error: *mut crate::core::xcb::xcb_ffi::xcb_generic_error_t = std::ptr::null_mut();

            $(let $atom_name = $functions.xcb_intern_atom($connection, false, stringify!($atom_name));)+
            $(
                let $atom_name = $functions.xcb_intern_atom_reply($connection, $atom_name, core::ptr::addr_of_mut!(error));
                $atom_var_name = if let Some(atom_reply) = $atom_name.as_mut() {
                    let atom = atom_reply.atom;

                    std::mem::drop(std::boxed::Box::from_raw($atom_name));

                    atom
                } else {
                    panic!(concat!("Could not load atom ", stringify!($atom_name)));
                };
            )+
        }
    };
}


pub struct Atoms {
    pub wm_protocols: xcb_atom_t,
    pub wm_delete_window: xcb_atom_t,
    pub net_wm_ping: xcb_atom_t,
    pub net_wm_name: xcb_atom_t,
    pub utf8_string: xcb_atom_t,
    pub atom: xcb_atom_t,
}

impl Atoms {
    pub fn new(connection: *mut xcb_connection_t, functions: &XCBFunctions) -> Atoms {
        create_atoms!(connection, functions; 
            wm_protocols = WM_PROTOCOLS; 
            wm_delete_window = WM_DELETE_WINDOW; 
            net_wm_ping = _NET_WM_PING;
            net_wm_name = _NET_WM_NAME;
            utf8_string = UTF8_STRING;
            atom = ATOM
        );
        
        Self {wm_protocols, wm_delete_window, net_wm_ping, net_wm_name, utf8_string, atom}
    }
}