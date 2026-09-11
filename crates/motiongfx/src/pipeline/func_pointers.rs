use crate::pipeline::bake::BakeClipCtx;
use crate::pipeline::sample::SampleCtx;

/// A type-erased per-clip bake function pointer.
#[derive(Debug, Clone, Copy)]
pub struct BakeClipFnPtr(*const ());

unsafe impl Send for BakeClipFnPtr {}
unsafe impl Sync for BakeClipFnPtr {}

impl BakeClipFnPtr {
    pub const fn new<W>(f: BakeClipFn<W>) -> Self {
        Self(f as *const ())
    }

    /// # Safety
    ///
    /// `W` must match the type used when constructing this pointer.
    pub const unsafe fn typed_unchecked<W>(&self) -> BakeClipFn<W> {
        unsafe {
            core::mem::transmute::<*const (), BakeClipFn<W>>(self.0)
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

pub type BakeClipFn<W> = fn(BakeClipCtx<'_, W>);
pub type SampleFn<W> = fn(SampleCtx<'_, W>);
