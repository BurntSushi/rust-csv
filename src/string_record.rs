use std::io;
use std::ops;
use std::result;
use std::str;

use error::{Error, FromUtf8Error, Result, new_from_utf8_error};
use reader::Reader;
use byte_record::{self, ByteRecord, ByteRecordIter};

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
    // this module. Namely, after calling `read_record_bytes`, it is possible
    // for `record` to contain invalid UTF-8. We check for this in the
    // `validate` method, and if it does have invalid UTF-8, we clear the
    // record.
    let pos = rdr.position().clone();
    let read_res = rdr.read_record_bytes(&mut record.0);
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
        (Ok(_), Err(err)) => Err(Error::Utf8 { pos: pos, err: err }),
        (Ok(eof), Ok(())) => Ok(eof),
    }
}

/// A single CSV record stored as valid UTF-8 bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StringRecord(ByteRecord);

impl Default for StringRecord {
    fn default() -> StringRecord {
        StringRecord::new()
    }
}

impl StringRecord {
    /// Create a new empty `StringRecord`.
    pub fn new() -> StringRecord {
        StringRecord(ByteRecord::new())
    }

    /// Create a new empty `StringRecord` with the given capacity.
    pub fn with_capacity(capacity: usize) -> StringRecord {
        StringRecord(ByteRecord::with_capacity(capacity))
    }

    /// Create a new `StringRecord` from a `ByteRecord`.
    ///
    /// Note that this does UTF-8 validation. If the given `ByteRecord` does
    /// not contain valid UTF-8, then this returns an error. The error includes
    /// the UTF-8 error and the original `ByteRecord`.
    pub fn from_byte_record(
        mut record: ByteRecord,
    ) -> result::Result<StringRecord, FromUtf8Error> {
        match byte_record::validate(&mut record) {
            Ok(()) => Ok(StringRecord(record)),
            Err(err) => Err(new_from_utf8_error(record, err)),
        }
    }

    /// Return the field at index `i`.
    ///
    /// If no field at index `i` exists, then this returns `None`.
    pub fn get(&self, i: usize) -> Option<&str> {
        self.0.get(i).map(|bytes| {
            // This is safe because we guarantee that all string records
            // have a valid UTF-8 buffer. It's also safe because we
            // individually check each field for valid UTF-8.
            unsafe { str::from_utf8_unchecked(bytes) }
        })
    }

    /// Returns true if and only if this record is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of fields in this record.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Clear this record so that it has zero fields.
    ///
    /// Note that it is not necessary to clear the record to reuse it with
    /// the CSV reader.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Convert this `StringRecord` into `ByteRecord`.
    pub fn into_byte_record(self) -> ByteRecord {
        self.0
    }

    /// Returns an iterator over all fields in this record.
    pub fn iter(&self) -> StringRecordIter {
        StringRecordIter(self.0.iter())
    }
}

impl ops::Index<usize> for StringRecord {
    type Output = str;
    fn index(&self, i: usize) -> &str { self.get(i).unwrap() }
}

impl<'a> IntoIterator for &'a StringRecord {
    type IntoIter = StringRecordIter<'a>;
    type Item = &'a str;
    fn into_iter(self) -> StringRecordIter<'a> {
        self.iter()
    }
}

/// An iterator over the fields in a string record.
pub struct StringRecordIter<'a>(ByteRecordIter<'a>);

impl<'a> Iterator for StringRecordIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        self.0.next().map(|bytes| {
            // See StringRecord::get for safety argument.
            unsafe { str::from_utf8_unchecked(bytes) }
        })
    }
}
