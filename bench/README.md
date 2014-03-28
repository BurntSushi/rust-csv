You can see the [results on my machine 
here](https://github.com/BurntSushi/rust-csv/blob/master/bench/results).

To run the short, medium and large benchmarks, make sure Go is installed run 
this in ./golang

    go test -bench '.*'

To run the super huge benchmark (3.6GB), you'll need to download the zip from
http://www2.census.gov/acs2010_5yr/pums/csv_pus.zip and put 'ss10pusa.csv' in
../examples/data/ss10pusa.csv.

Then compile and run:

    go build -o huge-go
    time ./huge-go

To run the huge benchmark for Rust, make sure ss10pusa.csv is in the same 
location as above and run:

    rustc -O huge.rs -o huge-rust
    time ./huge-rust

