package main

import (
	"context"
	"encoding/json"
	"os"
	"time"

	"github.com/agusrichard/icu-vitals-stream/simulator/internal"
	"github.com/agusrichard/icu-vitals-stream/simulator/producer"
	"github.com/google/uuid"
)

type Patient struct {
	ID       string
	State    internal.SimulatorState
	producer producer.Producer
}

func NewPatient(p producer.Producer) *Patient {
	return &Patient{
		ID:       uuid.New().String(),
		State:    internal.Stable,
		producer: p,
	}
}

func (p *Patient) Run(ctx context.Context) {
	internal.LogInfo("Start running patient %s streaming...\n", p.ID)
	ticker := time.NewTicker(1 * time.Second)
	defer ticker.Stop()

	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "\t")
	for {
		select {
		case <-ticker.C:
			vitals := p.sampleVitals()
			if err := enc.Encode(vitals); err != nil {
				internal.LogErr(err, "encode error patient %s", p.ID)
				return
			}
			if err := p.producer.Send(vitals); err != nil {
					internal.LogErr(err, "send error patient %s", p.ID)
				}
		case <-ctx.Done():
			return
		}
	}
}

func (p *Patient) sampleVitals() internal.VitalSigns {
	p.State = internal.NextState(p.State)
	return internal.SampleVitals(p.ID, p.State)
}
