c_vec [![Build Status](https://api.travis-ci.org/GuillaumeGomez/c_vec-rs.png?branch=master)](https://travis-ci.org/GuillaumeGomez/c_vec-rs)
=====

Old rust c_vec struct. It works just like the old one:

```Rust
extern crate libc;
extern crate c_vec;

use c_vec::CVec;

fn some_func(cvec: *mut libc::c_int, len: uint) {
    let v = CVec::new(cvec, len);

    println!("converted from c array: {}", v.as_slice());
}
```

Usage
=====

You can use it directly by adding this line to your `Cargo.toml` file:

```Rust
[dependencies]
c_vec = "^1.0.0"
```

Here's is the [crates.io](https://crates.io/crates/c_vec) page for `c_vec`.

License
=======

This project is under the MIT and Apache 2.0 licenses. Please look at the license files for more information.
