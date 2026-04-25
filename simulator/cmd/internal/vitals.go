package internal

import "time"

type SimulatorState string

const (
	Stable        SimulatorState = "STABLE"
	Deteriorating SimulatorState = "DETERIORATING"
	Critical      SimulatorState = "CRITICAL"
)

type ConsciousnessLevel string

const (
	Alert        ConsciousnessLevel = "ALERT"
	Voice        ConsciousnessLevel = "VOICE"
	Pain         ConsciousnessLevel = "PAIN"
	Unresponsive ConsciousnessLevel = "UNRESPONSIVE"
)

type VitalSigns struct {
	PatientID      string         `json:"patient_id"`
	Timestamp      time.Time      `json:"timestamp"`
	SimulatorState SimulatorState `json:"simulator_state"`

	// 7 NEWS2 parameters
	RespirationRate    int                `json:"respiration_rate"`
	OxygenSaturation   int                `json:"oxygen_saturation"`
	SupplementalO2     bool               `json:"supplemental_o2"`
	Temperature        float64            `json:"temperature"`
	SystolicBP         int                `json:"systolic_bp"`
	HeartRate          int                `json:"heart_rate"`
	ConsciousnessLevel ConsciousnessLevel `json:"consciousness_level"`
}
