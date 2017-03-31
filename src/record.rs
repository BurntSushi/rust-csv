use std::ops;

/// A single CSV record stored as raw bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ByteRecord {
    /// All fields in this record, stored contiguously.
    fields: Vec<u8>,
    /// The ending index of each field. The last value is always equal to
    /// the length of `fields`.
    ends: Vec<usize>,
}

impl ByteRecord {
    /// Create a new empty `ByteRecord`.
    pub fn new() -> ByteRecord {
        ByteRecord::with_capacity(10)
    }

    /// Create a new empty `ByteRecord` with the given capacity.
    pub fn with_capacity(capacity: usize) -> ByteRecord {
        ByteRecord { fields: vec![0; capacity], ends: vec![] }
    }

    /// Return the field at index `i`.
    ///
    /// If no field at index `i` exists, then this returns `None`.
    pub fn get(&self, i: usize) -> Option<&[u8]> {
        self.bounds(i).map(|range| &self.fields[range])
    }

    /// Returns the number of fields in this record.
    pub fn len(&self) -> usize {
        self.ends.len()
    }

    /// Clear this record so that it has zero fields.
    ///
    /// Note that it is not necessary to clear the record to reuse it with
    /// the CSV reader.
    pub fn clear(&mut self) {
        self.fields.clear();
        self.ends.clear();
    }

    /// Return the underlying storage.
    #[doc(hidden)]
    pub fn as_parts(&mut self) -> (&mut Vec<u8>, &mut Vec<usize>) {
        // TODO(burntsushi): Use `pub(crate)` when it stabilizes.
        (&mut self.fields, &mut self.ends)
    }

    /// Returns the bounds of field `i`.
    fn bounds(&self, i: usize) -> Option<ops::Range<usize>> {
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
