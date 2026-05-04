package internal

import (
	"testing"
)

// buildStableVitals returns a VitalSigns snapshot at the centre of the Stable range,
// used as the starting point for drift tests.
func buildStableVitals() VitalSigns {
	return VitalSigns{
		PatientID:          "test-patient",
		SimulatorState:     Stable,
		RespirationRate:    16,
		OxygenSaturation:   97,
		SupplementalO2:     false,
		Temperature:        36.65,
		SystolicBP:         120,
		HeartRate:          70,
		ConsciousnessLevel: Alert,
	}
}

// TestDriftConvergence verifies that after 30 ticks drifting toward SepticShock,
// HR and SBP are measurably closer to the SepticShock target than the Stable starting value.
func TestDriftConvergence(t *testing.T) {
	vitals := buildStableVitals()

	for range 30 {
		vitals = DriftVitals("test-patient", SepticShock, vitals)
	}

	hrTarget := stateTargetCenters[SepticShock].heartRate
	hrStart := 70.0
	hrDistanceAfter := abs(float64(vitals.HeartRate) - hrTarget)
	hrDistanceBefore := abs(hrStart - hrTarget)
	if hrDistanceAfter >= hrDistanceBefore {
		t.Errorf("HR did not converge toward SepticShock target %.1f: started %.1f apart, now %.1f apart",
			hrTarget, hrDistanceBefore, hrDistanceAfter)
	}

	sbpTarget := stateTargetCenters[SepticShock].systolicBP
	sbpStart := 120.0
	sbpDistanceAfter := abs(float64(vitals.SystolicBP) - sbpTarget)
	sbpDistanceBefore := abs(sbpStart - sbpTarget)
	if sbpDistanceAfter >= sbpDistanceBefore {
		t.Errorf("SBP did not converge toward SepticShock target %.1f: started %.1f apart, now %.1f apart",
			sbpTarget, sbpDistanceBefore, sbpDistanceAfter)
	}
}

// TestDriftClamp verifies that no parameter ever escapes its physiological bounds,
// even after 100 ticks starting from extreme near-boundary values.
func TestDriftClamp(t *testing.T) {
	extreme := VitalSigns{
		PatientID:          "test-patient",
		SimulatorState:     SepticShock,
		RespirationRate:    5,
		OxygenSaturation:   61,
		Temperature:        32.1,
		SystolicBP:         41,
		HeartRate:          21,
		ConsciousnessLevel: Unresponsive,
	}

	for range 100 {
		extreme = DriftVitals("test-patient", SepticShock, extreme)

		if extreme.HeartRate < 20 || extreme.HeartRate > 220 {
			t.Errorf("HeartRate %d out of bounds [20, 220]", extreme.HeartRate)
		}
		if extreme.RespirationRate < 4 || extreme.RespirationRate > 60 {
			t.Errorf("RespirationRate %d out of bounds [4, 60]", extreme.RespirationRate)
		}
		if extreme.OxygenSaturation < 60 || extreme.OxygenSaturation > 100 {
			t.Errorf("OxygenSaturation %d out of bounds [60, 100]", extreme.OxygenSaturation)
		}
		if extreme.Temperature < 32.0 || extreme.Temperature > 43.0 {
			t.Errorf("Temperature %.2f out of bounds [32.0, 43.0]", extreme.Temperature)
		}
		if extreme.SystolicBP < 40 || extreme.SystolicBP > 260 {
			t.Errorf("SystolicBP %d out of bounds [40, 260]", extreme.SystolicBP)
		}
	}
}

// TestConsciousnessStableAlwaysAlert verifies that consciousness never changes from Alert
// when drifting in Stable state, since sampleConsciousness always returns Alert for Stable.
func TestConsciousnessStableAlwaysAlert(t *testing.T) {
	vitals := buildStableVitals()

	for range 100 {
		vitals = DriftVitals("test-patient", Stable, vitals)
		if vitals.ConsciousnessLevel != Alert {
			t.Errorf("expected Alert in Stable state, got %s", vitals.ConsciousnessLevel)
		}
	}
}

// TestConsciousnessChangeRate verifies the 15% gate produces a realistic change rate
// over 200 ticks in DeterioratingSepsis — expected ~30 changes, accepted range [10, 70].
func TestConsciousnessChangeRate(t *testing.T) {
	vitals := buildStableVitals()
	vitals.SimulatorState = DeterioratingSepsis

	changes := 0
	for range 200 {
		prev := vitals.ConsciousnessLevel
		vitals = DriftVitals("test-patient", DeterioratingSepsis, vitals)
		if vitals.ConsciousnessLevel != prev {
			changes++
		}
	}

	if changes < 10 || changes > 70 {
		t.Errorf("consciousness changed %d times in 200 ticks; expected between 10 and 70", changes)
	}
}

func abs(x float64) float64 {
	if x < 0 {
		return -x
	}
	return x
}
