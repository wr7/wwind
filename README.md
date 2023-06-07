# wwind
wwind is a cross-platform UI library that is meant to provide a nice programming experience and significantly reduce runtime errors and undefined/unexpected behavior. It is meant to do all of this while still providing reasonable performance and platform consistency. 
## Support
wwind currently supports the Win32 API and X11, but I plan on adding support for Cocoa and Wayland.
## Example
```rust
use wwind::{WWindInstance, WWindState, Window, Color, RectRegion};

fn main() {
    let instance = WWindInstance::new(|state: &mut WWindState| {
        let mut window = state.add_window(100, 100, 500, 500, "test title");

        window.on_redraw(|_state, window, _region| {
            let (w, h) = window.get_size();
            let rect = RectRegion { x: 0, y: 0, width: w, height: h };

            let mut context = window.get_drawing_context();

            // Draw background //

            context.set_draw_color(Color::from_hex(0xffffff));
            context.draw_rectangle(rect);
            
            // Draw lines //

            context.set_draw_color(Color::from_rgb(0, 255, 0));
            context.draw_line(0, 0, w, h);
            
            context.set_draw_color(Color::from_rgb(255,0,0));

            context.draw_line(w, 0, 0, h);
        });
    });

    instance.unwrap().run();
}
```
## Code Layout
- Each platform has a "CoreState" that implements the `CoreStateImplementation` trait (`src/core/core_state_implementation.rs`). All enabled and supported states are enumerated by `CoreStateEnum` which also implements the `CoreStateImplementation` trait. 
- `CoreState` (planned to be removed), `CoreWindow`, and `CoreDrawingContext` provide unsafe abstractions over what they represent. 
- `WWindState`, `DrawingContext`, and `Window` provide the public abstractions for this library.
- Finally, `WWindInstance` represents an application/event loop.

*Note: wwind is heavily work in progress, so many features are missing, and the project organisation is far from optimal*
