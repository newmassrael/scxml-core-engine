// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// A Go machine reads a composed payload at any depth (docs/adr/0086, section
// 6), run with no script engine, as the Rust and Kotlin machines do
// (backends/rust/tests/tests/a_composed_payload_is_read_at_any_depth.rs,
// AComposedPayloadIsReadAtAnyDepthTest.kt). The committed machine under
// `static_composed_payload/` is generated from
// `sce-build/tests/fixtures/static_datamodel/static_composed_payload.scxml`
// (regen: `scripts/regen_static_datamodel_go.sh`).
//
// A delivery whose every list is full is lifted into the machine's own types,
// and each read at depths 1 to 3 is what a host reads back. A delivery one past
// a bound at depth 2 is refused whole (section 4.2): none of the content runs,
// and `error.execution` is raised once. A value a host injects through the seam
// is held to every bound its type declares, at any depth, before anything is
// sent (section 5.4): within them it is read as any delivery, and one past a
// bound is answered as the refusal that names its path, and raises nothing.

package static_datamodel

import (
	"fmt"
	"reflect"
	"strings"
	"testing"

	"github.com/newmassrael/sce-forge-runtime/optional"
	sce "github.com/newmassrael/sce-go-runtime"

	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_composed_payload"
)

// composedItem is one item of a batch as a delivery carries it, its `tags`
// holding tags texts.
func composedItem(id, tags int) string {
	words := make([]string, tags)
	for i := range words {
		words[i] = fmt.Sprintf(`"tag%d"`, i)
	}
	return fmt.Sprintf(`{"id":%d,"amount":"9007199254740993","label":"abcdefgh","level":"high","due":null,"blob":"wxyz","tags":[%s]}`,
		id, strings.Join(words, ","))
}

// composedBatch is a delivery of `batch.sent` whose every list is full but the
// items' `tags`, item n holding tags[n-1] texts.
func composedBatch(tags ...int) string {
	items := make([]string, len(tags))
	for i, n := range tags {
		items[i] = composedItem(i+1, n)
	}
	return `{"items":[` + strings.Join(items, ",") +
		`],"grid":[[1,2],[3,4]],"levels":["low","critical"],"maybe":[7,null],"spare":[1,2],"note":"abcd","late":[5,6]}`
}

// settleComposed runs a machine to quiescence after an event: one that needs
// its scheduler is ticked as the scenarios tick one, any other stepped.
func settleComposed(policy any, step, tick func()) {
	if scheduled, ok := policy.(interface{ NeedsEventScheduler() bool }); ok && scheduled.NeedsEventScheduler() {
		for i := 0; i < settleTicks; i++ {
			tick()
		}
		return
	}
	step()
}

// composedPayloadEngine is the engine of a started static_composed_payload
// machine, and the machine.
func composedPayloadEngine() (*sce.Engine[static_composed_payload.StaticComposedPayloadState, static_composed_payload.StaticComposedPayloadEvent], *static_composed_payload.StaticComposedPayloadPolicy) {
	policy := static_composed_payload.NewStaticComposedPayloadPolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[static_composed_payload.StaticComposedPayloadState, static_composed_payload.StaticComposedPayloadEvent](&policy)
	engine.Initialize()
	return engine, &policy
}

// deliveredComposedPayload is a static_composed_payload machine that took one
// delivery of `batch.sent` carrying data.
func deliveredComposedPayload(data string) *static_composed_payload.StaticComposedPayloadPolicy {
	engine, policy := composedPayloadEngine()
	engine.RaiseExternalByName("batch.sent", data)
	settleComposed(policy, engine.Step, engine.Tick)
	return policy
}

func TestAFullDeliveryIsReadAtEveryDepth(t *testing.T) {
	policy := deliveredComposedPayload(composedBatch(4, 4, 4))
	if got := policy.Refusals(); got != 0 {
		t.Errorf("a delivery of its type raised %d errors", got)
	}
	// Depth 1: the length of a list of the payload.
	if got := policy.ItemCount(); got != 3 {
		t.Errorf("item_count is %d, not 3", got)
	}
	// Depth 2: a loop over each item's list, and a field of an element read
	// through its index.
	if got := policy.TagCount(); got != 12 {
		t.Errorf("tag_count is %d, not 12", got)
	}
	if got := policy.WordCount(); got != 12 {
		t.Errorf("word_count is %d, not 12", got)
	}
	if got := policy.IdSum(); got != 6 {
		t.Errorf("id_sum is %d, not 6", got)
	}
	// A list of lists, walked at both depths and read by two indices.
	if got := policy.CellSum(); got != 10 {
		t.Errorf("cell_sum is %d, not 10", got)
	}
	if got := policy.Cell(); got != 3 {
		t.Errorf("cell is %d, not 3", got)
	}
	// Depth 3: the length of the first item's `tags`.
	if got := policy.FirstTags(); got != 4 {
		t.Errorf("first_tags is %d, not 4", got)
	}
	// An element that may be absent, present.
	if got := policy.FirstMaybe(); !got.Equal(optional.Some[uint16](7)) {
		t.Errorf("first_maybe is %v, not 7", got)
	}
	// A list taken whole into a list of a larger bound.
	if got := policy.Late(); !reflect.DeepEqual(got, []uint8{5, 6}) {
		t.Errorf("late is %v, not [5 6]", got)
	}
}

func TestAListPastItsBoundAtDepthTwoIsRefusedWhole(t *testing.T) {
	policy := deliveredComposedPayload(composedBatch(4, 5, 4))
	if got := policy.Refusals(); got != 1 {
		t.Errorf("error.execution reached the document %d times, not once", got)
	}
	if got := policy.ItemCount(); got != 0 {
		t.Errorf("item_count is %d: some of the content ran", got)
	}
	if got := policy.Late(); len(got) != 0 {
		t.Errorf("late is %v: something of the delivery was kept", got)
	}
}

// composedHostItem is an item a host builds, its `tags` holding the texts given.
func composedHostItem(id uint32, tags ...string) static_composed_payload.StaticComposedPayloadItemRecord {
	return static_composed_payload.NewStaticComposedPayloadItemRecord(id, 2, "abc",
		static_composed_payload.StaticComposedPayloadSignalLevelEnumLow, optional.None[int64](),
		[]byte("wx"), tags)
}

// injectComposedPayload hands a started machine's seam `batch.sent` with late
// and items, and answers the machine after it ran and what the seam answered.
func injectComposedPayload(late []uint8, items ...static_composed_payload.StaticComposedPayloadItemRecord) (*static_composed_payload.StaticComposedPayloadPolicy, error) {
	engine, policy := composedPayloadEngine()
	err := static_composed_payload.StaticComposedPayloadRaiseBatchSent(engine,
		items,
		[][]uint8{{1, 2}},
		[]static_composed_payload.StaticComposedPayloadSignalLevelEnum{static_composed_payload.StaticComposedPayloadSignalLevelEnumLow},
		[]optional.Of[uint16]{optional.Some[uint16](9), optional.None[uint16]()},
		optional.None[[]uint8](),
		"ab",
		late)
	settleComposed(policy, engine.Step, engine.Tick)
	return policy, err
}

func TestAValueAHostInjectsWithinItsBoundsIsReadAsAnyDelivery(t *testing.T) {
	policy, err := injectComposedPayload([]uint8{5, 6}, composedHostItem(4, "t0", "t1"))
	if err != nil {
		t.Fatalf("a value within every bound was refused: %v", err)
	}
	if got := policy.Refusals(); got != 0 {
		t.Errorf("a value within every bound raised %d errors", got)
	}
	if got := policy.ItemCount(); got != 1 {
		t.Errorf("item_count is %d, not 1", got)
	}
	if got := policy.TagCount(); got != 2 {
		t.Errorf("tag_count is %d, not 2", got)
	}
	if got := policy.IdSum(); got != 4 {
		t.Errorf("id_sum is %d, not 4", got)
	}
	if got := policy.CellSum(); got != 3 {
		t.Errorf("cell_sum is %d, not 3", got)
	}
	if got := policy.FirstMaybe(); !got.Equal(optional.Some[uint16](9)) {
		t.Errorf("first_maybe is %v, not 9", got)
	}
	if got := policy.Late(); !reflect.DeepEqual(got, []uint8{5, 6}) {
		t.Errorf("late is %v, not [5 6]", got)
	}
}

// `late` is a list<uint8>[2]: three elements are no value of its type, and the
// seam refuses them before anything is sent.
func TestAListAHostInjectsOnePastItsBoundIsRefusedBeforeItIsSent(t *testing.T) {
	policy, err := injectComposedPayload([]uint8{5, 6, 7}, composedHostItem(4, "t0"))
	if err == nil || err.Error() != "'late' holds 3 elements, past the 2 its type declares" {
		t.Errorf("three elements in a bound of two were answered %v", err)
	}
	if got := policy.ItemCount(); got != 0 {
		t.Errorf("item_count is %d: the refused event was raised", got)
	}
	if got := policy.Refusals(); got != 0 {
		t.Errorf("a value refused at the seam raised %d errors in the machine", got)
	}
}

// `tags` is a list<string[8]>[4] inside each item: an item of five tags is
// refused at the path of the list in the element, and a tag of nine bytes at
// its own.
func TestARecordAHostBuildsOnePastAnInnerBoundIsRefusedAtItsPath(t *testing.T) {
	policy, err := injectComposedPayload([]uint8{5}, composedHostItem(1, "a", "b", "c", "d", "e"))
	if err == nil || err.Error() != "'items/0/tags' holds 5 elements, past the 4 its type declares" {
		t.Errorf("five tags in a bound of four were answered %v", err)
	}
	if got := policy.ItemCount(); got != 0 {
		t.Errorf("item_count is %d: the refused event was raised", got)
	}
	_, err = injectComposedPayload([]uint8{5}, composedHostItem(1, "ninebytes"))
	if err == nil || err.Error() != "'items/0/tags/0' is 9 bytes, past the 8 its type declares" {
		t.Errorf("a tag of nine bytes in a bound of eight was answered %v", err)
	}
}
