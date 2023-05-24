use super::{CoreStateImplementation, CoreWindowRef, CoreState, core_state_implementation::WWindCoreEvent};
use x11rb::{
    protocol::{
        Event,
        xproto::{self, PropMode, EventMask, CreateWindowAux, Screen, create_window, WindowClass, map_window, change_property, destroy_window, EXPOSE_EVENT, CLIENT_MESSAGE_EVENT, send_event},
},
    rust_connection::{RustConnection, ConnectError, ConnectionError, ParseError, ReplyError}, atom_manager, connection::Connection,
};

mod error;
pub use error::RbError;

pub struct X11RbState {
    connection: RustConnection,
    atoms: Atoms,
    screen: Screen,
}

atom_manager! {
    pub Atoms:
    AtomCookie {
        WM_PROTOCOLS,
        WM_DELETE_WINDOW,
        _NET_WM_PING,
        _NET_WM_NAME,
        UTF8_STRING,
        ATOM,
    }
}

impl CoreStateImplementation for X11RbState {
    type Error = RbError;
    type Window = u32;

    unsafe fn new() -> Result<Self, Self::Error> {
        let (connection, screen_number) = x11rb::connect(None)?;
        let screen = connection.setup().roots[screen_number].clone();

        let atoms = Atoms::new(&connection)?;
        let atoms = atoms.reply()?;

        Ok(Self {connection, atoms, screen})
    }

    fn set_window_title(&mut self, window: Self::Window, title: &str) {
        unsafe {
            xproto::change_property(&self.connection, PropMode::REPLACE, window, self.atoms._NET_WM_NAME, self.atoms.UTF8_STRING, 8, title.len() as u32, title.as_bytes());
        }
    }

    fn add_window(&mut self, x: i16, y: i16, height: u16, width: u16, title: &str) -> Result<Self::Window, Self::Error> {
        unsafe {
            let window = self.connection.generate_id()?;

            let event_mask = EventMask::EXPOSURE;
            let window_aux = CreateWindowAux::new().event_mask(event_mask).background_pixel(self.screen.black_pixel);

            let root = self.screen.root;
            let root_visual = self.screen.root_visual;

            create_window(&self.connection, 0, window, root, x, y, width, height, 1, WindowClass::COPY_FROM_PARENT, root_visual, &window_aux)?;

            map_window(&self.connection, window).unwrap();

            let protocols = [self.atoms.WM_DELETE_WINDOW, self.atoms._NET_WM_PING];
            let protocol_len = protocols.len() as u32;
            let (_, protocols, _) = protocols.align_to::<u8>();

            change_property(&self.connection, PropMode::REPLACE, window, self.atoms.WM_PROTOCOLS, self.atoms.ATOM, 32, protocol_len, protocols).unwrap();

            self.set_window_title(window, title);

            self.connection.flush()?;

            Ok(window)
        }
    }

    unsafe fn destroy_window(&mut self, window: Self::Window) {
        destroy_window(&self.connection, window).unwrap();
    }

    unsafe fn wait_for_events(&mut self) -> Option<WWindCoreEvent> {
    
        let event = self.connection.wait_for_event();

        let event = if let Ok(event) = event {event} else {
            return None
        };

        match event {
            Event::Expose(_) => { // XCB_EXPOSE
                eprintln!("Expose");
            },
            Event::ClientMessage(event) => { // XCB_CLIENT_MESSAGE
                println!("client event");
                if event.type_ == self.atoms.WM_PROTOCOLS {
                    let protocol = event.data.as_data32()[0];
    
                    if protocol == 0 {
                        return None;
                    }
    
                    
                    if protocol == self.atoms.WM_DELETE_WINDOW {
                        return Some(WWindCoreEvent::CloseWindow(event.window.into()))
            
                    } else if protocol == self.atoms._NET_WM_PING {
                        let mut reply = event;
                        
                        reply.window = self.screen.root;
    
                        println!("pong");
                        send_event(&self.connection, false, self.screen.root, EventMask::SUBSTRUCTURE_NOTIFY | EventMask::RESIZE_REDIRECT, reply).unwrap();
    
                        self.connection.flush().unwrap();
                    } else {
                        println!("unknown client event type {:?}", event.type_);
                    }
    
                }
            }
            event => {
                println!("Unknown event: {event:?}");
            }
        }
        None
    }
}