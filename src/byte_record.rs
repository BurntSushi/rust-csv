use std::cmp;
use std::io;
use std::mem;
use std::ops;
use std::result;
use std::ptr;
use std::str;

use error::{
    Result, Error, FromUtf8Error, Utf8Error,
    new_from_utf8_error, new_utf8_error,
};
use reader::Reader;

/// Retrieve the underlying parts of a byte record.
pub fn as_parts(
    record: &mut ByteRecord,
) -> (&mut Vec<u8>, &mut Vec<usize>) {
    // TODO(burntsushi): Use `pub(crate)` when it stabilizes.
    (&mut record.fields, &mut record.bounds.ends)
}

/// Set the number of fields in the given record record.
pub fn set_len(record: &mut ByteRecord, len: usize) {
    // TODO(burntsushi): Use `pub(crate)` when it stabilizes.
    record.bounds.len = len;
}

/// Expand the capacity for storing fields.
pub fn expand_fields(record: &mut ByteRecord) {
    // TODO(burntsushi): Use `pub(crate)` when it stabilizes.
    let new_len = record.fields.len().checked_mul(2).unwrap();
    record.fields.resize(cmp::max(4, new_len), 0);
}

/// Expand the capacity for storing field ending positions.
pub fn expand_ends(record: &mut ByteRecord) {
    // TODO(burntsushi): Use `pub(crate)` when it stabilizes.
    record.bounds.expand();
}

/// Validate the given record as UTF-8.
///
/// If it's not UTF-8, return an error.
///
/// This never modifies the contents of this record.
pub fn validate(record: &ByteRecord) -> result::Result<(), Utf8Error> {
    // TODO(burntsushi): Use `pub(crate)` when it stabilizes.

    // If the entire buffer is ASCII, then we have nothing to fear.
    if record.fields[..record.bounds.end()].iter().all(|&b| b <= 0x7F) {
        return Ok(());
    }
    // Otherwise, we must check each field individually to ensure that
    // it's valid UTF-8.
    for (i, field) in record.iter().enumerate() {
        if let Err(err) = str::from_utf8(field) {
            return Err(new_utf8_error(i, err.valid_up_to()));
        }
    }
    Ok(())
}

/// A single CSV record stored as raw bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ByteRecord {
    /// All fields in this record, stored contiguously.
    fields: Vec<u8>,
    /// The number of and location of each field in this record.
    bounds: Bounds,
}

impl Default for ByteRecord {
    fn default() -> ByteRecord {
        ByteRecord::new()
    }
}

impl ByteRecord {
    /// Create a new empty `ByteRecord`.
    pub fn new() -> ByteRecord {
        ByteRecord::with_capacity(0)
    }

    /// Create a new empty `ByteRecord` with the given capacity.
    pub fn with_capacity(capacity: usize) -> ByteRecord {
        ByteRecord { fields: vec![0; capacity], bounds: Bounds::default() }
    }

    /// Return the field at index `i`.
    ///
    /// If no field at index `i` exists, then this returns `None`.
    pub fn get(&self, i: usize) -> Option<&[u8]> {
        self.bounds.get(i).map(|range| &self.fields[range])
    }

    /// Returns true if and only if this record is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of fields in this record.
    pub fn len(&self) -> usize {
        self.bounds.len
    }

    /// Clear this record so that it has zero fields.
    ///
    /// Note that it is not necessary to clear the record to reuse it with
    /// the CSV reader.
    pub fn clear(&mut self) {
        self.bounds.len = 0;
    }

    /// Returns an iterator over all fields in this record.
    pub fn iter(&self) -> ByteRecordIter {
        ByteRecordIter { r: self, start: 0, i: 0 }
    }

    /// Add a new field.
    fn add(&mut self, field: &[u8]) {
        let (s, e) = (self.bounds.end(), self.bounds.end() + field.len());
        while s + e > self.fields.len() {
            expand_fields(self);
        }
        self.fields[s..e].copy_from_slice(field);
        self.add_end(e);
    }

    /// Add a new end.
    fn add_end(&mut self, pos: usize) {
        assert!(pos <= self.fields.len());
        self.bounds.add(pos);
    }
}

/// The bounds of fields in a single record.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Bounds {
    /// The ending index of each field. Guaranteed to fall on UTF-8 boundaries.
    ends: Vec<usize>,
    /// The number of fields in this record.
    ///
    /// Technically, we could drop this field and maintain an invariant that
    /// `ends.len()` is always the number of fields, but doing that efficiently
    /// requires unsafe. We play it safe at almost no cost.
    len: usize,
}

impl Default for Bounds {
    fn default() -> Bounds {
        Bounds { ends: vec![], len: 0 }
    }
}

impl Bounds {
    /// Returns the bounds of field `i`.
    fn get(&self, i: usize) -> Option<ops::Range<usize>> {
        if i >= self.len {
            return None;
        }
        let end = match self.ends.get(i) {
            None => return None,
            Some(&end) => end,
        };
        let start = match i.checked_sub(1).and_then(|i| self.ends.get(i)) {
            None => 0,
            Some(&start) => start,
        };
        Some(ops::Range { start: start, end: end })
    }

    /// Returns a slice of ending positions of all fields.
    fn ends(&self) -> &[usize] {
        &self.ends[..self.len]
    }

    /// Return the last position of the last field.
    ///
    /// If there are no fields, this returns `0`.
    #[inline(always)]
    fn end(&self) -> usize {
        self.ends().last().map(|&i| i).unwrap_or(0)
    }

    /// Convert an absolute position into the record to a field index and a
    /// position within that field.
    ///
    /// If the given position is past the end of the last field, then `None`
    /// is returned.
    fn absolute_to_field(&self, i: usize) -> Option<(usize, usize)> {
        if i >= self.end() {
            return None;
        }
        let field = match self.ends().binary_search(&i) {
            Err(i) => i,
            // If we land on an end, then it's the start of the next field.
            Ok(i) => i.checked_add(1).unwrap(),
        };
        Some((field, i - self.get(field).unwrap().start))
    }

    /// Returns the number of fields in these bounds.
    fn len(&self) -> usize {
        self.len
    }

    /// Expand the capacity for storing field ending positions.
    fn expand(&mut self) {
        let new_len = self.ends.len().checked_mul(2).unwrap();
        self.ends.resize(cmp::max(4, new_len), 0);
    }

    /// Add a new field with the given ending position.
    fn add(&mut self, pos: usize) {
        if self.len >= self.ends.len() {
            self.expand();
        }
        self.ends[self.len] = pos;
        self.len += 1;
    }
}

impl ops::Index<usize> for ByteRecord {
    type Output = [u8];
    fn index(&self, i: usize) -> &[u8] { self.get(i).unwrap() }
}

impl<'a> IntoIterator for &'a ByteRecord {
    type IntoIter = ByteRecordIter<'a>;
    type Item = &'a [u8];
    fn into_iter(self) -> ByteRecordIter<'a> {
        self.iter()
    }
}

/// An iterator over the fields in a byte record.
pub struct ByteRecordIter<'a> {
    r: &'a ByteRecord,
    start: usize,
    i: usize,
}

impl<'a> Iterator for ByteRecordIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<&'a [u8]> {
        match self.r.bounds.ends().get(self.i) {
            None => None,
            Some(&end) => {
                let field = &self.r.fields[self.start..end];
                self.start = end;
                self.i += 1;
                Some(field)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use string_record::StringRecord;

    use super::ByteRecord;

    fn b(s: &str) -> &[u8] { s.as_bytes() }

    #[test]
    fn record_1() {
        let mut rec = ByteRecord::new();
        rec.add(b"foo");

        assert_eq!(rec.len(), 1);
        assert_eq!(rec.get(0), Some(b("foo")));
        assert_eq!(rec.get(1), None);
        assert_eq!(rec.get(2), None);
    }

    #[test]
    fn record_2() {
        let mut rec = ByteRecord::new();
        rec.add(b"foo");
        rec.add(b"quux");

        assert_eq!(rec.len(), 2);
        assert_eq!(rec.get(0), Some(b("foo")));
        assert_eq!(rec.get(1), Some(b("quux")));
        assert_eq!(rec.get(2), None);
        assert_eq!(rec.get(3), None);
    }

    #[test]
    fn empty_record() {
        let rec = ByteRecord::new();

        assert_eq!(rec.len(), 0);
        assert_eq!(rec.get(0), None);
        assert_eq!(rec.get(1), None);
    }

    #[test]
    fn empty_field_1() {
        let mut rec = ByteRecord::new();
        rec.add(b"");

        assert_eq!(rec.len(), 1);
        assert_eq!(rec.get(0), Some(b("")));
        assert_eq!(rec.get(1), None);
        assert_eq!(rec.get(2), None);
    }

    #[test]
    fn empty_field_2() {
        let mut rec = ByteRecord::new();
        rec.add(b"");
        rec.add(b"");

        assert_eq!(rec.len(), 2);
        assert_eq!(rec.get(0), Some(b("")));
        assert_eq!(rec.get(1), Some(b("")));
        assert_eq!(rec.get(2), None);
        assert_eq!(rec.get(3), None);
    }

    #[test]
    fn empty_surround_1() {
        let mut rec = ByteRecord::new();
        rec.add(b"foo");
        rec.add(b"");
        rec.add(b"quux");

        assert_eq!(rec.len(), 3);
        assert_eq!(rec.get(0), Some(b("foo")));
        assert_eq!(rec.get(1), Some(b("")));
        assert_eq!(rec.get(2), Some(b("quux")));
        assert_eq!(rec.get(3), None);
        assert_eq!(rec.get(4), None);
    }

    #[test]
    fn empty_surround_2() {
        let mut rec = ByteRecord::new();
        rec.add(b"foo");
        rec.add(b"");
        rec.add(b"quux");
        rec.add(b"");

        assert_eq!(rec.len(), 4);
        assert_eq!(rec.get(0), Some(b("foo")));
        assert_eq!(rec.get(1), Some(b("")));
        assert_eq!(rec.get(2), Some(b("quux")));
        assert_eq!(rec.get(3), Some(b("")));
        assert_eq!(rec.get(4), None);
        assert_eq!(rec.get(5), None);
    }

    #[test]
    fn utf8_error_1() {
        let mut rec = ByteRecord::new();
        rec.add(b"foo");
        rec.add(b"b\xFFar");

        let err = StringRecord::from_byte_record(rec).unwrap_err();
        assert_eq!(err.utf8_error().field(), 1);
        assert_eq!(err.utf8_error().valid_up_to(), 1);
    }

    #[test]
    fn utf8_error_2() {
        let mut rec = ByteRecord::new();
        rec.add(b"\xFF");

        let err = StringRecord::from_byte_record(rec).unwrap_err();
        assert_eq!(err.utf8_error().field(), 0);
        assert_eq!(err.utf8_error().valid_up_to(), 0);
    }

    #[test]
    fn utf8_error_3() {
        let mut rec = ByteRecord::new();
        rec.add(b"a\xFF");

        let err = StringRecord::from_byte_record(rec).unwrap_err();
        assert_eq!(err.utf8_error().field(), 0);
        assert_eq!(err.utf8_error().valid_up_to(), 1);
    }

    #[test]
    fn utf8_error_4() {
        let mut rec = ByteRecord::new();
        rec.add(b"a");
        rec.add(b"b");
        rec.add(b"c");
        rec.add(b"d");
        rec.add(b"xyz\xFF");

        let err = StringRecord::from_byte_record(rec).unwrap_err();
        assert_eq!(err.utf8_error().field(), 4);
        assert_eq!(err.utf8_error().valid_up_to(), 3);
    }

    #[test]
    fn utf8_error_5() {
        let mut rec = ByteRecord::new();
        rec.add(b"a");
        rec.add(b"b");
        rec.add(b"c");
        rec.add(b"d");
        rec.add(b"\xFFxyz");

        let err = StringRecord::from_byte_record(rec).unwrap_err();
        assert_eq!(err.utf8_error().field(), 4);
        assert_eq!(err.utf8_error().valid_up_to(), 0);
    }

    // This tests a tricky case where a single field on its own isn't valid
    // UTF-8, but the concatenation of all fields is.
    #[test]
    fn utf8_error_6() {
        let mut rec = ByteRecord::new();
        rec.add(b"a\xc9");
        rec.add(b"\x91b");

        let err = StringRecord::from_byte_record(rec).unwrap_err();
        assert_eq!(err.utf8_error().field(), 0);
        assert_eq!(err.utf8_error().valid_up_to(), 1);
    }

    // This tests that we can always clear a `ByteRecord` and get a guaranteed
    // successful conversion to UTF-8. This permits reusing the allocation.
    #[test]
    fn utf8_clear_ok() {
        let mut rec = ByteRecord::new();
        rec.add(b"\xFF");
        assert!(StringRecord::from_byte_record(rec).is_err());

        let mut rec = ByteRecord::new();
        rec.add(b"\xFF");
        rec.clear();
        assert!(StringRecord::from_byte_record(rec).is_ok());
    }
}
