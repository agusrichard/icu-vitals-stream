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
		{DeterioratingSepsis, 0.955},
		{DeterioratingRespiratory, 0.977},
		{DeterioratingCardiac, 1.00},
	},
	DeterioratingSepsis: {
		{Stable, 0.01},
		{DeterioratingSepsis, 0.97},
		{DeterioratingRespiratory, 0.975},
		{DeterioratingCardiac, 0.980},
		{SepticShock, 1.00},
	},
	DeterioratingRespiratory: {
		{Stable, 0.01},
		{DeterioratingSepsis, 0.015},
		{DeterioratingRespiratory, 0.975},
		{DeterioratingCardiac, 0.980},
		{SepticShock, 1.00},
	},
	DeterioratingCardiac: {
		{Stable, 0.01},
		{DeterioratingSepsis, 0.015},
		{DeterioratingRespiratory, 0.020},
		{DeterioratingCardiac, 0.980},
		{SepticShock, 1.00},
	},
	PostOpRecovering: {
		{Stable, 0.04},
		{DeterioratingSepsis, 0.043},
		{DeterioratingRespiratory, 0.046},
		{DeterioratingCardiac, 0.048},
		{PostOpRecovering, 0.998},
		{SepticShock, 1.00},
	},
	SepticShock: {
		{Stable, 0.01},
		{DeterioratingSepsis, 0.060},
		{DeterioratingRespiratory, 0.065},
		{DeterioratingCardiac, 0.070},
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
