// This is a copy of the `std::io::BufferedReader` with one additional
// method: `clear`. It resets the buffer to be empty (thereby losing any
// unread data).
use std::cmp;
use std::io::{Reader, Buffer, IoResult};
use std::slice;

static DEFAULT_BUF_SIZE: uint = 1024 * 64;

pub struct BufferedReader<R> {
    inner: R,
    buf: Vec<u8>,
    pos: uint,
    cap: uint,
}

impl<R: Reader> BufferedReader<R> {
    /// Creates a new `BufferedReader` with the specified buffer capacity
    pub fn with_capacity(cap: uint, inner: R) -> BufferedReader<R> {
        // It's *much* faster to create an uninitialized buffer than it is to
        // fill everything in with 0. This buffer is entirely an implementation
        // detail and is never exposed, so we're safe to not initialize
        // everything up-front. This allows creation of BufferedReader
        // instances to be very cheap (large mallocs are not nearly as
        // expensive as large callocs).
        let mut buf = Vec::with_capacity(cap);
        unsafe { buf.set_len(cap); }
        BufferedReader {
            inner: inner,
            buf: buf,
            pos: 0,
            cap: 0,
        }
    }

    pub fn new(inner: R) -> BufferedReader<R> {
        BufferedReader::with_capacity(DEFAULT_BUF_SIZE, inner)
    }

    pub fn get_mut(&mut self) -> &mut R { &mut self.inner }

    pub fn clear(&mut self) {
        let cap = self.buf.capacity();
        unsafe { self.buf.set_len(cap); }
        self.pos = 0;
        self.cap = 0;
    }
}

impl<R: Reader> Buffer for BufferedReader<R> {
    fn fill_buf<'a>(&'a mut self) -> IoResult<&'a [u8]> {
        if self.pos == self.cap {
            self.cap = try!(self.inner.read(self.buf[mut]));
            self.pos = 0;
        }
        Ok(self.buf[self.pos..self.cap])
    }

    fn consume(&mut self, amt: uint) {
        self.pos += amt;
        assert!(self.pos <= self.cap);
    }
}

impl<R: Reader> Reader for BufferedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<uint> {
        if self.pos == self.cap && buf.len() >= self.buf.capacity() {
            return self.inner.read(buf);
        }
        let nread = {
            let available = try!(self.fill_buf());
            let nread = cmp::min(available.len(), buf.len());
            slice::bytes::copy_memory(buf, available[..nread]);
            nread
        };
        self.pos += nread;
        Ok(nread)
    }
}
