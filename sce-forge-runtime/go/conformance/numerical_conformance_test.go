// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Cross-language numerical conformance harness (Go half).
//
// The fixture packages imported below are generated before this test runs by
// ./generate.sh, which invokes sce-codegen on the SCXML files under
// tests/forge/resources/ and writes one Go package per fixture into
// generated/<name>/<name>.go. Run `go generate ./conformance/...` (or
// `make -C conformance test`) to regenerate them. Each fixture subdirectory
// is gitignored — the single source of truth is the SCXML, not a committed
// copy of the Go output.
//
// Once the fixtures exist, this test runs them against the reference vectors
// in tests/forge/conformance/numerical_reference.json, the same file used by
// the Rust, C++, Python, and Kotlin conformance tests.

//go:generate ./generate.sh

package conformance

import (
	"encoding/json"
	"math"
	"os"
	"path/filepath"
	"runtime"
	"testing"

	cprog "github.com/newmassrael/sce-forge-runtime/conformance/generated/condition_programming"
	crng "github.com/newmassrael/sce-forge-runtime/conformance/generated/condition_range"
	cthr "github.com/newmassrael/sce-forge-runtime/conformance/generated/condition_threshold"
	fdb "github.com/newmassrael/sce-forge-runtime/conformance/generated/filter_debounce"
	fma "github.com/newmassrael/sce-forge-runtime/conformance/generated/filter_moving_average"
	i1d "github.com/newmassrael/sce-forge-runtime/conformance/generated/interpolation_1d_linear"
	i2d "github.com/newmassrael/sce-forge-runtime/conformance/generated/interpolation_2d_bilinear"
	obs "github.com/newmassrael/sce-forge-runtime/conformance/generated/observer_coolant"
	pdiam "github.com/newmassrael/sce-forge-runtime/conformance/generated/procedure_diamond"
	plin "github.com/newmassrael/sce-forge-runtime/conformance/generated/procedure_linear"
	pstart "github.com/newmassrael/sce-forge-runtime/conformance/generated/procedure_startup_check"
	tbit "github.com/newmassrael/sce-forge-runtime/conformance/generated/transform_bitwise"
	tmul "github.com/newmassrael/sce-forge-runtime/conformance/generated/transform_multi_output"
	ttemp "github.com/newmassrael/sce-forge-runtime/conformance/generated/transform_temperature"
)

type reference struct {
	Version         int                     `json:"version"`
	FloatTolerance  float64                 `json:"float_tolerance"`
	PureFunctions   map[string]pureSpec     `json:"pure_functions"`
	StatefulFilters map[string]sequenceSpec `json:"stateful_filters"`
	Observers       map[string]sequenceSpec `json:"observers"`
}

type pureSpec struct {
	Cases []pureCase `json:"cases"`
}

// pureCase keeps args and expected as RawMessage so each test can decode them
// with the type the generated Go function actually takes (args may hold ints,
// floats, bools, or strings depending on the fixture; expected may be a scalar
// or a compound JSON object).
type pureCase struct {
	Args     []json.RawMessage `json:"args"`
	Expected json.RawMessage   `json:"expected"`
}

type sequenceSpec struct {
	Sequence []sequenceStep `json:"sequence"`
}

type sequenceStep struct {
	Input          json.RawMessage `json:"input"`
	Expected       json.RawMessage `json:"expected"`
	ExpectedEvents []string        `json:"expected_events"`
}

func repoRoot(t *testing.T) string {
	_, thisFile, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("runtime.Caller failed")
	}
	// thisFile: <root>/sce-forge-runtime/go/conformance/numerical_conformance_test.go
	return filepath.Clean(filepath.Join(filepath.Dir(thisFile), "..", "..", ".."))
}

func loadReference(t *testing.T) reference {
	t.Helper()
	path := filepath.Join(repoRoot(t), "tests", "forge", "conformance", "numerical_reference.json")
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read reference: %v", err)
	}
	var ref reference
	if err := json.Unmarshal(data, &ref); err != nil {
		t.Fatalf("parse reference: %v", err)
	}
	return ref
}

func assertClose(t *testing.T, actual, expected, tol float64, label string) {
	t.Helper()
	if diff := math.Abs(actual - expected); diff > tol {
		t.Errorf("%s: actual=%v expected=%v diff=%v tol=%v", label, actual, expected, diff, tol)
	}
}

func unmarshalOrFatal(t *testing.T, raw json.RawMessage, dst interface{}, label string) {
	t.Helper()
	if err := json.Unmarshal(raw, dst); err != nil {
		t.Fatalf("%s: %v", label, err)
	}
}

func TestInterpolation1dLinear(t *testing.T) {
	ref := loadReference(t)
	spec := ref.PureFunctions["interpolation_1d_linear"]
	for _, c := range spec.Cases {
		var rpm uint16
		unmarshalOrFatal(t, c.Args[0], &rpm, "interpolation_1d_linear rpm")
		var expected float64
		unmarshalOrFatal(t, c.Expected, &expected, "interpolation_1d_linear expected")
		actual := i1d.Lookup(rpm)
		assertClose(t, actual, expected, ref.FloatTolerance, "interpolation_1d_linear")
	}
}

func TestInterpolation2dBilinear(t *testing.T) {
	ref := loadReference(t)
	spec := ref.PureFunctions["interpolation_2d_bilinear"]
	for _, c := range spec.Cases {
		var rpm uint16
		var load uint8
		unmarshalOrFatal(t, c.Args[0], &rpm, "interpolation_2d_bilinear rpm")
		unmarshalOrFatal(t, c.Args[1], &load, "interpolation_2d_bilinear load")
		var expected float64
		unmarshalOrFatal(t, c.Expected, &expected, "interpolation_2d_bilinear expected")
		actual := i2d.Lookup(rpm, load)
		assertClose(t, actual, expected, ref.FloatTolerance, "interpolation_2d_bilinear")
	}
}

func TestFilterMovingAverage(t *testing.T) {
	ref := loadReference(t)
	spec := ref.StatefulFilters["filter_moving_average"]
	filt := fma.NewFilterMovingAverage()
	for _, step := range spec.Sequence {
		var input, expected float64
		unmarshalOrFatal(t, step.Input, &input, "filter_moving_average input")
		unmarshalOrFatal(t, step.Expected, &expected, "filter_moving_average expected")
		actual := filt.Update(input)
		assertClose(t, actual, expected, ref.FloatTolerance, "filter_moving_average")
	}
}

func TestFilterDebounce(t *testing.T) {
	ref := loadReference(t)
	spec := ref.StatefulFilters["filter_debounce"]
	filt := fdb.NewFilterDebounce()
	for i, step := range spec.Sequence {
		var input, expected bool
		unmarshalOrFatal(t, step.Input, &input, "filter_debounce input")
		unmarshalOrFatal(t, step.Expected, &expected, "filter_debounce expected")
		actual := filt.Update(input)
		if actual != expected {
			t.Errorf("filter_debounce step %d input=%v: actual=%v expected=%v",
				i, input, actual, expected)
		}
	}
}

func coolantTagName(tag obs.ForgeDomainTag) string {
	switch tag {
	case obs.ForgeDomainTagEmitWarning:
		return "EMIT_WARNING"
	case obs.ForgeDomainTagClearWarning:
		return "CLEAR_WARNING"
	case obs.ForgeDomainTagEmergencyShutdown:
		return "EMERGENCY_SHUTDOWN"
	default:
		return "unknown"
	}
}

func TestObserverCoolant(t *testing.T) {
	ref := loadReference(t)
	spec := ref.Observers["observer_coolant"]
	var observer obs.ObserverCoolant
	for i, step := range spec.Sequence {
		var input float64
		unmarshalOrFatal(t, step.Input, &input, "observer_coolant input")
		queue := observer.Update(input)
		actual := make([]string, 0, queue.Len())
		for _, tag := range queue.AsSlice() {
			actual = append(actual, coolantTagName(tag))
		}
		if len(actual) != len(step.ExpectedEvents) {
			t.Errorf("observer_coolant step %d input=%v: got %v expected %v",
				i, input, actual, step.ExpectedEvents)
			continue
		}
		for j := range actual {
			if actual[j] != step.ExpectedEvents[j] {
				t.Errorf("observer_coolant step %d input=%v: got %v expected %v",
					i, input, actual, step.ExpectedEvents)
				break
			}
		}
	}
}

func TestTransformTemperature(t *testing.T) {
	ref := loadReference(t)
	spec := ref.PureFunctions["transform_temperature"]
	for _, c := range spec.Cases {
		var raw uint16
		unmarshalOrFatal(t, c.Args[0], &raw, "transform_temperature raw")
		var expected float64
		unmarshalOrFatal(t, c.Expected, &expected, "transform_temperature expected")
		actual := ttemp.ComputeTemperature(raw)
		assertClose(t, actual, expected, ref.FloatTolerance, "transform_temperature")
	}
}

func TestTransformBitwise(t *testing.T) {
	ref := loadReference(t)
	spec := ref.PureFunctions["transform_bitwise"]
	for _, c := range spec.Cases {
		var byteVal uint8
		unmarshalOrFatal(t, c.Args[0], &byteVal, "transform_bitwise byte")
		var expected struct {
			HighNibble uint8 `json:"high_nibble"`
			LowNibble  uint8 `json:"low_nibble"`
		}
		unmarshalOrFatal(t, c.Expected, &expected, "transform_bitwise expected")
		actualHigh := tbit.ComputeHighNibble(byteVal)
		actualLow := tbit.ComputeLowNibble(byteVal)
		if actualHigh != expected.HighNibble {
			t.Errorf("transform_bitwise(%d) high: actual=%d expected=%d", byteVal, actualHigh, expected.HighNibble)
		}
		if actualLow != expected.LowNibble {
			t.Errorf("transform_bitwise(%d) low: actual=%d expected=%d", byteVal, actualLow, expected.LowNibble)
		}
	}
}

func TestTransformMultiOutput(t *testing.T) {
	ref := loadReference(t)
	spec := ref.PureFunctions["transform_multi_output"]
	for _, c := range spec.Cases {
		var celsius float64
		unmarshalOrFatal(t, c.Args[0], &celsius, "transform_multi_output celsius")
		var expected struct {
			Fahrenheit float64 `json:"fahrenheit"`
			Kelvin     float64 `json:"kelvin"`
		}
		unmarshalOrFatal(t, c.Expected, &expected, "transform_multi_output expected")
		actualF := tmul.ComputeFahrenheit(celsius)
		actualK := tmul.ComputeKelvin(celsius)
		assertClose(t, actualF, expected.Fahrenheit, ref.FloatTolerance, "transform_multi_output fahrenheit")
		assertClose(t, actualK, expected.Kelvin, ref.FloatTolerance, "transform_multi_output kelvin")
	}
}

func TestConditionThreshold(t *testing.T) {
	ref := loadReference(t)
	spec := ref.PureFunctions["condition_threshold"]
	for _, c := range spec.Cases {
		var coolant, oil, maxTemp float64
		unmarshalOrFatal(t, c.Args[0], &coolant, "condition_threshold coolant")
		unmarshalOrFatal(t, c.Args[1], &oil, "condition_threshold oil")
		unmarshalOrFatal(t, c.Args[2], &maxTemp, "condition_threshold max")
		var expected bool
		unmarshalOrFatal(t, c.Expected, &expected, "condition_threshold expected")
		actual := cthr.ConditionThreshold(coolant, oil, maxTemp)
		if actual != expected {
			t.Errorf("condition_threshold(%v,%v,%v): actual=%v expected=%v", coolant, oil, maxTemp, actual, expected)
		}
	}
}

func TestConditionRange(t *testing.T) {
	ref := loadReference(t)
	spec := ref.PureFunctions["condition_range"]
	for _, c := range spec.Cases {
		var rpm, minRpm, maxRpm uint32
		unmarshalOrFatal(t, c.Args[0], &rpm, "condition_range rpm")
		unmarshalOrFatal(t, c.Args[1], &minRpm, "condition_range minRpm")
		unmarshalOrFatal(t, c.Args[2], &maxRpm, "condition_range maxRpm")
		var expected bool
		unmarshalOrFatal(t, c.Expected, &expected, "condition_range expected")
		actual := crng.ConditionRange(rpm, minRpm, maxRpm)
		if actual != expected {
			t.Errorf("condition_range(%d,%d,%d): actual=%v expected=%v", rpm, minRpm, maxRpm, actual, expected)
		}
	}
}

func TestConditionProgramming(t *testing.T) {
	ref := loadReference(t)
	spec := ref.PureFunctions["condition_programming"]
	for _, c := range spec.Cases {
		var engineStop, ignition bool
		unmarshalOrFatal(t, c.Args[0], &engineStop, "condition_programming engineStop")
		unmarshalOrFatal(t, c.Args[1], &ignition, "condition_programming ignition")
		var expected bool
		unmarshalOrFatal(t, c.Expected, &expected, "condition_programming expected")
		actual := cprog.ConditionProgramming(engineStop, ignition)
		if actual != expected {
			t.Errorf("condition_programming(%v,%v): actual=%v expected=%v", engineStop, ignition, actual, expected)
		}
	}
}

type procedureExpected struct {
	Completed  bool   `json:"completed"`
	FinalState string `json:"final_state"`
}

func TestProcedureLinear(t *testing.T) {
	ref := loadReference(t)
	spec := ref.PureFunctions["procedure_linear"]
	for _, c := range spec.Cases {
		var value int32
		unmarshalOrFatal(t, c.Args[0], &value, "procedure_linear value")
		var expected procedureExpected
		unmarshalOrFatal(t, c.Expected, &expected, "procedure_linear expected")
		result := plin.Execute(value)
		if result.Completed != expected.Completed || result.FinalState != expected.FinalState {
			t.Errorf("procedure_linear(%d): actual={%v,%q} expected={%v,%q}",
				value, result.Completed, result.FinalState, expected.Completed, expected.FinalState)
		}
	}
}

func TestProcedureDiamond(t *testing.T) {
	ref := loadReference(t)
	spec := ref.PureFunctions["procedure_diamond"]
	for _, c := range spec.Cases {
		var sensorValue uint16
		var mode string
		unmarshalOrFatal(t, c.Args[0], &sensorValue, "procedure_diamond sensor_value")
		unmarshalOrFatal(t, c.Args[1], &mode, "procedure_diamond mode")
		var expected procedureExpected
		unmarshalOrFatal(t, c.Expected, &expected, "procedure_diamond expected")
		result := pdiam.Execute(sensorValue, mode)
		if result.Completed != expected.Completed || result.FinalState != expected.FinalState {
			t.Errorf("procedure_diamond(%d,%q): actual={%v,%q} expected={%v,%q}",
				sensorValue, mode, result.Completed, result.FinalState, expected.Completed, expected.FinalState)
		}
	}
}

func TestProcedureStartupCheck(t *testing.T) {
	ref := loadReference(t)
	spec := ref.PureFunctions["procedure_startup_check"]
	for _, c := range spec.Cases {
		var voltage, temperature float32
		unmarshalOrFatal(t, c.Args[0], &voltage, "procedure_startup_check voltage")
		unmarshalOrFatal(t, c.Args[1], &temperature, "procedure_startup_check temperature")
		var expected procedureExpected
		unmarshalOrFatal(t, c.Expected, &expected, "procedure_startup_check expected")
		result := pstart.Execute(voltage, temperature)
		if result.Completed != expected.Completed || result.FinalState != expected.FinalState {
			t.Errorf("procedure_startup_check(%v,%v): actual={%v,%q} expected={%v,%q}",
				voltage, temperature, result.Completed, result.FinalState, expected.Completed, expected.FinalState)
		}
	}
}
