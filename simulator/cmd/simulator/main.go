package main

import (
	"flag"
	"sync"

	"github.com/agusrichard/icu-vitals-stream/simulator/internal"
)

func main() {
	patients := flag.Int("patients", 1, "Number of patients to simulate")
	flag.Parse()
	println("Number of patients to simulate:", *patients)

	var wg sync.WaitGroup
	for i := 0; i < *patients; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			patient := internal.NewPatient()
			patient.Run()
		}()
	}

	wg.Wait()
}
