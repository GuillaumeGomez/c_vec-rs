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

#[cfg(test)]
#[macro_use]
extern crate doc_comment;

#[cfg(test)]
doctest!("../README.md");

mod c_slice;
mod c_vec;

pub use c_slice::*;
pub use c_vec::*;

#[cfg(test)]
mod tests {
    extern crate libc;

    use super::{CSlice, CVec};
    use std::ptr;

    // allocation of CVec
    fn v_malloc(n: usize) -> CVec<u8> {
        unsafe {
            let mem = libc::malloc(n as _) as *mut u8;
            CVec::new_with_dtor(mem, n, |mem| {
                libc::free((mem) as *mut _);
            })
        }
    }

    // allocation of CSlice
    macro_rules! s_malloc {
        ($n:expr) => {{
            unsafe {
                let mem: *mut u8 = libc::malloc($n as _) as *mut _;
                CSlice::new(mem, $n)
            }
        }};
    }

    #[test]
    fn vec_test_basic() {
        let mut cv = v_malloc(16);

        *cv.get_mut(3).unwrap() = 8;
        *cv.get_mut(4).unwrap() = 9;
        assert_eq!(*cv.get(3).unwrap(), 8);
        assert_eq!(*cv.get(4).unwrap(), 9);
        assert_eq!(*cv.get(3).unwrap(), cv[3]);
        assert_eq!(*cv.get(4).unwrap(), cv[4]);
        assert_eq!(cv.len(), 16);
    }

    #[test]
    fn slice_test_basic() {
        let mut cs = v_malloc(16);

        cs[3] = 8;
        cs[4] = 9;
        assert_eq!(cs[3], 8);
        assert_eq!(cs[4], 9);
        assert_eq!(cs.len(), 16);

        let cs = cs.as_cslice();
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
        let cs = s_malloc!(16);

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
            let cv =
                CVec::new_with_dtor(1 as *mut isize, 0, |_| panic!("Don't run this destructor!"));
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
        let mut cv = v_malloc(2);
        {
            let mut cs = cv.as_cslice_mut();

            cs[0] = 13;
            cs[1] = 26;
        }
        assert_eq!(*cv.get(0).unwrap(), 13);
        assert_eq!(*cv.get(1).unwrap(), 26);
    }

    #[test]
    fn convert_test() {
        let mut cv = v_malloc(2);
        {
            let mut cs = cv.as_cslice_mut();
            cs[0] = 1;
            cs[1] = 99;
            let v: Vec<_> = cs.into();
            assert_eq!(1, v[0]);
            assert_eq!(99, v[1]);
        }
        assert_eq!(1, cv[0]);
        assert_eq!(99, cv[1]);
    }

    #[test]
    fn iter_cvec() {
        let mut cv = v_malloc(2);

        {
            let mut cs = cv.as_cslice_mut();

            cs[0] = 13;
            cs[1] = 26;
        }

        let mut iter = cv.iter();
        assert_eq!(iter.next(), Some(&13));
        assert_eq!(iter.next(), Some(&26));
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn iter_cslice() {
        let mut cs = v_malloc(2);

        cs[0] = 13;
        cs[1] = 26;

        let slice = cs.as_cslice();
        let mut iter = slice.iter();
        assert_eq!(iter.next(), Some(&13));
        assert_eq!(iter.next(), Some(&26));
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None);
    }
}
