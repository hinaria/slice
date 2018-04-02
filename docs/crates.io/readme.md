# slice.

create slices of io objects implementing `std::io::Read` and `std::io::Write`.

## example usage.

```rust
use { std::fs::File, slice::IoSlice };


let source = File::open("/home/annie/data.png")?;
let start  = 10;
let length = 1000;


// create a subset of `home/annie/data.png`, consisting of 1000 bytes beginning
// from the 10th byte.
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