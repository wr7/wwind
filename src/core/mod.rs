use crate::{state::CoreStateData, Color, RectRegion, WWindState};

mod core_state_implementation;

pub(crate) use self::core_state_implementation::CoreWindowRef;
pub(crate) use core_state_implementation::CoreStateEnum;
pub use core_state_implementation::CoreStateImplementation;

#[cfg(windows)]
mod win32;
#[cfg(x11)]
mod x11rb;

pub use core_state_implementation::DrawingContextEnum;
pub use core_state_implementation::WWindCoreEvent;
