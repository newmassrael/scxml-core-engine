// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// docs/SCE_TTCN3_LANE.md §12, §13.1 S2: every trace of a suite of the corpus
// replays, through the suite's runner (testprofile.RunRunner, the program of §4
// run in place), to its expected log, byte for byte. The suites are the ones
// sce-codegen writes into a package each beside this file (regen:
// `scripts/regen_static_datamodel_go.sh`); runtime_suite's own traces are
// replayed by a_test_suite_runs_through_its_runner_test.go, beside the rest of
// what that suite is held to.
//
// The expected logs are the files every backend is held to, the canonical logs
// a reviewed run of the Rust runner wrote. Each test is the twin of the Rust
// case of the same name in
// backends/rust/tests/tests/the_corpus_replays_to_its_expected_logs.rs.

package static_datamodel

import (
	"bytes"
	"os"
	"path/filepath"
	"reflect"
	"sort"
	"strings"
	"testing"

	"github.com/newmassrael/sce-go-runtime/testprofile"

	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/boundary_suite"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/job_suite"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/nested_suite"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/orders_suite"
)

// corpusReplayed is a suite of the corpus and how its runner is run over its
// traces.
type corpusReplayed struct {
	// suite is the suite's unit name, which names the directory of its runs.
	suite string
	// tables is the suite's table, which the runner runs.
	tables testprofile.Suite
	// parameter reads a module parameter's value from the command line.
	parameter testprofile.ParameterReader
	// world selects every test case, as `--world` takes it.
	world string
	// verdicts is what the runner prints: each test case its world selects, in
	// the suite's order, with its verdict.
	verdicts string
}

// corpusRunsDir is the directory of the runs of suite: its `traces/` and
// `expected_logs/`.
func corpusRunsDir(suite string) string {
	return filepath.Join("..", "..", "..", "..", "..", "sce-build", "tests", "fixtures", "test_profile", "runs",
		suite)
}

// corpusFileNames are the names of the files in dir, sorted.
func corpusFileNames(t *testing.T, dir string) []string {
	t.Helper()
	entries, err := os.ReadDir(dir)
	if err != nil {
		t.Fatalf("the directory %s is read: %v", dir, err)
	}
	names := make([]string, 0, len(entries))
	for _, entry := range entries {
		names = append(names, entry.Name())
	}
	sort.Strings(names)
	return names
}

// corpusReplays runs the suite's runner over every trace of its directory, in
// the world that selects every test case, and holds what it prints to the
// verdicts it expects, every log it writes to its expected log, and the set of
// logs to the set of expected logs.
func corpusReplays(t *testing.T, replayed corpusReplayed) {
	t.Helper()
	runs := corpusRunsDir(replayed.suite)
	logs := filepath.Join(t.TempDir(), "logs")
	var out, errs bytes.Buffer
	line := []string{"--trace-dir", filepath.Join(runs, "traces"), "--log-dir", logs, "--world", replayed.world}
	if status := testprofile.RunRunner(replayed.tables, replayed.parameter, line, &out, &errs); status != 0 {
		t.Fatalf("status %d: %s", status, errs.String())
	}
	if out.String() != replayed.verdicts {
		t.Errorf("the standard output is %q: %s", out.String(), errs.String())
	}
	expectedDir := filepath.Join(runs, "expected_logs")
	var expected []string
	for _, name := range corpusFileNames(t, expectedDir) {
		testCase, ok := strings.CutSuffix(name, ".json")
		if !ok {
			t.Errorf("the expected log %s is a .json file", name)
		}
		expected = append(expected, testCase)
	}
	var written []string
	for _, name := range corpusFileNames(t, logs) {
		testCase, ok := strings.CutSuffix(name, ".log.json")
		written = append(written, testCase)
		if !ok {
			t.Errorf("the runner wrote %s, which is no log of a case", name)
			continue
		}
		raw, err := os.ReadFile(filepath.Join(expectedDir, testCase+".json"))
		if err != nil {
			t.Errorf("the case %s has no expected log: %v", testCase, err)
			continue
		}
		want, ended := strings.CutSuffix(string(raw), "\n")
		if !ended {
			t.Errorf("the expected log of %s ends in one newline", testCase)
		}
		got, err := os.ReadFile(filepath.Join(logs, name))
		if err != nil {
			t.Fatalf("the log of %s is read: %v", testCase, err)
		}
		if string(got) != want {
			t.Errorf("the log of %s of %s is\n%s\nnot its expected log\n%s", testCase, replayed.suite, got, want)
		}
	}
	if !reflect.DeepEqual(written, expected) {
		t.Errorf("the runner wrote a log of each case that has an expected log, and of no other: wrote %v, "+
			"expected %v", written, expected)
	}
}

// docs/SCE_TTCN3_LANE.md §12.2: signed and 64-bit integers in a message, the
// verdicts `fail` and `none`, the two ways to `error` a document leads a run to,
// a test case that can never end and a dynamic error, and a branch taken before
// the states it leaves log their exits (docs/SCE_TEST_PROFILE.md §4.4).
func TestTheCorpusReplaysToItsExpectedLogs_TheBoundarySuiteReplaysToItsExpectedLogs(t *testing.T) {
	corpusReplays(t, corpusReplayed{
		suite:     "boundary_suite",
		tables:    boundary_suite.Suite(),
		parameter: boundary_suite.ModuleParameter,
		world:     "",
		verdicts: "exchanges_signed_numbers pass\nanswers_another_job fail\nsets_no_verdict none\n" +
			"never_ends error\nsends_to_a_removed_ptc error\ntakes_before_its_exits pass\n",
	})
}

// docs/SCE_TEST_PROFILE.md §11.8: a status every matcher of `every_matcher`
// accepts, at 1 ms, which the tester's branch reads as a composed message; the
// helper is told and passes, and the tester settles for the duration its start
// gives and passes at 501 ms.
func TestTheCorpusReplaysToItsExpectedLogs_TheJobSuiteReplaysToItsExpectedLogs(t *testing.T) {
	corpusReplays(t, corpusReplayed{
		suite:     "job_suite",
		tables:    job_suite.Suite(),
		parameter: job_suite.ModuleParameter,
		world:     "job-timeout,job-retry",
		verdicts:  "completes_in_time pass\n",
	})
}

// docs/SCE_TEST_PROFILE.md §9.5: an order of two lines, a customer and a card,
// which `order_placed_match` accepts over its union, its list of records and its
// template references, at 1 ms.
func TestTheCorpusReplaysToItsExpectedLogs_TheOrdersSuiteReplaysToItsExpectedLogs(t *testing.T) {
	corpusReplays(t, corpusReplayed{
		suite:     "orders_suite",
		tables:    orders_suite.Suite(),
		parameter: orders_suite.ModuleParameter,
		world:     "",
		verdicts:  "orders_case pass\n",
	})
}

// docs/SCE_TEST_PROFILE.md §4.5 (P5), §4.7 (P7): an order at 100 ms, which the
// atomic state's own targetless branch takes before its ancestor's, and one at
// 500 ms, which the ancestor's internal branch takes, restarting the running
// timer; the inner timer then runs out twice and the tester passes at 1250 ms.
func TestTheCorpusReplaysToItsExpectedLogs_TheNestedSuiteReplaysToItsExpectedLogs(t *testing.T) {
	corpusReplays(t, corpusReplayed{
		suite:     "nested_suite",
		tables:    nested_suite.Suite(),
		parameter: nested_suite.ModuleParameter,
		world:     "",
		verdicts:  "nested_case pass\n",
	})
}
