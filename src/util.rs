use std::{marker::PhantomData, cell::Cell, rc::Rc, any::Any};
#[derive(Default)]
pub struct PhantomUnsend {
    _phantomdata: PhantomData<Rc<()>>,
}
