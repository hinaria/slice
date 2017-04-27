use std::io::Error;
use std::io::ErrorKind;
use std::io::Read;
use std::io::Result;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;


pub struct IoSlice<T> where T: Seek {
    // note: the value of `begin`, `length`, `remaining`, and `begin + length` will never be greater than the value of `std::i64::max`
    // note: `length` and `remaining` will never be greater than `std::usize::max`

    underlying: T,
    begin:      u64,
    length:     u64,
    remaining:  u64,
}

impl<T> IoSlice<T> where T: Seek {
    pub fn new(mut source: T, begin: u64, length: u64) -> Result<IoSlice<T>> {
        // :: check invariants
        let u64_max   = std::i64::MAX as u64;
        let usize_max = std::usize::MAX as u64;

        if begin > u64_max || length > usize_max || begin + length > u64_max {
            return Err(Error::from(ErrorKind::InvalidInput));
        }


        // :: attempt to seek to the requested position.
        let seek = SeekFrom::Start(begin);

        match source.seek(seek) {
            Ok(position) => {
                if position == begin {
                    Ok(())
                } else {
                    // if the source does not seek us to the `begin` position, then this stream is "invalid".
                    Err(Error::from(ErrorKind::InvalidInput))
                }
            },
            Err(error) => {
                Err(error)
            },
        }?;


        // :: seek ok, now return the struct
        let slice = IoSlice {
            underlying: source,
            begin:      begin,
            length:     length,
            remaining:  length,
        };

        Ok(slice)
    }

    pub fn len(&self) -> u64 {
        self.length
    }

    pub fn position(&self) -> u64 {
        self.length - self.remaining
    }
}

impl<T> Seek for IoSlice<T> where T: Seek {
    fn seek(&mut self, position: SeekFrom) -> Result<u64> {
        // :: make sure that position(i64) is not more `std::i64::max`.
        if match position { SeekFrom::Start(x) => x as u64, SeekFrom::Current(x) => x as u64, SeekFrom::End(x) => x as u64 } > std::i64::MAX as u64 {
            return Err(Error::from(ErrorKind::InvalidInput));
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
            if self.underlying.seek(SeekFrom::Start(absolute))? == absolute {
                let new = absolute - self.begin;

                self.remaining = self.length - new;
                return Ok(new);
            }

            return Err(Error::from(ErrorKind::Other));
        }

        // the new requested position is out of bounds, return eof. we don't allow seeking out of bounds.
        Err(Error::from(ErrorKind::UnexpectedEof))
    }
}

impl<T> Read for IoSlice<T> where T: Read + Seek {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        // `std::io::read::read()` can only read `usize::max` bytes at once.
        let remaining   = std::cmp::min(self.remaining, std::usize::MAX as u64) as usize;
        let request     = std::cmp::min(remaining, buffer.len());
        let actual      = self.underlying.read(&mut buffer[..request])?;
        self.remaining -= actual as u64;

        Ok(actual)
    }

    fn read_to_end(&mut self, buffer: &mut Vec<u8>) -> Result<usize> {
        let length    = buffer.len();
        let remaining = self.remaining as usize;

        buffer.reserve(remaining);

        unsafe {
            let ptr   = buffer.as_mut_ptr().offset(length as isize);
            let slice = std::slice::from_raw_parts_mut(ptr, remaining);

            self.underlying.read_exact(slice)?;

            buffer.set_len(length + remaining);
            self.remaining = 0;
        }

        Ok(remaining)
    }
}

impl<T> Write for IoSlice<T> where T: Write + Seek {
    fn write(&mut self, buffer: &[u8]) -> Result<usize> {
        if buffer.len() as u64 > self.remaining {
            return Err(Error::from(ErrorKind::UnexpectedEof));
        }

        let actual = self.underlying.write(buffer)?;
        self.remaining -= actual as u64;

        Ok(actual)
    }

    fn flush(&mut self) -> Result<()> {
        self.underlying.flush()
    }
}
