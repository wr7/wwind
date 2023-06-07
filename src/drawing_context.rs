use std::marker::PhantomData;

use crate::{
    core::{CoreStateImplementation, DrawingContextEnum},
    state::CoreStateData,
    util::PhantomUnsend,
    Color, RectRegion,
};

pub struct DrawingContext<'a> {
    pub(crate) context: DrawingContextEnum,
    pub(crate) data: *mut CoreStateData,
    pub(crate) _unsend: PhantomUnsend,
    pub(crate) _phantom_data: PhantomData<&'a ()>,
}

impl<'a> DrawingContext<'a> {
    pub fn draw_line(&mut self, x1: u16, y1: u16, x2: u16, y2: u16) {
        let context = self.context;

        self.get_data_mut()
            .core_state
            .draw_line(context, x1, y1, x2, y2)
            .unwrap()
    }

    pub fn draw_rectangle(&mut self, rectangle: RectRegion) {
        let context = self.context;

        self.get_data_mut()
            .core_state
            .draw_rectangle(context, rectangle)
            .unwrap()
    }

    pub fn set_draw_color(&mut self, color: Color) {
        let context = self.context;

        self.get_data_mut()
            .core_state
            .set_draw_color(context, color)
            .unwrap()
    }
}

impl<'a> DrawingContext<'a> {
    fn get_data_mut(&mut self) -> &mut CoreStateData {
        unsafe { &mut *self.data }
    }
}
