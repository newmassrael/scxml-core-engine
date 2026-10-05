// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// datamodel="sce-static" (docs/SCE_ACCEPTED_SUBSET.md §2.15) under generated Go:
// a variable is a field of the policy, every expression was lowered to Go at
// build time, and no script engine is built.
//
// The scenarios here are the ones Kotlin, Rust, C++ and the Interpreter replay —
// `sce-build/tests/fixtures/static_datamodel/scenarios/<machine>.json`, whose
// expected values are derived from the document, not observed from a backend —
// so the engines are held to one answer. A scenario is a list of steps, each an
// external event with its payload and what the machine must hold after it runs
// to quiescence: its current state and any of its published variables.
//
// The machines are the ones scripts/regen_static_datamodel_go.sh commits, one
// package each. Those with no scenario here are imported for their effect: a
// machine nobody built would be a machine nobody type-checked.

package static_datamodel

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"

	_ "github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_history"
	_ "github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_timers"

	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_block_ends"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_block_ends_list"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_counter"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_donedata"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_donedata_content"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_donedata_record"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_enum"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_event_arrival"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_event_wildcard"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_foreach"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_host_call"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_host_call_arguments"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_list"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_overflow"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_payload"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_payload_enum"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_payload_relay"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_real"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_record"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_record_enum"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_record_fields"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_record_list"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_record_real"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_send_content"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_send_delay"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_send_namelist"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_send_params"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_string_capacity"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_whole_payload"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_wire_enum"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/sync_client"
)

// dayRecord is a `record:Day` as a host reads it: each field through the reader
// the record gives it.
type dayRecord interface {
	Year() uint16
	Month() uint8
	DayOfMonth() uint8
}

// dayJSON is a day as a scenario states it: its fields by the schema's ids.
func dayJSON(day dayRecord) any {
	return map[string]any{"year": day.Year(), "month": day.Month(), "dayOfMonth": day.DayOfMonth()}
}

// daysJSON is a list of such days, in order.
func daysJSON[R dayRecord](days []R) any {
	out := make([]any, len(days))
	for i, day := range days {
		out[i] = dayJSON(day)
	}
	return out
}

// readingRecord is a `record:Reading` as a host reads it: a small integer and a
// 64-bit real.
type readingRecord interface {
	Sensor() uint8
	Value() float64
}

// readingJSON is a reading as a scenario states it: its fields by the schema's ids.
func readingJSON(reading readingRecord) any {
	return map[string]any{"sensor": reading.Sensor(), "value": reading.Value()}
}

// scenarioDir is where the scenarios live, from this package's directory.
const scenarioDir = "../../../../../sce-build/tests/fixtures/static_datamodel/scenarios"

// machine is one generated machine as a scenario sees it: events by their
// document name, and the published variables by theirs.
type machine struct {
	send func(name, data string)
	// resolves is whether an event arriving under `name` reaches the machine at
	// all, as the engine decides it (§scxml-3.12.1) — false is a name it drops.
	resolves func(name string) bool
	// advance moves the machine's time on by `ms` and runs what that made due:
	// the machine runs on a manual clock, so a wait is the one a step names.
	advance   func(ms int64)
	state     func() string
	ended     func() bool
	donedata  func() string
	variables map[string]func() any
}

// drive starts `policy` and returns it as a machine.
func drive[S interface {
	comparable
	String() string
}, E comparable](policy sce.StatePolicy[S, E], variables map[string]func() any) machine {
	engine := sce.NewEngine[S, E](policy)
	engine.SetClock(sce.NewManualClock(0))
	engine.Initialize()
	return machine{
		send: func(name, data string) {
			engine.RaiseExternalByName(name, data)
			engine.Step()
		},
		advance: engine.AdvanceTimeMs,
		resolves: func(name string) bool {
			_, ok := engine.ResolveEventByName(name)
			return ok
		},
		// The current state is the atomic one: a compound state is active for as
		// long as one of its children is, and is no more where the machine is.
		state: func() string {
			var atomic []S
			for _, state := range engine.GetActiveStates() {
				if !policy.IsCompoundState(state) {
					atomic = append(atomic, state)
				}
			}
			if len(atomic) != 1 {
				return fmt.Sprintf("<%d active atomic states>", len(atomic))
			}
			return atomic[0].String()
		},
		ended:     engine.IsInFinalState,
		donedata:  engine.DonedataAtFinal,
		variables: variables,
	}
}

// elements is a slice as the list of its elements — a `[]uint8` is a list of
// numbers here, not the byte string JSON would write it as.
func elements(value any) any {
	v := reflect.ValueOf(value)
	if v.Kind() != reflect.Slice {
		return value
	}
	out := make([]any, v.Len())
	for i := range out {
		out[i] = elements(v.Index(i).Interface())
	}
	return out
}

// asJSON is `value` as the scenario writes it: a JSON value, so a `uint32` and
// the number a scenario states compare equal whatever their Go types.
func asJSON(t *testing.T, value any) any {
	t.Helper()
	text, err := json.Marshal(elements(value))
	if err != nil {
		t.Fatalf("a variable is not JSON: %v", err)
	}
	var out any
	if err := json.Unmarshal(text, &out); err != nil {
		t.Fatalf("a variable's JSON does not read back: %v", err)
	}
	return out
}

// replay runs scenario `name` against `m`. Every step names an event (or none,
// for the machine as started) and what it must hold afterwards. A name no event
// of the machine matches is a misspelt step unless the step says it expects the
// drop (`"dropped": true`), which is then what is held.
func replay(t *testing.T, name string, m machine) {
	t.Helper()
	text, err := os.ReadFile(filepath.Join(scenarioDir, name+".json"))
	if err != nil {
		t.Fatalf("scenario %s is not readable: %v", name, err)
	}
	var scenario struct {
		Steps []struct {
			Note      string          `json:"note"`
			Event     *string         `json:"event"`
			AdvanceMs *int64          `json:"advance_ms"`
			Dropped   bool            `json:"dropped"`
			Data      json.RawMessage `json:"data"`
			Expect  struct {
				State     *string        `json:"state"`
				Ended     bool           `json:"ended"`
				Donedata  any            `json:"donedata"`
				Variables map[string]any `json:"variables"`
			} `json:"expect"`
		} `json:"steps"`
	}
	if err := json.Unmarshal(text, &scenario); err != nil {
		t.Fatalf("scenario %s is not a scenario: %v", name, err)
	}
	if len(scenario.Steps) == 0 {
		t.Fatalf("scenario %s has no steps, which judges nothing", name)
	}
	for n, step := range scenario.Steps {
		where := func() string { return fmt.Sprintf("%s step %d: %s", name, n, step.Note) }
		if step.AdvanceMs != nil {
			m.advance(*step.AdvanceMs)
		}
		if step.Event != nil {
			if m.resolves(*step.Event) == step.Dropped {
				t.Errorf("%s: the machine's events %s `%s`", where(), map[bool]string{true: "match", false: "do not match"}[step.Dropped], *step.Event)
			}
			data := ""
			if len(step.Data) > 0 {
				data = string(step.Data)
			}
			m.send(*step.Event, data)
		}
		if step.Expect.Ended {
			if !m.ended() {
				t.Errorf("%s: the machine ended in a top-level <final>", where())
			}
			// What its <donedata> left for the invoking parent, as the JSON the
			// parent reads: the pairs that were carried, and no other, or the
			// string an inline <content> spells.
			if step.Expect.Donedata != nil {
				var got any
				if err := json.Unmarshal([]byte(m.donedata()), &got); err != nil {
					t.Errorf("%s: the donedata %q is not JSON: %v", where(), m.donedata(), err)
				} else if !reflect.DeepEqual(got, step.Expect.Donedata) {
					t.Errorf("%s: the donedata is %v, not %v", where(), got, step.Expect.Donedata)
				}
			}
			continue
		}
		if step.Expect.State != nil {
			if got := m.state(); got != *step.Expect.State {
				t.Errorf("%s: the current state is %q, not %q", where(), got, *step.Expect.State)
			}
		}
		for variable, want := range step.Expect.Variables {
			read, ok := m.variables[variable]
			if !ok {
				t.Errorf("%s: the driver publishes no variable %q", where(), variable)
				continue
			}
			if got := asJSON(t, read()); !reflect.DeepEqual(got, want) {
				t.Errorf("%s: variable %q is %v, not %v", where(), variable, got, want)
			}
		}
	}
}

// unexportedUint reads an integer field a machine keeps to itself — a variable
// not declared published has no reader, so that renaming it never changes what
// a host was written against — as the scenario names it. Reading is all the
// reflection does.
func unexportedUint(policy any, field string) uint64 {
	return reflect.ValueOf(policy).Elem().FieldByName(field).Uint()
}

func counter() machine {
	policy := static_counter.NewStaticCounterPolicy()
	policy.SessionID = sce.GenerateSessionID()
	return drive[static_counter.StaticCounterState, static_counter.StaticCounterEvent](&policy, map[string]func() any{
		"count": func() any { return policy.Count() },
		"ready": func() any { return policy.Ready() },
	})
}

// A counter counts to its flag and lets go: guards, `<assign>`, `<if>` and
// `<elseif>` are native, and `In()` asks the machine's own configuration.
func TestTheCounterCountsToItsFlagAndLetsGo(t *testing.T) {
	replay(t, "static_counter", counter())
}

func TestTheCounterStopsAtItsBoundAndRefusesGo(t *testing.T) {
	replay(t, "static_counter_bound", counter())
}

// An event arrives by name from outside the document, so the names it can
// arrive under are open (§scxml-3.12.1): a name the document never writes
// reaches the transition whose descriptor is a token prefix of it, and one no
// descriptor matches is dropped.
func TestAnEventArrivesUnderANameTheDocumentDoesNotWrite(t *testing.T) {
	policy := static_event_arrival.NewStaticEventArrivalPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_event_arrival", drive[static_event_arrival.StaticEventArrivalState, static_event_arrival.StaticEventArrivalEvent](&policy, map[string]func() any{
		"requests": func() any { return policy.Requests() },
		"specials": func() any { return policy.Specials() },
	}))
}

// ...and where the document listens with `event="*"`, a name no descriptor it
// writes extends is delivered as the wildcard event instead of being dropped.
func TestAnEventArrivesUnderANameOnlyTheWildcardTakes(t *testing.T) {
	policy := static_event_wildcard.NewStaticEventWildcardPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_event_wildcard", drive[static_event_wildcard.StaticEventWildcardState, static_event_wildcard.StaticEventWildcardEvent](&policy, map[string]func() any{
		"requests": func() any { return policy.Requests() },
		"strays":   func() any { return policy.Strays() },
	}))
}

// A checked integer operation that overflows is a failure, not a wrapped value
// (§scxml-3.4.1): the variable keeps what it held, `error.execution` is raised,
// and a guard over the overflowing sum is false.
func TestAnOverflowingOperationFailsInsteadOfWrapping(t *testing.T) {
	policy := static_overflow.NewStaticOverflowPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_overflow", drive[static_overflow.StaticOverflowState, static_overflow.StaticOverflowEvent](&policy, map[string]func() any{
		"level":    func() any { return policy.Level() },
		"refusals": func() any { return policy.Refusals() },
	}))
}

// An event's typed payload is read in a guard and in assignments, and a
// computation that does not fit skips its assignment and says so. The payload
// goes in as the JSON text every other producer fills, and the machine lifts the
// typed fields out of it.
func TestAnEventsTypedPayloadIsReadInAGuardAndInContent(t *testing.T) {
	policy := static_payload.NewStaticPayloadPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_payload", drive[static_payload.StaticPayloadState, static_payload.StaticPayloadEvent](&policy, map[string]func() any{
		"day":        func() any { return policy.Day() },
		"late":       func() any { return policy.Late() },
		"sinceEpoch": func() any { return policy.SinceEpoch() },
		"refusals":   func() any { return policy.Refusals() },
	}))
}

// An event's payload carries an enum field, the variant's declared name: a guard
// compares it to a variant and an assignment stores it in a variable of the enum.
func TestAnEventsPayloadCarriesAnEnumField(t *testing.T) {
	policy := static_payload_enum.NewStaticPayloadEnumPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_payload_enum", drive[static_payload_enum.StaticPayloadEnumState, static_payload_enum.StaticPayloadEnumEvent](&policy, map[string]func() any{
		"layout": func() any { return policy.Layout().String() },
		"zoom":   func() any { return policy.Zoom() },
		"agenda": func() any { return policy.Agenda() },
		"shown":  func() any { return policy.Shown() },
	}))
}

// The payload of the event a transition is on is carried on as the <param>s of
// a <send>: an enum field as the name its enum declares and an integer, read
// where the send runs.
func TestThePayloadOfAnEventIsCarriedOnAsParams(t *testing.T) {
	policy := static_payload_relay.NewStaticPayloadRelayPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_payload_relay", drive[static_payload_relay.StaticPayloadRelayState, static_payload_relay.StaticPayloadRelayEvent](&policy, map[string]func() any{
		"layout":   func() any { return policy.Layout().String() },
		"zoom":     func() any { return policy.Zoom() },
		"relays":   func() any { return policy.Relays() },
		"refusals": func() any { return policy.Refusals() },
	}))
}

// W3C SCXML 3.12.2 / 4.9: content that reads a payload a delivery did not carry
// is an execution error, which stops the block before any of it runs. A day was
// picked first, so what the content would have written from a payload that is
// not there — `late` false, `day` the field's zero — is told from what the
// machine already held.
func TestADeliveryWithoutThePayloadRunsNoneOfTheContent(t *testing.T) {
	policy := static_payload.NewStaticPayloadPolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[static_payload.StaticPayloadState, static_payload.StaticPayloadEvent](&policy)
	engine.Initialize()

	engine.RaiseExternalByName("day.picked", `{"year": 2026, "month": 9, "dayOfMonth": 20}`)
	engine.Step()
	if policy.Day() != 20 || !policy.Late() {
		t.Fatalf("a day after the fifteenth was picked: day = %d, late = %v", policy.Day(), policy.Late())
	}

	engine.RaiseExternalByName("day.picked", "")
	engine.Step()
	if policy.Day() != 20 || !policy.Late() {
		t.Errorf("no field was assigned: day = %d, late = %v, want 20 and true", policy.Day(), policy.Late())
	}
}

// An enum variable holds a variant of its enum, is compared with `===` and `!==`
// to a variant or to another variable of the same enum, and takes a conditional
// of two variants. The scenario names a value as the enum document does, so the
// machine's own type is read back through its `String`.
func TestAnEnumVariableHoldsAVariantOfItsEnum(t *testing.T) {
	policy := static_enum.NewStaticEnumPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_enum", drive[static_enum.StaticEnumState, static_enum.StaticEnumEvent](&policy, map[string]func() any{
		"layout": func() any { return policy.Layout().String() },
		// `previous` is the machine's own, which the scenario states anyway.
		"previous": func() any {
			return static_enum.StaticEnumViewModeEnum(unexportedUint(&policy, "vPrevious")).String()
		},
		"changes": func() any { return policy.Changes() },
	}))
}

// A list is filled to its bound and emptied: an append to a full one appends
// nothing and raises error.execution, and what a host reads of a list is a copy.
func TestAListIsFilledToItsBoundAndEmptied(t *testing.T) {
	policy := static_list.NewStaticListPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_list", drive[static_list.StaticListState, static_list.StaticListEvent](&policy, map[string]func() any{
		"picked":   func() any { return policy.Picked() },
		"refusals": func() any { return policy.Refusals() },
		"count":    func() any { return policy.Count() },
	}))
}

// A `<foreach>` walks the list as it was when the loop began, binds its item and
// index, and an error in its body ends the block that holds it.
func TestAForeachWalksAListVariable(t *testing.T) {
	policy := static_foreach.NewStaticForeachPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_foreach", drive[static_foreach.StaticForeachState, static_foreach.StaticForeachEvent](&policy, map[string]func() any{
		"picked":    func() any { return policy.Picked() },
		"total":     func() any { return policy.Total() },
		"weighted":  func() any { return policy.Weighted() },
		"small":     func() any { return policy.Small() },
		"crossings": func() any { return policy.Crossings() },
		"visited":   func() any { return policy.Visited() },
		"finished":  func() any { return policy.Finished() },
		"errors":    func() any { return policy.Errors() },
	}))
}

// A 64-bit real is a native float64 field: a product and a sum, a quotient, a
// guard comparing it with a literal, and a `<foreach>` summing a list of reals.
func TestARealIsANativeBinary64Field(t *testing.T) {
	policy := static_real.NewStaticRealPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_real", drive[static_real.StaticRealState, static_real.StaticRealEvent](&policy, map[string]func() any{
		"level":   func() any { return policy.Level() },
		"total":   func() any { return policy.Total() },
		"drift":   func() any { return policy.Drift() },
		"samples": func() any { return policy.Samples() },
		"errors":  func() any { return policy.Errors() },
	}))
}

func TestAnAppendThatFailsEndsItsBlock(t *testing.T) {
	policy := static_block_ends_list.NewStaticBlockEndsListPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_block_ends_list", drive[static_block_ends_list.StaticBlockEndsListState, static_block_ends_list.StaticBlockEndsListEvent](&policy, map[string]func() any{
		"picked":      func() any { return policy.Picked() },
		"afterAppend": func() any { return policy.AfterAppend() },
		"errors":      func() any { return policy.Errors() },
	}))
}

// A record variable is built whole from its `<sce:set>`s, read field by field,
// and updated a field at a time — from the machine's own value and from a typed
// event payload. A field assignment that does not fit is skipped, and the one
// after it in the same block is not processed.
func TestARecordIsBuiltWholeAndUpdatedAFieldAtATime(t *testing.T) {
	policy := static_record_fields.NewStaticRecordFieldsPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_record_fields", drive[static_record_fields.StaticRecordFieldsState, static_record_fields.StaticRecordFieldsEvent](&policy, map[string]func() any{
		"shown":    func() any { return dayJSON(policy.Shown()) },
		"refusals": func() any { return policy.Refusals() },
	}))
}

// A list of records is filled by name from a record variable or a loop's item,
// walked by a `<foreach>`, and a record is taken whole.
func TestAListHoldsRecordsAndAForeachWalksThem(t *testing.T) {
	policy := static_record_list.NewStaticRecordListPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_record_list", drive[static_record_list.StaticRecordListState, static_record_list.StaticRecordListEvent](&policy, map[string]func() any{
		"days":   func() any { return daysJSON(policy.Days()) },
		"copies": func() any { return daysJSON(policy.Copies()) },
		"draft":  func() any { return dayJSON(policy.Draft()) },
		"last":   func() any { return dayJSON(policy.Last()) },
		"total":  func() any { return policy.Total() },
		"errors": func() any { return policy.Errors() },
	}))
}

// A record with a 64-bit real field is built whole, written a field at a time,
// and replaced from a typed payload without losing a bit of the real it carried.
func TestARecordHoldsARealFieldToTheBit(t *testing.T) {
	policy := static_record_real.NewStaticRecordRealPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_record_real", drive[static_record_real.StaticRecordRealState, static_record_real.StaticRecordRealEvent](&policy, map[string]func() any{
		"last": func() any { return readingJSON(policy.Last()) },
		"sum":  func() any { return policy.Sum() },
	}))
}

// A record may hold an enum: its field is read back by the name the enum
// document declares.
func TestARecordHoldsAnEnumField(t *testing.T) {
	policy := static_record_enum.NewStaticRecordEnumPolicy()
	policy.SessionID = sce.GenerateSessionID()
	viewJSON := func(view static_record_enum.StaticRecordEnumViewRecord) any {
		return map[string]any{"layout": view.Layout().String(), "zoom": view.Zoom()}
	}
	replay(t, "static_record_enum", drive[static_record_enum.StaticRecordEnumState, static_record_enum.StaticRecordEnumEvent](&policy, map[string]func() any{
		"shown": func() any { return viewJSON(policy.Shown()) },
		"seen": func() any {
			seen := policy.Seen()
			out := make([]any, len(seen))
			for i, view := range seen {
				out[i] = viewJSON(view)
			}
			return out
		},
		"weeks": func() any { return policy.Weeks() },
		"flips": func() any { return policy.Flips() },
	}))
}

// The payload of an event is a record of its schema taken whole: it replaces a
// record variable in one assignment and is appended whole to a list, the enum
// field read back by the name the enum document declares.
func TestThePayloadOfAnEventIsTakenWholeAsARecord(t *testing.T) {
	policy := static_whole_payload.NewStaticWholePayloadPolicy()
	policy.SessionID = sce.GenerateSessionID()
	viewJSON := func(view static_whole_payload.StaticWholePayloadViewRecord) any {
		return map[string]any{"layout": view.Layout().String(), "zoom": view.Zoom()}
	}
	replay(t, "static_whole_payload", drive[static_whole_payload.StaticWholePayloadState, static_whole_payload.StaticWholePayloadEvent](&policy, map[string]func() any{
		"shown": func() any { return viewJSON(policy.Shown()) },
		"seen": func() any {
			seen := policy.Seen()
			out := make([]any, len(seen))
			for i, view := range seen {
				out[i] = viewJSON(view)
			}
			return out
		},
		"updates": func() any { return policy.Updates() },
		"agendas": func() any { return policy.Agendas() },
		"others":  func() any { return policy.Others() },
	}))
}

// An enum value as a <param> crosses as the name its enum declares for it: a
// variable, a field of a record variable and a conditional, sent and read back
// through the schema.
func TestAnEnumValueCrossesAsTheNameItsEnumDeclares(t *testing.T) {
	policy := static_wire_enum.NewStaticWireEnumPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_wire_enum", drive[static_wire_enum.StaticWireEnumState, static_wire_enum.StaticWireEnumEvent](&policy, map[string]func() any{
		"layout": func() any { return policy.Layout().String() },
		"shown": func() any {
			shown := policy.Shown()
			return map[string]any{"layout": shown.Layout().String(), "zoom": shown.Zoom()}
		},
		"received":   func() any { return policy.Received().String() },
		"deliveries": func() any { return policy.Deliveries() },
	}))
}

// A top-level final hands its done event the pairs of its <donedata>, each read
// from the machine's fields when the state is entered; a pair whose value does
// not fit its type is left out and the others still cross.
func TestAFinalHandsItsDoneEventItsParams(t *testing.T) {
	policy := static_donedata.NewStaticDonedataPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_donedata", drive[static_donedata.StaticDonedataState, static_donedata.StaticDonedataEvent](&policy, map[string]func() any{
		"count": func() any { return policy.Count() },
	}))
}

// A top-level final whose <donedata> names a record in its `<content expr>` hands
// its done event the pairs of the record's fields, read when the state is entered.
func TestAFinalHandsItsDoneEventTheRecordItsContentNames(t *testing.T) {
	policy := static_donedata_record.NewStaticDonedataRecordPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_donedata_record", drive[static_donedata_record.StaticDonedataRecordState, static_donedata_record.StaticDonedataRecordEvent](&policy, map[string]func() any{
		"shown": func() any {
			shown := policy.Shown()
			return map[string]any{"layout": shown.Layout().String(), "zoom": shown.Zoom()}
		},
	}))
}

// A top-level final whose <donedata> is inline <content> hands its done event the
// text as the string it spells, with no script engine to read it as a number.
func TestAFinalHandsItsDoneEventTheTextItsContentSpells(t *testing.T) {
	policy := static_donedata_content.NewStaticDonedataContentPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_donedata_content", drive[static_donedata_content.StaticDonedataContentState, static_donedata_content.StaticDonedataContentEvent](&policy, map[string]func() any{
		"count": func() any { return policy.Count() },
	}))
}

// The `delayexpr` of a <send> is a string computed from the machine's fields when
// the send runs, and read as the CSS2 time it must be; the machine runs on a
// manual clock, which the scenario's `advance_ms` steps move on.
func TestASendsDelayIsComputedWhenItRuns(t *testing.T) {
	policy := static_send_delay.NewStaticSendDelayPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_send_delay", drive[static_send_delay.StaticSendDelayState, static_send_delay.StaticSendDelayEvent](&policy, map[string]func() any{
		"wait":     func() any { return policy.Wait() },
		"beats":    func() any { return policy.Beats() },
		"refusals": func() any { return policy.Refusals() },
	}))
}

// The `<content expr>` of a <send> names a record, which crosses as the pairs of
// its fields: a record variable and the payload of the event the transition is
// on, taken whole.
func TestASendCarriesTheRecordItsContentNames(t *testing.T) {
	policy := static_send_content.NewStaticSendContentPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_send_content", drive[static_send_content.StaticSendContentState, static_send_content.StaticSendContentEvent](&policy, map[string]func() any{
		"shown": func() any {
			shown := policy.Shown()
			return map[string]any{"layout": shown.Layout().String(), "zoom": shown.Zoom()}
		},
		"received": func() any { return policy.Received().String() },
		"level":    func() any { return policy.Level() },
		"relays":   func() any { return policy.Relays() },
	}))
}

// The `namelist` of a <send> names variables the machine holds, each carried as
// the pair `<param name="x" expr="x"/>` it abbreviates, an enum value among
// them as the name its enum declares.
func TestASendCarriesTheVariablesItsNamelistNames(t *testing.T) {
	policy := static_send_namelist.NewStaticSendNamelistPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_send_namelist", drive[static_send_namelist.StaticSendNamelistState, static_send_namelist.StaticSendNamelistEvent](&policy, map[string]func() any{
		"layout":     func() any { return policy.Layout().String() },
		"zoom":       func() any { return policy.Zoom() },
		"received":   func() any { return policy.Received().String() },
		"level":      func() any { return policy.Level() },
		"deliveries": func() any { return policy.Deliveries() },
	}))
}

// A <send> hands its event the pairs of its <param>s, read from the machine's
// fields when it runs; a pair whose value does not fit its type is left out, the
// message still goes, and the receiver refuses it for the field it finds missing.
func TestASendCarriesItsParamsAsTheTypedValuesOfTheMachine(t *testing.T) {
	policy := static_send_params.NewStaticSendParamsPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_send_params", drive[static_send_params.StaticSendParamsState, static_send_params.StaticSendParamsEvent](&policy, map[string]func() any{
		"total":        func() any { return policy.Total() },
		"ok":           func() any { return policy.Ok() },
		"tag":          func() any { return policy.Tag() },
		"partialTotal": func() any { return policy.PartialTotal() },
		"refusals":     func() any { return policy.Refusals() },
	}))
}

// A string variable is held to the UTF-8 bytes it declares, which is what a Go
// string is made of: an assignment past the bound writes nothing, raises
// error.execution and ends its block.
func TestAStringIsHeldToItsBytes(t *testing.T) {
	policy := static_string_capacity.NewStaticStringCapacityPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_string_capacity", drive[static_string_capacity.StaticStringCapacityState, static_string_capacity.StaticStringCapacityEvent](&policy, map[string]func() any{
		"title":    func() any { return policy.Title() },
		"body":     func() any { return policy.Body() },
		"copied":   func() any { return policy.Copied() },
		"refusals": func() any { return policy.Refusals() },
	}))
}

// A guard calls an imported algorithm with the record's own fields, and the
// call is the package the algorithm's own generation put it in.
func TestAGuardCallsAnImportedAlgorithm(t *testing.T) {
	policy := static_record.NewStaticRecordPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_record", drive[static_record.StaticRecordState, static_record.StaticRecordEvent](&policy, map[string]func() any{
		"shown":    func() any { return dayJSON(policy.Shown()) },
		"refusals": func() any { return policy.Refusals() },
	}))
}

// A sync run composed of the standard sync rules: each rule an imported
// algorithm, called from a guard or an assignment, and the run's payloads the
// standard event schemas'.
func TestASyncRunIsComposedOfTheStandardSyncRules(t *testing.T) {
	policy := sync_client.NewSyncClientPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "sync_client", drive[sync_client.SyncClientState, sync_client.SyncClientEvent](&policy, map[string]func() any{
		"byToken":     func() any { return policy.ByToken() },
		"fullListing": func() any { return policy.FullListing() },
		"outcome":     func() any { return policy.Outcome() },
		"retryAt":     func() any { return policy.RetryAt() },
		"deleted":     func() any { return policy.Deleted() },
		"uploaded":    func() any { return policy.Uploaded() },
		"discarded":   func() any { return policy.Discarded() },
		"pages":       func() any { return policy.Pages() },
		"refusals":    func() any { return policy.Refusals() },
	}))
}

// An error ends the block it stands in (W3C SCXML 4.9): the statements after a
// failed `<assign>`, after a failed `<if>` cond, or inside a branch that failed
// do not run, while the next block does.
func TestAnErrorEndsTheBlockItStandsIn(t *testing.T) {
	policy := static_block_ends.NewStaticBlockEndsPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_block_ends", drive[static_block_ends.StaticBlockEndsState, static_block_ends.StaticBlockEndsEvent](&policy, map[string]func() any{
		"a":           func() any { return policy.A() },
		"b":           func() any { return policy.B() },
		"afterAssign": func() any { return policy.AfterAssign() },
		"thenRan":     func() any { return policy.ThenRan() },
		"elseRan":     func() any { return policy.ElseRan() },
		"afterIf":     func() any { return policy.AfterIf() },
		"inBranch":    func() any { return policy.InBranch() },
		"afterBranch": func() any { return policy.AfterBranch() },
		"afterOk":     func() any { return policy.AfterOk() },
		"errors":      func() any { return policy.Errors() },
	}))
}

// recordingHost keeps what a machine asked of its host, as `(value…)` text.
type recordingHost struct{ calls []string }

func (h *recordingHost) ShowAttempts(count uint32, exhausted bool) {
	h.calls = append(h.calls, fmt.Sprintf("%d,%t", count, exhausted))
}

func (h *recordingHost) Report(next uint8) {
	h.calls = append(h.calls, fmt.Sprintf("%d", next))
}

// A host action takes the machine's variables as typed arguments, each read when
// the call is made: one call per entry of `idle`, with the datamodel as it
// stood.
func TestAHostActionTakesTypedDatamodelArguments(t *testing.T) {
	host := &recordingHost{}
	policy := static_host_call.NewStaticHostCallPolicy(host)
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[static_host_call.StaticHostCallState, static_host_call.StaticHostCallEvent](&policy)
	engine.Initialize()
	for i := 0; i < 4; i++ {
		engine.RaiseExternalByName("retry", "")
		engine.Step()
	}
	// The fourth retry finds `attempts < 3` false and re-enters nothing.
	want := []string{"0,false", "1,false", "2,false", "3,true"}
	if !reflect.DeepEqual(host.calls, want) {
		t.Errorf("the host heard %v, want %v", host.calls, want)
	}
}

// An argument that cannot be computed is a failure, not a wrapped value: the
// host is not called, and `error.execution` is raised in the call's place.
func TestAnArgumentThatOverflowsStopsTheCallAndRaisesAnError(t *testing.T) {
	host := &recordingHost{}
	policy := static_host_call_arguments.NewStaticHostCallArgumentsPolicy(host)
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[static_host_call_arguments.StaticHostCallArgumentsState, static_host_call_arguments.StaticHostCallArgumentsEvent](&policy)
	engine.Initialize()

	engine.RaiseExternalByName("fine", "")
	engine.Step()
	if want := []string{"251"}; !reflect.DeepEqual(host.calls, want) {
		t.Errorf("250 + 1 fits a uint8: the host heard %v, want %v", host.calls, want)
	}
	if got := policy.Errors(); got != 0 {
		t.Errorf("errors = %d, want 0", got)
	}

	engine.RaiseExternalByName("overflow", "")
	engine.Step()
	if want := []string{"251"}; !reflect.DeepEqual(host.calls, want) {
		t.Errorf("250 + 10 does not fit, so the host is not called with 4: it heard %v, want %v", host.calls, want)
	}
	if got := policy.Errors(); got != 1 {
		t.Errorf("error.execution was raised and the machine saw it: errors = %d, want 1", got)
	}
}
