# slice.

[crates.io](https://crates.io/crates/slice) · [docs.rs](https://docs.rs/slice/0.0.4/slice) · `slice = "0.0.4"`

create slices of io objects `std::io::Read` and `std::io::Write`.

if you have a file (or any other object), you can create a slice (or view) into some subset of it.

`IoSlice` will implement both `std::io::Read` and `std::io::Write` if the source implements them (or only one if the
source implements only one).

## example usage.

```rust
use { std::fs::File, slice::IoSlice };


let source = File::open("/home/annie/data.png")?;
let start  = 10;
let length = 1000;


// create a slice into `home/annie/data.png`, consisting of bytes [10 .. 10 + 1000]
// of that file.
//
// `slice` impls both `std::io::Read` and `std::io::Write` because `source`
// does too.
let slice = IoSlice::new(source, start, length);


// use like any other `std::io::Read` or `std::io::Write`:
//
//     slice.read_to_string(...)?;
//     slice.read_exact(...)?;
//     slice.write_all(...)?;
//
//     writeln!(slice, "hello {}", name)?;
//
```
