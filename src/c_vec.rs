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

use crate::{CSlice, CSliceMut};

/// Iterator over [`CVec`].
///
/// You can get it from the [`CVec::iter`] method.
///
/// # Example
///
/// ```
/// use c_vec::CVec;
///
/// let slice = &mut [0, 1, 2];
/// let ptr = slice.as_mut_ptr();
/// let cvec = unsafe { CVec::new(ptr, slice.len()) };
/// let iter = cvec.iter();
/// ```
pub struct CVecIter<'a, T: 'a> {
    inner: &'a CVec<T>,
    pos: usize,
}

impl<'a, T> Iterator for CVecIter<'a, T> {
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

/// Mutable iterator over [`CVec`].
///
/// You can get it from the [`CVec::iter_mut`] method.
///
/// # Example
///
/// ```
/// use c_vec::CVec;
///
/// let slice = &mut [0, 1, 2];
/// let ptr = slice.as_mut_ptr();
/// let mut cvec = unsafe { CVec::new(ptr, slice.len()) };
/// let iter = cvec.iter_mut();
/// ```
pub struct CVecIterMut<'a, T: 'a> {
    inner: &'a mut CVec<T>,
    pos: usize,
}

impl<'a, T> Iterator for CVecIterMut<'a, T> {
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

/// The type representing a foreign mutable chunk of memory.
///
/// # Example
///
/// ```
/// use c_vec::CVec;
///
/// let slice = &mut [0, 1, 2];
/// let ptr = slice.as_mut_ptr();
/// let cvec = unsafe { CVec::new(ptr, slice.len()) };
/// ```
pub struct CVec<T> {
    base: *mut T,
    len: usize,
    dtor: Option<Box<dyn FnOnce(*mut T)>>,
}

impl<T> Drop for CVec<T> {
    fn drop(&mut self) {
        if let Some(f) = self.dtor.take() {
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
    ///
    /// # Example
    ///
    /// ```
    /// use c_vec::CVec;
    ///
    /// let slice = &mut [0, 1, 2];
    /// let ptr = slice.as_mut_ptr();
    /// let cvec = unsafe { CVec::new(ptr, slice.len()) };
    /// ```
    pub unsafe fn new(base: *mut T, len: usize) -> CVec<T> {
        assert!(!base.is_null());
        CVec {
            base,
            len,
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
    ///
    /// # Example
    ///
    /// ```
    /// use c_vec::CVec;
    ///
    /// let slice = &mut [0, 1, 2];
    /// let ptr = slice.as_mut_ptr();
    /// let cvec = unsafe { CVec::new_with_dtor(ptr, slice.len(), |_| println!("free time!")) };
    /// ```
    pub unsafe fn new_with_dtor<F>(base: *mut T, len: usize, dtor: F) -> CVec<T>
    where
        F: FnOnce(*mut T) + 'static,
    {
        assert!(!base.is_null());
        let dtor = Box::new(dtor);
        CVec {
            base,
            len,
            dtor: Some(dtor),
        }
    }

    /// Retrieves an element at a given index, returning [`None`] if the requested
    /// index is greater than the length of the vector.
    ///
    /// # Example
    ///
    /// ```
    /// use c_vec::CVec;
    ///
    /// let slice = &mut [0, 1, 2];
    /// let ptr = slice.as_mut_ptr();
    /// let cvec = unsafe { CVec::new(ptr, slice.len()) };
    /// assert_eq!(cvec.get(1), slice.get(1));
    /// ```
    pub fn get(&self, ofs: usize) -> Option<&T> {
        if ofs < self.len {
            Some(unsafe { &*self.base.add(ofs) })
        } else {
            None
        }
    }

    /// Returns a reference to an element without doing any check.
    ///
    /// # Example
    ///
    /// ```
    /// use c_vec::CVec;
    ///
    /// let slice = &mut [0, 1, 2];
    /// let ptr = slice.as_mut_ptr();
    /// let cvec = unsafe { CVec::new(ptr, slice.len()) };
    /// unsafe {
    ///     assert_eq!(cvec.get_unchecked(1), slice.get_unchecked(1));
    /// }
    /// ```
    pub unsafe fn get_unchecked(&self, ofs: usize) -> &T {
        &*self.base.add(ofs)
    }

    /// Retrieves a mutable element at a given index, returning [`None`] if the
    /// requested index is greater than the length of the vector.
    ///
    /// # Example
    ///
    /// ```
    /// use c_vec::CVec;
    ///
    /// let slice = &mut [0, 1, 2];
    /// let ptr = slice.as_mut_ptr();
    /// let mut cvec = unsafe { CVec::new(ptr, slice.len()) };
    /// if let Some(el) = cvec.get_mut(1) {
    ///     *el += 10;
    /// }
    /// assert_eq!(cvec[1], 11);
    /// ```
    pub fn get_mut(&mut self, ofs: usize) -> Option<&mut T> {
        if ofs < self.len {
            Some(unsafe { &mut *self.base.add(ofs) })
        } else {
            None
        }
    }

    /// Returns a mutable reference to an element without doing any check.
    ///
    /// # Example
    ///
    /// ```
    /// use c_vec::CVec;
    ///
    /// let slice = &mut [0, 1, 2];
    /// let ptr = slice.as_mut_ptr();
    /// let mut cvec = unsafe { CVec::new(ptr, slice.len()) };
    /// unsafe { *cvec.get_unchecked_mut(1) += 10; }
    /// assert_eq!(cvec[1], 11);
    /// ```
    pub unsafe fn get_unchecked_mut<'a>(&'a mut self, ofs: usize) -> &mut T {
        &mut *self.base.add(ofs)
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
    /// value of [`CVec::get`]`(0)`.
    ///
    /// # Example
    ///
    /// ```
    /// use c_vec::CVec;
    ///
    /// let slice = &mut [0, 1, 2];
    /// let ptr = slice.as_mut_ptr();
    /// let cvec = unsafe { CVec::new(ptr, slice.len()) };
    /// assert_eq!(unsafe { cvec.into_inner() }, ptr);
    /// ```
    pub unsafe fn into_inner(mut self) -> *mut T {
        self.dtor = None;
        self.base
    }

    /// Returns the number of items in this vector.
    ///
    /// # Example
    ///
    /// ```
    /// use c_vec::CVec;
    ///
    /// let slice = &mut [0, 1, 2];
    /// let ptr = slice.as_mut_ptr();
    /// let cvec = unsafe { CVec::new(ptr, slice.len()) };
    /// assert_eq!(cvec.len(), slice.len());
    /// ```
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns whether this vector is empty.
    ///
    /// # Example
    ///
    /// ```
    /// use c_vec::CVec;
    ///
    /// let slice = &mut [0, 1, 2];
    /// let ptr = slice.as_mut_ptr();
    /// let cvec = unsafe { CVec::new(ptr, slice.len()) };
    /// assert_eq!(cvec.is_empty(), slice.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a [`CSlice`] which is a "view" over the data.
    ///
    /// # Example
    ///
    /// ```
    /// use c_vec::CVec;
    ///
    /// let slice = &mut [0, 1, 2];
    /// let ptr = slice.as_mut_ptr();
    /// let cvec = unsafe { CVec::new(ptr, slice.len()) };
    /// let cslice = cvec.as_cslice();
    /// ```
    pub fn as_cslice<'a>(&'a self) -> CSlice<'a, T> {
        CSlice {
            base: self.base,
            len: self.len,
            _phantom: PhantomData,
        }
    }

    /// Returns a [`CSliceMut`] which is a mutable "view" over the data.
    ///
    /// # Example
    ///
    /// ```
    /// use c_vec::CVec;
    ///
    /// let slice = &mut [0, 1, 2];
    /// let ptr = slice.as_mut_ptr();
    /// let mut cvec = unsafe { CVec::new(ptr, slice.len()) };
    /// let cslice = cvec.as_cslice_mut();
    /// ```
    pub fn as_cslice_mut<'a>(&'a mut self) -> CSliceMut<'a, T> {
        CSliceMut {
            base: self.base,
            len: self.len,
            _phantom: PhantomData,
        }
    }

    /// Returns an iterator over `CVec` data.
    ///
    /// # Example
    ///
    /// ```
    /// use c_vec::CVec;
    ///
    /// let slice = &mut [0, 1, 2];
    /// let ptr = slice.as_mut_ptr();
    /// let cvec = unsafe { CVec::new(ptr, slice.len()) };
    /// for elem in cvec.iter() {
    ///     println!("=> {}", elem);
    /// }
    /// ```
    pub fn iter<'a>(&'a self) -> CVecIter<'a, T> {
        CVecIter {
            inner: self,
            pos: 0,
        }
    }

    /// Returns a mutable iterator over `CVec` data.
    ///
    /// # Example
    ///
    /// ```
    /// use c_vec::CVec;
    ///
    /// let slice = &mut [0, 1, 2];
    /// let ptr = slice.as_mut_ptr();
    /// let mut cvec = unsafe { CVec::new(ptr, slice.len()) };
    /// for elem in cvec.iter_mut() {
    ///     *elem += 1;
    /// }
    /// assert_eq!(cvec[0], 1);
    /// ```
    pub fn iter_mut<'a>(&'a mut self) -> CVecIterMut<'a, T> {
        CVecIterMut {
            inner: self,
            pos: 0,
        }
    }
}

impl<T> AsRef<[T]> for CVec<T> {
    /// View the stored data as a slice.
    fn as_ref(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.base as *const T, self.len) }
    }
}

impl<T> AsMut<[T]> for CVec<T> {
    /// View the stored data as a slice.
    fn as_mut(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.base, self.len) }
    }
}

impl<T> Index<usize> for CVec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        assert!(index < self.len);
        unsafe { &*self.base.add(index) }
    }
}

impl<T> IndexMut<usize> for CVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        assert!(index < self.len);
        unsafe { &mut *self.base.add(index) }
    }
}

impl<T: Clone> Into<Vec<T>> for CVec<T> {
    fn into(self: CVec<T>) -> Vec<T> {
        self.as_cslice().into()
    }
}
