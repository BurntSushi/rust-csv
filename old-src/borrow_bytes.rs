use std::borrow::{Cow, ToOwned};
use ByteString;

/// A trait that permits borrowing byte vectors.
///
/// This is useful for providing an API that can abstract over Unicode
/// strings and byte strings.
pub trait BorrowBytes {
    /// Borrow a byte vector.
    fn borrow_bytes<'a>(&'a self) -> &'a [u8];
}

impl BorrowBytes for String {
    fn borrow_bytes(&self) -> &[u8] { self.as_bytes() }
}

impl BorrowBytes for str {
    fn borrow_bytes(&self) -> &[u8] { self.as_bytes() }
}

impl BorrowBytes for ByteString {
    fn borrow_bytes(&self) -> &[u8] { &**self }
}

impl BorrowBytes for [u8] {
    fn borrow_bytes(&self) -> &[u8] { self }
}

impl<'a, B: ?Sized> BorrowBytes for Cow<'a, B>
        where B: BorrowBytes + ToOwned, <B as ToOwned>::Owned: BorrowBytes {
    fn borrow_bytes(&self) -> &[u8] {
        match *self {
            Cow::Borrowed(v) => v.borrow_bytes(),
            Cow::Owned(ref v) => v.borrow_bytes(),
        }
    }
}

impl<'a, T: ?Sized + BorrowBytes> BorrowBytes for &'a T {
    fn borrow_bytes(&self) -> &[u8] { (*self).borrow_bytes() }
}
