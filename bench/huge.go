package main

import (
	"bytes"
	"encoding/csv"
	"io"
	"io/ioutil"
	"log"
	"os"
)

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

func readAll(r io.Reader) {
	csvr := csv.NewReader(r)
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

func main() {
	// This is a 3.6GB file from a data set that can be downloaded here:
	// http://www2.census.gov/acs2010_5yr/pums/csv_pus.zip
	huge := "../examples/data/ss10pusa.csv"
	f, err := os.Open(huge)
	if err != nil {
		log.Fatal(err)
	}
	readAll(f)
}
