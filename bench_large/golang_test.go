package main

import (
	"bytes"
	"io/ioutil"
	"log"
	"os"
	"testing"
)

func BenchmarkReadCsv(b *testing.B) {
	rdr := asByteReader("../examples/data/bench.csv")

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		rdr.Seek(0, 0)
		readAll(rdr)
	}
}

func asByteReader(fpath string) *bytes.Reader {
	f, err := os.Open(fpath)
	if err != nil {
		log.Fatal(err)
	}
	bs, err := ioutil.ReadAll(f)
	if err != nil {
		log.Fatal(err)
	}
	return bytes.NewReader(bs)
}
