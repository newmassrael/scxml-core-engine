// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package broadcastlog

// The Go arm's stress runs of the `broadcast-log` kind, written as histories
// (ADR 0062 section 5 and 6).
//
// These tests run the log of this package on real goroutines, record every
// operation each goroutine made, and write one history per run in the JSON form
// every backend writes (tests/forge/conformance/broadcast_history.schema.json).
// They judge nothing about linearizability: `sce-codegen check-broadcast-history`
// does, for this backend as for the others, so no backend carries a checker of
// its own. The Go gate (scripts/gates/forge-go.sh) sets
// SCE_BROADCAST_HISTORY_DIR, runs these, then runs the command over what they
// wrote. Without the variable there is nowhere to write, and the tests are
// skipped by name.
//
// The recording follows the queue arm's: one counter that every goroutine
// increments immediately before and immediately after each call, so a
// read-modify-write chain on one atomic orders the readings with the calls
// between them and "returned before invoked" in the record means it in the run.
// Every read is recorded, the empty ones and the lagged ones too, because those
// are results the checker must account for.
//
// The runs are short on purpose. The checker's general search is exponential in
// the readers' operations in the worst case (ADR 0062 section 5), so a run is
// a few dozen operations a participant and the volume comes from the number of
// runs.

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"sort"
	"strings"
	"sync"
	"sync/atomic"
	"testing"
)

// historyOperation is one observed operation in the JSON form. The numeric
// fields are pointers so that a zero is written and an absent field is not.
type historyOperation struct {
	Call     string  `json:"call"`
	Outcome  string  `json:"outcome,omitempty"`
	Value    *uint64 `json:"value,omitempty"`
	Seq      *uint64 `json:"seq,omitempty"`
	Skipped  *uint64 `json:"skipped,omitempty"`
	From     string  `json:"from,omitempty"`
	At       *uint64 `json:"at,omitempty"`
	Cursor   *uint64 `json:"cursor,omitempty"`
	Head     *uint64 `json:"head,omitempty"`
	Invoked  uint64  `json:"invoked"`
	Returned uint64  `json:"returned"`
}

type historyDocument struct {
	Version   int                  `json:"version"`
	Retention int                  `json:"retention"`
	Publisher []historyOperation   `json:"publisher"`
	Readers   [][]historyOperation `json:"readers"`
}

func ptr(v uint64) *uint64 { return &v }

func publishOperation(value, seq, invoked, returned uint64) historyOperation {
	return historyOperation{Call: "publish", Value: ptr(value), Seq: ptr(seq), Invoked: invoked, Returned: returned}
}

func openOperation(from string, at *uint64, cursor, invoked, returned uint64) historyOperation {
	return historyOperation{Call: "open", From: from, At: at, Cursor: ptr(cursor), Invoked: invoked, Returned: returned}
}

func itemOperation(seq, value, invoked, returned uint64) historyOperation {
	return historyOperation{Call: "read", Outcome: "item", Seq: ptr(seq), Value: ptr(value), Invoked: invoked, Returned: returned}
}

func emptyOperation(invoked, returned uint64) historyOperation {
	return historyOperation{Call: "read", Outcome: "empty", Invoked: invoked, Returned: returned}
}

func laggedOperation(skipped, invoked, returned uint64) historyOperation {
	return historyOperation{Call: "read", Outcome: "lagged", Skipped: ptr(skipped), Invoked: invoked, Returned: returned}
}

func headOperation(head, invoked, returned uint64) historyOperation {
	return historyOperation{Call: "head", Head: ptr(head), Invoked: invoked, Returned: returned}
}

const (
	// runsPerShape runs of each log shape are recorded: short runs, many of
	// them, because the schedules that matter are a small fraction of those a
	// machine gives.
	runsPerShape = 25
	// publishesPerRun entries are published in each run.
	publishesPerRun = 24
	// readsPerReader reads each reader makes in each run, whatever they find.
	readsPerReader = 24
)

// historyShape is a log shape the runs record: the retention the log under test
// is built with and the width of its entries. The log holds one slot more than
// the retention, which the history does not say.
type historyShape struct{ retention, words int }

// historyShapes are the shapes recorded. A retention of one is the narrowest
// log, whose readers lag at nearly every publish; the widest has more entries
// retained than a reader reads between two publishes.
var historyShapes = []historyShape{{1, 1}, {2, 2}, {3, 2}, {7, 3}}

// historyOpening is how one recorded reader opens: from where, after how many
// yields to the scheduler. A run opens readers at the oldest entry, at the
// head, and at a fixed sequence number, which the log clamps to the head.
type historyOpening struct {
	from   string
	at     uint64
	yields int
}

var historyOpenings = []historyOpening{{"oldest", 0, 0}, {"head", 0, 8}, {"at", 3, 4}}

// historyDir is where the histories are written, or false when nowhere is
// named.
func historyDir(t *testing.T) (string, bool) {
	t.Helper()
	dir := os.Getenv("SCE_BROADCAST_HISTORY_DIR")
	if dir == "" {
		t.Skip("SCE_BROADCAST_HISTORY_DIR names where the histories are written; it is unset")
		return "", false
	}
	if err := os.MkdirAll(dir, 0o755); err != nil {
		t.Fatalf("cannot create %s: %v", dir, err)
	}
	return dir, true
}

func writeHistory(t *testing.T, dir, name string, document historyDocument) {
	t.Helper()
	document.Version = 1
	// A participant that made no operation is an empty list in the history, not
	// `null`: Go marshals a nil slice as null, which the format refuses.
	if document.Publisher == nil {
		document.Publisher = []historyOperation{}
	}
	if document.Readers == nil {
		document.Readers = [][]historyOperation{}
	}
	for i := range document.Readers {
		if document.Readers[i] == nil {
			document.Readers[i] = []historyOperation{}
		}
	}
	text, err := json.Marshal(document)
	if err != nil {
		t.Fatalf("%s: %v", name, err)
	}
	path := filepath.Join(dir, name+".json")
	if err := os.WriteFile(path, append(text, '\n'), 0o644); err != nil {
		t.Fatalf("cannot write %s: %v", path, err)
	}
}

// recordRun records one publisher and three readers on four goroutines through
// a log of the shape, and returns what they observed.
func recordRun(t *testing.T, shape historyShape) historyDocument {
	t.Helper()
	log := New(shape.words, shape.retention+1)
	var clock atomic.Uint64
	tick := func() uint64 { return clock.Add(1) }
	start := make(chan struct{})

	var publisherOps []historyOperation
	readerOps := make([][]historyOperation, len(historyOpenings))
	var wg sync.WaitGroup

	wg.Add(1)
	go func() {
		defer wg.Done()
		publisher, ok := log.Publisher()
		if !ok {
			t.Error("the publisher place is free")
			return
		}
		defer publisher.Release()
		words := make([]uint64, shape.words)
		<-start
		for i := 0; i < publishesPerRun; i++ {
			// The value is never zero, so a slot that was not written does not
			// look like an entry; every word of an entry is the value.
			value := uint64(i) + 1
			for k := range words {
				words[k] = value
			}
			invoked := tick()
			seq := publisher.Publish(words)
			returned := tick()
			publisherOps = append(publisherOps, publishOperation(value, seq, invoked, returned))
			if i%4 == 3 {
				invoked = tick()
				head := log.Head()
				returned = tick()
				publisherOps = append(publisherOps, headOperation(head, invoked, returned))
			}
			runtime.Gosched()
		}
	}()

	for who, opening := range historyOpenings {
		wg.Add(1)
		go func(who int, opening historyOpening) {
			defer wg.Done()
			out := make([]uint64, shape.words)
			var ops []historyOperation
			<-start
			for y := 0; y < opening.yields; y++ {
				runtime.Gosched()
			}
			var reader Reader
			var at *uint64
			invoked := tick()
			switch opening.from {
			case "oldest":
				reader = log.ReaderOldest()
			case "head":
				reader = log.ReaderHead()
			default:
				reader = log.ReaderAt(opening.at)
				at = ptr(opening.at)
			}
			returned := tick()
			ops = append(ops, openOperation(opening.from, at, reader.Cursor(), invoked, returned))
			for i := 0; i < readsPerReader; i++ {
				invoked = tick()
				result := log.Read(&reader, out)
				returned = tick()
				switch result.Kind {
				case ReadItem:
					// Every word of an entry is the value it was published
					// with. A copy that mixed two entries is a failure of the
					// run here, and is also recorded by its first word for the
					// checker to refuse.
					for k, word := range out {
						if word != out[0] {
							t.Errorf("shape %+v reader %d: item %d has word %d equal to %d, not %d, a torn entry",
								shape, who, result.Seq, k, word, out[0])
						}
					}
					ops = append(ops, itemOperation(result.Seq, out[0], invoked, returned))
				case ReadEmpty:
					ops = append(ops, emptyOperation(invoked, returned))
				case ReadLagged:
					ops = append(ops, laggedOperation(result.Skipped, invoked, returned))
				default:
					t.Errorf("shape %+v reader %d: answer %+v", shape, who, result)
				}
				if i%8 == 7 {
					invoked = tick()
					head := log.Head()
					returned = tick()
					ops = append(ops, headOperation(head, invoked, returned))
				}
				runtime.Gosched()
			}
			readerOps[who] = ops
		}(who, opening)
	}

	close(start)
	wg.Wait()
	return historyDocument{Retention: shape.retention, Publisher: publisherOps, Readers: readerOps}
}

func TestTheBroadcastLogRunsAreWrittenAsHistories(t *testing.T) {
	dir, ok := historyDir(t)
	if !ok {
		return
	}
	for _, shape := range historyShapes {
		for run := 0; run < runsPerShape; run++ {
			document := recordRun(t, shape)
			if len(document.Publisher) < publishesPerRun {
				t.Fatalf("shape %+v run %d: the publisher recorded %d operations", shape, run, len(document.Publisher))
			}
			writeHistory(t, dir, fmt.Sprintf("go_broadcast_r%d_w%d_%d", shape.retention, shape.words, run), document)
		}
	}
}

// A reader that made no operation is a nil slice, which encoding/json writes as
// null. The history format refuses null, so the writer must turn it into the
// empty list. Pinned here without a run.
func TestAParticipantThatMadeNoOperationIsWrittenAsAnEmptyListNotNull(t *testing.T) {
	dir := t.TempDir()
	document := historyDocument{
		Retention: 2,
		Publisher: []historyOperation{publishOperation(1, 0, 1, 2)},
		Readers: [][]historyOperation{
			nil,
			{openOperation("oldest", nil, 0, 3, 4), itemOperation(0, 1, 5, 6)},
		},
	}
	writeHistory(t, dir, "no_operation", document)
	text, err := os.ReadFile(filepath.Join(dir, "no_operation.json"))
	if err != nil {
		t.Fatal(err)
	}
	if strings.Contains(string(text), "null") {
		t.Fatalf("a participant with no operation was written as null: %s", text)
	}
	if !strings.Contains(string(text), `"readers":[[],[{`) {
		t.Fatalf("the first reader is not the empty list: %s", text)
	}
	empty := t.TempDir()
	writeHistory(t, empty, "nothing", historyDocument{Retention: 1})
	text, err = os.ReadFile(filepath.Join(empty, "nothing.json"))
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(text), `"publisher":[]`) || !strings.Contains(string(text), `"readers":[]`) {
		t.Fatalf("a run with no participants is not written as empty lists: %s", text)
	}
}

// Each form of operation carries exactly the keys the schema allows it
// (additionalProperties is false and each form forbids the others' keys), and a
// zero is written, not dropped as an absent field.
func TestEachOperationIsWrittenWithExactlyTheKeysItsFormAllows(t *testing.T) {
	zero := uint64(0)
	cases := []struct {
		name string
		op   historyOperation
		keys []string
	}{
		{"publish", publishOperation(0, 0, 1, 2), []string{"call", "invoked", "returned", "seq", "value"}},
		{"open at the oldest", openOperation("oldest", nil, 0, 1, 2), []string{"call", "cursor", "from", "invoked", "returned"}},
		{"open at a sequence number", openOperation("at", &zero, 0, 1, 2), []string{"at", "call", "cursor", "from", "invoked", "returned"}},
		{"read an item", itemOperation(0, 0, 1, 2), []string{"call", "invoked", "outcome", "returned", "seq", "value"}},
		{"read empty", emptyOperation(1, 2), []string{"call", "invoked", "outcome", "returned"}},
		{"read lagged", laggedOperation(3, 1, 2), []string{"call", "invoked", "outcome", "returned", "skipped"}},
		{"head", headOperation(0, 1, 2), []string{"call", "head", "invoked", "returned"}},
	}
	for _, c := range cases {
		text, err := json.Marshal(c.op)
		if err != nil {
			t.Fatalf("%s: %v", c.name, err)
		}
		var decoded map[string]any
		if err := json.Unmarshal(text, &decoded); err != nil {
			t.Fatalf("%s: %v", c.name, err)
		}
		got := make([]string, 0, len(decoded))
		for key := range decoded {
			got = append(got, key)
		}
		sort.Strings(got)
		want := append([]string(nil), c.keys...)
		sort.Strings(want)
		if strings.Join(got, ",") != strings.Join(want, ",") {
			t.Errorf("%s is written with the keys %v, want %v", c.name, got, want)
		}
	}
}

// A recorded run is well formed in the ways the recorder alone can break: the
// clock readings are distinct and ordered within a participant, and the
// publisher's sequence numbers are its program order. The checker would refuse
// a run that is not, as malformed; this holds the recorder to it without it.
func TestARecordedRunKeepsItsClockAndItsSequenceNumbersInOrder(t *testing.T) {
	document := recordRun(t, historyShape{retention: 2, words: 2})
	seen := map[uint64]bool{}
	participants := append([][]historyOperation{document.Publisher}, document.Readers...)
	for who, ops := range participants {
		var last uint64
		for at, op := range ops {
			if op.Invoked <= last || op.Returned <= op.Invoked {
				t.Fatalf("participant %d operation %d: invoked %d, returned %d, after %d", who, at, op.Invoked, op.Returned, last)
			}
			if seen[op.Invoked] || seen[op.Returned] {
				t.Fatalf("participant %d operation %d: a clock reading was taken twice", who, at)
			}
			seen[op.Invoked], seen[op.Returned] = true, true
			last = op.Returned
		}
	}
	published := uint64(0)
	for _, op := range document.Publisher {
		if op.Call != "publish" {
			continue
		}
		if *op.Seq != published || *op.Value != published+1 {
			t.Fatalf("publish number %d took the sequence number %d with value %d", published, *op.Seq, *op.Value)
		}
		published++
	}
	if published != publishesPerRun {
		t.Fatalf("the publisher recorded %d publishes, want %d", published, publishesPerRun)
	}
	for who, ops := range document.Readers {
		if len(ops) == 0 || ops[0].Call != "open" {
			t.Fatalf("reader %d does not begin by opening", who)
		}
	}
}
