use csv_core::{ReadRecordResult, ReaderBuilder};

fn read(nfa: bool, chunks: &[&[u8]]) -> Vec<String> {
    let mut rdr = ReaderBuilder::new().nfa(nfa).build();
    let mut out = [0u8; 64];
    let mut ends = [0usize; 8];
    let (mut outpos, mut endpos) = (0, 0);
    let mut fields = vec![];
    let mut chunks = chunks.iter().copied().chain(std::iter::once(&b""[..]));
    let mut data = chunks.next().unwrap();
    loop {
        let (res, nin, nout, nend) =
            rdr.read_record(data, &mut out[outpos..], &mut ends[endpos..]);
        data = &data[nin..];
        outpos += nout;
        endpos += nend;
        match res {
            ReadRecordResult::InputEmpty => match chunks.next() {
                Some(c) => data = c,
                None => break,
            },
            ReadRecordResult::Record => {
                let mut start = 0;
                for &end in &ends[..endpos] {
                    fields.push(
                        String::from_utf8_lossy(&out[start..end]).into_owned(),
                    );
                    start = end;
                }
                outpos = 0;
                endpos = 0;
            }
            ReadRecordResult::End => break,
            r => panic!("{r:?}"),
        }
    }
    fields
}

#[test]
fn repro_nfa_record_split_across_calls() {
    let chunks: &[&[u8]] = &[b"ab,c", b"d,ef\n"];
    let dfa = read(false, chunks);
    let nfa = read(true, chunks);
    println!("dfa={dfa:?}\nnfa={nfa:?}");
    assert_eq!(dfa, ["ab", "cd", "ef"]);
    assert_eq!(nfa, dfa);
}
