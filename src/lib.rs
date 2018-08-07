//! Create slices of IO objects - `std::io::Read` and `std::io::Write`.
//!
//! If you have a file (or any other object), you can create a slice (or view) into some subset of it.
//!
//! `IoSlice` impls both `std::io::Read` and `std::io::Write` when the source implements them (and only one if the source
//! only implements one).
//!
//! ## example usage.
//!
//! ```rust
//! use { std::fs::File, slice::IoSlice };
//!
//!
//! let source = File::open("/home/annie/data.png")?;
//! let start  = 10;
//! let length = 1000;
//!
//!
//! // create a slice into `home/annie/data.png`, consisting of bytes [10 .. 10 + 1000]
//! // of that file.
//! //
//! // `slice` impls both `std::io::Read` and `std::io::Write` because `source`
//! // does too.
//! let slice = IoSlice::new(source, start, length);
//!
//!
//! // use like any other `std::io::Read` or `std::io::Write`:
//! //
//! //     slice.read_to_string(...)?;
//! //     slice.read_exact(...)?;
//! //     slice.write_all(...)?;
//! //
//! //     writeln!(slice, "hello {}", name)?;
//! //
//! ```



use {
    std::fs::File,
    std::io::Read,
    std::io::Seek,
    std::io::SeekFrom,
    std::io::Write,
};



/// A slice, subset, or view into some object.
///
/// `IoSlice` impls both `std::io::Read` and `std::io::Write` when the source implements them (and only one if the source
/// only implements one).
///
/// ## example usage.
///
/// ```rust
/// use { std::fs::File, slice::IoSlice };
///
///
/// let source = File::open("/home/annie/data.png")?;
/// let start  = 10;
/// let length = 1000;
///
///
/// // create a slice into `home/annie/data.png`, consisting of bytes [10 .. 10 + 1000]
/// // of that file.
/// //
/// // `slice` impls both `std::io::Read` and `std::io::Write` because `source`
/// // does too.
/// let slice = IoSlice::new(source, start, length);
///
///
/// // use like any other `std::io::Read` or `std::io::Write`:
/// //
/// //     slice.read_to_string(...)?;
/// //     slice.read_exact(...)?;
/// //     slice.write_all(...)?;
/// //
/// //     writeln!(slice, "hello {}", name)?;
/// //
/// ```
#[derive(Debug)]
pub struct IoSlice<T> where T: Seek {
    // `IoSlice` supports slicing streams up to 9,000 PiB in size (`i64::max` bytes).
    //
    // the value of `begin`, `length`, `remaining`, `begin + length` will never be greater than `std::max::i64`. these
    // invariants are guarenteed by `IoSlice::new(...)`.

    underlying: T,
    begin:      u64,
    length:     u64,
    remaining:  u64,
}

impl<T> IoSlice<T> where T: Seek {
    /// create a new slice into a specific subset of `source`.
    pub fn new(mut source: T, begin: u64, length: u64) -> Result<IoSlice<T>, std::io::Error> {
        // :: check invariants
        let i64_max = std::i64::MAX as u64;

        if begin > i64_max || length > i64_max || begin + length > i64_max {
            return Err(std::io::ErrorKind::InvalidInput.into());
        }


        // :: attempt to seek to the requested position.
        //        if our request to seek to `begin` does not place us at `begin`, this stream is "invalid".
        let seek = SeekFrom::Start(begin);

        if source.seek(seek)? == begin {
            let underlying = source;
            let remaining  = length;

            Ok(IoSlice { underlying, begin, length, remaining })
        } else {
            Err(std::io::ErrorKind::InvalidInput.into())
        }
    }

    /// returns the total length of this io slice.
    pub fn len(&self) -> u64 {
        self.length
    }

    /// returns the current position of this slice.
    pub fn pos(&self) -> u64 {
        self.position()
    }

    /// returns the current position of this slice.
    pub fn position(&self) -> u64 {
        self.length - self.remaining
    }
}

impl<T> Seek for IoSlice<T> where T: Seek {
    fn seek(&mut self, position: SeekFrom) -> Result<u64, std::io::Error> {
        // :: make sure that position(i64) is not more `std::i64::max`.
        if match position { SeekFrom::Start(x) => x as u64, SeekFrom::Current(x) => x as u64, SeekFrom::End(x) => x as u64 } > std::i64::MAX as u64 {
            return Err(std::io::ErrorKind::InvalidInput.into());
        }


        // :: then calculate the new stream offset.
        let absolute = match position {
            SeekFrom::Start(value)   => self.begin + value as u64,
            SeekFrom::Current(value) => self.begin + self.length - self.remaining + value as u64,
            SeekFrom::End(value)     => self.begin + self.length + value as u64,
        };


        // :: seek.
        //
        // if the new requested position is in bounds. seek to it, and make sure that the new position is the one we
        // requested.
        if absolute >= self.begin && absolute <= self.begin + self.length {
            let seek = SeekFrom::Start(absolute);

            if self.underlying.seek(seek)? == absolute {
                let new = absolute - self.begin;

                self.remaining = self.length - new;
                return Ok(new);
            }

            return Err(std::io::ErrorKind::Other.into());
        }

        // the new requested position is out of bounds, return eof. we don't allow seeking out of bounds.
        Err(std::io::ErrorKind::UnexpectedEof.into())
    }
}

impl<T> Read for IoSlice<T> where T: Read + Seek {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, std::io::Error> {
        // `std::io::read::read()` can only read `usize::max` bytes at once.
        let remaining   = std::cmp::min(self.remaining, std::usize::MAX as u64) as usize;
        let request     = std::cmp::min(remaining, buffer.len());
        let actual      = self.underlying.read(&mut buffer[..request])?;

        self.remaining -= actual as u64;

        Ok(actual)
    }

    fn read_to_end(&mut self, buffer: &mut Vec<u8>) -> Result<usize, std::io::Error> {
        if self.remaining > std::usize::MAX as u64 {
            return Err(std::io::ErrorKind::InvalidInput.into())
        }

        let length    = buffer.len();
        let remaining = self.remaining as usize;

        buffer.reserve(remaining);

        unsafe {
            let pointer = buffer.as_mut_ptr().add(length);
            let slice   = std::slice::from_raw_parts_mut(pointer, remaining);

            self.underlying.read_exact(slice)?;
            buffer.set_len(length + remaining);

            self.remaining = 0;
        }

        Ok(remaining)
    }
}

impl<T> Write for IoSlice<T> where T: Write + Seek {
    fn write(&mut self, buffer: &[u8]) -> Result<usize, std::io::Error> {
        if buffer.len() as u64 > self.remaining {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }

        let actual = self.underlying.write(buffer)?;

        self.remaining -= actual as u64;

        Ok(actual)
    }

    fn write_all(&mut self, buffer: &[u8]) -> Result<(), std::io::Error> {
        if buffer.len() as u64 > self.remaining {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }

        self.underlying.write_all(buffer)
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        self.underlying.flush()
    }
}

impl<T> Clone for IoSlice<T> where T: Clone + Seek {
    fn clone(&self) -> IoSlice<T> {
        IoSlice {
            underlying: self.underlying.clone(),
            begin:      self.begin,
            length:     self.length,
            remaining:  self.remaining,
        }
    }
}

impl<T> TryClone for IoSlice<T> where T: TryClone + Seek {
    fn try_clone(&self) -> Result<IoSlice<T>, std::io::Error> {
        let clone = IoSlice {
            underlying: self.underlying.try_clone()?,
            begin:      self.begin,
            length:     self.length,
            remaining:  self.remaining,
        };

        Ok(clone)
    }
}



/// Object cloning that can potentially fail.
pub trait TryClone: Sized {
    fn try_clone(&self) -> Result<Self, std::io::Error>;
}

impl TryClone for File {
    fn try_clone(&self) -> Result<Self, std::io::Error> {
        self.try_clone()
    }
}
