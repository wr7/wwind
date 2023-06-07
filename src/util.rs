use std::{marker::PhantomData, rc::Rc};
#[derive(Default, Clone, Copy)]
pub struct PhantomUnsend {
    _phantomdata: PhantomData<Rc<()>>,
}
