use super::{BakeCtx, SampleCtx};

/// A type-erased bake function pointer.
#[derive(Debug, Clone, Copy)]
pub struct BakeFnPtr(*const ());

unsafe impl Send for BakeFnPtr {}
unsafe impl Sync for BakeFnPtr {}

impl BakeFnPtr {
    pub const fn new<W>(f: BakeFn<W>) -> Self {
        Self(f as *const ())
    }

    /// # Safety
    ///
    /// `W` must match the type used when constructing this pointer.
    pub const unsafe fn typed_unchecked<W>(&self) -> BakeFn<W> {
        unsafe {
            core::mem::transmute::<*const (), BakeFn<W>>(self.0)
        }
    }
}

/// A type-erased sample function pointer.
#[derive(Debug, Clone, Copy)]
pub struct SampleFnPtr(*const ());

unsafe impl Send for SampleFnPtr {}
unsafe impl Sync for SampleFnPtr {}

impl SampleFnPtr {
    pub const fn new<W>(f: SampleFn<W>) -> Self {
        Self(f as *const ())
    }

    /// # Safety
    ///
    /// `W` must match the type used when constructing this pointer.
    pub const unsafe fn typed_unchecked<W>(&self) -> SampleFn<W> {
        unsafe {
            core::mem::transmute::<*const (), SampleFn<W>>(self.0)
        }
    }
}

pub type BakeFn<W> = fn(BakeCtx<'_, W>);
pub type SampleFn<W> = fn(SampleCtx<'_, W>);
