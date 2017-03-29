use std::ops;

/// A single CSV record stored as raw bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ByteRecord {
    /// All fields in this record, stored contiguously.
    fields: Vec<u8>,
    /// The starting index of each field. The first value is always zero.
    starts: Vec<usize>,
}

impl ByteRecord {
    /// Return the field at index `i`.
    ///
    /// If no field at index `i` exists, then this returns `None`.
    pub fn get(&self, i: usize) -> Option<&[u8]> {
        self.bounds(i).map(|range| &self.fields[range])
    }

    /// Returns the number of fields in this record.
    pub fn len(&self) -> usize {
        self.starts.len()
    }

    /// Clear this record so that it has zero fields.
    ///
    /// This permits the record to be reused.
    pub fn clear(&mut self) {
        self.fields.clear();
        self.starts.clear();
    }

    /// Return the underlying storage.
    #[doc(hidden)]
    pub fn as_parts(&mut self) -> (&mut Vec<u8>, &mut Vec<usize>) {
        // TODO(burntsushi): Use `pub(crate)` when it stabilizes.
        (&mut self.fields, &mut self.starts)
    }

    /// Add a new field starting at the end of the internal buffer.
    #[doc(hidden)]
    pub fn add_start(&mut self) {
        // TODO(burntsushi): Use `pub(crate)` when it stabilizes.
        self.starts.push(self.fields.len());
    }

    /// Returns the bounds of field `i`.
    fn bounds(&self, i: usize) -> Option<ops::Range<usize>> {
        let start = match self.starts.get(i) {
            None => return None,
            Some(&start) => start,
        };
        let end = match i.checked_add(1).and_then(|i| self.starts.get(i)) {
            None => self.fields.len(),
            Some(&end) => end,
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
