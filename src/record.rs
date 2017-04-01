use std::cmp;
use std::ops;

use error::{FromUtf8Error, new_from_utf8_error, new_utf8_error};

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

/// A single CSV record stored as valid UTF-8 bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StringRecord {
    /// All fields in this record, stored contiguously.
    fields: String,
    /// The number of and location of each field in this record.
    bounds: Bounds,
}

impl Default for StringRecord {
    fn default() -> StringRecord {
        StringRecord::new()
    }
}

impl StringRecord {
    /// Create a new empty `StringRecord`.
    pub fn new() -> StringRecord {
        StringRecord::with_capacity(10)
    }

    /// Create a new empty `StringRecord` with the given capacity.
    pub fn with_capacity(capacity: usize) -> StringRecord {
        let fields = String::with_capacity(capacity);
        StringRecord { fields: fields, bounds: Bounds::default() }
    }

    /// Create a new `StringRecord` from a `ByteRecord`.
    ///
    /// Note that this does UTF-8 validation. If the given `ByteRecord` does
    /// not contain valid UTF-8, then this returns an error. The error includes
    /// the UTF-8 error and the original `ByteRecord`.
    pub fn from_byte_record(
        record: ByteRecord,
    ) -> Result<StringRecord, FromUtf8Error> {
        let ByteRecord { fields, bounds } = record;
        let err = match String::from_utf8(fields) {
            Err(err) => err,
            Ok(fields) => {
                return Ok(StringRecord { fields: fields, bounds: bounds });
            }
        };
        let upto = err.utf8_error().valid_up_to();
        let fields = err.into_bytes();
        let record = ByteRecord { fields: fields, bounds: bounds };
        let field = match record.bounds.ends().binary_search(&upto) {
            Ok(i) => i.checked_add(1).unwrap(),
            Err(i) => i,
        };
        // BREADCRUMBS:
        //
        // 1. Figure out how to compute upto relative to the field. It's ugly.
        // 2. String::from_utf8(fields) above is wrong. We probably want a
        //    subset of fields based on the last end.
        // 3. Is there a better abstraction here?
        // assert!(
        // let upto =
        // println!("ends: {:?}, upto: {:?}", record.bounds.ends(), upto);
        let err = new_utf8_error(field, 0);
        Err(new_from_utf8_error(record, err))
    }

    /// Return the field at index `i`.
    ///
    /// If no field at index `i` exists, then this returns `None`.
    pub fn get(&self, i: usize) -> Option<&str> {
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

    /// Convert this `StringRecord` into `ByteRecord`.
    pub fn into_byte_record(self) -> ByteRecord {
        ByteRecord {
            fields: self.fields.into_bytes(),
            bounds: self.bounds,
        }
    }
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
        ByteRecord::with_capacity(10)
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

    /// Set the number of fields in this record.
    #[doc(hidden)]
    pub fn set_len(&mut self, len: usize) {
        // TODO(burntsushi): Use `pub(crate)` when it stabilizes.
        self.bounds.len = len;
    }

    /// Return the underlying storage.
    #[doc(hidden)]
    pub fn as_parts(&mut self) -> (&mut Vec<u8>, &mut Vec<usize>) {
        // TODO(burntsushi): Use `pub(crate)` when it stabilizes.
        (&mut self.fields, &mut self.bounds.ends)
    }

    /// Expand the capacity for storing fields.
    #[doc(hidden)]
    pub fn expand_fields(&mut self) {
        // TODO(burntsushi): Use `pub(crate)` when it stabilizes.
        let new_len = self.fields.len().checked_mul(2).unwrap();
        self.fields.resize(cmp::max(4, new_len), 0);
    }

    /// Expand the capacity for storing field ending positions.
    #[doc(hidden)]
    pub fn expand_ends(&mut self) {
        // TODO(burntsushi): Use `pub(crate)` when it stabilizes.
        self.bounds.expand();
    }

    /// Add a new end.
    #[doc(hidden)]
    pub fn add_end(&mut self, pos: usize) {
        // TODO(burntsushi): Use `pub(crate)` when it stabilizes.
        assert!(pos <= self.fields.len());
        self.bounds.add(pos);
    }
}

impl ops::Index<usize> for ByteRecord {
    type Output = [u8];
    fn index(&self, i: usize) -> &[u8] { self.get(i).unwrap() }
}

/// An iterator over the fields in a byte record.
pub struct ByteRecordIter<'a> {
    r: &'a ByteRecord,
    i: usize,
}

impl<'a> Iterator for ByteRecordIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<&'a [u8]> {
        match self.r.get(self.i) {
            None => None,
            Some(field) => {
                self.i += 1;
                Some(field)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ByteRecord, StringRecord};

    fn b(s: &str) -> &[u8] { s.as_bytes() }

    #[test]
    fn record_1() {
        let mut rec = ByteRecord::with_capacity(0);
        rec.fields.extend_from_slice(b("foo"));
        rec.add_end(3);
        assert_eq!(rec.len(), 1);
        assert_eq!(rec.get(0), Some(b("foo")));
        assert_eq!(rec.get(1), None);
        assert_eq!(rec.get(2), None);
    }

    #[test]
    fn record_2() {
        let mut rec = ByteRecord::with_capacity(0);
        rec.fields.extend_from_slice(b("foo"));
        rec.add_end(3);
        rec.fields.extend_from_slice(b("quux"));
        rec.add_end(7);
        assert_eq!(rec.len(), 2);
        assert_eq!(rec.get(0), Some(b("foo")));
        assert_eq!(rec.get(1), Some(b("quux")));
        assert_eq!(rec.get(2), None);
        assert_eq!(rec.get(3), None);
    }

    #[test]
    fn empty_record() {
        let rec = ByteRecord::with_capacity(0);
        assert_eq!(rec.len(), 0);
        assert_eq!(rec.get(0), None);
        assert_eq!(rec.get(1), None);
    }

    #[test]
    fn empty_field_1() {
        let mut rec = ByteRecord::with_capacity(0);
        rec.add_end(0);
        assert_eq!(rec.len(), 1);
        assert_eq!(rec.get(0), Some(b("")));
        assert_eq!(rec.get(1), None);
        assert_eq!(rec.get(2), None);
    }

    #[test]
    fn empty_field_2() {
        let mut rec = ByteRecord::with_capacity(0);
        rec.add_end(0);
        rec.add_end(0);
        assert_eq!(rec.len(), 2);
        assert_eq!(rec.get(0), Some(b("")));
        assert_eq!(rec.get(1), Some(b("")));
        assert_eq!(rec.get(2), None);
        assert_eq!(rec.get(3), None);
    }

    #[test]
    fn empty_surround_1() {
        let mut rec = ByteRecord::with_capacity(0);
        rec.fields.extend_from_slice(b("foo"));
        rec.add_end(3);
        rec.add_end(3);
        rec.fields.extend_from_slice(b("quux"));
        rec.add_end(7);
        assert_eq!(rec.len(), 3);
        assert_eq!(rec.get(0), Some(b("foo")));
        assert_eq!(rec.get(1), Some(b("")));
        assert_eq!(rec.get(2), Some(b("quux")));
        assert_eq!(rec.get(3), None);
        assert_eq!(rec.get(4), None);
    }

    #[test]
    fn empty_surround_2() {
        let mut rec = ByteRecord::with_capacity(0);
        rec.fields.extend_from_slice(b("foo"));
        rec.add_end(3);
        rec.add_end(3);
        rec.fields.extend_from_slice(b("quux"));
        rec.add_end(7);
        rec.add_end(7);
        assert_eq!(rec.len(), 4);
        assert_eq!(rec.get(0), Some(b("foo")));
        assert_eq!(rec.get(1), Some(b("")));
        assert_eq!(rec.get(2), Some(b("quux")));
        assert_eq!(rec.get(3), Some(b("")));
        assert_eq!(rec.get(4), None);
        assert_eq!(rec.get(5), None);
    }

    #[test]
    fn utf8_error() {
        let mut rec = ByteRecord::with_capacity(0);
        rec.fields.extend_from_slice(b("foo"));
        rec.add_end(3);
        rec.fields.extend_from_slice(&b"b\xFFar"[..]);
        rec.add_end(7);

        assert_eq!(rec.len(), 2);

        let err = StringRecord::from_byte_record(rec).unwrap_err();
        println!("{:?}", err);
    }
}
