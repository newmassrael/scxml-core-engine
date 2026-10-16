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
	"sort"
	"strconv"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"

	_ "github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_history"
	_ "github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_timers"

	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/presence_peer"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/sheet_peer"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_block_ends"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_block_ends_list"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_bytes"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_bytes_wire"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_cancel_expr"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_child_host"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_child_host_hybrid"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_counter"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_donedata"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_donedata_content"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_donedata_record"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_enum"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_event_arrival"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_event_wildcard"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_foreach"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_history"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_host_call"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_host_call_arguments"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_invoke"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_invoke_params"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_invoke_string"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_list"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_list_assign"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_list_index"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_overflow"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_payload"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_payload_bytes"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_payload_enum"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_payload_relay"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_real"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_real32"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_record"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_record_bytes"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_record_enum"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_record_fields"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_record_list"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_record_list_index"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_record_real"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_record_real32"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_record_string"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_send_content"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_send_delay"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_send_event"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_send_idlocation"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_send_namelist"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_send_params"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_send_target"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_send_type"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_state_algorithm"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_string_capacity"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_text_record"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_timers"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_whole_payload"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_wire_enum"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_wire_wide"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/sync_client"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/todo_peer"
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

// entryRecord is a `record:Entry` as a host reads it: a line's id and amount.
type entryRecord interface {
	Id() uint32
	Amount() int32
}

// entryJSON is a line as a scenario states it: its fields by the schema's ids.
func entryJSON(entry entryRecord) any {
	return map[string]any{"id": entry.Id(), "amount": entry.Amount()}
}

// entriesJSON is a list of such lines, in order.
func entriesJSON[R entryRecord](entries []R) any {
	out := make([]any, len(entries))
	for i, entry := range entries {
		out[i] = entryJSON(entry)
	}
	return out
}

// totalRecord is a `record:Total` as a host reads it: how many lines and their sum.
type totalRecord interface {
	Count() uint32
	Sum() int32
}

// totalJSON is a total as a scenario states it: its fields by the schema's ids.
func totalJSON(total totalRecord) any {
	return map[string]any{"count": total.Count(), "sum": total.Sum()}
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

// reading32Record is a `record:Reading32` as a host reads it: a small integer and
// a 32-bit real.
type reading32Record interface {
	Sensor() uint8
	Value() float32
}

// reading32JSON is a reading32 as a scenario states it: the single is read as the
// float64 it widens to, which is exact, since JSON would print the float32 itself.
func reading32JSON(reading reading32Record) any {
	return map[string]any{"sensor": reading.Sensor(), "value": float64(reading.Value())}
}

// framedRecord is a `record:Framed` as a host reads it: a small integer and a byte
// string of at most eight bytes.
type framedRecord interface {
	Sensor() uint8
	Frame() []byte
}

// framedJSON is a framed record as a scenario states it: its fields by the schema's
// ids, the byte string as its byte-exact Latin-1 text.
func framedJSON(framed framedRecord) any {
	return map[string]any{"sensor": framed.Sensor(), "frame": sce.BytesAsPayloadText(framed.Frame())}
}

// framedsJSON is a list of such records, in order.
func framedsJSON[R framedRecord](frameds []R) any {
	out := make([]any, len(frameds))
	for i, framed := range frameds {
		out[i] = framedJSON(framed)
	}
	return out
}

// labelledRecord is a `record:Labelled` as a host reads it: a small integer and a
// string of at most eight UTF-8 bytes.
type labelledRecord interface {
	Sensor() uint8
	Label() string
}

// labelledJSON is a labelled record as a scenario states it: its fields by the
// schema's ids.
func labelledJSON(labelled labelledRecord) any {
	return map[string]any{"sensor": labelled.Sensor(), "label": labelled.Label()}
}

// labelledsJSON is a list of such records, in order.
func labelledsJSON[R labelledRecord](labelleds []R) any {
	out := make([]any, len(labelleds))
	for i, labelled := range labelleds {
		out[i] = labelledJSON(labelled)
	}
	return out
}

// A 64-bit integer as a scenario states it: the text of its digits, since a JSON
// number past 2^53 is not the integer it spells in every reader.
func digits(value uint64) string { return strconv.FormatUint(value, 10) }

func signedDigits(value int64) string { return strconv.FormatInt(value, 10) }

// slotRecord is a `record:Slot` (sce:std/text/text_slot) as a host reads it: a
// place, the version of the write that put its text there, and a text of at most
// thirty-two UTF-8 bytes.
type slotRecord interface {
	Slot() uint32
	Version() int64
	Text() string
}

// slotJSON is a slot record as a scenario states it: its fields by the schema's
// ids, the version as the text of its digits.
func slotJSON(slot slotRecord) any {
	return map[string]any{"slot": slot.Slot(), "version": signedDigits(slot.Version()), "text": slot.Text()}
}

// slotsJSON is a list of such records, in order.
func slotsJSON[R slotRecord](slots []R) any {
	out := make([]any, len(slots))
	for i, slot := range slots {
		out[i] = slotJSON(slot)
	}
	return out
}

// hlcRecord is a `record:Hlc` (sce:std/merge/hlc_timestamp) as a host reads it: a
// hybrid logical clock's wall time, counter and node.
type hlcRecord interface {
	WallTime() int64
	Counter() uint32
	NodeId() uint64
}

// hlcJSON is a stamp as a scenario states it: its fields by the schema's ids.
func hlcJSON(stamp hlcRecord) any {
	return map[string]any{"wallTime": signedDigits(stamp.WallTime()), "counter": stamp.Counter(), "nodeId": digits(stamp.NodeId())}
}

// lwwCellRecord is a `record:LwwCell` (sce:std/merge/lww_cell) as a host reads it:
// where the cell is, what it holds (a number, or a formula over two operands) and
// the stamp of the write that put it there.
type lwwCellRecord interface {
	hlcRecord
	Row() uint64
	Col() uint64
	Value() int64
	Op() uint8
	ARef() bool
	ARow() uint64
	ACol() uint64
	AVal() int64
	BRef() bool
	BRow() uint64
	BCol() uint64
	BVal() int64
}

// lwwCellsJSON is a list of such cells, in order.
func lwwCellsJSON[R lwwCellRecord](cells []R) any {
	out := make([]any, len(cells))
	for i, cell := range cells {
		out[i] = map[string]any{
			"row": digits(cell.Row()), "col": digits(cell.Col()), "value": signedDigits(cell.Value()),
			"wallTime": signedDigits(cell.WallTime()), "counter": cell.Counter(), "nodeId": digits(cell.NodeId()),
			"op":   cell.Op(),
			"aRef": cell.ARef(), "aRow": digits(cell.ARow()), "aCol": digits(cell.ACol()), "aVal": signedDigits(cell.AVal()),
			"bRef": cell.BRef(), "bRow": digits(cell.BRow()), "bCol": digits(cell.BCol()), "bVal": signedDigits(cell.BVal()),
		}
	}
	return out
}

// sheetShownRecord is a `record:Shown` (sce:std/merge/sheet_shown) as a host reads
// it: a place of the sheet, the number it shows and the fault when it shows none.
type sheetShownRecord interface {
	Row() uint64
	Col() uint64
	Value() int64
	Fault() uint8
}

// sheetShownJSON is a list of such records, in order.
func sheetShownJSON[R sheetShownRecord](shown []R) any {
	out := make([]any, len(shown))
	for i, record := range shown {
		out[i] = map[string]any{
			"row": digits(record.Row()), "col": digits(record.Col()),
			"value": signedDigits(record.Value()), "fault": record.Fault(),
		}
	}
	return out
}

// rgaNodeRecord is a `record:RgaNode` (sce:std/merge/rga_node) as a host reads it:
// an element's id and the id of the one it was put in after.
type rgaNodeRecord interface {
	Key() uint64
	After() uint64
}

// rgaNodesJSON is a list of such elements, in order.
func rgaNodesJSON[R rgaNodeRecord](nodes []R) any {
	out := make([]any, len(nodes))
	for i, node := range nodes {
		out[i] = map[string]any{"key": digits(node.Key()), "after": digits(node.After())}
	}
	return out
}

// rgaGoneRecord is a `record:RgaGone` (sce:std/merge/rga_gone) as a host reads it:
// the id of an element taken out and the node that took it out.
type rgaGoneRecord interface {
	Key() uint64
	By() uint64
}

// rgaGonesJSON is a list of such removals, in order.
func rgaGonesJSON[R rgaGoneRecord](gones []R) any {
	out := make([]any, len(gones))
	for i, gone := range gones {
		out[i] = map[string]any{"key": digits(gone.Key()), "by": digits(gone.By())}
	}
	return out
}

// orsetEntryRecord is a `record:Entry` (sce:std/merge/orset_entry) as a host reads
// it: the element, and the stamp of the add that put it there.
type orsetEntryRecord interface {
	hlcRecord
	Element() uint64
}

// presenceEntryRecord is a `record:Seen` (presence_seen) as a host reads it: a
// client an entry of the list of who is online is for, the latest heartbeat stamp
// the copy holds for it, and the renewal of the entry that stamp belongs to.
type presenceEntryRecord interface {
	Client() uint32
	Last() int64
	Gen() uint32
}

// presenceEntriesJSON is a list of such entries, in order. The stamp is 64-bit and
// crosses as its digits.
func presenceEntriesJSON[R presenceEntryRecord](entries []R) any {
	out := make([]any, len(entries))
	for i, entry := range entries {
		out[i] = map[string]any{"client": entry.Client(), "last": signedDigits(entry.Last()), "gen": entry.Gen()}
	}
	return out
}

// orsetEntriesJSON is a list of such entries, in order.
func orsetEntriesJSON[R orsetEntryRecord](entries []R) any {
	out := make([]any, len(entries))
	for i, entry := range entries {
		out[i] = map[string]any{
			"element": digits(entry.Element()), "wallTime": signedDigits(entry.WallTime()),
			"counter": entry.Counter(), "nodeId": digits(entry.NodeId()),
		}
	}
	return out
}

// merkleNodeRecord is a `record:Node` (sce:std/merge/merkle_node) as a host reads it: one
// node of a digest, by its depth and prefix, and the hash of the changes under it.
type merkleNodeRecord interface {
	Depth() uint8
	Prefix() uint64
	Hash() uint32
}

// merkleNodesJSON is a list of such nodes, in order.
func merkleNodesJSON[R merkleNodeRecord](nodes []R) any {
	out := make([]any, len(nodes))
	for i, node := range nodes {
		out[i] = map[string]any{"depth": node.Depth(), "prefix": digits(node.Prefix()), "hash": node.Hash()}
	}
	return out
}

// todoValueRecord is a `record:Value` (sce:std/merge/mv_value) as a host reads it: which
// to-do, which field of it, the number and the text the field holds, and the
// stamp of the write that put them there.
type todoValueRecord interface {
	hlcRecord
	Id() uint64
	Field() uint64
	Value() int64
	Text() string
}

// todoValuesJSON is a list of such values, in order.
func todoValuesJSON[R todoValueRecord](values []R) any {
	out := make([]any, len(values))
	for i, value := range values {
		out[i] = map[string]any{
			"id": digits(value.Id()), "field": digits(value.Field()), "value": signedDigits(value.Value()),
			"text": value.Text(), "wallTime": signedDigits(value.WallTime()),
			"counter": value.Counter(), "nodeId": digits(value.NodeId()),
		}
	}
	return out
}

// occurrenceRecord is a `record:Occurrence` (event_occurrence) as a host reads it: one
// occurrence of a scheduled to-do in an agenda, its start and end as instants.
type occurrenceRecord interface {
	Id() uint64
	Start() int64
	End() int64
	Nth() uint32
	Conflict() uint8
}

// occurrencesJSON is a list of such occurrences, in order.
func occurrencesJSON[R occurrenceRecord](occurrences []R) any {
	out := make([]any, len(occurrences))
	for i, occurrence := range occurrences {
		out[i] = map[string]any{
			"id": digits(occurrence.Id()), "start": signedDigits(occurrence.Start()),
			"end": signedDigits(occurrence.End()), "nth": occurrence.Nth(),
			"conflict": occurrence.Conflict(),
		}
	}
	return out
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
	advance func(ms int64)
	state   func() string
	// configuration is every active state, a compound or a parallel one with the
	// atomic ones below it, by the id the document gives it.
	configuration func() []string
	ended         func() bool
	donedata      func() string
	variables     map[string]func() any
	// hostCalls is what the machine asked of its host since it was last asked,
	// oldest first, each as a scenario's `host_calls` writes it: the name the
	// document gives the action and its arguments in the order the document gives
	// them. Nil for a machine whose host is not one that records.
	hostCalls func() []any
	// childHosts is the child hosts the machine's host was asked for since it was
	// last asked, oldest asked first, and what each child asked of its host, each
	// as a scenario's `child_hosts` writes it (childHostBook.take). Nil for a
	// machine whose children are not given hosts that record.
	childHosts func() []any
	// sent is what the machine sent its Mesh peers (a `<send target="#name">`)
	// since it was last asked, oldest first, each as a scenario's `sent` writes it:
	// the target as the document writes it, the event's name, and the event's data
	// where it has any. Nil for a machine whose driver records none.
	sent func() []any
}

// sentToPeer is one Mesh send as a scenario's `sent` writes it. The data is the
// JSON value the request's text spells; a text that is not JSON is kept under a
// member no scenario states, so the step that reads it fails on it.
func sentToPeer(request sce.HostSendRequest) any {
	entry := map[string]any{"target": request.Target, "event": request.EventName}
	if request.EventData != "" {
		var data any
		if err := json.Unmarshal([]byte(request.EventData), &data); err != nil {
			entry["data that is not JSON"] = request.EventData
		} else {
			entry["data"] = data
		}
	}
	return entry
}

// settleTicks is how many rounds a host gives a machine that needs `Tick` after
// an event before it reads the machine back. The engine reports no point at
// which it has settled, so a host runs rounds: a child session takes an event
// the parent forwarded in one round and its end reaches the parent in the next,
// which is two for each level of child. Five is what the other backends' suites
// give a machine, and a round that finds nothing to do changes nothing.
const settleTicks = 5

// drive starts `policy` and returns it as a machine.
func drive[S interface {
	comparable
	String() string
}, E comparable](policy sce.StatePolicy[S, E], variables map[string]func() any) machine {
	engine := sce.NewEngine[S, E](policy)
	engine.SetClock(sce.NewManualClock(0))
	// A machine with no Mesh router fails each send to a peer as error.execution,
	// so the router that records them is in place before the machine starts: what
	// its start sends belongs to the first step. A machine that sends to no peer
	// never reaches it.
	var sent []any
	engine.RegisterMeshRouter(func(request sce.HostSendRequest) []sce.HostSendResponse {
		sent = append(sent, sentToPeer(request))
		return nil
	})
	engine.Initialize()
	// The generated policy says which call the machine needs: `Step` drains the
	// queues and nothing else, and a machine with a delayed send or a child
	// session is driven by `Tick`, which also runs those.
	scheduled, needsScheduler := any(policy).(interface{ NeedsEventScheduler() bool })
	needsScheduler = needsScheduler && scheduled.NeedsEventScheduler()
	settle := func() {
		if !needsScheduler {
			engine.Step()
			return
		}
		for i := 0; i < settleTicks; i++ {
			engine.Tick()
		}
	}
	// A machine that starts a child session in its first state has that child
	// running, and maybe ended, before any event arrives; it is read as the
	// events leave it, after the rounds that take.
	if needsScheduler {
		settle()
	}
	return machine{
		send: func(name, data string) {
			engine.RaiseExternalByName(name, data)
			settle()
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
		configuration: func() []string {
			var ids []string
			for _, state := range engine.GetActiveStates() {
				ids = append(ids, state.String())
			}
			return ids
		},
		ended:     engine.IsInFinalState,
		donedata:  engine.DonedataAtFinal,
		variables: variables,
		// The sends made since it was last called, oldest first, and forgets them.
		sent: func() []any {
			taken := sent
			sent = nil
			if taken == nil {
				return []any{}
			}
			return taken
		},
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
			Expect    struct {
				State         *string        `json:"state"`
				Configuration []string       `json:"configuration"`
				Ended         bool           `json:"ended"`
				Donedata      any            `json:"donedata"`
				Variables     map[string]any `json:"variables"`
				HostCalls     *[]any         `json:"host_calls"`
				ChildHosts    *[]any         `json:"child_hosts"`
				Sent          *[]any         `json:"sent"`
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
		// What the machine asked of its host in this step. Read on every step so
		// that a step which states nothing does not hand its calls to the next, and
		// a step that states some needs a driver that records them.
		var asked any
		if m.hostCalls != nil {
			asked = asJSON(t, m.hostCalls())
		}
		if step.Expect.HostCalls != nil {
			if m.hostCalls == nil {
				t.Errorf("%s: this driver records no host calls, so it cannot hold a step to them", where())
			} else if !reflect.DeepEqual(asked, any(*step.Expect.HostCalls)) {
				t.Errorf("%s: the machine asked its host %v, not %v", where(), asked, *step.Expect.HostCalls)
			}
		}
		// The child hosts the machine's host was asked for in this step, and what
		// each child asked of its host, read and held for the same reason.
		var children any
		if m.childHosts != nil {
			children = asJSON(t, m.childHosts())
		}
		if step.Expect.ChildHosts != nil {
			if m.childHosts == nil {
				t.Errorf("%s: this driver records no child hosts, so it cannot hold a step to them", where())
			} else if !reflect.DeepEqual(children, any(*step.Expect.ChildHosts)) {
				t.Errorf("%s: the child hosts were %v, not %v", where(), children, *step.Expect.ChildHosts)
			}
		}
		// What the machine sent its Mesh peers in this step, read on every step for
		// the reason the host calls are. The order of the sends is part of the
		// answer; the order a send's data spells its members in is not, and both
		// sides are JSON values here.
		var sent any
		if m.sent != nil {
			sent = asJSON(t, m.sent())
		}
		if step.Expect.Sent != nil {
			if m.sent == nil {
				t.Errorf("%s: this driver records no sends to peers, so it cannot hold a step to them", where())
			} else if !reflect.DeepEqual(sent, any(*step.Expect.Sent)) {
				t.Errorf("%s: the machine sent its peers %v, not %v", where(), sent, *step.Expect.Sent)
			}
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
		// The configuration is a set: the order the machine lists its active
		// states in is not part of the answer.
		if step.Expect.Configuration != nil {
			got := append([]string(nil), m.configuration()...)
			want := append([]string(nil), step.Expect.Configuration...)
			sort.Strings(got)
			sort.Strings(want)
			if !reflect.DeepEqual(got, want) {
				t.Errorf("%s: the active states are %v, not %v", where(), got, want)
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

// A list variable takes what an algorithm returns as a list, whole; a call that
// fails leaves the list as it was and raises error.execution.
func TestAListTakesWhatAnAlgorithmReturnsWhole(t *testing.T) {
	policy := static_list_assign.NewStaticListAssignPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_list_assign", drive[static_list_assign.StaticListAssignState, static_list_assign.StaticListAssignEvent](&policy, map[string]func() any{
		"shown":    func() any { return policy.Shown() },
		"few":      func() any { return policy.Few() },
		"first":    func() any { return policy.First() },
		"count":    func() any { return policy.Count() },
		"size":     func() any { return policy.Size() },
		"refusals": func() any { return policy.Refusals() },
	}))
}

// A statechart hands the algorithms the records and lists it holds, and the
// event's own payload, and takes back what they return, written only when the
// call succeeded and held to the bound of the variable it is written to.
func TestAStatechartHandsItsStateToAnAlgorithmAndTakesItBack(t *testing.T) {
	policy := static_state_algorithm.NewStaticStateAlgorithmPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_state_algorithm", drive[static_state_algorithm.StaticStateAlgorithmState, static_state_algorithm.StaticStateAlgorithmEvent](&policy, map[string]func() any{
		"entries":  func() any { return entriesJSON(policy.Entries()) },
		"total":    func() any { return totalJSON(policy.Total()) },
		"last":     func() any { return entryJSON(policy.Last()) },
		"ids":      func() any { return policy.Ids() },
		"idsum":    func() any { return policy.Idsum() },
		"large":    func() any { return policy.Large() },
		"refusals": func() any { return policy.Refusals() },
	}))
}

// A statechart hands the standard text algorithms the record and the list of
// records it holds, whose text is bounded, and takes back what they return: a
// text past the bound is refused where the payload is taken, the state as it was.
func TestAStatechartHandsBoundedTextToAnAlgorithmAndTakesItBack(t *testing.T) {
	policy := static_text_record.NewStaticTextRecordPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_text_record", drive[static_text_record.StaticTextRecordState, static_text_record.StaticTextRecordEvent](&policy, map[string]func() any{
		"slots":    func() any { return slotsJSON(policy.Slots()) },
		"held":     func() any { return slotJSON(policy.Held()) },
		"incoming": func() any { return slotJSON(policy.Incoming()) },
		"known":    func() any { return policy.Known() },
		"refusals": func() any { return policy.Refusals() },
	}))
}

// An element is read by its index like any number, and an index below zero or
// not below the length is a failure: nothing is read and error.execution says so.
func TestAListElementIsReadByItsIndex(t *testing.T) {
	policy := static_list_index.NewStaticListIndexPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_list_index", drive[static_list_index.StaticListIndexState, static_list_index.StaticListIndexEvent](&policy, map[string]func() any{
		"picked":   func() any { return policy.Picked() },
		"cursor":   func() any { return policy.Cursor() },
		"under":    func() any { return policy.Under() },
		"first":    func() any { return policy.First() },
		"last":     func() any { return policy.Last() },
		"ordered":  func() any { return policy.Ordered() },
		"room":     func() any { return policy.Room() },
		"count":    func() any { return policy.Count() },
		"refusals": func() any { return policy.Refusals() },
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

// A 32-bit real is a native float32 field: every operation on it is rounded to
// binary32 where it is made. A host reads it as the float64 it widens to, which
// is exact, and that is the number the scenario states.
func TestARealIsANativeBinary32Field(t *testing.T) {
	policy := static_real32.NewStaticReal32Policy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_real32", drive[static_real32.StaticReal32State, static_real32.StaticReal32Event](&policy, map[string]func() any{
		"level": func() any { return float64(policy.Level()) },
		"drift": func() any { return float64(policy.Drift()) },
		"wide":  func() any { return policy.Wide() },
		"total": func() any { return float64(policy.Total()) },
		"samples": func() any {
			// A list of float32 is read element by element as the float64 each
			// widens to: JSON would print the float32 itself, `0.3`.
			widened := make([]float64, 0, len(policy.Samples()))
			for _, v := range policy.Samples() {
				widened = append(widened, float64(v))
			}
			return widened
		},
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

// A field of the element a list of records is indexed at is read like any
// number, and an index outside the list is a failure.
func TestAFieldOfAnIndexedRecordIsRead(t *testing.T) {
	policy := static_record_list_index.NewStaticRecordListIndexPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_record_list_index", drive[static_record_list_index.StaticRecordListIndexState, static_record_list_index.StaticRecordListIndexEvent](&policy, map[string]func() any{
		"cursor":   func() any { return policy.Cursor() },
		"day":      func() any { return policy.Day() },
		"year":     func() any { return policy.Year() },
		"sum":      func() any { return policy.Sum() },
		"ordered":  func() any { return policy.Ordered() },
		"refusals": func() any { return policy.Refusals() },
		"picked":   func() any { return dayJSON(policy.Picked()) },
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

// A record with a 32-bit real field is built whole, written a field at a time,
// and replaced from a typed payload: the double the payload carried lands as the
// single nearest it.
func TestARecordHoldsASingleFieldAsTheSingleNearestThePayload(t *testing.T) {
	policy := static_record_real32.NewStaticRecordReal32Policy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_record_real32", drive[static_record_real32.StaticRecordReal32State, static_record_real32.StaticRecordReal32Event](&policy, map[string]func() any{
		"last":   func() any { return reading32JSON(policy.Last()) },
		"sum":    func() any { return float64(policy.Sum()) },
		"errors": func() any { return policy.Errors() },
	}))
}

// A record's bytes field is held to the bytes its schema declares: an assignment
// past the bound — from a literal or a bytes variable — writes nothing, raises
// error.execution and ends its block, and a list of such records holds copies with
// their bytes. A scenario states a byte string as its byte-exact Latin-1 text.
func TestARecordHoldsABytesFieldWithinTheBoundItsSchemaDeclares(t *testing.T) {
	policy := static_record_bytes.NewStaticRecordBytesPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_record_bytes", drive[static_record_bytes.StaticRecordBytesState, static_record_bytes.StaticRecordBytesEvent](&policy, map[string]func() any{
		"last":    func() any { return framedJSON(policy.Last()) },
		"spare":   func() any { return sce.BytesAsPayloadText(policy.Spare()) },
		"frames":  func() any { return framedsJSON(policy.Frames()) },
		"size":    func() any { return policy.Size() },
		"matches": func() any { return policy.Matches() },
		"misses":  func() any { return policy.Misses() },
		"leads":   func() any { return policy.Leads() },
		"errors":  func() any { return policy.Errors() },
	}))
}

// A host that writes into the bytes of a record it was handed changes nothing of the
// machine: the reader the record gives the field answers a copy, and so does the
// reader of a list of records, whose records hold the machine's own slices.
func TestAHostThatWritesIntoARecordsBytesItWasHandedChangesNothingOfTheMachine(t *testing.T) {
	policy := static_record_bytes.NewStaticRecordBytesPolicy()
	policy.SessionID = sce.GenerateSessionID()
	handed := policy.Last().Frame()
	handed[0] = 'z'
	if got := sce.BytesAsPayloadText(policy.Last().Frame()); got != "ab" {
		t.Fatalf("the record holds %q after a host wrote into its copy, want %q", got, "ab")
	}
}

// The bytes an event's payload carries are read into a bytes variable, into a
// record's field and into a whole record, each held to its own bound: a value past it
// writes nothing, raises error.execution and ends its block. The wire spells a byte
// string as its byte-exact Latin-1 text, which a scenario states it as.
func TestThePayloadOfAnEventCarriesBytesHeldWithinTheirBounds(t *testing.T) {
	policy := static_payload_bytes.NewStaticPayloadBytesPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_payload_bytes", drive[static_payload_bytes.StaticPayloadBytesState, static_payload_bytes.StaticPayloadBytesEvent](&policy, map[string]func() any{
		"last":    func() any { return framedJSON(policy.Last()) },
		"held":    func() any { return sce.BytesAsPayloadText(policy.Held()) },
		"frames":  func() any { return framedsJSON(policy.Frames()) },
		"size":    func() any { return policy.Size() },
		"matches": func() any { return policy.Matches() },
		"misses":  func() any { return policy.Misses() },
		"errors":  func() any { return policy.Errors() },
	}))
}

// A host that raised an event with a slice of its own and then wrote into it changes
// nothing of the machine: the variable, the record's field and the whole record each
// hold a copy of the bytes the machine read, so the bound it judged still describes
// what it holds.
func TestAHostThatWritesIntoTheBytesItRaisedAnEventWithChangesNothingOfTheMachine(t *testing.T) {
	policy := static_payload_bytes.NewStaticPayloadBytesPolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[static_payload_bytes.StaticPayloadBytesState, static_payload_bytes.StaticPayloadBytesEvent](&policy)
	engine.SetClock(sce.NewManualClock(0))
	engine.Initialize()

	toVariable := []byte("wxyz")
	static_payload_bytes.RaiseFramedTaken(engine, 100, toVariable)
	engine.Step()
	toVariable[0] = 'X'
	if got := sce.BytesAsPayloadText(policy.Held()); got != "wxyz" {
		t.Fatalf("the variable holds %q after a host wrote into the slice it raised the event with, want %q", got, "wxyz")
	}

	toField := []byte("abcd")
	static_payload_bytes.RaiseFramedTaken(engine, 7, toField)
	engine.Step()
	toField[0] = 'X'
	if got := sce.BytesAsPayloadText(policy.Last().Frame()); got != "abcd" {
		t.Fatalf("the record's field holds %q after the same write, want %q", got, "abcd")
	}

	whole := []byte("whole")
	static_payload_bytes.RaiseFramedTaken(engine, 200, whole)
	engine.Step()
	whole[0] = 'X'
	if got := sce.BytesAsPayloadText(policy.Last().Frame()); got != "whole" {
		t.Fatalf("the record taken whole holds %q after the same write, want %q", got, "whole")
	}
	if got := sce.BytesAsPayloadText(policy.Frames()[0].Frame()); got != "whole" {
		t.Fatalf("the list holds %q after the same write, want %q", got, "whole")
	}
}

// A record's string field is held to the UTF-8 bytes its schema declares: an
// assignment past the bound — from a literal, a string variable or a payload —
// writes nothing, raises error.execution and ends its block, and a list of such
// records holds copies with their text.
func TestARecordHoldsAStringFieldWithinTheBoundItsSchemaDeclares(t *testing.T) {
	policy := static_record_string.NewStaticRecordStringPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_record_string", drive[static_record_string.StaticRecordStringState, static_record_string.StaticRecordStringEvent](&policy, map[string]func() any{
		"last":   func() any { return labelledJSON(policy.Last()) },
		"note":   func() any { return policy.Note() },
		"labels": func() any { return labelledsJSON(policy.Labels()) },
		"errors": func() any { return policy.Errors() },
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
		"leads": func() any { return policy.Leads() },
	}))
}

// A 64-bit integer crosses a `<send>`'s `<param>` as the text of its digits and is
// read back from it: the low digits of a value past 2^53 are the ones that were
// sent, and a number past 2^53 - 1 is refused where it is read.
func TestA64BitIntegerCrossesAsItsDigits(t *testing.T) {
	policy := static_wire_wide.NewStaticWireWidePolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_wire_wide", drive[static_wire_wide.StaticWireWideState, static_wire_wide.StaticWireWideEvent](&policy, map[string]func() any{
		"low":      func() any { return policy.Low() },
		"smallLow": func() any { return policy.SmallLow() },
		"large":    func() any { return policy.Large() },
		"negative": func() any { return policy.Negative() },
		"relays":   func() any { return policy.Relays() },
		"refusals": func() any { return policy.Refusals() },
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

// A byte string crosses a <param> and a <donedata> as its byte-exact Latin-1 text:
// sent to itself and read back through the typed payload, and carried by the done
// event's data.
func TestAByteStringCrossesAParamAsItsLatin1Text(t *testing.T) {
	policy := static_bytes_wire.NewStaticBytesWirePolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_bytes_wire", drive[static_bytes_wire.StaticBytesWireState, static_bytes_wire.StaticBytesWireEvent](&policy, map[string]func() any{
		"held":   func() any { return sce.BytesAsPayloadText(policy.Held()) },
		"echo":   func() any { return sce.BytesAsPayloadText(policy.Echo()) },
		"relays": func() any { return policy.Relays() },
		"errors": func() any { return policy.Errors() },
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

// A top-level final whose <donedata> carries a <content expr> that names one value
// hands its done event that value as its whole data: a number as its digits, a
// string quoted, and one that cannot be computed as the empty string.
func TestAFinalHandsItsDoneEventTheValueItsContentNames(t *testing.T) {
	t.Run("a number", func(t *testing.T) {
		policy := static_donedata_content.NewStaticDonedataContentPolicy()
		policy.SessionID = sce.GenerateSessionID()
		replay(t, "static_donedata_content_value", drive[static_donedata_content.StaticDonedataContentState, static_donedata_content.StaticDonedataContentEvent](&policy, map[string]func() any{
			"count": func() any { return policy.Count() },
		}))
	})
	t.Run("a string", func(t *testing.T) {
		policy := static_donedata_content.NewStaticDonedataContentPolicy()
		policy.SessionID = sce.GenerateSessionID()
		replay(t, "static_donedata_content_text", drive[static_donedata_content.StaticDonedataContentState, static_donedata_content.StaticDonedataContentEvent](&policy, map[string]func() any{
			"count": func() any { return policy.Count() },
		}))
	})
	t.Run("a value that cannot be computed", func(t *testing.T) {
		policy := static_donedata_content.NewStaticDonedataContentPolicy()
		policy.SessionID = sce.GenerateSessionID()
		replay(t, "static_donedata_content_lost", drive[static_donedata_content.StaticDonedataContentState, static_donedata_content.StaticDonedataContentEvent](&policy, map[string]func() any{
			"count": func() any { return policy.Count() },
		}))
	})
}

// The `eventexpr` of a <send> is a string computed from the machine's fields when
// the send runs, and names the event the send delivers.
func TestASendsEventIsNamedWhenItRuns(t *testing.T) {
	policy := static_send_event.NewStaticSendEventPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_send_event", drive[static_send_event.StaticSendEventState, static_send_event.StaticSendEventEvent](&policy, map[string]func() any{
		"pings":    func() any { return policy.Pings() },
		"pongs":    func() any { return policy.Pongs() },
		"refusals": func() any { return policy.Refusals() },
	}))
}

// The `targetexpr` of a <send> is a string computed from the machine's fields when
// the send runs, held to the routes the document declares as `sce:targets`
// (docs/adr/0005, decision 3): the send goes by the entry that matches, and a value
// in none of them is error.communication with nothing sent.
func TestASendsTargetIsChosenAmongTheDeclaredRoutes(t *testing.T) {
	policy := static_send_target.NewStaticSendTargetPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_send_target", drive[static_send_target.StaticSendTargetState, static_send_target.StaticSendTargetEvent](&policy, map[string]func() any{
		"landed":  func() any { return policy.Landed() },
		"refused": func() any { return policy.Refused() },
	}))
}

// The `typeexpr` of a <send> is a string computed from the machine's fields when
// the send runs, held to the processors the document declares as `sce:types`
// (docs/adr/0005, decision 3): the send is delivered by the processor the matching
// entry names, and a value in none of them is error.execution with nothing sent.
func TestASendsTypeIsChosenAmongTheDeclaredProcessors(t *testing.T) {
	policy := static_send_type.NewStaticSendTypePolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_send_type", drive[static_send_type.StaticSendTypeState, static_send_type.StaticSendTypeEvent](&policy, map[string]func() any{
		"landed":  func() any { return policy.Landed() },
		"refused": func() any { return policy.Refused() },
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

// The `sendidexpr` of a <cancel> is a string computed from the machine's fields
// when the cancel runs, the id of the delayed send it removes; the machine runs
// on a manual clock, which the scenario's `advance_ms` steps move on.
func TestACancelRemovesTheSendItsIdNames(t *testing.T) {
	policy := static_cancel_expr.NewStaticCancelExprPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_cancel_expr", drive[static_cancel_expr.StaticCancelExprState, static_cancel_expr.StaticCancelExprEvent](&policy, map[string]func() any{
		"a_fired":  func() any { return policy.AFired() },
		"b_fired":  func() any { return policy.BFired() },
		"refusals": func() any { return policy.Refusals() },
	}))
}

// childHostBook is the child hosts a machine's host was asked for, in the order
// asked, and what each child asked of its host. A child owns the host it was
// given, so each host is made here (open) and writes into the book this reads.
type childHostBook struct{ children []*bookedChild }

// bookedChild is one child host the machine's host was asked for.
type bookedChild struct {
	invoke    string
	start     int
	candidate string
	calls     []any
	taken     int
	announced bool
}

// open is the machine's host being asked for the host of a child its `invoke`
// started: `candidate` is the stem of the document when the invoke names one of
// several, and empty for a child written inline.
func (b *childHostBook) open(invoke, candidate string) *bookedChild {
	start := 1
	for _, child := range b.children {
		if child.invoke == invoke {
			start++
		}
	}
	child := &bookedChild{invoke: invoke, start: start, candidate: candidate}
	b.children = append(b.children, child)
	return child
}

// call is the child asking its host for `action`, with `args` in the order the
// document gives them.
func (c *bookedChild) call(action string, args ...any) {
	c.calls = append(c.calls, map[string]any{"action": action, "args": append([]any{}, args...)})
}

// take is the children asked for or called since it was last called, oldest asked
// first, each as a scenario's `child_hosts` writes it: the invoke, the start of
// that invoke, the candidate's document stem where the invoke names one, and the
// calls made in this step alone. A child asked for earlier that called nothing
// since has no entry.
func (b *childHostBook) take() []any {
	fresh := []any{}
	for _, child := range b.children {
		calls := append([]any{}, child.calls[child.taken:]...)
		if child.announced && len(calls) == 0 {
			continue
		}
		child.taken = len(child.calls)
		child.announced = true
		entry := map[string]any{"invoke": child.invoke, "start": child.start, "host_calls": calls}
		if child.candidate != "" {
			entry["candidate"] = child.candidate
		}
		fresh = append(fresh, entry)
	}
	return fresh
}

// recordedWorkerHost is the host a `worker` child of `static_child_host` acts
// through.
type recordedWorkerHost struct{ child *bookedChild }

func (h recordedWorkerHost) Started()               { h.child.call("started") }
func (h recordedWorkerHost) Finished(steps uint32) { h.child.call("finished", steps) }

// recordingParentHost is the host of `static_child_host`: it declares no act of
// its own, and answers a host for each start of `worker`.
type recordingParentHost struct{ book *childHostBook }

func (h recordingParentHost) ActionsForWorker() static_child_host.StaticChildHostSceSynthInvokeWorkerActions {
	return recordedWorkerHost{h.book.open("worker", "")}
}

// recordedFirstHost and recordedSecondHost are the hosts of the two candidates of
// `static_child_host_hybrid`, each with an act of its own.
type recordedFirstHost struct{ child *bookedChild }

func (h recordedFirstHost) FirstRan() { h.child.call("first_ran") }

type recordedSecondHost struct{ child *bookedChild }

func (h recordedSecondHost) SecondRan() { h.child.call("second_ran") }

// recordingHybridParentHost answers the host for the candidate it is asked about,
// under the stem of that candidate's document, and for no other.
type recordingHybridParentHost struct{ book *childHostBook }

func (h recordingHybridParentHost) ActionsForWorkStaticHostedFirst() static_child_host_hybrid.StaticHostedFirstActions {
	return recordedFirstHost{h.book.open("work", "static_hosted_first")}
}

func (h recordingHybridParentHost) ActionsForWorkStaticHostedSecond() static_child_host_hybrid.StaticHostedSecondActions {
	return recordedSecondHost{h.book.open("work", "static_hosted_second")}
}

// A child that declares `<sce:action>`s is built with the host its parent's host
// answers for it, each time the invocation starts, and what the child asked of
// that host is what the scenario's `child_hosts` holds.
func TestEachStartOfAChildIsGivenAHostOfItsOwn(t *testing.T) {
	book := &childHostBook{}
	policy := static_child_host.NewStaticChildHostPolicy(recordingParentHost{book})
	policy.SessionID = sce.GenerateSessionID()
	m := drive[static_child_host.StaticChildHostState, static_child_host.StaticChildHostEvent](&policy, map[string]func() any{
		"completed": func() any { return policy.Completed() },
	})
	m.childHosts = book.take
	replay(t, "static_child_host", m)
}

// A hybrid `<invoke>` is given the host answered for the candidate it starts, and
// for no other.
func TestACandidateIsGivenTheHostAnsweredForIt(t *testing.T) {
	book := &childHostBook{}
	policy := static_child_host_hybrid.NewStaticChildHostHybridPolicy(recordingHybridParentHost{book})
	policy.SessionID = sce.GenerateSessionID()
	m := drive[static_child_host_hybrid.StaticChildHostHybridState, static_child_host_hybrid.StaticChildHostHybridEvent](&policy, map[string]func() any{
		"completed": func() any { return policy.Completed() },
	})
	m.childHosts = book.take
	replay(t, "static_child_host_hybrid", m)
}

// A child session an <invoke> started is driven through its parent by
// autoforward, takes the events it waits for in order, and its end reaches the
// parent as `done.invoke`, which the parent counts.
func TestAnInvokedChildIsDrivenAndCounted(t *testing.T) {
	policy := static_invoke.NewStaticInvokePolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_invoke", drive[static_invoke.StaticInvokeState, static_invoke.StaticInvokeEvent](&policy, map[string]func() any{
		"completed": func() any { return policy.Completed() },
	}))
}

// An <invoke> hands its child the values its <param>s and `namelist` name, as
// they stand when the invoke executes, after the entry actions, and once.
func TestAnInvokeHandsItsChildItsValuesOnce(t *testing.T) {
	policy := static_invoke_params.NewStaticInvokeParamsPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_invoke_params", drive[static_invoke_params.StaticInvokeParamsState, static_invoke_params.StaticInvokeParamsEvent](&policy, map[string]func() any{
		"completed": func() any { return policy.Completed() },
	}))
}

// A string an <invoke> hands its child is held to the bound the child declared,
// in bytes: a value past it is left out and raises `error.execution`, and the
// child still starts.
func TestAStringHandedToAChildIsHeldToItsBoundInBytes(t *testing.T) {
	policy := static_invoke_string.NewStaticInvokeStringPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_invoke_string", drive[static_invoke_string.StaticInvokeStringState, static_invoke_string.StaticInvokeStringEvent](&policy, map[string]func() any{
		"completed": func() any { return policy.Completed() },
		"errors":    func() any { return policy.Errors() },
	}))
}

// Leaving the state that holds an <invoke> cancels the child, which then ends
// nothing and counts nothing.
func TestLeavingTheStateOfAnInvokeCancelsTheChild(t *testing.T) {
	policy := static_invoke.NewStaticInvokePolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_invoke_abort", drive[static_invoke.StaticInvokeState, static_invoke.StaticInvokeEvent](&policy, map[string]func() any{
		"completed": func() any { return policy.Completed() },
	}))
}

// A <history> remembers what its parent held when it was left, and entering it
// brings that back: the shallow one the child that was active, the deep one the
// atomic states below, and a deep one of a <parallel> both regions at once. The
// scenario states the whole active configuration of each step.
func TestAHistoryBringsBackWhatItsParentHeld(t *testing.T) {
	policy := static_history.NewStaticHistoryPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_history", drive[static_history.StaticHistoryState, static_history.StaticHistoryEvent](&policy, map[string]func() any{
		"resumed": func() any { return policy.Resumed() },
	}))
}

// Four delayed sends armed on entering a state are delivered when each is due,
// two due the same moment in the order they were sent, and the last takes the
// machine to its final state; the machine runs on a manual clock, which the
// scenario's `advance_ms` steps move on.
func TestTimersDeliverEachSendWhenItIsDue(t *testing.T) {
	policy := static_timers.NewStaticTimersPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_timers", drive[static_timers.StaticTimersState, static_timers.StaticTimersEvent](&policy, map[string]func() any{
		"trace": func() any { return policy.Trace() },
	}))
}

// The same machine stopped: a <cancel> by the id of the longest send removes that
// one and no other, so the machine never ends.
func TestStoppingCancelsTheSendItsIdNamesAndNoOther(t *testing.T) {
	policy := static_timers.NewStaticTimersPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_timers_stop", drive[static_timers.StaticTimersState, static_timers.StaticTimersEvent](&policy, map[string]func() any{
		"trace": func() any { return policy.Trace() },
	}))
}

// The `idlocation` of a <send> names a string variable the machine writes the id
// it generates for the send to, which a later <cancel sendidexpr> names; the
// machine runs on a manual clock, which the scenario's `advance_ms` steps move
// on.
func TestASendHandsTheDocumentAnIdACancelCanName(t *testing.T) {
	policy := static_send_idlocation.NewStaticSendIdlocationPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_send_idlocation", drive[static_send_idlocation.StaticSendIdlocationState, static_send_idlocation.StaticSendIdlocationEvent](&policy, map[string]func() any{
		"first_fired":  func() any { return policy.FirstFired() },
		"second_fired": func() any { return policy.SecondFired() },
		"refusals":     func() any { return policy.Refusals() },
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

// A bytes variable is held to the bytes it declares, as a string is to its UTF-8
// bytes: an assignment past the bound writes nothing, raises error.execution and
// ends its block. A scenario states a byte string as its byte-exact Latin-1 text,
// which is what the host's reader answers once it is spelled so.
func TestAByteStringIsHeldToItsBound(t *testing.T) {
	policy := static_bytes.NewStaticBytesPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_bytes", drive[static_bytes.StaticBytesState, static_bytes.StaticBytesEvent](&policy, map[string]func() any{
		"frame":   func() any { return sce.BytesAsPayloadText(policy.Frame()) },
		"tail":    func() any { return sce.BytesAsPayloadText(policy.Tail()) },
		"size":    func() any { return policy.Size() },
		"matches": func() any { return policy.Matches() },
		"misses":  func() any { return policy.Misses() },
		"errors":  func() any { return policy.Errors() },
	}))
}

// A host that is handed a byte string is handed a copy of it: the slice is the
// machine's own, and a write into it would change the variable behind its bound.
func TestAHostThatWritesIntoAByteStringItWasHandedChangesNothingOfTheMachine(t *testing.T) {
	policy := static_bytes.NewStaticBytesPolicy()
	policy.SessionID = sce.GenerateSessionID()
	handed := policy.Frame()
	handed[0] = 'z'
	if got := sce.BytesAsPayloadText(policy.Frame()); got != "ab" {
		t.Fatalf("the machine holds %q after a host wrote into its copy, want %q", got, "ab")
	}
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

// todoCopy is one copy of the shared to-do list, as a scenario reads it.
func todoCopy() machine {
	policy := todo_peer.NewTodoPeerPolicy()
	policy.SessionID = sce.GenerateSessionID()
	return drive[todo_peer.TodoPeerState, todo_peer.TodoPeerEvent](&policy, map[string]func() any{
		"node":       func() any { return digits(policy.Node()) },
		"hub":        func() any { return policy.Hub() },
		"clock":      func() any { return hlcJSON(policy.Clock()) },
		"adds":       func() any { return orsetEntriesJSON(policy.Adds()) },
		"tombstones": func() any { return orsetEntriesJSON(policy.Tombstones()) },
		"values":     func() any { return todoValuesJSON(policy.Values()) },
		"digest":     func() any { return merkleNodesJSON(policy.Digest()) },
		"refusals":   func() any { return policy.Refusals() },
		"agenda":     func() any { return occurrencesJSON(policy.Agenda()) },
	})
}

// A copy of a shared to-do list is replayed through the afternoon the world
// simulation ran it through, as each of the three that took part saw it: every
// event it was given, what it held after each, and what it sent its peers. The
// to-dos it knows are an observed-remove set (`adds` and `tombstones`), each
// field of one is a value record kept by its stamp, and the stamps are 64-bit and
// cross as their digits.
func TestACopyOfASharedTodoListIsReplayedAsEachPeerSawIt(t *testing.T) {
	t.Run("alice", func(t *testing.T) { replay(t, "todo_peer_alice", todoCopy()) })
	t.Run("bob", func(t *testing.T) { replay(t, "todo_peer_bob", todoCopy()) })
	t.Run("the hub", func(t *testing.T) { replay(t, "todo_peer_server", todoCopy()) })
}

// presenceCopy is one copy of the list of who is online, as a scenario reads it.
func presenceCopy() machine {
	policy := presence_peer.NewPresencePeerPolicy()
	policy.SessionID = sce.GenerateSessionID()
	return drive[presence_peer.PresencePeerState, presence_peer.PresencePeerEvent](&policy, map[string]func() any{
		"node":     func() any { return policy.Node() },
		"hub":      func() any { return policy.Hub() },
		"ttl":      func() any { return policy.Ttl() },
		"lease":    func() any { return policy.Lease() },
		"online":   func() any { return presenceEntriesJSON(policy.Online()) },
		"serial":   func() any { return policy.Serial() },
		"lost":     func() any { return policy.Lost() },
		"refusals": func() any { return policy.Refusals() },
	})
}

// A copy of the list of who is online is replayed through the afternoon the world
// simulation ran it through, as each of the three that took part saw it: every
// event it was given, the time that passed between them (its entries are let go by
// the machine's own delayed send, on the manual clock `drive` gives it), what it held
// after each, and what it sent its peers.
func TestACopyOfWhoIsOnlineIsReplayedAsEachPeerSawIt(t *testing.T) {
	t.Run("alice", func(t *testing.T) { replay(t, "presence_peer_alice", presenceCopy()) })
	t.Run("bob", func(t *testing.T) { replay(t, "presence_peer_bob", presenceCopy()) })
	t.Run("the hub", func(t *testing.T) { replay(t, "presence_peer_server", presenceCopy()) })
}

// sheetCopy is one copy of the shared sheet, as a scenario reads it.
func sheetCopy() machine {
	policy := sheet_peer.NewSheetPeerPolicy()
	policy.SessionID = sce.GenerateSessionID()
	return drive[sheet_peer.SheetPeerState, sheet_peer.SheetPeerEvent](&policy, map[string]func() any{
		"node":     func() any { return digits(policy.Node()) },
		"hub":      func() any { return policy.Hub() },
		"clock":    func() any { return hlcJSON(policy.Clock()) },
		"cells":    func() any { return lwwCellsJSON(policy.Cells()) },
		"rows":     func() any { return rgaNodesJSON(policy.Rows()) },
		"gone":     func() any { return rgaGonesJSON(policy.Gone()) },
		"cols":     func() any { return rgaNodesJSON(policy.Cols()) },
		"goneCols": func() any { return rgaGonesJSON(policy.GoneCols()) },
		"shown":    func() any { return sheetShownJSON(policy.Shown()) },
		"lamport":  func() any { return digits(policy.Lamport()) },
		"orphans":  func() any { return policy.Orphans() },
		"refusals": func() any { return policy.Refusals() },
	})
}

// A copy of a shared sheet of numbers is replayed the same way: its cells merged
// by the standard `lww_write`, its rows and columns placed by `rga_insert` and
// taken out by `rga_remove`, and its clock moved by `hlc_send` and `hlc_receive`,
// with what the hub passes on to the other copies held send by send.
func TestACopyOfASharedSheetIsReplayedAsEachPeerSawIt(t *testing.T) {
	t.Run("alice", func(t *testing.T) { replay(t, "sheet_peer_alice", sheetCopy()) })
	t.Run("bob", func(t *testing.T) { replay(t, "sheet_peer_bob", sheetCopy()) })
	t.Run("the hub", func(t *testing.T) { replay(t, "sheet_peer_server", sheetCopy()) })
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

// scenarioHost keeps what a machine asked of its host as a scenario writes it:
// the name the document gives the action, and its arguments in the order the
// document gives them.
type scenarioHost struct{ calls []any }

func (h *scenarioHost) ShowAttempts(count uint32, exhausted bool) {
	h.calls = append(h.calls, map[string]any{"action": "showAttempts", "args": []any{count, exhausted}})
}

// take is the calls made since it was last called, oldest first, and forgets them.
func (h *scenarioHost) take() []any {
	taken := h.calls
	h.calls = nil
	if taken == nil {
		return []any{}
	}
	return taken
}

// A native host action takes the machine's variables as typed arguments, each
// read when the call is made, and the host the scenario's `host_calls` are read
// from records them: one call per entry of `idle`, with the datamodel as it
// stood.
func TestAHostActionTellsItsHostWhatTheDatamodelHeld(t *testing.T) {
	host := &scenarioHost{}
	policy := static_host_call.NewStaticHostCallPolicy(host)
	policy.SessionID = sce.GenerateSessionID()
	m := drive[static_host_call.StaticHostCallState, static_host_call.StaticHostCallEvent](&policy, map[string]func() any{})
	m.hostCalls = host.take
	replay(t, "static_host_call", m)
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
