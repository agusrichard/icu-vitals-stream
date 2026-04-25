package main

import (
	"github.com/agusrichard/icu-vitals-stream/simulator/cmd/internal"
)

func main() {
	patient := internal.NewPatient("richard")
	done := make(chan struct{})
	go func() {
		defer close(done)
		patient.Run()
	}()
	<-done
}
