// Copyright 2012 The Rust Project Developers.
// Copyright 2015 Guillaume Gomez
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::marker::PhantomData;
use std::ops::{Index, IndexMut};
use std::slice;

/// Iterator over [`CSlice`].
///
/// You can get it from the [`CSlice::iter`] method.
pub struct CSliceIter<'a, T: 'a> {
    inner: &'a CSlice<'a, T>,
    pos: usize,
}

impl<'a, T> Iterator for CSliceIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.inner.len() {
            None
        } else {
            self.pos += 1;
            Some(unsafe { self.inner.get_unchecked(self.pos - 1) })
        }
    }
}

/// The type representing an 'unsafe' non-mutable foreign chunk of memory
pub struct CSlice<'a, T> {
    pub(crate) base: *const T,
    pub(crate) len: usize,
    pub(crate) _phantom: PhantomData<&'a ()>,
}

impl<'a, T> CSlice<'a, T> {
    /// Create a `CSlice` from a raw pointer to a buffer with a given length.
    ///
    /// Panics if the given pointer is null. The returned vector will not attempt
    /// to deallocate the vector when dropped.
    ///
    /// # Arguments
    ///
    /// * base - A raw pointer to a buffer
    /// * len - The number of elements in the buffer
    pub unsafe fn new(base: *const T, len: usize) -> CSlice<'a, T> {
        assert!(!base.is_null());
        CSlice {
            base,
            len,
            _phantom: PhantomData,
        }
    }

    /// Retrieves an element at a given index, returning `None` if the requested
    /// index is greater than the length of the vector.
    pub fn get(&'a self, ofs: usize) -> Option<&'a T> {
        if ofs < self.len {
            Some(unsafe { &*self.base.add(ofs) })
        } else {
            None
        }
    }

    /// Returns a reference to an element without doing any check.
    pub unsafe fn get_unchecked(&'a self, ofs: usize) -> &'a T {
        &*self.base.add(ofs)
    }

    /// Returns the number of items in this vector.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns whether this vector is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over CVec.
    pub fn iter(&'a self) -> CSliceIter<'a, T> {
        CSliceIter {
            inner: self,
            pos: 0,
        }
    }
}

impl<'a, T> AsRef<[T]> for CSlice<'a, T> {
    /// View the stored data as a slice.
    fn as_ref(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.base as *const T, self.len) }
    }
}

impl<'a, T> Index<usize> for CSlice<'a, T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        assert!(index < self.len);
        unsafe { &*self.base.add(index) }
    }
}

impl<'a, T: Clone> Into<Vec<T>> for CSlice<'a, T> {
    fn into(self: CSlice<'a, T>) -> Vec<T> {
        let mut v = Vec::with_capacity(self.len);
        v.extend_from_slice(self.as_ref());
        v
    }
}

/// Iterator over [`CSliceMut`].
///
/// You can get it from the [`CSliceMut::iter`] method.
pub struct CSliceMutIter<'a, T: 'a> {
    inner: &'a CSliceMut<'a, T>,
    pos: usize,
}

impl<'a, T> Iterator for CSliceMutIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.inner.len() {
            None
        } else {
            self.pos += 1;
            Some(unsafe { self.inner.get_unchecked(self.pos - 1) })
        }
    }
}

/// Mutable iterator over [`CSliceMut`].
///
/// You can get it from the [`CSliceMut::iter_mut`] method.
pub struct CSliceMutIterMut<'a, T> {
    inner: &'a mut CSliceMut<'a, T>,
    pos: usize,
}

impl<'a, T> Iterator for CSliceMutIterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.inner.len() {
            None
        } else {
            self.pos += 1;
            Some(unsafe { &mut *self.inner.base.add(self.pos - 1) })
        }
    }
}

/// The type representing an 'unsafe' mutable foreign chunk of memory
pub struct CSliceMut<'a, T> {
    pub(crate) base: *mut T,
    pub(crate) len: usize,
    pub(crate) _phantom: PhantomData<&'a ()>,
}

impl<'a, T> CSliceMut<'a, T> {
    /// Create a `CSlice` from a raw pointer to a buffer with a given length.
    ///
    /// Panics if the given pointer is null. The returned vector will not attempt
    /// to deallocate the vector when dropped.
    ///
    /// # Arguments
    ///
    /// * base - A raw pointer to a buffer
    /// * len - The number of elements in the buffer
    pub unsafe fn new(base: *mut T, len: usize) -> CSliceMut<'a, T> {
        assert!(!base.is_null());
        Self {
            base,
            len,
            _phantom: PhantomData,
        }
    }

    /// Retrieves an element at a given index, returning `None` if the requested
    /// index is greater than the length of the vector.
    pub fn get(&self, ofs: usize) -> Option<&T> {
        if ofs < self.len {
            Some(unsafe { &*self.base.add(ofs) })
        } else {
            None
        }
    }

    /// Returns a reference to an element without doing any check.
    pub unsafe fn get_unchecked(&self, ofs: usize) -> &T {
        &*self.base.add(ofs)
    }

    /// Retrieves a mutable element at a given index, returning `None` if the
    /// requested index is greater than the length of the vector.
    pub fn get_mut(&mut self, ofs: usize) -> Option<&mut T> {
        if ofs < self.len {
            Some(unsafe { &mut *self.base.add(ofs) })
        } else {
            None
        }
    }

    /// Returns a mutable reference to an element without doing any check.
    pub unsafe fn get_unchecked_mut(&mut self, ofs: usize) -> &mut T {
        &mut *self.base.add(ofs)
    }

    /// Returns the number of items in this vector.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns whether this vector is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over `CSliceMut`.
    pub fn iter(&'a self) -> CSliceMutIter<'a, T> {
        CSliceMutIter {
            inner: self,
            pos: 0,
        }
    }

    /// Returns a mutable iterator over `CSliceMut`.
    pub fn iter_mut(&'a mut self) -> CSliceMutIterMut<'a, T> {
        CSliceMutIterMut {
            inner: self,
            pos: 0,
        }
    }
}

impl<'a, T> AsRef<[T]> for CSliceMut<'a, T> {
    /// View the stored data as a slice.
    fn as_ref(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.base as *const T, self.len) }
    }
}

impl<'a, T> AsMut<[T]> for CSliceMut<'a, T> {
    /// View the stored data as a slice.
    fn as_mut(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.base, self.len) }
    }
}

impl<'a, T> Index<usize> for CSliceMut<'a, T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        assert!(index < self.len);
        unsafe { &*self.base.add(index) }
    }
}

impl<'a, T> IndexMut<usize> for CSliceMut<'a, T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        assert!(index < self.len);
        unsafe { &mut *self.base.add(index) }
    }
}

impl<'a, T: Clone> Into<Vec<T>> for CSliceMut<'a, T> {
    fn into(self: CSliceMut<'a, T>) -> Vec<T> {
        let mut v = Vec::with_capacity(self.len);
        v.extend_from_slice(self.as_ref());
        v
    }
}
