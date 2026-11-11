// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package testprofile

import (
	"fmt"
	"io"
	"log"
	"os"
	"path/filepath"
	"strings"

	sce "github.com/newmassrael/sce-go-runtime"
	"github.com/newmassrael/sce-go-runtime/testprofile/canonical"
)

// The runner program of a generated suite (docs/SCE_TTCN3_LANE.md §4, §10.5):
// it runs every test case a world selects against its recorded input trace and
// writes each one's canonical test log.
//
// The unit of a suite generates the program's entry, `Main`, which hands
// RunnerMain the suite's tables and the reader of its module parameters
// (`ModuleParameter`); a module makes it a program with a `package main` of
// one line that calls it. The command line is the same on every backend:
//
//	<runner> --trace-dir <dir> --log-dir <dir> [--world <feature>,...] [--module-parameter <id>=<value>]...
//
//   - `--trace-dir`: the directory of the traces, `<test case>.json` each
//     (`schemas/sce-test-trace.v1.schema.json`);
//   - `--log-dir`: the directory the logs are written to, `<test case>.log.json`
//     each, the canonical test log's text with nothing after it
//     (`schemas/sce-test-log.v1.schema.json`), made where it is missing;
//   - `--world`: the features the world enables, by name, separated by commas;
//     none where it is not given (ADR 0057 §2);
//   - `--module-parameter`: a module parameter's value in place of its default,
//     written as `<sce:module-parameter value>` writes a literal of its type
//     (docs/SCE_TEST_PROFILE.md §11.4, §11.8); repeated, one per parameter.
//
// Each test case run prints `<test case> <verdict>` on its own line, and
// nothing else goes to the standard output. A test case whose trace is missing
// or refused ends with `error` and its log is written, the reason on the
// standard error (P13). The exit status is 0 when every selected test case ran
// and its log was written, whatever its verdict: the verdict is the log's. It
// is 2 when the command line is not one of the above, a module parameter's
// value is refused, or a log cannot be written.
//
// The program's logic is RunRunner, over the streams it is handed, so a test
// runs it in place as the program runs it.
//
// Cross-language siblings: `sce_rust_runtime::test::runner` (Rust),
// `SCE::Test::Runner` (C++), `com.sce.runtime.test.runRunner` (Kotlin).

// ParameterReader is the reader of a suite's module parameters, generated as
// `ModuleParameter` in the suite's unit: the saved value of the module
// parameter id of its declared type, read from literal, the value the command
// line gives it, or why it is not one.
type ParameterReader func(id, literal string) (ModuleParameterValue, error)

// RunnerUsage is the program's usage line.
const RunnerUsage = "usage: <runner> --trace-dir <dir> --log-dir <dir> " +
	"[--world <feature>,...] [--module-parameter <id>=<value>]..."

// ParameterLiteral is one `--module-parameter`: the id of the parameter and
// the literal the command line gives it.
type ParameterLiteral struct {
	// ID is the module parameter's id.
	ID string
	// Literal is its value as the command line writes it.
	Literal string
}

// RunnerOptions is what the command line asks of the runner.
type RunnerOptions struct {
	// TraceDir is the directory of the traces, `<test case>.json` each.
	TraceDir string
	// LogDir is the directory the logs are written to, `<test case>.log.json`
	// each.
	LogDir string
	// World is the features the world enables.
	World Features
	// ModuleParameters is each `--module-parameter`, in the order given.
	ModuleParameters []ParameterLiteral
}

// ParseRunnerOptions is the options args give, the program's name not among
// them; refused, naming why, where they are not the program's command line.
func ParseRunnerOptions(args []string) (RunnerOptions, error) {
	var options RunnerOptions
	var traceDir, logDir, world *string
	// Every option takes one value: the line is read a pair at a time.
	for at := 0; at < len(args); at += 2 {
		option := args[at]
		if at+1 == len(args) {
			return RunnerOptions{}, fmt.Errorf("%s is given no value", option)
		}
		value := args[at+1]
		switch {
		case option == "--trace-dir" && traceDir == nil:
			traceDir = &value
		case option == "--log-dir" && logDir == nil:
			logDir = &value
		case option == "--world" && world == nil:
			world = &value
		case option == "--module-parameter":
			id, literal, split := strings.Cut(value, "=")
			if !split {
				return RunnerOptions{}, fmt.Errorf("--module-parameter %s is not <id>=<value>", value)
			}
			for _, given := range options.ModuleParameters {
				if given.ID == id {
					return RunnerOptions{}, fmt.Errorf("the module parameter '%s' is given twice", id)
				}
			}
			options.ModuleParameters = append(options.ModuleParameters, ParameterLiteral{ID: id, Literal: literal})
		case option == "--trace-dir" || option == "--log-dir" || option == "--world":
			return RunnerOptions{}, fmt.Errorf("%s is given twice", option)
		default:
			return RunnerOptions{}, fmt.Errorf("%s is no option of the runner", option)
		}
	}
	if traceDir == nil {
		return RunnerOptions{}, fmt.Errorf("--trace-dir is not given")
	}
	if logDir == nil {
		return RunnerOptions{}, fmt.Errorf("--log-dir is not given")
	}
	options.TraceDir = *traceDir
	options.LogDir = *logDir
	options.World = Features{}
	if world != nil {
		for _, feature := range strings.Split(*world, ",") {
			if feature != "" {
				options.World[feature] = true
			}
		}
	}
	return options, nil
}

// ReadTraceFile is the trace of the test case testCase, `<dir>/<test
// case>.json`; a *TraceRefusal naming its path where it is not read, or the
// refusal of ParseTrace.
func ReadTraceFile(dir, testCase string) (Trace, error) {
	path := filepath.Join(dir, testCase+".json")
	text, err := os.ReadFile(path)
	if err != nil {
		return Trace{}, &TraceRefusal{Reason: fmt.Sprintf("%s is not read: %v", path, err)}
	}
	return ParseTrace(string(text))
}

// RunRunner is the runner program of suite, its module parameters read by
// parameter, on the command line args, the program's name not among them
// (docs/SCE_TTCN3_LANE.md §4): each `<test case> <verdict>` line written to
// stdout, every diagnostic to stderr. It answers the program's exit status.
func RunRunner(suite Suite, parameter ParameterReader, args []string, stdout, stderr io.Writer) int {
	usage := func(why string) int {
		fmt.Fprintln(stderr, why)
		fmt.Fprintln(stderr, RunnerUsage)
		return 2
	}
	options, err := ParseRunnerOptions(args)
	if err != nil {
		return usage(err.Error())
	}
	overrides := &ModuleParameters{}
	for _, given := range options.ModuleParameters {
		value, err := parameter(given.ID, given.Literal)
		if err != nil {
			return usage(fmt.Sprintf("--module-parameter %s=%s: %v", given.ID, given.Literal, err))
		}
		overrides.Set(given.ID, value)
	}
	results, err := RunSuiteWithTraces(suite, options.World, func(testCase string) (Trace, error) {
		return ReadTraceFile(options.TraceDir, testCase)
	}, overrides)
	if err != nil {
		return usage(err.Error())
	}
	if err := os.MkdirAll(options.LogDir, 0o755); err != nil {
		fmt.Fprintf(stderr, "%s is not made: %v\n", options.LogDir, err)
		return 2
	}
	for _, result := range results {
		for _, line := range result.Result.Log {
			if line.Kind == LogError {
				fmt.Fprintf(stderr, "%s: %s: %s\n", result.TestCase, line.Component, line.Detail)
			}
		}
		path := filepath.Join(options.LogDir, result.TestCase+".log.json")
		if err := os.WriteFile(path, []byte(result.CanonicalLog(suite.Name)), 0o644); err != nil {
			fmt.Fprintf(stderr, "%s is not written: %v\n", path, err)
			return 2
		}
		fmt.Fprintf(stdout, "%s %s\n", result.TestCase, result.Result.Verdict)
	}
	return 0
}

// RunnerMain is the runner program of suite, its module parameters read by
// parameter, on the command line of the process: RunRunner over the standard
// output and the standard error, after which the process exits with the status
// it answers. The `Main` of a generated suite's unit calls it.
//
// The standard output is the contract's, the `<test case> <verdict>` lines and
// nothing else, which a lane parses. So the program keeps the process's
// standard output for those lines and hands os.Stdout to the standard error
// while the cases run: a machine's `<log>`, which a generated Go statechart
// writes with fmt.Println, goes there, as does the engine's diagnostics, which
// the runtime writes through the standard logger.
func RunnerMain(suite Suite, parameter ParameterReader) {
	os.Exit(runWithStdoutToStderr(suite, parameter, os.Args[1:]))
}

// runWithStdoutToStderr is RunRunner with the process's standard output kept
// for the contract's lines and os.Stdout and the standard logger writing to
// the standard error while it runs.
func runWithStdoutToStderr(suite Suite, parameter ParameterReader, args []string) int {
	contract := os.Stdout
	os.Stdout = os.Stderr
	log.SetOutput(os.Stderr)
	defer func() { os.Stdout = contract }()
	return RunRunner(suite, parameter, args, contract, os.Stderr)
}

// UndeclaredParameter is the refusal of a value for the module parameter id,
// which the suite does not declare.
func UndeclaredParameter(id string) *OverrideRefusal {
	return &OverrideRefusal{Reason: fmt.Sprintf("the suite declares no module parameter '%s'", id)}
}

// IntegerParameter is a value of the integer type N of the module parameter
// id: the text of its decimal digits, as the canonical form writes one, saved
// by write, the writer of a saved value of N (`sce.SavedWrite<Type>`).
func IntegerParameter[N canonical.Integral](id, literal string, write func(N) any) (ModuleParameterValue, error) {
	number, err := canonical.ReadInteger[N](literal, id)
	if err != nil {
		return nil, &OverrideRefusal{Reason: err.Error()}
	}
	return write(number), nil
}

// BoolParameter is a value of the `bool` module parameter id: `true` or
// `false`.
func BoolParameter(id, literal string) (ModuleParameterValue, error) {
	switch literal {
	case "true":
		return sce.SavedWriteBool(true), nil
	case "false":
		return sce.SavedWriteBool(false), nil
	}
	return nil, &OverrideRefusal{Reason: fmt.Sprintf("%s: '%s' is neither true nor false", id, literal)}
}

// TextParameter is a value of a text module parameter: the text itself. Its
// bound, like every value's type, is held where the component reads it, as
// for a default.
func TextParameter(literal string) (ModuleParameterValue, error) {
	return sce.SavedWriteString(literal), nil
}

// EnumParameter is a value of an enum module parameter: the name its document
// declares for the variant, which is how a value of an enum is saved. A name
// its enum does not declare is refused where the component reads it, as for a
// default.
func EnumParameter(literal string) (ModuleParameterValue, error) {
	return sce.SavedWriteString(literal), nil
}

// BytesParameter is a value of the byte string module parameter id: its octets
// in hex of either case, two digits each, as a literal of
// docs/SCE_TEST_PROFILE.md §11.4 writes them.
func BytesParameter(id, literal string) (ModuleParameterValue, error) {
	refusal := &OverrideRefusal{Reason: fmt.Sprintf("%s: '%s' is not octets in hex", id, literal)}
	if len(literal)%2 != 0 {
		return nil, refusal
	}
	octets := make([]byte, 0, len(literal)/2)
	for at := 0; at < len(literal); at += 2 {
		high, isHigh := anyCaseHexDigit(literal[at])
		low, isLow := anyCaseHexDigit(literal[at+1])
		if !isHigh || !isLow {
			return nil, refusal
		}
		octets = append(octets, high<<4|low)
	}
	return sce.SavedWriteBytes(octets), nil
}

// anyCaseHexDigit is the value of one ASCII hex digit of either case, and
// false for any other byte: digits alone, where a reader of a number would
// also read a sign.
func anyCaseHexDigit(digit byte) (byte, bool) {
	switch {
	case digit >= '0' && digit <= '9':
		return digit - '0', true
	case digit >= 'a' && digit <= 'f':
		return digit - 'a' + 10, true
	case digit >= 'A' && digit <= 'F':
		return digit - 'A' + 10, true
	}
	return 0, false
}
