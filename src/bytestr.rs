use std::fmt;
use std::hash;
use std::ops;

/// A trait that encapsulates a `Vec<T>` or a `&[T]`.
pub trait IntoVector<T> {
    /// Convert the underlying value to a vector.
    fn into_vec(self) -> Vec<T>;
}

impl<T> IntoVector<T> for Vec<T> {
    fn into_vec(self) -> Vec<T> { self }
}

impl<'a, T: Clone> IntoVector<T> for &'a [T] {
    fn into_vec(self) -> Vec<T> { self.to_vec() }
}

/// A type that represents unadulterated byte strings.
///
/// Byte strings represent *any* 8 bit character encoding. There are no
/// restrictions placed on the type of encoding used. (This means that there
/// may be *multiple* encodings in any particular byte string!)
///
/// Many CSV files in the wild aren't just malformed with respect to RFC 4180,
/// but they are commonly *not* UTF-8 encoded. Even worse, some of them are
/// encoded improperly. Therefore, any useful CSV parser must be flexible with
/// respect to encodings.
///
/// Thus, this CSV parser uses byte strings internally. This means that
/// quotes and field and record separators *must* be ASCII. Otherwise,
/// the parser places no other restrictions on the content of data in each
/// cell.
///
/// Note that most of the methods in the encoder/decoder will assume UTF-8
/// encoding, but they also expose some lower level methods that use byte
/// strings when absolutely necessary. This type is exposed in case you need
/// to deal with the raw bytes directly.
#[deriving(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ByteString(Vec<u8>);

impl ByteString {
    /// Create a new byte string from a vector or slice of bytes.
    pub fn from_bytes<S: IntoVector<u8>>(bs: S) -> ByteString {
        ByteString(bs.into_vec())
    }

    /// Consumes this byte string into a vector of bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        let ByteString(chars) = self;
        chars
    }

    /// Returns this byte string as a slice of bytes.
    pub fn as_bytes<'a>(&'a self) -> &'a [u8] {
        let &ByteString(ref chars) = self;
        chars[]
    }

    /// Consumes the byte string and decodes it into a Unicode string. If the
    /// decoding fails, then the original ByteString is returned.
    pub fn as_utf8_string(self) -> Result<String, ByteString> {
        String::from_utf8(self.into_bytes()).map_err(ByteString)
    }
}

impl fmt::Show for ByteString {
    /// Writes the underlying bytes as a `&[u8]`.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // XXX: Ideally, we could just do this:
        //
        //    f.write(chars[])
        //
        // and let the output device figure out how to render it. But it seems
        // the formatting infrastructure assumes that the data is UTF-8
        // encodable, which obviously doesn't work with raw byte strings.
        //
        // For now, we just show the bytes, e.g., `[255, 50, 48, 49, ...]`.
        write!(f, "{}", self[])
    }
}

impl AsSlice<u8> for ByteString {
    #[inline]
    fn as_slice<'a>(&'a self) -> &'a [u8] {
        let ByteString(ref chars) = *self;
        chars.as_slice()
    }
}

impl ops::Slice<uint, [u8]> for ByteString {
    #[inline]
    fn as_slice_<'a>(&'a self) -> &'a [u8] {
        self.as_slice()
    }

    #[inline]
    fn slice_from_or_fail<'a>(&'a self, start: &uint) -> &'a [u8] {
        self.as_slice().slice_from_or_fail(start)
    }

    #[inline]
    fn slice_to_or_fail<'a>(&'a self, end: &uint) -> &'a [u8] {
        self.as_slice().slice_to_or_fail(end)
    }

    #[inline]
    fn slice_or_fail<'a>(&'a self, start: &uint, end: &uint) -> &'a [u8] {
        self.as_slice().slice_or_fail(start, end)
    }
}

impl<H: hash::Writer> hash::Hash<H> for ByteString {
    fn hash(&self, hasher: &mut H) {
        self[].hash(hasher);
    }
}

impl<S: Str> Equiv<S> for ByteString {
    fn equiv(&self, other: &S) -> bool {
        self.as_bytes() == other.as_slice().as_bytes()
    }
}

impl Collection for ByteString {
    fn len(&self) -> uint { self.as_bytes().len() }
}
