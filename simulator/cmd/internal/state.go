package internal

import "math/rand"

type SimulatorState string

const (
	Stable                   SimulatorState = "STABLE"
	DeterioratingSepsis      SimulatorState = "DETERIORATING_SEPSIS"
	DeterioratingRespiratory SimulatorState = "DETERIORATING_RESPIRATORY"
	DeterioratingCardiac     SimulatorState = "DETERIORATING_CARDIAC"
	PostOpRecovering         SimulatorState = "POST_OP_RECOVERING"
	SepticShock              SimulatorState = "SEPTIC_SHOCK"
)

type ConsciousnessLevel string

const (
	Alert        ConsciousnessLevel = "ALERT"
	NewConfusion ConsciousnessLevel = "NEW_CONFUSION"
	Voice        ConsciousnessLevel = "VOICE"
	Pain         ConsciousnessLevel = "PAIN"
	Unresponsive ConsciousnessLevel = "UNRESPONSIVE"
)

type weightedTransition struct {
	state                 SimulatorState
	cumulativeProbability float64
}

var transitions = map[SimulatorState][]weightedTransition{
	Stable: {
		{Stable, 0.93},
		{DeterioratingSepsis, 0.95},
		{DeterioratingRespiratory, 0.97},
		{DeterioratingCardiac, 0.98},
		{PostOpRecovering, 0.99},
		{SepticShock, 1.00},
	},
	DeterioratingSepsis: {
		{Stable, 0.08},
		{DeterioratingSepsis, 0.88},
		{DeterioratingRespiratory, 0.91},
		{DeterioratingCardiac, 0.93},
		{PostOpRecovering, 0.95},
		{SepticShock, 1.00},
	},
	DeterioratingRespiratory: {
		{Stable, 0.08},
		{DeterioratingSepsis, 0.11},
		{DeterioratingRespiratory, 0.93},
		{DeterioratingCardiac, 0.97},
		{PostOpRecovering, 0.99},
		{SepticShock, 1.00},
	},
	DeterioratingCardiac: {
		{Stable, 0.08},
		{DeterioratingSepsis, 0.10},
		{DeterioratingRespiratory, 0.14},
		{DeterioratingCardiac, 0.96},
		{PostOpRecovering, 0.98},
		{SepticShock, 1.00},
	},
	PostOpRecovering: {
		{Stable, 0.20},
		{DeterioratingSepsis, 0.24},
		{DeterioratingRespiratory, 0.28},
		{DeterioratingCardiac, 0.32},
		{PostOpRecovering, 0.99},
		{SepticShock, 1.00},
	},
	SepticShock: {
		{Stable, 0.02},
		{DeterioratingSepsis, 0.17},
		{DeterioratingRespiratory, 0.22},
		{DeterioratingCardiac, 0.27},
		{PostOpRecovering, 0.30},
		{SepticShock, 1.00},
	},
}

func NextState(current SimulatorState) SimulatorState {
	r := rand.Float64()
	for _, t := range transitions[current] {
		if r < t.cumulativeProbability {
			return t.state
		}
	}

	return current
}
