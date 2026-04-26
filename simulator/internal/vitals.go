package internal

import (
	"math/rand"
	"time"
)

type VitalSigns struct {
	PatientID      string         `json:"patient_id"`
	Timestamp      time.Time      `json:"timestamp"`
	SimulatorState SimulatorState `json:"simulator_state"`

	RespirationRate    int                `json:"respiration_rate"`
	OxygenSaturation   int                `json:"oxygen_saturation"`
	SupplementalO2     bool               `json:"supplemental_o2"`
	Temperature        float64            `json:"temperature"`
	SystolicBP         int                `json:"systolic_bp"`
	HeartRate          int                `json:"heart_rate"`
	ConsciousnessLevel ConsciousnessLevel `json:"consciousness_level"`
}

func SampleVitals(patientID string, state SimulatorState) VitalSigns {
	return VitalSigns{
		PatientID:          patientID,
		Timestamp:          time.Now().UTC(),
		SimulatorState:     state,
		RespirationRate:    sampleRR(state),
		OxygenSaturation:   sampleSpO2(state),
		SupplementalO2:     sampleSupplementalO2(state),
		Temperature:        sampleTemp(state),
		SystolicBP:         sampleBP(state),
		HeartRate:          sampleHR(state),
		ConsciousnessLevel: sampleConsciousness(state),
	}
}

func sampleRR(state SimulatorState) int {
	switch state {
	case Stable:
		return randInt(12, 20)
	case DeterioratingSepsis:
		return randInt(20, 30)
	case DeterioratingRespiratory:
		return randInt(25, 35)
	case DeterioratingCardiac:
		return randInt(18, 26)
	case PostOpRecovering:
		return randInt(14, 22)
	case SepticShock:
		return randInt(25, 38)
	default:
		return randInt(12, 20)
	}
}

func sampleSpO2(state SimulatorState) int {
	switch state {
	case Stable:
		return randInt(95, 99)
	case DeterioratingSepsis:
		return randInt(93, 97)
	case DeterioratingRespiratory:
		return randInt(88, 93)
	case DeterioratingCardiac:
		return randInt(92, 96)
	case PostOpRecovering:
		return randInt(93, 97)
	case SepticShock:
		return randInt(85, 91)
	default:
		return randInt(95, 99)
	}
}

func sampleSupplementalO2(state SimulatorState) bool {
	switch state {
	case Stable:
		return false
	case DeterioratingSepsis:
		return rand.Float64() < 0.20
	case DeterioratingRespiratory:
		return true
	case DeterioratingCardiac:
		return rand.Float64() < 0.40
	case PostOpRecovering:
		return rand.Float64() < 0.30
	case SepticShock:
		return true
	default:
		return false
	}
}

func sampleTemp(state SimulatorState) float64 {
	switch state {
	case Stable:
		return randFloat(36.1, 37.2)
	case DeterioratingSepsis:
		return randFloat(38.5, 40.0)
	case DeterioratingRespiratory:
		return randFloat(36.5, 37.5)
	case DeterioratingCardiac:
		return randFloat(36.0, 37.0)
	case PostOpRecovering:
		return randFloat(36.8, 37.8)
	case SepticShock:
		return randFloat(38.5, 41.0)
	default:
		return randFloat(36.1, 37.2)
	}
}

func sampleBP(state SimulatorState) int {
	switch state {
	case Stable:
		return randInt(110, 130)
	case DeterioratingSepsis:
		return randInt(85, 105)
	case DeterioratingRespiratory:
		return randInt(105, 125)
	case DeterioratingCardiac:
		return randInt(80, 100)
	case PostOpRecovering:
		return randInt(100, 120)
	case SepticShock:
		return randInt(60, 85)
	default:
		return randInt(110, 130)
	}
}

func sampleHR(state SimulatorState) int {
	switch state {
	case Stable:
		return randInt(60, 80)
	case DeterioratingSepsis:
		return randInt(100, 140)
	case DeterioratingRespiratory:
		return randInt(90, 120)
	case DeterioratingCardiac:
		return randInt(100, 150)
	case PostOpRecovering:
		return randInt(75, 95)
	case SepticShock:
		return randInt(120, 160)
	default:
		return randInt(60, 80)
	}
}

func sampleConsciousness(state SimulatorState) ConsciousnessLevel {
	r := rand.Float64()
	switch state {
	case Stable, PostOpRecovering:
		return Alert
	case DeterioratingSepsis:
		// NewConfusion is the earliest and most important sepsis marker
		switch {
		case r < 0.60:
			return Alert
		case r < 0.85:
			return NewConfusion
		default:
			return Voice
		}
	case DeterioratingRespiratory, DeterioratingCardiac:
		// Hypoxia and low cerebral perfusion cause confusion before full unresponsiveness
		switch {
		case r < 0.70:
			return Alert
		case r < 0.90:
			return NewConfusion
		default:
			return Voice
		}
	case SepticShock:
		// Too severe for mere confusion — cardiovascular collapse impairs brain perfusion deeply
		switch {
		case r < 0.33:
			return Voice
		case r < 0.66:
			return Pain
		default:
			return Unresponsive
		}
	default:
		return Alert
	}
}

func randInt(min, max int) int {
	return rand.Intn(max-min+1) + min
}

func randFloat(min, max float64) float64 {
	return rand.Float64()*(max-min) + min
}
