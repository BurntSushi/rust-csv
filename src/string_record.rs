use std::io;
use std::iter::FromIterator;
use std::ops::{self, Range};
use std::result;
use std::str;

use serde::Deserialize;

use deserializer::deserialize_string_record;
use error::{Error, FromUtf8Error, Result, new_from_utf8_error};
use reader::Reader;
use byte_record::{self, ByteRecord, ByteRecordIter, Position};

/// A safe function for reading CSV data into a `StringRecord`.
///
/// This relies on the internal representation of `StringRecord`.
#[inline(always)]
pub fn read<R: io::Read>(
    rdr: &mut Reader<R>,
    record: &mut StringRecord,
) -> Result<bool> {
    // TODO(burntsushi): Define this as a method using `pub(crate)` when that
    // stabilizes.

    // SAFETY: Note that despite the absence of `unsafe` in this function, this
    // code is critical to upholding the safety of other `unsafe` blocks in
    // this module. Namely, after calling `read_byte_record`, it is possible
    // for `record` to contain invalid UTF-8. We check for this in the
    // `validate` method, and if it does have invalid UTF-8, we clear the
    // record. (It is bad for `record` to contain invalid UTF-8 because other
    // accessor methods, like `get`, assume that every field is valid UTF-8.)
    let pos = rdr.position().clone();
    let read_res = rdr.read_byte_record(&mut record.0);
    let utf8_res = match byte_record::validate(&mut record.0) {
        Ok(()) => Ok(()),
        Err(err) => {
            // If this record isn't valid UTF-8, then completely wipe it.
            record.0.clear();
            Err(err)
        }
    };
    match (read_res, utf8_res) {
        (Err(err), _) => Err(err),
        (Ok(_), Err(err)) => Err(Error::Utf8 { pos: Some(pos), err: err }),
        (Ok(eof), Ok(())) => Ok(eof),
    }
}

/// A single CSV record stored as valid UTF-8 bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StringRecord(ByteRecord);

impl Default for StringRecord {
    #[inline]
    fn default() -> StringRecord {
        StringRecord::new()
    }
}

impl StringRecord {
    /// Create a new empty `StringRecord`.
    #[inline]
    pub fn new() -> StringRecord {
        StringRecord(ByteRecord::new())
    }

    /// Create a new empty `StringRecord` with the given capacity.
    ///
    /// `buffer` refers to the capacity of the buffer used to store the
    /// actual row contents. `fields` refers to the number of fields one
    /// might expect to store.
    #[inline]
    pub fn with_capacity(buffer: usize, fields: usize) -> StringRecord {
        StringRecord(ByteRecord::with_capacity(buffer, fields))
    }

    /// Create a new `StringRecord` from a `ByteRecord`.
    ///
    /// Note that this does UTF-8 validation. If the given `ByteRecord` does
    /// not contain valid UTF-8, then this returns an error. The error includes
    /// the UTF-8 error and the original `ByteRecord`.
    #[inline]
    pub fn from_byte_record(
        mut record: ByteRecord,
    ) -> result::Result<StringRecord, FromUtf8Error> {
        match byte_record::validate(&mut record) {
            Ok(()) => Ok(StringRecord(record)),
            Err(err) => Err(new_from_utf8_error(record, err)),
        }
    }

    /// Deserialize this record.
    ///
    /// The `D` type parameter refers to the type that this record should be
    /// deserialized into.
    ///
    /// An optional `headers` parameter permits deserializing into a struct
    /// based on its field names (corresponding to header values) rather than
    /// the order in which the fields are defined.
    pub fn deserialize<'de, D: Deserialize<'de>>(
        &self,
        headers: Option<&StringRecord>,
    ) -> Result<D> {
        deserialize_string_record(self, headers)
    }

    /// Returns an iterator over all fields in this record.
    #[inline]
    pub fn iter(&self) -> StringRecordIter {
        self.into_iter()
    }

    /// Return the field at index `i`.
    ///
    /// If no field at index `i` exists, then this returns `None`.
    #[inline]
    pub fn get(&self, i: usize) -> Option<&str> {
        self.0.get(i).map(|bytes| {
            // This is safe because we guarantee that all string records
            // have a valid UTF-8 buffer. It's also safe because we
            // individually check each field for valid UTF-8.
            unsafe { str::from_utf8_unchecked(bytes) }
        })
    }

    /// Returns true if and only if this record is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of fields in this record.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Clear this record so that it has zero fields.
    ///
    /// Note that it is not necessary to clear the record to reuse it with
    /// the CSV reader.
    #[inline]
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Return the position of this record, if available.
    #[inline]
    pub fn position(&self) -> Option<&Position> {
        self.0.position()
    }

    /// Set the position of this record.
    #[inline]
    pub fn set_position(&mut self, pos: Option<Position>) {
        self.0.set_position(pos);
    }

    /// Return the start and end position of a field in this record.
    ///
    /// If no such field exists at the given index, then return `None`.
    ///
    /// The range returned can be used with the slice returned by `as_slice`.
    #[inline]
    pub fn range(&self, i: usize) -> Option<Range<usize>> {
        self.0.range(i)
    }

    /// Return the entire row as a single string slice.
    #[inline]
    pub fn as_slice(&self) -> &str {
        // This is safe because we guarantee that each field is valid UTF-8.
        // If each field is valid UTF-8, then the entire buffer (up to the end
        // of the last field) must also be valid UTF-8.
        unsafe { str::from_utf8_unchecked(self.0.as_slice()) }
    }

    /// Return a reference to this record's raw `ByteRecord`.
    #[inline]
    pub fn as_byte_record(&self) -> &ByteRecord {
        &self.0
    }

    /// Convert this `StringRecord` into a `ByteRecord`.
    #[inline]
    pub fn into_byte_record(self) -> ByteRecord {
        self.0
    }

    /// Add a new field to this record.
    #[inline]
    pub fn push_field(&mut self, field: &str) {
        self.0.push_field(field.as_bytes());
    }
}

impl ops::Index<usize> for StringRecord {
    type Output = str;
    #[inline]
    fn index(&self, i: usize) -> &str { self.get(i).unwrap() }
}

impl<T: AsRef<str>> From<Vec<T>> for StringRecord {
    #[inline]
    fn from(xs: Vec<T>) -> StringRecord {
        StringRecord::from_iter(xs.into_iter())
    }
}

impl<'a, T: AsRef<str>> From<&'a [T]> for StringRecord {
    #[inline]
    fn from(xs: &'a [T]) -> StringRecord {
        StringRecord::from_iter(xs)
    }
}

impl<T: AsRef<str>> FromIterator<T> for StringRecord {
    #[inline]
    fn from_iter<I: IntoIterator<Item=T>>(iter: I) -> StringRecord {
        let mut record = StringRecord::new();
        record.extend(iter);
        record
    }
}

impl<T: AsRef<str>> Extend<T> for StringRecord {
    #[inline]
    fn extend<I: IntoIterator<Item=T>>(&mut self, iter: I) {
        for x in iter {
            self.push_field(x.as_ref());
        }
    }
}

impl<'a> IntoIterator for &'a StringRecord {
    type IntoIter = StringRecordIter<'a>;
    type Item = &'a str;

    #[inline]
    fn into_iter(self) -> StringRecordIter<'a> {
        StringRecordIter(self.0.iter())
    }
}

/// An iterator over the fields in a string record.
pub struct StringRecordIter<'a>(ByteRecordIter<'a>);

impl<'a> Iterator for StringRecordIter<'a> {
    type Item = &'a str;

    #[inline]
    fn next(&mut self) -> Option<&'a str> {
        self.0.next().map(|bytes| {
            // See StringRecord::get for safety argument.
            unsafe { str::from_utf8_unchecked(bytes) }
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }

    #[inline]
    fn count(self) -> usize {
        self.0.len()
    }
}

impl<'a> DoubleEndedIterator for StringRecordIter<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a str> {
        self.0.next_back().map(|bytes| {
            // See StringRecord::get for safety argument.
            unsafe { str::from_utf8_unchecked(bytes) }
        })
    }
}
