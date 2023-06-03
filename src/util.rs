use std::{marker::PhantomData, rc::Rc, ops::{Deref, DerefMut}, mem::MaybeUninit};
#[derive(Default)]
pub struct PhantomUnsend {
    _phantomdata: PhantomData<Rc<()>>,
}

/// A type that guarentees that `T` will be [forgotten](std::mem::forget) before `'a`.
pub struct ForgetGuard<'a, T> {
    inner: MaybeUninit<T>,
    _phantomdata: PhantomData<&'a T>,
}

impl<'a, T> ForgetGuard<'a, T> {
    pub fn new(inner: T) -> Self {
        Self {inner: MaybeUninit::new(inner), _phantomdata: Default::default()}
    }
}

impl<'a, T> Deref for ForgetGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {self.inner.assume_init_ref()}
    }
}

impl<'a, T> DerefMut for ForgetGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {self.inner.assume_init_mut()}
    }
}