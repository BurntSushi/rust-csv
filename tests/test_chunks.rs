use csv::ByteRecord;
use std::error::Error;
use std::io::Read;

fn parse_csv(
    chunk: impl Read,
    output: &mut Vec<ByteRecord>,
) -> Result<u64, Box<dyn Error>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .complete_only(true)
        .from_reader(chunk);

    output.extend(reader.byte_records().map(Result::unwrap));
    Ok(reader.position().byte())
}

#[test]
fn test_chunks() {
    let input_chunks = vec![
        &b"col_a,col_b,col_c\n0aaaa,0bbbb,0cccc\n1aaaa,1bbbb,1cc"[..],
        &b"cc\n"[..],
        &b"2aaaa,2bbbb"[..],
        &b",2cccc\n"[..],
        &b"3aaaa,3bbbb,3cccc\n4aaaa,4bbbb,4cccc\n5aaaa,5bb"[..],
        &b"bb,5cccc"[..],
        &b"\n"[..],
        &b"6aaa"[..],
    ];
    let mut unparsed = Vec::new();
    let mut next_unparsed = Vec::new();
    let mut output = Vec::new();
    for chunk in input_chunks.iter() {
        let bytes_parsed =
            parse_csv(Read::chain(unparsed.as_slice(), *chunk), &mut output)
                .unwrap();
        let stored_bytes_parsed =
            std::cmp::min(bytes_parsed as usize, unparsed.len());
        let chunk_bytes_parsed = bytes_parsed as usize - stored_bytes_parsed;
        next_unparsed.extend_from_slice(&unparsed[stored_bytes_parsed..]);
        next_unparsed.extend_from_slice(&chunk[chunk_bytes_parsed..]);
        unparsed.truncate(0);
        std::mem::swap(&mut unparsed, &mut next_unparsed);
    }
    assert_eq!(output[0].as_slice(), &b"col_acol_bcol_c"[..]);
    assert_eq!(output[1].as_slice(), &b"0aaaa0bbbb0cccc"[..]);
    assert_eq!(output[2].as_slice(), &b"1aaaa1bbbb1cccc"[..]);
    assert_eq!(output[3].as_slice(), &b"2aaaa2bbbb2cccc"[..]);
    assert_eq!(output[4].as_slice(), &b"3aaaa3bbbb3cccc"[..]);
    assert_eq!(output[5].as_slice(), &b"4aaaa4bbbb4cccc"[..]);
    assert_eq!(output[6].as_slice(), &b"5aaaa5bbbb5cccc"[..]);
}

#[derive(Debug)]
struct ChunkReader<'a> {
    read: usize,
    chunk: &'a [u8],
    unparsed: Vec<u8>,
}

impl<'a> ChunkReader<'a> {
    fn feed(&mut self, input: &'a [u8]) {
        self.unparsed.extend_from_slice(self.chunk);
        self.chunk = input;
    }

    fn consumed(&mut self, bytes: usize) {
        let of_unparsed = std::cmp::min(self.unparsed.len(), bytes);
        self.unparsed = self.unparsed.split_off(of_unparsed);
        let of_read = bytes - of_unparsed;
        self.chunk = &self.chunk[of_read..];
        self.read = 0;
    }
}

impl<'a> std::io::Read for ChunkReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let internal = if self.read < self.unparsed.len() {
            &self.unparsed[self.read..]
        } else {
            &self.chunk[(self.read - self.unparsed.len())..]
        };
        let len = std::cmp::min(buf.len(), internal.len());
        buf[..len].copy_from_slice(&internal[..len]);
        self.read += len;

        Ok(len)
    }
}

#[test]
fn test_chunks_2() {
    let input_chunks = vec![
        &b"col_a,col_b,col_c\n0aaaa,0bbbb,0cccc\n1aaaa,1bbbb,1cc"[..],
        &b"cc\n"[..],
        &b"2aaaa,2bbbb"[..],
        &b",2cccc\n"[..],
        &b"3aaaa,3bbbb,3cccc\n4aaaa,4bbbb,4cccc\n5aaaa,5bb"[..],
        &b"bb,5cccc"[..],
        &b"\n"[..],
        &b"6aaa"[..],
    ];
    let mut output = Vec::new();
    let mut reader =
        csv::ReaderBuilder::new().complete_only(true).from_reader(
            ChunkReader { read: 0, chunk: &[], unparsed: Vec::new() },
        );
    let mut bytes_parsed_prev = 0;
    for chunk in input_chunks.iter() {
        reader.inner_mut().feed(chunk);
        output.extend(reader.byte_records().map(Result::unwrap));
        let consumed = reader.position().byte() as usize - bytes_parsed_prev;
        bytes_parsed_prev = reader.position().byte() as usize;
        reader.inner_mut().consumed(consumed);
    }
    assert_eq!(output[0].as_slice(), &b"0aaaa0bbbb0cccc"[..]);
    assert_eq!(output[1].as_slice(), &b"1aaaa1bbbb1cccc"[..]);
    assert_eq!(output[2].as_slice(), &b"2aaaa2bbbb2cccc"[..]);
    assert_eq!(output[3].as_slice(), &b"3aaaa3bbbb3cccc"[..]);
    assert_eq!(output[4].as_slice(), &b"4aaaa4bbbb4cccc"[..]);
    assert_eq!(output[5].as_slice(), &b"5aaaa5bbbb5cccc"[..]);
}
