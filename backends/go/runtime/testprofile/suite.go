// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package testprofile

import "fmt"

// A test suite of the test profile as its runner reads it
// (docs/SCE_TEST_PROFILE.md §4.11, §11.7, §11.8, §12.2): the test cases in run
// order, each with its guard and the selection that runs it in a world of the
// suite's features, and the suite's module parameters; and the runner, which
// runs the cases a world selects one at a time, in that order.
//
// The tables are generated from the `test-case` and `test-suite` documents (a
// case's configuration from its `test-configuration`). They are data: what a
// case does is its components' machines, which the run drives (TestCaseRun).
// The runner is a library function, with no `main`: a generated or
// hand-written test calls it with the system adapters that play the SUT.
//
// Cross-language siblings: `sce_rust_runtime::test::run_suite` (Rust),
// `SCE::Test::runSuite` (C++), `com.sce.runtime.test.runSuite` (Kotlin).

// Features are the features a world of a suite's feature model enables
// (ADR 0057 §2): a configuration of the model, closed under its `requires`
// edges.
type Features map[string]bool

// FeaturesOf is the world that enables each of features and no other.
func FeaturesOf(features ...string) Features {
	world := make(Features, len(features))
	for _, feature := range features {
		world[feature] = true
	}
	return world
}

// Enabled reports whether world enables feature: the one test a selection is
// made of.
func Enabled(world Features, feature string) bool {
	return world[feature]
}

// TestCase is a `test-case` (§11.7): its name and what it runs in.
//
// The configuration and the machines of its components are its
// configuration's; the module parameters are the ids every component of the
// configuration reads, which the suite must give.
type TestCase struct {
	// Name is the test case's name in the suite and in the test log.
	Name string
	// Configuration is the configuration it runs.
	Configuration Configuration
	// Machines builds the machines of the configuration's component types.
	Machines MachineFactory
	// ModuleParameters are the ids of the module parameters its components
	// read.
	ModuleParameters []string
}

// Selection is a selection (§11.8): whether a world of the suite's features
// runs a test case. A condition of ADR 0057's grammar, lowered to the features
// it names.
type Selection func(world Features) bool

// Execute is one `<sce:execute>` (§11.8, P11): a test case, its guard, and its
// selection.
type Execute struct {
	// TestCase is the test case.
	TestCase TestCase
	// Guard is the test case's time limit, or NoGuard.
	Guard Guard
	// Selection is the selection, or nil for a test case every world runs.
	Selection Selection
}

// Suite is a `test-suite` (§11.8): its test cases in run order and its module
// parameters at their defaults.
type Suite struct {
	// Name is the suite's name.
	Name string
	// ModuleParameters are the module parameters at their defaults; nil is
	// none.
	ModuleParameters *ModuleParameters
	// Executes are the test cases, in run order.
	Executes []Execute
}

// CaseResult is what the runner reports of one test case it ran.
type CaseResult struct {
	// TestCase is the test case's name.
	TestCase string
	// Result is its verdict and log.
	Result TestCaseResult
}

// OverrideRefusal is an override of a module parameter the suite does not
// declare: the caller's mistake, not a verdict of any test case, so no case is
// run.
type OverrideRefusal struct {
	// Reason says why.
	Reason string
}

func (r *OverrideRefusal) Error() string {
	return r.Reason
}

// refused is P13: the test case cannot start, so it ends at once with
// `error`.
func refused(testCase, why string) CaseResult {
	return CaseResult{
		TestCase: testCase,
		Result: TestCaseResult{
			Verdict: VerdictError,
			EndedAt: 0,
			Log:     []LogRecord{{At: 0, Component: MTCName, Kind: LogError, Detail: why}},
		},
	}
}

// RunSuite runs the test cases of suite that world selects (P11).
//
// One at a time, on the calling goroutine, in the suite's order, each a fresh
// run from `NOW = 0` with fresh entities against the adapter adapters builds
// for it, given the test case's name, so that no case sees another's trace.
// Each component reads the suite's module parameters, with the values
// overrides gives in place of their defaults; nil overrides none.
//
// A test case whose components read a module parameter the run has no value
// for ends with `error` (P13) before any component is created, as does one for
// which adapters builds no adapter (a nil SystemAdapter).
//
// It answers an *OverrideRefusal, and runs no case, when overrides gives a
// module parameter the suite does not declare.
func RunSuite(
	suite Suite,
	world Features,
	adapters func(testCase string) SystemAdapter,
	overrides *ModuleParameters,
) ([]CaseResult, error) {
	parameters := suite.ModuleParameters.Copy()
	for _, id := range overrides.IDs() {
		if !parameters.Has(id) {
			return nil, &OverrideRefusal{Reason: fmt.Sprintf(
				"overrides the module parameter '%s', which the suite '%s' does not declare", id, suite.Name)}
		}
		value, _ := overrides.Of(id)
		parameters.Set(id, value)
	}
	var results []CaseResult
	for _, execute := range suite.Executes {
		if execute.Selection != nil && !execute.Selection(world) {
			continue
		}
		testCase := execute.TestCase
		missing, unread := "", false
		for _, id := range testCase.ModuleParameters {
			if !parameters.Has(id) {
				missing, unread = id, true
				break
			}
		}
		if unread {
			results = append(results, refused(testCase.Name, fmt.Sprintf(
				"the test case reads the module parameter '%s', which is given no value", missing)))
			continue
		}
		adapter := adapters(testCase.Name)
		if adapter == nil {
			results = append(results, refused(testCase.Name, "no system adapter is given for the test case"))
			continue
		}
		run := NewTestCaseRun(testCase.Configuration, testCase.Machines, adapter, execute.Guard, parameters)
		results = append(results, CaseResult{TestCase: testCase.Name, Result: run.Run()})
	}
	return results, nil
}
