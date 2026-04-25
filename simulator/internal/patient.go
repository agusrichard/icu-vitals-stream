package internal

import (
	"encoding/json"
	"fmt"
	"os"
	"time"

	"github.com/google/uuid"
)

type Patient struct {
	ID    string
	State SimulatorState
}

func NewPatient() *Patient {
	return &Patient{
		ID:    uuid.New().String(),
		State: Stable,
	}
}

func (p *Patient) Run() {
	fmt.Fprintf(os.Stderr, "Start running patient %s streaming...\n", p.ID)
	ticker := time.NewTicker(1 * time.Second)
	defer ticker.Stop()

	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "\t")
	end := time.After(1 * time.Minute)
	for {
		select {
		case <-ticker.C:
			fmt.Fprintf(os.Stderr, "==> Patient %s parameters...\n", p.ID)
			if err := enc.Encode(p.sampleVitals()); err != nil {
				fmt.Fprintf(os.Stderr, "encode error patient %s: %v\n", p.ID, err)
				return
			}
		case <-end:
			return
		}
	}
}

func (p *Patient) sampleVitals() VitalSigns {
	p.State = NextState(p.State)
	return sampleVitals(p.ID, p.State)
}
