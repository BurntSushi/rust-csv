use std::cmp;
use std::iter::FromIterator;
use std::ops::{self, Range};
use std::result;
use std::str;

use error::{Utf8Error, new_utf8_error};
use string_record::StringRecord;

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
        ByteRecord::with_capacity(0, 0)
    }

    /// Create a new empty `ByteRecord` with the given capacity settings.
    ///
    /// `buffer` refers to the capacity of the buffer used to store the
    /// actual row contents. `fields` refers to the number of fields one
    /// might expect to store.
    pub fn with_capacity(buffer: usize, fields: usize) -> ByteRecord {
        ByteRecord {
            fields: vec![0; buffer],
            bounds: Bounds::with_capacity(fields),
        }
    }

    /// Returns an iterator over all fields in this record.
    pub fn iter(&self) -> ByteRecordIter {
        self.into_iter()
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
        self.bounds.len()
    }

    /// Clear this record so that it has zero fields.
    ///
    /// Note that it is not necessary to clear the record to reuse it with
    /// the CSV reader.
    pub fn clear(&mut self) {
        self.bounds.len = 0;
    }

    /// Add a new field to this record.
    pub fn push_field(&mut self, field: &[u8]) {
        let (s, e) = (self.bounds.end(), self.bounds.end() + field.len());
        while e > self.fields.len() {
            expand_fields(self);
        }
        self.fields[s..e].copy_from_slice(field);
        self.bounds.add(e);
    }

    /// Return the start and end position of a field in this record.
    ///
    /// If no such field exists at the given index, then return `None`.
    ///
    /// The range returned can be used with the slice returned by `as_slice`.
    pub fn range(&self, i: usize) -> Option<Range<usize>> {
        self.bounds.get(i)
    }

    /// Return the entire row as a single byte slice.
    pub fn as_slice(&self) -> &[u8] {
        &self.fields[..self.bounds.end()]
    }
}

/// The bounds of fields in a single record.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Bounds {
    /// The ending index of each field.
    ends: Vec<usize>,
    /// The number of fields in this record.
    ///
    /// Technically, we could drop this field and maintain an invariant that
    /// `ends.len()` is always the number of fields, but doing that efficiently
    /// requires attention to safety. We play it safe at essentially no cost.
    len: usize,
}

impl Default for Bounds {
    fn default() -> Bounds {
        Bounds::with_capacity(0)
    }
}

impl Bounds {
    /// Create a new set of bounds with the given capacity for storing the
    /// ends of fields.
    fn with_capacity(capacity: usize) -> Bounds {
        Bounds { ends: vec![0; capacity], len: 0 }
    }

    /// Returns the bounds of field `i`.
    fn get(&self, i: usize) -> Option<Range<usize>> {
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
    fn end(&self) -> usize {
        self.ends().last().map(|&i| i).unwrap_or(0)
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

impl From<StringRecord> for ByteRecord {
    fn from(record: StringRecord) -> ByteRecord { record.into_byte_record() }
}

impl<T: AsRef<[u8]>> From<Vec<T>> for ByteRecord {
    fn from(xs: Vec<T>) -> ByteRecord {
        ByteRecord::from_iter(&xs)
    }
}

impl<'a, T: AsRef<[u8]>> From<&'a [T]> for ByteRecord {
    fn from(xs: &'a [T]) -> ByteRecord {
        ByteRecord::from_iter(xs)
    }
}

impl<T: AsRef<[u8]>> FromIterator<T> for ByteRecord {
    fn from_iter<I: IntoIterator<Item=T>>(iter: I) -> ByteRecord {
        let mut record = ByteRecord::new();
        record.extend(iter);
        record
    }
}

impl<T: AsRef<[u8]>> Extend<T> for ByteRecord {
    fn extend<I: IntoIterator<Item=T>>(&mut self, iter: I) {
        for x in iter {
            self.push_field(x.as_ref());
        }
    }
}

/// An iterator over the fields in a byte record.
pub struct ByteRecordIter<'a> {
    /// The record we are iterating over.
    r: &'a ByteRecord,
    /// The starting index of the previous field. (For reverse iteration.)
    last_start: usize,
    /// The ending index of the previous field. (For forward iteration.)
    last_end: usize,
    /// The index of forward iteration.
    i_forward: usize,
    /// The index of reverse iteration.
    i_reverse: usize,
}

impl<'a> IntoIterator for &'a ByteRecord {
    type IntoIter = ByteRecordIter<'a>;
    type Item = &'a [u8];
    fn into_iter(self) -> ByteRecordIter<'a> {
        ByteRecordIter {
            r: self,
            last_start: self.as_slice().len(),
            last_end: 0,
            i_forward: 0,
            i_reverse: self.len(),
        }
    }
}

impl<'a> ExactSizeIterator for ByteRecordIter<'a> {}

impl<'a> Iterator for ByteRecordIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<&'a [u8]> {
        if self.i_forward == self.i_reverse {
            None
        } else {
            let start = self.last_end;
            let end = self.r.bounds.ends()[self.i_forward];
            self.i_forward += 1;
            self.last_end = end;
            Some(&self.r.fields[start..end])
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let x = self.i_reverse - self.i_forward;
        (x, Some(x))
    }

    fn count(self) -> usize {
        self.len()
    }
}

impl<'a> DoubleEndedIterator for ByteRecordIter<'a> {
    fn next_back(&mut self) -> Option<&'a [u8]> {
        if self.i_forward == self.i_reverse {
            None
        } else {
            self.i_reverse -= 1;
            let start = self.i_reverse
                .checked_sub(1)
                .map(|i| self.r.bounds.ends()[i])
                .unwrap_or(0);
            let end = self.last_start;
            self.last_start = start;
            Some(&self.r.fields[start..end])
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
        rec.push_field(b"foo");

        assert_eq!(rec.len(), 1);
        assert_eq!(rec.get(0), Some(b("foo")));
        assert_eq!(rec.get(1), None);
        assert_eq!(rec.get(2), None);
    }

    #[test]
    fn record_2() {
        let mut rec = ByteRecord::new();
        rec.push_field(b"foo");
        rec.push_field(b"quux");

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
        rec.push_field(b"");

        assert_eq!(rec.len(), 1);
        assert_eq!(rec.get(0), Some(b("")));
        assert_eq!(rec.get(1), None);
        assert_eq!(rec.get(2), None);
    }

    #[test]
    fn empty_field_2() {
        let mut rec = ByteRecord::new();
        rec.push_field(b"");
        rec.push_field(b"");

        assert_eq!(rec.len(), 2);
        assert_eq!(rec.get(0), Some(b("")));
        assert_eq!(rec.get(1), Some(b("")));
        assert_eq!(rec.get(2), None);
        assert_eq!(rec.get(3), None);
    }

    #[test]
    fn empty_surround_1() {
        let mut rec = ByteRecord::new();
        rec.push_field(b"foo");
        rec.push_field(b"");
        rec.push_field(b"quux");

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
        rec.push_field(b"foo");
        rec.push_field(b"");
        rec.push_field(b"quux");
        rec.push_field(b"");

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
        rec.push_field(b"foo");
        rec.push_field(b"b\xFFar");

        let err = StringRecord::from_byte_record(rec).unwrap_err();
        assert_eq!(err.utf8_error().field(), 1);
        assert_eq!(err.utf8_error().valid_up_to(), 1);
    }

    #[test]
    fn utf8_error_2() {
        let mut rec = ByteRecord::new();
        rec.push_field(b"\xFF");

        let err = StringRecord::from_byte_record(rec).unwrap_err();
        assert_eq!(err.utf8_error().field(), 0);
        assert_eq!(err.utf8_error().valid_up_to(), 0);
    }

    #[test]
    fn utf8_error_3() {
        let mut rec = ByteRecord::new();
        rec.push_field(b"a\xFF");

        let err = StringRecord::from_byte_record(rec).unwrap_err();
        assert_eq!(err.utf8_error().field(), 0);
        assert_eq!(err.utf8_error().valid_up_to(), 1);
    }

    #[test]
    fn utf8_error_4() {
        let mut rec = ByteRecord::new();
        rec.push_field(b"a");
        rec.push_field(b"b");
        rec.push_field(b"c");
        rec.push_field(b"d");
        rec.push_field(b"xyz\xFF");

        let err = StringRecord::from_byte_record(rec).unwrap_err();
        assert_eq!(err.utf8_error().field(), 4);
        assert_eq!(err.utf8_error().valid_up_to(), 3);
    }

    #[test]
    fn utf8_error_5() {
        let mut rec = ByteRecord::new();
        rec.push_field(b"a");
        rec.push_field(b"b");
        rec.push_field(b"c");
        rec.push_field(b"d");
        rec.push_field(b"\xFFxyz");

        let err = StringRecord::from_byte_record(rec).unwrap_err();
        assert_eq!(err.utf8_error().field(), 4);
        assert_eq!(err.utf8_error().valid_up_to(), 0);
    }

    // This tests a tricky case where a single field on its own isn't valid
    // UTF-8, but the concatenation of all fields is.
    #[test]
    fn utf8_error_6() {
        let mut rec = ByteRecord::new();
        rec.push_field(b"a\xc9");
        rec.push_field(b"\x91b");

        let err = StringRecord::from_byte_record(rec).unwrap_err();
        assert_eq!(err.utf8_error().field(), 0);
        assert_eq!(err.utf8_error().valid_up_to(), 1);
    }

    // This tests that we can always clear a `ByteRecord` and get a guaranteed
    // successful conversion to UTF-8. This permits reusing the allocation.
    #[test]
    fn utf8_clear_ok() {
        let mut rec = ByteRecord::new();
        rec.push_field(b"\xFF");
        assert!(StringRecord::from_byte_record(rec).is_err());

        let mut rec = ByteRecord::new();
        rec.push_field(b"\xFF");
        rec.clear();
        assert!(StringRecord::from_byte_record(rec).is_ok());
    }

    #[test]
    fn iter() {
        let data = vec!["foo", "bar", "baz", "quux", "wat"];
        let rec = ByteRecord::from(&*data);
        let got: Vec<&str> = rec.iter()
            .map(|x| ::std::str::from_utf8(x).unwrap())
            .collect();
        assert_eq!(data, got);
    }

    #[test]
    fn iter_reverse() {
        let mut data = vec!["foo", "bar", "baz", "quux", "wat"];
        let rec = ByteRecord::from(&*data);
        let got: Vec<&str> = rec.iter()
            .rev()
            .map(|x| ::std::str::from_utf8(x).unwrap())
            .collect();
        data.reverse();
        assert_eq!(data, got);
    }

    #[test]
    fn iter_forward_and_reverse() {
        let data = vec!["foo", "bar", "baz", "quux", "wat"];
        let rec = ByteRecord::from(data);
        let mut it = rec.iter();

        assert_eq!(it.next_back(), Some(b("wat")));
        assert_eq!(it.next(), Some(b("foo")));
        assert_eq!(it.next(), Some(b("bar")));
        assert_eq!(it.next_back(), Some(b("quux")));
        assert_eq!(it.next(), Some(b("baz")));
        assert_eq!(it.next_back(), None);
        assert_eq!(it.next(), None);
    }
}
