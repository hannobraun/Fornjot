//! FFI-safe versions of common `std` types.

use std::{
    alloc::{GlobalAlloc, Layout, System},
    fmt::{self, Debug, Display, Formatter},
    ops::Deref,
    ptr::NonNull,
};

/// A FFI-safe version of `Vec<T>`.
#[repr(C)]
pub struct FfiSafeVec<T> {
    ptr: NonNull<T>,
    len: usize,
}

impl<T: Debug> Debug for FfiSafeVec<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", &**self)
    }
}

impl<T: PartialEq> PartialEq for FfiSafeVec<T> {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl<T> From<Vec<T>> for FfiSafeVec<T> {
    fn from(mut items: Vec<T>) -> Self {
        // Safety: To avoid accidental double-frees and other memory issues, we
        // need to go through a specific dance.
        unsafe {
            // first, get a pointer to the first element and its length
            let first_item = items.as_mut_ptr();
            let len = items.len();

            // next, tell Vec to forget about these items so it won't try to
            // run their destructors if we return early (e.g. via a panic).
            // We've now taken over ownership of the items, but *not* the Vec's
            // backing array.
            items.set_len(0);

            // Use the system allocator to create some space for our
            // FfiSafeVec's buffer.
            let layout = Layout::array::<T>(len).unwrap();
            let ptr: *mut T = System::default().alloc(layout).cast();
            let ptr = NonNull::new(ptr).expect("Allocation failed");

            // Now, we can copy the items across
            std::ptr::copy_nonoverlapping(first_item, ptr.as_ptr(), len);

            // the items are gone, time to free the original vec
            drop(items);

            FfiSafeVec { ptr, len }
        }
    }
}

impl<T: Copy> From<FfiSafeVec<T>> for Box<[T]> {
    fn from(v: FfiSafeVec<T>) -> Self {
        Box::from(&*v)
    }
}

impl<T> FromIterator<T> for FfiSafeVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let vec: Vec<T> = iter.into_iter().collect();
        vec.into()
    }
}

impl<T> Deref for FfiSafeVec<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        // Safety: We control "ptr" and "len", so we know they are always
        // initialized and within bounds.
        unsafe {
            let FfiSafeVec { ptr, len } = *self;
            std::slice::from_raw_parts(ptr.as_ptr(), len)
        }
    }
}

impl<T> Drop for FfiSafeVec<T> {
    fn drop(&mut self) {
        let FfiSafeVec { ptr, len } = *self;
        let ptr = ptr.as_ptr();

        for i in 0..self.len {
            // Safety: We control the "len" field, so the item we're accessing
            // is always within bounds. We also don't touch values after their
            // destructors are called.
            unsafe {
                let item = ptr.add(i);
                std::ptr::drop_in_place(item);
            }
        }

        // Safety: This vec is immutable, so we're using the same layout as the
        // original allocation. It's also not possible to touch the allocation
        // after Drop completes.
        unsafe {
            let layout = Layout::array::<T>(len).unwrap();
            System::default().dealloc(ptr.cast(), layout);
        }
    }
}

// Safety: We're Send+Sync as long as the underlying type is.
unsafe impl<T: Send> Send for FfiSafeVec<T> {}
unsafe impl<T: Sync> Sync for FfiSafeVec<T> {}

/// A FFI-safe version of `Box<str>`.
#[repr(transparent)]
#[derive(Debug, PartialEq)]
pub struct FfiSafeString(FfiSafeVec<u8>);

impl From<String> for FfiSafeString {
    fn from(s: String) -> Self {
        FfiSafeString(s.into_bytes().into())
    }
}

impl From<FfiSafeString> for Box<str> {
    fn from(s: FfiSafeString) -> Self {
        Box::from(&*s)
    }
}

impl Display for FfiSafeString {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&**self, f)
    }
}

impl Deref for FfiSafeString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        // Safety: The only way to create a FfiSafeString is from a valid Rust
        // string, so we can skip the UTF-8 checks.
        unsafe { std::str::from_utf8_unchecked(&*self.0) }
    }
}

/// A version of `Result` that is `#[repr(C)]`.
#[repr(C)]
pub enum FfiSafeResult<T, E> {
    Ok(T),
    Err(E),
}

impl<T, E> From<Result<T, E>> for FfiSafeResult<T, E> {
    fn from(result: Result<T, E>) -> Self {
        match result {
            Ok(ok) => FfiSafeResult::Ok(ok),
            Err(err) => FfiSafeResult::Err(err),
        }
    }
}

impl<T, E> From<FfiSafeResult<T, E>> for Result<T, E> {
    fn from(result: FfiSafeResult<T, E>) -> Self {
        match result {
            FfiSafeResult::Ok(ok) => Result::Ok(ok),
            FfiSafeResult::Err(err) => Result::Err(err),
        }
    }
}

#[repr(C)]
pub(crate) struct Slice<T> {
    ptr: NonNull<T>,
    len: usize,
}

impl<T> Slice<T> {
    /// Create a new [`Slice`] from a slice.
    ///
    /// # Safety
    ///
    /// It is the caller's responsibility to make sure this [`Slice`] doesn't
    /// outlive the slice that was passed in.
    pub unsafe fn from_slice(items: &[T]) -> Self {
        let ptr = items.as_ptr();
        let len = items.len();
        Slice {
            // Safety: It's okay to cast away the const because you can't mutate
            // a slice.
            ptr: NonNull::new(ptr as *mut T).unwrap(),
            len,
        }
    }

    pub unsafe fn into_slice<'a>(self) -> &'a [T] {
        let Slice { ptr, len } = self;
        std::slice::from_raw_parts(ptr.as_ptr(), len)
    }
}

impl<T: Debug> Debug for Slice<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&**self, f)
    }
}

impl<T: PartialEq> PartialEq for Slice<T> {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl<T> Deref for Slice<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        // Safety: We control both "ptr" and "len", so the array is always
        // initialized and within bounds.
        //
        // The lifetime of the &[T] is also bound to the lifetime of &self, so
        // this should be safe as long as people can never get a Slice<T> that
        // outlives the data it points to.
        unsafe {
            let Slice { ptr, len, .. } = *self;
            std::slice::from_raw_parts(ptr.as_ptr(), len)
        }
    }
}

#[repr(transparent)]
pub(crate) struct StringSlice(Slice<u8>);

impl StringSlice {
    /// Create a new [`StringSlice`].
    ///
    /// # Safety
    ///
    /// It is the caller's responsibility to make sure this [`Slice`] doesn't
    /// outlive the slice that was passed in.
    pub unsafe fn from_str(s: &str) -> StringSlice {
        StringSlice(Slice::from_slice(s.as_bytes()))
    }

    pub unsafe fn into_str<'a>(self) -> &'a str {
        let bytes = self.0.into_slice();
        std::str::from_utf8_unchecked(bytes)
    }
}

impl Deref for StringSlice {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        // Safety: the only way you can construct a StringSlice is via a string.
        unsafe { std::str::from_utf8_unchecked(&*self.0) }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct BoxedError {
    msg: FfiSafeString,
}

impl Display for BoxedError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.msg, f)
    }
}

impl std::error::Error for BoxedError {}

impl From<Box<dyn std::error::Error + Send + Sync>> for BoxedError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        // TODO: is it worth capturing the message from each source error, too?
        BoxedError {
            msg: err.to_string().into(),
        }
    }
}
