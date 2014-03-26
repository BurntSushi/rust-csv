package main

import (
	"encoding/csv"
	"io"
	"log"
	"os"
	"testing"
)

func BenchmarkShort(b *testing.B) {
	for i := 0; i < b.N; i++ {
		// Yes, this is including opening the file in the benchmark.
		// But this is how it's done in the Rust benchmark too.
		// Should convert to byte buffer in both...
		readAll("../examples/data/short.csv")
	}
}

func BenchmarkMedium(b *testing.B) {
	for i := 0; i < b.N; i++ {
		// Yes, this is including opening the file in the benchmark.
		// But this is how it's done in the Rust benchmark too.
		// Should convert to byte buffer in both...
		readAll("../examples/data/medium.csv")
	}
}

func readAll(fp string) {
	f, err := os.Open(fp);
	if err != nil {
		log.Fatal(err)
	}
	csvr := csv.NewReader(f)
	for {
		_, err := csvr.Read()
		if err != nil {
			if err == io.EOF {
				break
			}
			log.Fatal(err)
		}
	}
}
