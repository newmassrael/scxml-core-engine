// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.12 and docs/adr/0055, step 10 — a host-run `<invoke>`
// whose `sce:result` record holds values that may be absent. Go compile+run
// gate.
//
// The committed machine in this directory is generated from
// `sce-build/tests/fixtures/host_processor/statechart_host_invoker_optional.scxml`
// (regen: `scripts/regen_host_processor.sh`) and is built with NO script engine:
// a `datamodel="sce-static"` machine holds its variables as fields.
//
// The host answers through the generated typed interface, so the record it hands
// in holds each field that may be absent as `optional.Of[T]`; the engine delivers
// the record as JSON, an absent value as `null` with its key written all the
// same; and the machine reads each field where a test proves it present. What
// these hold apart is the three states a field has — absent, present and zero or
// empty, and present — which a sentinel could not: `Some("")` is a label,
// `Some(false)` is a flag that was answered, and `Some(0)` is a day.

package statechart_host_invoker_optional

import (
	"math"
	"testing"

	"github.com/newmassrael/sce-forge-runtime/optional"
	sce "github.com/newmassrael/sce-go-runtime"
)

// The type the fixture was compiled for; `scripts/regen_host_processor.sh`
// passes the same string to the invoker declaration.
const declaredType = "x-sce-host"

type Engine = sce.Engine[StatechartHostInvokerOptionalState, StatechartHostInvokerOptionalEvent]

// lookupHost implements the generated interface; its work outlives the call, so
// each test completes the start it was handed.
type lookupHost struct {
	tokens  []uint64
	cancels []uint64
}

func (h *lookupHost) StartLookup(token uint64) *StatechartHostInvokerOptionalLookupResult {
	h.tokens = append(h.tokens, token)
	return nil
}

func (h *lookupHost) CancelLookup(token uint64) {
	h.cancels = append(h.cancels, token)
}

type started struct {
	engine *Engine
	policy *StatechartHostInvokerOptionalPolicy
	host   *lookupHost
}

// A machine with the host's typed side registered, standing at `idle`.
func newStarted() started {
	policy := NewStatechartHostInvokerOptionalPolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[StatechartHostInvokerOptionalState, StatechartHostInvokerOptionalEvent](&policy)
	host := &lookupHost{}
	RegisterStatechartHostInvokerOptionalXSceHostInvoker(engine, host)
	engine.Initialize()
	return started{engine: engine, policy: &policy, host: host}
}

// ask drives `ask`, which enters `asking` and starts the lookup, and returns the
// token the host was started with.
func (s started) ask(t *testing.T) uint64 {
	t.Helper()
	s.engine.RaiseExternalByName("ask", "")
	s.engine.Step()
	if len(s.host.tokens) != 1 {
		t.Fatalf("the lookup was started %d times, want 1", len(s.host.tokens))
	}
	return s.host.tokens[0]
}

// What the machine published after a completion: `answered` counts the
// completions it read, and the rest is what each field of the record held.
type answer struct {
	answered     uint8
	rowsSeen     uint8
	totalSeen    uint64
	atPresent    bool
	atValue      int64
	labelPresent bool
	labelValue   string
	hitKnown     bool
	hitTrue      bool
}

func (s started) read() answer {
	p := s.policy
	return answer{
		answered:     p.Answered(),
		rowsSeen:     p.RowsSeen(),
		totalSeen:    p.TotalSeen(),
		atPresent:    p.AtPresent(),
		atValue:      p.AtValue(),
		labelPresent: p.LabelPresent(),
		labelValue:   p.LabelValue(),
		hitKnown:     p.HitKnown(),
		hitTrue:      p.HitTrue(),
	}
}

// complete asks, completes the lookup with `result` through the typed interface
// and settles the machine.
func complete(t *testing.T, result StatechartHostInvokerOptionalLookupResult) answer {
	t.Helper()
	s := newStarted()
	token := s.ask(t)
	if !CompleteStatechartHostInvokerOptionalLookup(s.engine, token, result) {
		t.Fatal("a running invocation's typed completion was refused")
	}
	s.engine.Step()
	return s.read()
}

// Every field that may be absent is: the machine reads three absent values, and
// the count every answer carries.
func TestAnAbsentFieldIsReadAsAbsent(t *testing.T) {
	got := complete(t, StatechartHostInvokerOptionalLookupResult{
		Rows:  3,
		Total: 5,
		At:    optional.None[int64](),
		Label: optional.None[string](),
		Hit:   optional.None[bool](),
	})
	want := answer{answered: 1, rowsSeen: 3, totalSeen: 5}
	if got != want {
		t.Errorf("an all-absent answer: got %+v, want %+v", got, want)
	}
}

// Every field is present: the machine reads each value.
func TestAPresentFieldIsReadAsItsValue(t *testing.T) {
	got := complete(t, StatechartHostInvokerOptionalLookupResult{
		Rows:  4,
		Total: 40,
		At:    optional.Some[int64](41),
		Label: optional.Some("ab"),
		Hit:   optional.Some(true),
	})
	want := answer{
		answered: 1, rowsSeen: 4, totalSeen: 40,
		atPresent: true, atValue: 41,
		labelPresent: true, labelValue: "ab",
		hitKnown: true, hitTrue: true,
	}
	if got != want {
		t.Errorf("an all-present answer: got %+v, want %+v", got, want)
	}
}

// A zero, an empty text and a false flag are values, not absences: each is
// present, and is read as itself.
func TestAZeroAnEmptyTextAndAFalseFlagAreValuesAndNotAbsences(t *testing.T) {
	got := complete(t, StatechartHostInvokerOptionalLookupResult{
		Rows:  1,
		Total: 0,
		At:    optional.Some[int64](0),
		Label: optional.Some(""),
		Hit:   optional.Some(false),
	})
	want := answer{
		answered: 1, rowsSeen: 1, totalSeen: 0,
		atPresent: true, atValue: 0,
		labelPresent: true, labelValue: "",
		hitKnown: true, hitTrue: false,
	}
	if got != want {
		t.Errorf("zero, empty and false are present: got %+v, want %+v", got, want)
	}
}

// Each field is independent: a day with no label and a flag with no day.
func TestEachFieldIsAbsentOrPresentOnItsOwn(t *testing.T) {
	got := complete(t, StatechartHostInvokerOptionalLookupResult{
		Rows:  2,
		Total: 9,
		At:    optional.Some[int64](7),
		Label: optional.None[string](),
		Hit:   optional.Some(true),
	})
	want := answer{
		answered: 1, rowsSeen: 2, totalSeen: 9,
		atPresent: true, atValue: 7,
		hitKnown: true, hitTrue: true,
	}
	if got != want {
		t.Errorf("a mixed answer: got %+v, want %+v", got, want)
	}
}

// The record's JSON writes the key of every field, `null` where the value is
// absent, in the order the schema declares them. A 64-bit integer is written as
// the text of its digits (`total` is one whether or not a value is absent), so
// the day being absent in both leaves what is held here the key, the `null` and
// the other values' own spelling.
func TestTheRecordIsWrittenWithNullForAnAbsentValue(t *testing.T) {
	absent := StatechartHostInvokerOptionalLookupResult{
		Rows:  3,
		Total: 5,
		At:    optional.None[int64](),
		Label: optional.None[string](),
		Hit:   optional.None[bool](),
	}
	if got, want := absent.Wire(), `{"rows":3,"total":"5","at":null,"label":null,"hit":null}`; got != want {
		t.Errorf("an all-absent record is %s, want %s", got, want)
	}
	mixed := StatechartHostInvokerOptionalLookupResult{
		Rows:  3,
		Total: 5,
		At:    optional.None[int64](),
		Label: optional.Some("ab"),
		Hit:   optional.Some(false),
	}
	if got, want := mixed.Wire(), `{"rows":3,"total":"5","at":null,"label":"ab","hit":false}`; got != want {
		t.Errorf("a record with a label and a false flag is %s, want %s", got, want)
	}
}

// A 64-bit integer crosses as the text of its digits, so the record the host
// completes with is read exactly past 2^53 - 1, where a JSON number held in a
// double is not: the first integer a double cannot hold, and the greatest value
// of each width.
func TestA64BitFieldCrossesAsTheTextOfItsDigits(t *testing.T) {
	wide := StatechartHostInvokerOptionalLookupResult{
		Rows:  1,
		Total: math.MaxUint64,
		At:    optional.Some[int64](9007199254740993),
		Label: optional.None[string](),
		Hit:   optional.None[bool](),
	}
	want := `{"rows":1,"total":"18446744073709551615","at":"9007199254740993","label":null,"hit":null}`
	if got := wide.Wire(); got != want {
		t.Errorf("a record past 2^53 - 1 is %s, want %s", got, want)
	}
	for _, c := range []struct {
		name  string
		total uint64
		at    int64
	}{
		{"the first integer a double cannot hold", 9007199254740993, 9007199254740993},
		{"the greatest of each width", math.MaxUint64, math.MaxInt64},
		{"the least signed value", 0, math.MinInt64},
		{"the greatest integer a double holds exactly", 9007199254740991, -9007199254740991},
	} {
		got := complete(t, StatechartHostInvokerOptionalLookupResult{
			Rows:  1,
			Total: c.total,
			At:    optional.Some(c.at),
			Label: optional.None[string](),
			Hit:   optional.None[bool](),
		})
		want := answer{answered: 1, rowsSeen: 1, totalSeen: c.total, atPresent: true, atValue: c.at}
		if got != want {
			t.Errorf("%s: got %+v, want %+v", c.name, got, want)
		}
	}
}

// A key left out is not an absent value: every field of the first revision of a
// schema is carried by every delivery, and `null` is the one spelling of absence,
// so a completion that lacks `at` is refused as any typed payload the data does
// not fit is, and the machine reads no answer.
func TestACompletionThatLeavesAKeyOutIsRefused(t *testing.T) {
	s := newStarted()
	token := s.ask(t)
	data := `{"rows":1,"total":"0","label":null,"hit":null}`
	if !s.engine.CompleteHostInvoke(declaredType, "lookup", token, data) {
		t.Fatal("a running invocation's completion was refused")
	}
	s.engine.Step()
	if got := s.read().answered; got != 0 {
		t.Errorf("a record without `at` was read as an answer: answered = %d", got)
	}
}
