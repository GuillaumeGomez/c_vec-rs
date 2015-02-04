// Copyright 2012 The Rust Project Developers.
// Copyright 2015 Guillaume Gomez
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Library to interface with chunks of memory allocated in C.
//!
//! It is often desirable to safely interface with memory allocated from C,
//! encapsulating the unsafety into allocation and destruction time.  Indeed,
//! allocating memory externally is currently the only way to give Rust shared
//! mut state with C programs that keep their own references; vectors are
//! unsuitable because they could be reallocated or moved at any time, and
//! importing C memory into a vector takes a one-time snapshot of the memory.
//!
//! This module simplifies the usage of such external blocks of memory.  Memory
//! is encapsulated into an opaque object after creation; the lifecycle of the
//! memory can be optionally managed by Rust, if an appropriate destructor
//! closure is provided.  Safety is ensured by bounds-checking accesses, which
//! are marshalled through get and set functions.
//!
//! There are three unsafe functions: the two constructors, and the
//! unwrap method. The constructors are unsafe for the
//! obvious reason (they act on a pointer that cannot be checked inside the
//! method), but `unwrap()` is somewhat more subtle in its unsafety.
//! It returns the contained pointer, but at the same time destroys the CVec
//! without running its destructor. This can be used to pass memory back to
//! C, but care must be taken that the ownership of underlying resources are
//! handled correctly, i.e. that allocated memory is eventually freed
//! if necessary.

#![feature(unsafe_destructor, core, std_misc)]

use std::mem;
use std::ops::{Drop, FnOnce};
use std::option::Option;
use std::option::Option::{Some, None};
use std::ptr::PtrExt;
use std::ptr;
use std::raw;
use std::slice::AsSlice;
use std::thunk::{Thunk};

/// The type representing a foreign chunk of memory
pub struct CVec<T> {
    base: *mut T,
    len: usize,
    dtor: Option<Thunk>,
}

#[unsafe_destructor]
impl<T> Drop for CVec<T> {
    fn drop(&mut self) {
        match self.dtor.take() {
            None => (),
            Some(f) => f.invoke(())
        }
    }
}

impl<T> CVec<T> {
    /// Create a `CVec` from a raw pointer to a buffer with a given length.
    ///
    /// Panics if the given pointer is null. The returned vector will not attempt
    /// to deallocate the vector when dropped.
    ///
    /// # Arguments
    ///
    /// * base - A raw pointer to a buffer
    /// * len - The number of elements in the buffer
    pub unsafe fn new(base: *mut T, len: usize) -> CVec<T> {
        assert!(base != ptr::null_mut());
        CVec {
            base: base,
            len: len,
            dtor: None,
        }
    }

    /// Create a `CVec` from a foreign buffer, with a given length,
    /// and a function to run upon destruction.
    ///
    /// Panics if the given pointer is null.
    ///
    /// # Arguments
    ///
    /// * base - A foreign pointer to a buffer
    /// * len - The number of elements in the buffer
    /// * dtor - A fn to run when the value is destructed, useful
    ///          for freeing the buffer, etc.
    pub unsafe fn new_with_dtor<F>(base: *mut T,
                                   len: usize,
                                   dtor: F)
                                   -> CVec<T>
        where F : FnOnce(), F : Send
    {
        assert!(base != ptr::null_mut());
        let dtor: Thunk = Thunk::new(dtor);
        CVec {
            base: base,
            len: len,
            dtor: Some(dtor)
        }
    }

    /// View the stored data as a mutable slice.
    pub fn as_mut_slice<'a>(&'a mut self) -> &'a mut [T] {
        unsafe {
            mem::transmute(raw::Slice { data: self.base as *const T, len: self.len })
        }
    }

    /// Retrieves an element at a given index, returning `None` if the requested
    /// index is greater than the length of the vector.
    pub fn get<'a>(&'a self, ofs: usize) -> Option<&'a T> {
        if ofs < self.len {
            Some(unsafe { &*self.base.offset(ofs as isize) })
        } else {
            None
        }
    }

    /// Retrieves a mutable element at a given index, returning `None` if the
    /// requested index is greater than the length of the vector.
    pub fn get_mut<'a>(&'a mut self, ofs: usize) -> Option<&'a mut T> {
        if ofs < self.len {
            Some(unsafe { &mut *self.base.offset(ofs as isize) })
        } else {
            None
        }
    }

    /// Unwrap the pointer without running the destructor
    ///
    /// This method retrieves the underlying pointer, and in the process
    /// destroys the CVec but without running the destructor. A use case
    /// would be transferring ownership of the buffer to a C function, as
    /// in this case you would not want to run the destructor.
    ///
    /// Note that if you want to access the underlying pointer without
    /// cancelling the destructor, you can simply call `transmute` on the return
    /// value of `get(0)`.
    pub unsafe fn into_inner(mut self) -> *mut T {
        self.dtor = None;
        self.base
    }

    /// Deprecated, use into_inner() instead
    #[deprecated = "renamed to into_inner()"]
    pub unsafe fn unwrap(self) -> *mut T { self.into_inner() }

    /// Returns the number of items in this vector.
    pub fn len(&self) -> usize { self.len }

    /// Returns whether this vector is empty.
    pub fn is_empty(&self) -> bool { self.len() == 0 }
}

impl<T> AsSlice<T> for CVec<T> {
    /// View the stored data as a slice.
    fn as_slice<'a>(&'a self) -> &'a [T] {
        unsafe {
            mem::transmute(raw::Slice { data: self.base as *const T, len: self.len })
        }
    }
}

#[cfg(test)]
mod tests {
    use prelude::v1::*;

    use super::CVec;
    use libc;
    use ptr;

    fn malloc(n: uint) -> CVec<u8> {
        unsafe {
            let mem = ptr::Unique(libc::malloc(n as libc::size_t));
            if mem.0.is_null() { ::alloc::oom() }

            CVec::new_with_dtor(mem.0 as *mut u8,
                                n,
                                move|| { libc::free(mem.0 as *mut libc::c_void); })
        }
    }

    #[test]
    fn test_basic() {
        let mut cv = malloc(16);

        *cv.get_mut(3).unwrap() = 8;
        *cv.get_mut(4).unwrap() = 9;
        assert_eq!(*cv.get(3).unwrap(), 8);
        assert_eq!(*cv.get(4).unwrap(), 9);
        assert_eq!(cv.len(), 16);
    }

    #[test]
    #[should_fail]
    fn test_panic_at_null() {
        unsafe {
            CVec::new(ptr::null_mut::<u8>(), 9);
        }
    }

    #[test]
    fn test_overrun_get() {
        let cv = malloc(16);

        assert!(cv.get(17).is_none());
    }

    #[test]
    fn test_overrun_set() {
        let mut cv = malloc(16);

        assert!(cv.get_mut(17).is_none());
    }

    #[test]
    fn test_unwrap() {
        unsafe {
            let cv = CVec::new_with_dtor(1 as *mut int,
                                         0,
                                         move|:| panic!("Don't run this destructor!"));
            let p = cv.into_inner();
            assert_eq!(p, 1 as *mut int);
        }
    }

}