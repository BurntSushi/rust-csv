use std::error::Error;
use std::io::Read;
use csv::ByteRecord;

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
        let bytes_parsed = parse_csv(
            Read::chain(unparsed.as_slice(), *chunk),
            &mut output
        )
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
