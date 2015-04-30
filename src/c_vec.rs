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
//! unwrapping method. The constructors are unsafe for the
//! obvious reason (they act on a pointer that cannot be checked inside the
//! method), but `into_inner()` is somewhat more subtle in its unsafety.
//! It returns the contained pointer, but at the same time destroys the CVec
//! without running its destructor. This can be used to pass memory back to
//! C, but care must be taken that the ownership of underlying resources are
//! handled correctly, i.e. that allocated memory is eventually freed
//! if necessary.

use std::ptr;
use std::slice;
use std::ops::{Index, IndexMut};

/// The type representing a foreign chunk of memory
pub struct CVec<T> {
    base: *mut T,
    len: usize,
    dtor: Option<Box<FnMut(*mut T)>>
}

impl<T> Drop for CVec<T> {
    fn drop(&mut self) {
        if let Some(mut f) = self.dtor.take() {
            f(self.base);
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
    /// * base - A unique pointer to a buffer
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
    /// * base - A unique pointer to a buffer
    /// * len - The number of elements in the buffer
    /// * dtor - A fn to run when the value is destructed, useful
    ///          for freeing the buffer, etc. `base` will be passed
    ///          to it as an argument.
    pub unsafe fn new_with_dtor<F>(base: *mut T,
                                   len: usize,
                                   dtor: F)
                                   -> CVec<T>
        where F: FnMut(*mut T) + 'static
    {
        assert!(base != ptr::null_mut());
        let dtor = Box::new(dtor);
        CVec {
            base: base,
            len: len,
            dtor: Some(dtor)
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

    /// Returns the number of items in this vector.
    pub fn len(&self) -> usize { self.len }

    /// Returns whether this vector is empty.
    pub fn is_empty(&self) -> bool { self.len() == 0 }

    /// Convert to CSlice
    pub fn as_cslice(&self) -> CSlice<T> {
        CSlice {
            base: self.base,
            len: self.len
        }
    }
}

impl<T> AsRef<[T]> for CVec<T> {
    /// View the stored data as a slice.
    fn as_ref(&self) -> &[T] {
        unsafe {
            slice::from_raw_parts(self.base as *const T, self.len)
        }
    }
}

impl<T> AsMut<[T]> for CVec<T> {
    /// View the stored data as a slice.
    fn as_mut(&mut self) -> &mut [T] {
        unsafe {
            slice::from_raw_parts_mut(self.base, self.len)
        }
    }
}

/// The type representing an 'unsafe' foreign chunk of memory
pub struct CSlice<T> {
    base: *mut T,
    len: usize
}

impl<T> CSlice<T> {
    /// Create a `CSlice` from a raw pointer to a buffer with a given length.
    ///
    /// Panics if the given pointer is null. The returned vector will not attempt
    /// to deallocate the vector when dropped.
    ///
    /// # Arguments
    ///
    /// * base - A raw pointer to a buffer
    /// * len - The number of elements in the buffer
    pub unsafe fn new(base: *mut T, len: usize) -> CSlice<T> {
        assert!(base != ptr::null_mut());
        CSlice {
            base: base,
            len: len
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

    /// Returns the number of items in this vector.
    pub fn len(&self) -> usize { self.len }

    /// Returns whether this vector is empty.
    pub fn is_empty(&self) -> bool { self.len() == 0 }
}

impl<T> AsRef<[T]> for CSlice<T> {
    /// View the stored data as a slice.
    fn as_ref(&self) -> &[T] {
        unsafe {
            slice::from_raw_parts(self.base as *const T, self.len)
        }
    }
}

impl<T> AsMut<[T]> for CSlice<T> {
    /// View the stored data as a slice.
    fn as_mut(&mut self) -> &mut [T] {
        unsafe {
            slice::from_raw_parts_mut(self.base, self.len)
        }
    }
}

impl<T> Index<usize> for CSlice<T> {
    type Output = T;

    fn index<'a>(&'a self, _index: usize) -> &'a T {
        assert!(_index < self.len);
        unsafe { &*self.base.offset(_index as isize) }
    }
}

impl<T> IndexMut<usize> for CSlice<T> {
    fn index_mut<'a>(&'a mut self, _index: usize) -> &'a mut T {
        assert!(_index < self.len);
        unsafe { &mut *self.base.offset(_index as isize) }
    }
}

#[cfg(test)]
mod tests {
    extern crate libc;

    use super::{CVec, CSlice};
    use std::ptr;

    // allocation of CVec
    fn v_malloc(n: usize) -> CVec<u8> {
        unsafe {
            let mem = libc::malloc(n as libc::size_t) as *mut u8;
            CVec::new_with_dtor(mem, n, |mem| { libc::free((mem) as *mut _); })
        }
    }

    // allocation of CSlice
    fn s_malloc(n: usize) -> CSlice<u8> {
        unsafe {
            let mem: *mut u8 = libc::malloc(n as libc::size_t) as *mut _;
            CSlice::new(mem, n)
        }
    }

    #[test]
    fn vec_test_basic() {
        let mut cv = v_malloc(16);

        *cv.get_mut(3).unwrap() = 8;
        *cv.get_mut(4).unwrap() = 9;
        assert_eq!(*cv.get(3).unwrap(), 8);
        assert_eq!(*cv.get(4).unwrap(), 9);
        assert_eq!(cv.len(), 16);
    }

    #[test]
    fn slice_test_basic() {
        let mut cs = s_malloc(16);

        cs[3] = 8;
        cs[4] = 9;
        assert_eq!(cs[3], 8);
        assert_eq!(cs[4], 9);
        assert_eq!(cs.len(), 16);
    }

    #[test]
    #[should_panic]
    fn vec_test_panic_at_null() {
        unsafe {
            CVec::new(ptr::null_mut::<u8>(), 9);
        }
    }

    #[test]
    #[should_panic]
    fn slice_test_panic_at_null() {
        unsafe {
            CSlice::new(ptr::null_mut::<u8>(), 9);
        }
    }

    #[test]
    fn vec_test_overrun_get() {
        let cv = v_malloc(16);

        assert!(cv.get(17).is_none());
    }

    #[test]
    #[should_panic]
    fn slice_test_overrun_get() {
        let cs = s_malloc(16);

        assert!(cs[17] == 18);
    }

    #[test]
    fn vec_test_overrun_set() {
        let mut cv = v_malloc(16);

        assert!(cv.get_mut(17).is_none());
    }

    #[test]
    fn vec_test_unwrap() {
        unsafe {
            let cv = CVec::new_with_dtor(1 as *mut isize,
                                         0,
                                         |_| panic!("Don't run this destructor!"));
            let p = cv.into_inner();
            assert_eq!(p, 1 as *mut isize);
        }
    }

    #[test]
    fn vec_to_slice_test() {
        let mut cv = v_malloc(2);

        *cv.get_mut(0).unwrap() = 10;
        *cv.get_mut(1).unwrap() = 12;
        let cs = cv.as_cslice();

        assert_eq!(cs[0], 10);
        assert_eq!(cs[1], 12);
    }

    #[test]
    fn slice_to_vec_test() {
        let cv = v_malloc(2);
        let mut cs = cv.as_cslice();

        cs[0] = 13;
        cs[1] = 26;
        assert_eq!(*cv.get(0).unwrap(), 13);
        assert_eq!(*cv.get(1).unwrap(), 26);
    }
}
