package main

import (
	"testing"
)

func BenchmarkShort(b *testing.B) {
	rdr := asByteReader("../examples/data/short.csv")

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		rdr.Seek(0, 0)
		readAll(rdr)
	}
}

func BenchmarkMedium(b *testing.B) {
	rdr := asByteReader("../examples/data/medium.csv")

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		rdr.Seek(0, 0)
		readAll(rdr)
	}
}

func BenchmarkLarge(b *testing.B) {
	rdr := asByteReader("../examples/data/large.csv")

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		rdr.Seek(0, 0)
		readAll(rdr)
	}
}
