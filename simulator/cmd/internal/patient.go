package internal

import (
	"encoding/json"
	"os"
	"time"
)

type Patient struct {
	ID string
}

func NewPatient(id string) *Patient {
	return &Patient{
		ID: id,
	}
}

func (p *Patient) Run() {
	ticker := time.NewTicker(1 * time.Second)
	defer ticker.Stop()

	enc := json.NewEncoder(os.Stdout)
	end := time.After(1 * time.Minute)
	for {
		select {
		case <-ticker.C:
			enc.Encode(p.templateVitals())
		case <-end:
			return
		}
	}
}

func (p *Patient) templateVitals() VitalSigns {
	return VitalSigns{
		PatientID:          p.ID,
		Timestamp:          time.Now().UTC(),
		RespirationRate:    15,
		OxygenSaturation:   98,
		SupplementalO2:     false,
		Temperature:        37.0,
		SystolicBP:         120,
		HeartRate:          72,
		ConsciousnessLevel: Alert,
		SimulatorState:     Stable,
	}
}
