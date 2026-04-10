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

	fdb "github.com/newmassrael/sce-forge-runtime/conformance/generated/filter_debounce"
	fma "github.com/newmassrael/sce-forge-runtime/conformance/generated/filter_moving_average"
	i1d "github.com/newmassrael/sce-forge-runtime/conformance/generated/interpolation_1d_linear"
	i2d "github.com/newmassrael/sce-forge-runtime/conformance/generated/interpolation_2d_bilinear"
	obs "github.com/newmassrael/sce-forge-runtime/conformance/generated/observer_coolant"
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

type pureCase struct {
	Args     []int64         `json:"args"`
	Expected json.RawMessage `json:"expected"`
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

func TestInterpolation1dLinear(t *testing.T) {
	ref := loadReference(t)
	spec := ref.PureFunctions["interpolation_1d_linear"]
	for _, c := range spec.Cases {
		rpm := uint16(c.Args[0])
		var expected float64
		if err := json.Unmarshal(c.Expected, &expected); err != nil {
			t.Fatalf("parse expected: %v", err)
		}
		actual := i1d.Lookup(rpm)
		assertClose(t, actual, expected, ref.FloatTolerance, "interpolation_1d_linear")
	}
}

func TestInterpolation2dBilinear(t *testing.T) {
	ref := loadReference(t)
	spec := ref.PureFunctions["interpolation_2d_bilinear"]
	for _, c := range spec.Cases {
		rpm := uint16(c.Args[0])
		load := uint8(c.Args[1])
		var expected float64
		if err := json.Unmarshal(c.Expected, &expected); err != nil {
			t.Fatalf("parse expected: %v", err)
		}
		actual := i2d.Lookup(rpm, load)
		assertClose(t, actual, expected, ref.FloatTolerance, "interpolation_2d_bilinear")
	}
}

func TestFilterMovingAverage(t *testing.T) {
	ref := loadReference(t)
	spec := ref.StatefulFilters["filter_moving_average"]
	filt := fma.NewFilterMovingAverage()
	for i, step := range spec.Sequence {
		var input, expected float64
		if err := json.Unmarshal(step.Input, &input); err != nil {
			t.Fatalf("step %d: parse input: %v", i, err)
		}
		if err := json.Unmarshal(step.Expected, &expected); err != nil {
			t.Fatalf("step %d: parse expected: %v", i, err)
		}
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
		if err := json.Unmarshal(step.Input, &input); err != nil {
			t.Fatalf("step %d: parse input: %v", i, err)
		}
		if err := json.Unmarshal(step.Expected, &expected); err != nil {
			t.Fatalf("step %d: parse expected: %v", i, err)
		}
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
		if err := json.Unmarshal(step.Input, &input); err != nil {
			t.Fatalf("step %d: parse input: %v", i, err)
		}
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
