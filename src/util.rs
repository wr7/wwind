use std::{marker::PhantomData, cell::Cell, rc::Rc};
#[derive(Default)]
pub struct PhantomUnsend {
    _phantomdata: PhantomData<Rc<()>>,
}