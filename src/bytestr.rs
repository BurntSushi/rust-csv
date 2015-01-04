use std::borrow::{BorrowFrom, Cow, IntoCow, ToOwned};
use std::fmt;
use std::hash;
use std::iter::FromIterator;
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

impl IntoVector<u8> for ByteString {
    fn into_vec(self) -> Vec<u8> { self.into_bytes() }
}

impl<'a> IntoVector<u8> for &'a str {
    fn into_vec(self) -> Vec<u8> { self.to_owned().into_bytes() }
}

impl<'a> IntoVector<u8> for String {
    fn into_vec(self) -> Vec<u8> { self.into_bytes() }
}

/// A trait that permits borrowing byte vectors.
///
/// This is useful for providing an API that can abstract over Unicode
/// strings and byte strings.
pub trait BorrowBytes for Sized? {
    /// Borrow a byte vector.
    fn borrow_bytes<'a>(&'a self) -> &'a [u8];
}

impl BorrowBytes for String {
    fn borrow_bytes(&self) -> &[u8] { self.as_slice().as_bytes() }
}

impl BorrowBytes for str {
    fn borrow_bytes(&self) -> &[u8] { self.as_bytes() }
}

impl BorrowBytes for Vec<u8> {
    fn borrow_bytes(&self) -> &[u8] { self.as_slice() }
}

impl BorrowBytes for ByteString {
    fn borrow_bytes(&self) -> &[u8] { self.as_slice() }
}

impl BorrowBytes for [u8] {
    fn borrow_bytes(&self) -> &[u8] { self }
}

impl<'a, T, Sized? B> BorrowBytes for Cow<'a, T, B>
        where T: BorrowBytes, B: BorrowBytes + ToOwned<T> {
    fn borrow_bytes(&self) -> &[u8] {
        match *self {
            Cow::Borrowed(v) => v.borrow_bytes(),
            Cow::Owned(ref v) => v.borrow_bytes(),
        }
    }
}

impl<'a, Sized? T: BorrowBytes> BorrowBytes for &'a T {
    fn borrow_bytes(&self) -> &[u8] { (*self).borrow_bytes() }
}

/// Encapsulate allocating of strings.
///
/// This is a temporary measure until the standard library provides more
/// impls for `std::string::IntoString`.
pub trait StrAllocating {
    /// Produce a new owned String.
    fn into_str(self) -> String;
}

impl StrAllocating for String {
    fn into_str(self) -> String { self }
}

impl<'a> StrAllocating for &'a str {
    fn into_str(self) -> String { self.to_owned() }
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
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ByteString(Vec<u8>);

impl ByteString {
    /// Create a new byte string from a vector or slice of bytes.
    pub fn from_bytes<S: IntoVector<u8>>(bs: S) -> ByteString {
        ByteString(bs.into_vec())
    }

    /// Consumes this byte string into a vector of bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }

    /// Returns this byte string as a slice of bytes.
    pub fn as_bytes<'a>(&'a self) -> &'a [u8] {
        &**self
    }

    /// Consumes the byte string and decodes it into a Unicode string. If the
    /// decoding fails, then the original ByteString is returned.
    pub fn into_utf8_string(self) -> Result<String, ByteString> {
        // FIXME: Figure out how to return an error here.
        String::from_utf8(self.into_bytes())
               .map_err(|err| ByteString(err.into_bytes()))
    }

    /// Return the number of bytes in the string.
    pub fn len(&self) -> uint {
        self.as_bytes().len()
    }

    /// Returns whether the byte string is empty or not.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
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
        self.as_bytes()
    }
}

impl ops::Deref for ByteString {
    type Target = [u8];
    fn deref<'a>(&'a self) -> &'a [u8] {
        &*self.0
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
        // WHOA. This used to be `(&*self).hash(hasher);`, but it introduced
        // a *major* performance regression that got fixed by using
        // `self.as_slice().hash(hasher);` instead. I didn't do any profiling,
        // but maybe the `(&*self)` was causing a copy somehow through the
        // `Deref` trait? No clue. ---AG
        //
        // TODO: Try `(&*self)` again (maybe when 1.0 hits). If the regression
        // remains, create a smaller reproducible example and report it as a
        // bug.
        self.as_slice().hash(hasher);
    }
}

impl<S: Str> PartialEq<S> for ByteString {
    fn eq(&self, other: &S) -> bool {
        self.as_bytes() == other.as_slice().as_bytes()
    }
}

impl FromIterator<u8> for ByteString {
    fn from_iter<I: Iterator<u8>>(it: I) -> ByteString {
        ByteString::from_bytes(it.collect::<Vec<_>>())
    }
}

impl BorrowFrom<ByteString> for [u8] {
    fn borrow_from(owned: &ByteString) -> &[u8] { owned.0.as_slice() }
}

impl ToOwned<ByteString> for [u8] {
    fn to_owned(&self) -> ByteString { ByteString(self.to_vec()) }
}

impl<'a> IntoCow<'a, ByteString, [u8]> for ByteString {
    fn into_cow(self) -> Cow<'a, ByteString, [u8]> {
        Cow::Owned(self)
    }
}
