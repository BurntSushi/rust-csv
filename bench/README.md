To run a micro benchmark using the 1.4MB `examples/data/bench.csv` data:

    go test -bench '.*'

To run similar benchmarks for Rust (on the same data, but will benchmark each
of the four access patterns), run `cargo bench` in the project root directory.

To run the super huge benchmark (3.6GB), you'll need to download the zip from
http://www2.census.gov/acs2010_5yr/pums/csv_pus.zip and put `ss10pusa.csv` in
`../examples/data/ss10pusa.csv`.

Then compile and run:

    go build -o huge-go
    time ./huge-go

To run the huge benchmark for Rust, make sure `ss10pusa.csv` is in the same 
location as above and run:

    rustc --opt-level=3 -Z lto -L ../target/release/ huge.rs -o huge-rust
    time ./huge-rust

To get libraries in `../target/release/`, run `cargo build --release` in the
project root directory.

(Please make sure that one CPU is pegged when running this benchmark. If it 
isn't, you're probably just testing the speed of your disk.)


### Results

Benchmarks were run on an Intel i3930K. Note that the 
'ns/iter' value is computed by each language's microbenchmark facilities. I 
suspect the granularity is big enough that the values are comparable.

For rust, --opt-level=3 was used.

```
Go                  41033948 ns/iter
Rust (decode)       24016498
Rust (string)       17052713
Rust (byte string)  14876428
Rust (byte slice)   11932269
```

You'll note that none of the above benchmarks use a particularly large CSV 
file. So I've also run a pretty rough benchmark on a huge CSV file (3.6GB). A 
single large benchmark isn't exactly definitive, but I think we can use it as a 
ballpark estimate.

The huge benchmark for both Rust and Go use buffering. The times are wall 
clock times. The file system cache was warm and no disk access occurred during
the benchmark. Both use a negligible and constant amount of memory (~1KB).

```
Go                 146 seconds
Rust (byte slice)   32 seconds
```

TODO: Fill in the other Rust access patterns for the huge benchmark. (The "byte 
slice" access pattern is the fastest.)

TODO: Benchmark with Python. (Estimate: "byte slice" is faster by around 2x, 
but the other access patterns are probably comparable.)

