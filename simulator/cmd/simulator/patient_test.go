package main

import (
	"testing"

	"github.com/agusrichard/icu-vitals-stream/simulator/internal"
	"github.com/agusrichard/icu-vitals-stream/simulator/producer"
)

func newTestPatient() *Patient {
	return NewPatient(producer.NewNoopProducer())
}

// TestNewPatientStartsUninitialized verifies that a freshly created patient
// has not yet sampled any vitals.
func TestNewPatientStartsUninitialized(t *testing.T) {
	p := newTestPatient()
	if p.initialized {
		t.Error("expected initialized to be false on a new patient")
	}
}

// TestPatientFirstSampleInitializes verifies that the first sampleVitals call
// sets the initialized flag and populates currentVitals with the patient's own ID.
func TestPatientFirstSampleInitializes(t *testing.T) {
	p := newTestPatient()

	vitals := p.sampleVitals()

	if !p.initialized {
		t.Error("expected initialized to be true after first sampleVitals call")
	}
	if p.currentVitals.PatientID != p.ID {
		t.Errorf("currentVitals.PatientID = %q, want %q", p.currentVitals.PatientID, p.ID)
	}
	if vitals.PatientID != p.ID {
		t.Errorf("returned vitals.PatientID = %q, want %q", vitals.PatientID, p.ID)
	}
}

// TestPatientReturnedVitalsMatchCurrentVitals verifies that sampleVitals always
// returns the same value stored in currentVitals across multiple ticks.
func TestPatientReturnedVitalsMatchCurrentVitals(t *testing.T) {
	p := newTestPatient()

	for range 10 {
		returned := p.sampleVitals()
		if returned != p.currentVitals {
			t.Errorf("returned vitals do not match currentVitals after tick")
		}
	}
}

// TestPatientDriftUsesCurrentVitalsAsPrev verifies that after initialization,
// subsequent ticks produce readings that carry the patient's ID and a valid state,
// confirming DriftVitals is called rather than InitVitals.
func TestPatientDriftUsesCurrentVitalsAsPrev(t *testing.T) {
	p := newTestPatient()

	// Prime with a known extreme currentVitals so that drift from it
	// produces values far from what a fresh InitVitals would sample.
	p.initialized = true
	p.State = internal.SepticShock
	p.currentVitals = internal.VitalSigns{
		PatientID:          p.ID,
		SimulatorState:     internal.SepticShock,
		HeartRate:          20,
		RespirationRate:    4,
		OxygenSaturation:   60,
		Temperature:        32.0,
		SystolicBP:         40,
		ConsciousnessLevel: internal.Unresponsive,
	}

	vitals := p.sampleVitals()

	// After one drift tick from HR=20 toward SepticShock target (140),
	// HR must have moved upward — it cannot still be at 20 (which is the clamp floor).
	if vitals.HeartRate <= 20 {
		t.Errorf("expected HR to drift above 20 from extreme starting value, got %d", vitals.HeartRate)
	}
	if vitals.PatientID != p.ID {
		t.Errorf("vitals.PatientID = %q, want %q", vitals.PatientID, p.ID)
	}
}
