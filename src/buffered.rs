// This is a copy of the `std::io::BufReader` with one additional
// method: `clear`. It resets the buffer to be empty (thereby losing any
// unread data).
use std::cmp;
use std::fmt;
use std::io::{self, BufRead};
use std::slice;

static DEFAULT_BUF_SIZE: usize = 1024 * 64;

/// Wraps a `Read` and buffers input from it
///
/// It can be excessively inefficient to work directly with a `Read` instance.
/// For example, every call to `read` on `TcpStream` results in a system call.
/// A `BufReader` performs large, infrequent reads on the underlying `Read`
/// and maintains an in-memory buffer of the results.
pub struct BufReader<R> {
    inner: R,
    buf: io::Cursor<Vec<u8>>,
}

impl<R: io::Read> BufReader<R> {
    /// Creates a new `BufReader` with a default buffer capacity
    pub fn new(inner: R) -> BufReader<R> {
        BufReader::with_capacity(DEFAULT_BUF_SIZE, inner)
    }

    /// Creates a new `BufReader` with the specified buffer capacity
    pub fn with_capacity(cap: usize, inner: R) -> BufReader<R> {
        BufReader {
            inner: inner,
            buf: io::Cursor::new(Vec::with_capacity(cap)),
        }
    }

    /// Gets a reference to the underlying reader.
    #[allow(dead_code)] pub fn get_ref(&self) -> &R { &self.inner }

    /// Gets a mutable reference to the underlying reader.
    ///
    /// # Warning
    ///
    /// It is inadvisable to directly read from the underlying reader.
    pub fn get_mut(&mut self) -> &mut R { &mut self.inner }

    /// Unwraps this `BufReader`, returning the underlying reader.
    ///
    /// Note that any leftover data in the internal buffer is lost.
    #[allow(dead_code)] pub fn into_inner(self) -> R { self.inner }

    pub fn clear(&mut self) {
        self.buf.set_position(0);
        self.buf.get_mut().truncate(0);
    }
}

impl<R: io::Read> io::Read for BufReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // If we don't have any buffered data and we're doing a massive read
        // (larger than our internal buffer), bypass our internal buffer
        // entirely.
        if self.buf.get_ref().len() == self.buf.position() as usize &&
            buf.len() >= self.buf.get_ref().capacity() {
            return self.inner.read(buf);
        }
        try!(self.fill_buf());
        self.buf.read(buf)
    }
}

impl<R: io::Read> io::BufRead for BufReader<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        // If we've reached the end of our internal buffer then we need to
        // fetch some more data from the underlying reader.
        if self.buf.position() as usize == self.buf.get_ref().len() {
            self.buf.set_position(0);
            let v = self.buf.get_mut();
            v.truncate(0);
            let inner = &mut self.inner;
            try!(with_end_to_cap(v, |b| inner.read(b)));
        }
        self.buf.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.buf.consume(amt)
    }
}

impl<R> fmt::Debug for BufReader<R> where R: fmt::Debug {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "BufReader {{ reader: {:?}, buffer: {}/{} }}",
               self.inner, self.buf.position(), self.buf.get_ref().len())
    }
}

// Acquires a slice of the vector `v` from its length to its capacity
// (uninitialized data), reads into it, and then updates the length.
//
// This function is leveraged to efficiently read some bytes into a destination
// vector without extra copying and taking advantage of the space that's
// already in `v`.
//
// The buffer we're passing down, however, is pointing at uninitialized data
// (the end of a `Vec`), and many operations will be *much* faster if we don't
// have to zero it out. In order to prevent LLVM from generating an `undef`
// value when reads happen from this uninitialized memory, we force LLVM to
// think it's initialized by sending it through a black box. This should
// prevent actual undefined behavior after optimizations.
fn with_end_to_cap<F>(v: &mut Vec<u8>, f: F) -> io::Result<usize>
        where F: FnOnce(&mut [u8]) -> io::Result<usize> {
    unsafe {
        let n = try!(f({
            let base = v.as_mut_ptr().offset(v.len() as isize);
            slice::from_raw_parts_mut(base, v.capacity() - v.len())
        }));

        // If the closure (typically a `read` implementation) reported that it
        // read a larger number of bytes than the vector actually has, we need
        // to be sure to clamp the vector to at most its capacity.
        let new_len = cmp::min(v.capacity(), v.len() + n);
        v.set_len(new_len);
        return Ok(n);
    }
}
