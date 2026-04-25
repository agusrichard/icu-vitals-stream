package main

import (
	"fmt"
	"time"
)

func printOK(done chan struct{}) {
	ticker := time.NewTicker(1 * time.Second)
	defer ticker.Stop()

	end := time.After(1 * time.Minute)

	for {
		select {
		case <-ticker.C:
			fmt.Println("ok!")
		case <-end:
			done <- struct{}{}
			return
		}
	}
}

func main() {
	done := make(chan struct{})
	go printOK(done)
	<-done
}
