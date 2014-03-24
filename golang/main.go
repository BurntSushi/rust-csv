package main

import (
	"encoding/csv"
	"io"
	"log"
	"os"
)

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

func main() {
	readAll("./data/2012_nfl_pbp_data.csv")
	// readAll("/home/andrew/tmp/csv-huge/ss10pusa.csv") 
}
