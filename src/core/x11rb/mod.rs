use crate::{Color, RectRegion, WindowPositionData};

use super::{core_state_implementation::WWindCoreEvent, CoreStateImplementation, CoreWindowRef};
use x11rb::{
    atom_manager,
    connection::Connection,
    protocol::{
        xproto::{
            self, change_property, create_window, destroy_window, map_window, send_event,
            BackingStore, ChangeGCAux, ConnectionExt, CreateGCAux, CreateWindowAux, EventMask,
            GetWindowAttributesRequest, Point, PropMode, Rectangle, Screen, Segment, Visualtype,
            WindowClass, CLIENT_MESSAGE_EVENT, EXPOSE_EVENT,
        },
        Event,
    },
    rust_connection::{ConnectError, ConnectionError, ParseError, ReplyError, RustConnection},
};

mod error;
pub use error::RbError;

pub struct X11RbState {
    connection: RustConnection,
    graphics_context: u32,
    atoms: Atoms,
    screen: Screen,
    visual: Visualtype,
    red_shift: u8,
    green_shift: u8,
    blue_shift: u8,
}

fn get_first_bit_pos(mut num: u32) -> u8 {
    let mut pos = 0;
    while num & 1 == 0 {
        pos += 1;
        num = num >> 1;
    }
    pos
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

impl X11RbState {
    #[inline]
    fn get_color(&self, color: Color) -> u32 {
        (color.red as u32) << self.red_shift
            | (color.green as u32) << self.green_shift
            | (color.blue as u32) << self.blue_shift
    }
}

impl CoreStateImplementation for X11RbState {
    type Error = RbError;
    type Window = u32;
    type DrawingContext = Self::Window;

    unsafe fn new() -> Result<Self, Self::Error> {
        let (connection, screen_number) = x11rb::connect(None)?;

        let screen = connection.setup().roots[screen_number].clone();

        let depth = screen.root_depth;
        let mut visual = None;

        'outer: // X11 moment
        for d in screen.allowed_depths.iter() {
            if d.depth == depth {
                for v_type in d.visuals.iter() {
                    if v_type.visual_id == screen.root_visual {
                        let _ = visual.insert(*v_type);
                        break 'outer;
                    }
                }
            }
        }

        let visual = if let Some(visual) = visual {
            visual
        } else {
            todo!("Invalid root visual")
        };

        // let color = connection.alloc_color(screen.default_colormap, 65535, 0, 0)?.reply()?.pixel;
        // let color = visual.red_mask * 36 + visual.green_mask * 35 + visual.blue_mask * 38;
        let red_shift = get_first_bit_pos(visual.red_mask);
        let green_shift = get_first_bit_pos(visual.green_mask);
        let blue_shift = get_first_bit_pos(visual.blue_mask);

        // let color = 0x463a22;
        // let color = (visual.red_mask+1)/256 + (visual.green_mask+1)/256*36 + (visual.blue_mask+1)/256*38;

        let atoms = Atoms::new(&connection)?;
        let atoms = atoms.reply()?;

        let graphics_context = connection.generate_id()?;
        connection.create_gc(
            graphics_context,
            screen.root,
            &CreateGCAux::default()
                .foreground(screen.black_pixel)
                .background(screen.black_pixel)
                .line_width(2),
        )?;

        Ok(Self {
            connection,
            atoms,
            screen,
            graphics_context,
            visual,
            red_shift,
            green_shift,
            blue_shift,
        })
    }

    fn set_window_title(&mut self, window: Self::Window, title: &str) {
        unsafe {
            xproto::change_property(
                &self.connection,
                PropMode::REPLACE,
                window,
                self.atoms._NET_WM_NAME,
                self.atoms.UTF8_STRING,
                8,
                title.len() as u32,
                title.as_bytes(),
            );
        }
    }

    fn add_window(
        &mut self,
        x: i16,
        y: i16,
        height: u16,
        width: u16,
        title: &str,
    ) -> Result<Self::Window, Self::Error> {
        unsafe {
            let window = self.connection.generate_id()?;

            let event_mask = EventMask::EXPOSURE;
            let window_aux = CreateWindowAux::new()
                .event_mask(event_mask)
                // .background_pixel(self.screen.white_pixel)
                .backing_store(BackingStore::WHEN_MAPPED);

            let root = self.screen.root;
            let root_visual = self.screen.root_visual;

            create_window(
                &self.connection,
                0,
                window,
                root,
                x,
                y,
                width,
                height,
                1,
                WindowClass::COPY_FROM_PARENT,
                root_visual,
                &window_aux,
            )?;

            map_window(&self.connection, window).unwrap();

            let protocols = [self.atoms.WM_DELETE_WINDOW, self.atoms._NET_WM_PING];
            let protocol_len = protocols.len() as u32;
            let (_, protocols, _) = protocols.align_to::<u8>();

            change_property(
                &self.connection,
                PropMode::REPLACE,
                window,
                self.atoms.WM_PROTOCOLS,
                self.atoms.ATOM,
                32,
                protocol_len,
                protocols,
            )
            .unwrap();

            self.set_window_title(window, title);

            self.connection.flush()?;

            Ok(window)
        }
    }

    fn draw_line(
        &mut self,
        window: Self::Window,
        x1: u16,
        y1: u16,
        x2: u16,
        y2: u16,
    ) -> Result<(), Self::Error> {
        let x1 = x1 as _;
        let x2 = x2 as _;
        let y1 = y1 as _;
        let y2 = y2 as _;

        let segment = Segment { x1, y1, x2, y2 };

        self.connection
            .poly_segment(window, self.graphics_context, &[segment])?;

        Ok(())
    }

    fn draw_rectangle(
        &mut self,
        drawing_context: Self::DrawingContext,
        rectangle: RectRegion,
    ) -> Result<(), Self::Error> {
        let rect = Rectangle {
            x: rectangle.x as i16,
            y: rectangle.y as i16,
            width: rectangle.width,
            height: rectangle.height,
        };

        self.connection
            .poly_fill_rectangle(drawing_context, self.graphics_context, &[rect])?;

        Ok(())
    }

    unsafe fn destroy_window(&mut self, window: Self::Window) {
        destroy_window(&self.connection, window).unwrap();
    }

    unsafe fn wait_for_events(&mut self, event_handler: &mut unsafe fn(WWindCoreEvent)) {
        let event = self.connection.wait_for_event();

        let event = if let Ok(event) = event {
            event
        } else {
            return;
        };

        match event {
            Event::Expose(expose) => {
                // XCB_EXPOSE
                let region = RectRegion {
                    x: expose.x,
                    y: expose.y,
                    width: expose.width,
                    height: expose.height,
                };

                event_handler(WWindCoreEvent::Expose(expose.window.into(), region));
            }
            Event::ClientMessage(event) => {
                // XCB_CLIENT_MESSAGE
                if event.type_ == self.atoms.WM_PROTOCOLS {
                    let protocol = event.data.as_data32()[0];

                    if protocol == 0 {
                        return;
                    }

                    if protocol == self.atoms.WM_DELETE_WINDOW {
                        event_handler(WWindCoreEvent::CloseWindow(event.window.into()));

                        return;
                    } else if protocol == self.atoms._NET_WM_PING {
                        let mut reply = event;

                        reply.window = self.screen.root;

                        println!("pong");
                        send_event(
                            &self.connection,
                            false,
                            self.screen.root,
                            EventMask::SUBSTRUCTURE_NOTIFY | EventMask::RESIZE_REDIRECT,
                            reply,
                        )
                        .unwrap();

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
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.connection.flush()?;
        Ok(())
    }

    fn set_draw_color(
        &mut self,
        context: Self::DrawingContext,
        color: Color,
    ) -> Result<(), Self::Error> {
        let color = self.get_color(color);

        let values = ChangeGCAux::new().foreground(color).background(color);
        self.connection.change_gc(self.graphics_context, &values)?;

        Ok(())
    }

    fn get_size(&self, window: Self::Window) -> (u16, u16) {
        let geometry = self.connection.get_geometry(window).unwrap();
        let geometry = geometry.reply().unwrap();

        (geometry.width, geometry.height)
    }

    unsafe fn get_context(&mut self, window: Self::Window) -> Self::DrawingContext {
        window
    }
}
