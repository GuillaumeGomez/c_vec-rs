c_vec [![Build Status](https://api.travis-ci.org/GuillaumeGomez/c_vec-rs.png?branch=master)](https://travis-ci.org/GuillaumeGomez/c_vec-rs)
=====

Structures to wrap C arrays. Here's a little example:

```Rust
extern crate libc;
extern crate c_vec;

use c_vec::{CVec, CSlice};
use std::ptr::Unique;

fn some_func(cvec: *mut libc::c_int, len: uint) {
    // safe wrapper, you can pass a destructor with new_with_dtor() method
    let v = CVec::new(Unique::new(cvec), len);
    // unsafe wrapper with no destructor
    let s = CSlice::new(cvec, len);

    println!("cvec:   converted from c array: {}", v.as_ref());
    println!("cslice: converted from c array: {}", s.as_mut());
}
```

Usage
=====

You can use it directly by adding this line to your `Cargo.toml` file:

```Rust
[dependencies]
c_vec = "^1.0.0"
```

Here's the [crates.io](https://crates.io/crates/c_vec) page for `c_vec`.

License
=======

This project is under the MIT and Apache 2.0 licenses. Please take a look at the license files for more information.
