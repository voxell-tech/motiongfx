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

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyWorld;

    fn dummy_bake(_ctx: BakeCtx<'_, DummyWorld>) {}
    fn dummy_sample(_ctx: SampleCtx<'_, DummyWorld>) {}

    // ── BakeFnPtr ─────────────────────────────────────────────────────────────

    #[test]
    fn bake_fn_ptr_new_does_not_panic() {
        let _ptr = BakeFnPtr::new::<DummyWorld>(dummy_bake);
    }

    #[test]
    fn bake_fn_ptr_typed_unchecked_round_trips() {
        // Constructing a BakeFnPtr and recovering the same function pointer.
        let ptr = BakeFnPtr::new::<DummyWorld>(dummy_bake);
        // SAFETY: We use the same world type that was passed at construction.
        let recovered: BakeFn<DummyWorld> =
            unsafe { ptr.typed_unchecked::<DummyWorld>() };
        // Verify the recovered function is the same as the original by
        // comparing function pointers cast to integers.
        assert_eq!(
            recovered as usize,
            dummy_bake as usize,
            "Recovered function pointer should match original"
        );
    }

    #[test]
    fn bake_fn_ptr_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<BakeFnPtr>();
    }

    #[test]
    fn bake_fn_ptr_clone_and_copy_are_consistent() {
        let ptr = BakeFnPtr::new::<DummyWorld>(dummy_bake);
        let cloned = ptr.clone();
        let copied = ptr;
        // Both copies point to the same raw address.
        assert_eq!(ptr.0 as usize, cloned.0 as usize);
        assert_eq!(ptr.0 as usize, copied.0 as usize);
    }

    // ── SampleFnPtr ───────────────────────────────────────────────────────────

    #[test]
    fn sample_fn_ptr_new_does_not_panic() {
        let _ptr = SampleFnPtr::new::<DummyWorld>(dummy_sample);
    }

    #[test]
    fn sample_fn_ptr_typed_unchecked_round_trips() {
        let ptr = SampleFnPtr::new::<DummyWorld>(dummy_sample);
        // SAFETY: Same world type used at construction.
        let recovered: SampleFn<DummyWorld> =
            unsafe { ptr.typed_unchecked::<DummyWorld>() };
        assert_eq!(
            recovered as usize,
            dummy_sample as usize,
            "Recovered function pointer should match original"
        );
    }

    #[test]
    fn sample_fn_ptr_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SampleFnPtr>();
    }

    #[test]
    fn sample_fn_ptr_clone_and_copy_are_consistent() {
        let ptr = SampleFnPtr::new::<DummyWorld>(dummy_sample);
        let cloned = ptr.clone();
        let copied = ptr;
        assert_eq!(ptr.0 as usize, cloned.0 as usize);
        assert_eq!(ptr.0 as usize, copied.0 as usize);
    }

    // ── Cross-type distinction ─────────────────────────────────────────────

    #[test]
    fn bake_and_sample_fn_ptrs_are_independent() {
        // Two distinct functions should produce two distinct pointers.
        let bake_ptr = BakeFnPtr::new::<DummyWorld>(dummy_bake);
        let sample_ptr = SampleFnPtr::new::<DummyWorld>(dummy_sample);
        // The raw pointer values should be different because the functions differ.
        assert_ne!(
            bake_ptr.0 as usize,
            sample_ptr.0 as usize,
            "Different functions should have different raw pointer addresses"
        );
    }
}
