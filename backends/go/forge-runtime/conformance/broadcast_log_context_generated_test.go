// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package conformance

// The broadcast-log kind's generated packages over elements whose codec asks its
// caller for what its wire form does not carry (ADR 0062 section 4, "Caller inputs
// are part of the element"), compiled and used.
//
// An element's codec may take single-bit flag inputs on Encode and Decode, and a
// variant with no tag field takes its tag on Decode. The log stores them in the
// entry's context words, takes them on Publish and returns them in the item, so
// every reader decodes an entry under what it was published under. Here the package
// the generator writes for each of the two logs is compiled over the element
// packages it imports, and the typed Publish and Read are used on one goroutine: a
// rename in the codec, a context word in the wrong place or a flag read under the
// wrong bit stops the build or fails an assertion.
//
// The six documents are tests/forge/conformance/broadcast_log_context/*.scxml, shared
// by every backend's conformance lane and generated as one document set by
// generate.sh into the same tree as the other fixtures.

import (
	"errors"
	"testing"

	"github.com/newmassrael/sce-forge-runtime/broadcastlog"
	"github.com/newmassrael/sce-forge-runtime/codec"
	blogchoice "github.com/newmassrael/sce-forge-runtime/conformance/generated/broadcast_log_conformance_choice"
	blogchoicelarge "github.com/newmassrael/sce-forge-runtime/conformance/generated/broadcast_log_conformance_choice_large"
	blogchoicesmall "github.com/newmassrael/sce-forge-runtime/conformance/generated/broadcast_log_conformance_choice_small"
	blogchosen "github.com/newmassrael/sce-forge-runtime/conformance/generated/broadcast_log_conformance_chosen"
	blogflagged "github.com/newmassrael/sce-forge-runtime/conformance/generated/broadcast_log_conformance_flagged"
	blogflaggedevent "github.com/newmassrael/sce-forge-runtime/conformance/generated/broadcast_log_conformance_flagged_event"
	"github.com/newmassrael/sce-forge-runtime/optional"
)

// The generated aliases are the runtime's log, and the constructors build one.
var (
	_ *broadcastlog.Log = blogflagged.NewBroadcastLogConformanceFlagged()
	_ *broadcastlog.Log = blogchosen.NewBroadcastLogConformanceChosen()
)

// blogctxEvent is the element of the flagged log: a sensor id and a 16-bit value
// that is on the wire only under the flag.
func blogctxEvent(sensorID uint8, value optional.Of[uint16]) blogflaggedevent.BroadcastLogConformanceFlaggedEvent {
	return blogflaggedevent.BroadcastLogConformanceFlaggedEvent{SensorId: sensorID, Value: value}
}

// blogctxSmall is the choice as its one-byte arm.
func blogctxSmall(payload uint8) blogchoice.BroadcastLogConformanceChoice {
	return blogchoice.BroadcastLogConformanceChoice{
		Body: blogchoice.BroadcastLogConformanceChoiceVariant{
			BroadcastLogConformanceChoiceSmall: &blogchoicesmall.BroadcastLogConformanceChoiceSmall{Payload: payload},
		},
	}
}

// blogctxLarge is the choice as its two-byte arm.
func blogctxLarge(payload uint16) blogchoice.BroadcastLogConformanceChoice {
	return blogchoice.BroadcastLogConformanceChoice{
		Body: blogchoice.BroadcastLogConformanceChoiceVariant{
			BroadcastLogConformanceChoiceLarge: &blogchoicelarge.BroadcastLogConformanceChoiceLarge{Payload: payload},
		},
	}
}

// blogctxPublisher takes the publisher of a log that was just built.
func blogctxPublisher(t *testing.T, log *broadcastlog.Log) *broadcastlog.Publisher {
	t.Helper()
	publisher, ok := log.Publisher()
	if !ok {
		t.Fatal("a fresh log hands out its publisher")
	}
	return publisher
}

// blogctxPublishFlagged publishes element to the flagged log under hasValue.
func blogctxPublishFlagged(t *testing.T, publisher *broadcastlog.Publisher, element blogflaggedevent.BroadcastLogConformanceFlaggedEvent, hasValue bool) uint64 {
	t.Helper()
	seq, err := blogflagged.Publish(publisher, &element, blogflagged.BroadcastLogConformanceFlaggedFlags{HasValue: hasValue})
	if err != nil {
		t.Fatalf("publishing %+v under has_value=%v: %v", element, hasValue, err)
	}
	return seq
}

// blogctxPublishChosen publishes element to the chosen log under tag.
func blogctxPublishChosen(t *testing.T, publisher *broadcastlog.Publisher, element blogchoice.BroadcastLogConformanceChoice, tag byte) uint64 {
	t.Helper()
	seq, err := blogchosen.Publish(publisher, &element, tag)
	if err != nil {
		t.Fatalf("publishing %+v under tag %d: %v", element, tag, err)
	}
	return seq
}

// blogctxRawEntry is the words of the entry the reader's next read copies, as the
// runtime hands them over.
func blogctxRawEntry(t *testing.T, log *broadcastlog.Log, reader *broadcastlog.Reader) []uint64 {
	t.Helper()
	out := make([]uint64, log.Words())
	answer := log.Read(reader, out)
	if answer.Kind != broadcastlog.ReadItem {
		t.Fatalf("raw read: %+v, want an item", answer)
	}
	return out
}

// blogctxExpectWords fails unless got is want, word for word.
func blogctxExpectWords(t *testing.T, name string, got []uint64, want []uint64) {
	t.Helper()
	if len(got) != len(want) {
		t.Fatalf("%s: %d words, want %d", name, len(got), len(want))
	}
	for k := range want {
		if got[k] != want[k] {
			t.Fatalf("%s: word %d = %#x, want %#x", name, k, got[k], want[k])
		}
	}
}

// blogctxExpectFlagged reads the flagged log and fails unless the entry is seq, whole,
// with the flag it was published under.
func blogctxExpectFlagged(t *testing.T, log *blogflagged.BroadcastLogConformanceFlagged, reader *broadcastlog.Reader, seq uint64, element blogflaggedevent.BroadcastLogConformanceFlaggedEvent, hasValue bool) {
	t.Helper()
	answer, item, err := blogflagged.Read(log, reader)
	if err != nil || answer.Kind != broadcastlog.ReadItem || answer.Seq != seq || item == nil {
		t.Fatalf("entry %d: %+v, %+v, %v, want the item of that sequence number", seq, answer, item, err)
	}
	if *item.Element != element {
		t.Fatalf("entry %d came back as %+v, want %+v", seq, *item.Element, element)
	}
	if item.Flags.HasValue != hasValue {
		t.Fatalf("entry %d came back under has_value=%v, want %v: the flag it was published under", seq, item.Flags.HasValue, hasValue)
	}
}

func TestBroadcastLogContextAnEntryHoldsItsContextWordsBetweenTheLengthAndTheBytes(t *testing.T) {
	// The flagged element is an 8-bit id and a gated 16-bit value: at most 3 bytes.
	// One length word, one context word (one flag), one word of bytes.
	if blogflagged.MaxEncodedBytes != 3 || blogflagged.ContextWords != 1 || blogflagged.Words != 3 {
		t.Fatalf("flagged: MaxEncodedBytes, ContextWords, Words = %d, %d, %d, want 3, 1, 3",
			blogflagged.MaxEncodedBytes, blogflagged.ContextWords, blogflagged.Words)
	}
	flaggedLog := blogflagged.NewBroadcastLogConformanceFlagged()
	if flaggedLog.Words() != blogflagged.Words {
		t.Fatalf("the flagged log has %d words an entry, the package states %d", flaggedLog.Words(), blogflagged.Words)
	}
	// The choice is the larger of its arms, two bytes; the tag is one context word.
	if blogchosen.MaxEncodedBytes != 2 || blogchosen.ContextWords != 1 || blogchosen.Words != 3 {
		t.Fatalf("chosen: MaxEncodedBytes, ContextWords, Words = %d, %d, %d, want 2, 1, 3",
			blogchosen.MaxEncodedBytes, blogchosen.ContextWords, blogchosen.Words)
	}
	chosenLog := blogchosen.NewBroadcastLogConformanceChosen()
	if chosenLog.Words() != blogchosen.Words {
		t.Fatalf("the chosen log has %d words an entry, the package states %d", chosenLog.Words(), blogchosen.Words)
	}
}

func TestBroadcastLogContextTheFlagIsStoredInTheContextWordAndTheBytesFollowIt(t *testing.T) {
	log := blogflagged.NewBroadcastLogConformanceFlagged()
	publisher := blogctxPublisher(t, log)
	// Under the flag the element is three bytes (the id, then the value little-endian) and
	// the context word is 1; without it the element is the id alone and the word is 0.
	blogctxPublishFlagged(t, publisher, blogctxEvent(1, optional.Some(uint16(0x0203))), true)
	blogctxPublishFlagged(t, publisher, blogctxEvent(2, optional.None[uint16]()), false)

	reader := log.ReaderOldest()
	blogctxExpectWords(t, "entry 0", blogctxRawEntry(t, log, &reader), []uint64{3, 1, 0x020301})
	blogctxExpectWords(t, "entry 1", blogctxRawEntry(t, log, &reader), []uint64{1, 0, 0x02})
}

func TestBroadcastLogContextAReaderGetsBackTheFlagAnEntryWasPublishedUnder(t *testing.T) {
	log := blogflagged.NewBroadcastLogConformanceFlagged()
	publisher := blogctxPublisher(t, log)
	reader := log.ReaderOldest()

	// The same element shape, published under each answer of the flag: the value is on the
	// wire under true and absent under false, so the two frames are three bytes and one,
	// and neither can be decoded under the other's flag.
	with := blogctxEvent(1, optional.Some(uint16(0x0203)))
	without := blogctxEvent(2, optional.None[uint16]())
	blogctxPublishFlagged(t, publisher, with, true)
	blogctxPublishFlagged(t, publisher, without, false)

	blogctxExpectFlagged(t, log, &reader, 0, with, true)
	blogctxExpectFlagged(t, log, &reader, 1, without, false)
	if answer, item, err := blogflagged.Read(log, &reader); err != nil || answer.Kind != broadcastlog.ReadEmpty || item != nil {
		t.Fatalf("everything was read: %+v, %+v, %v", answer, item, err)
	}
}

func TestBroadcastLogContextEveryReaderOfAnEntryDecodesItUnderTheSameFlag(t *testing.T) {
	log := blogflagged.NewBroadcastLogConformanceFlagged()
	publisher := blogctxPublisher(t, log)
	early := log.ReaderOldest()
	late := log.ReaderOldest()
	with := blogctxEvent(5, optional.Some(uint16(500)))
	without := blogctxEvent(6, optional.None[uint16]())
	blogctxPublishFlagged(t, publisher, with, true)
	blogctxPublishFlagged(t, publisher, without, false)

	for _, reader := range []*broadcastlog.Reader{&early, &late} {
		blogctxExpectFlagged(t, log, reader, 0, with, true)
		blogctxExpectFlagged(t, log, reader, 1, without, false)
	}
}

func TestBroadcastLogContextALappedReaderResumesAtAnEntryWithItsOwnFlag(t *testing.T) {
	log := blogflagged.NewBroadcastLogConformanceFlagged()
	publisher := blogctxPublisher(t, log)
	slow := log.ReaderOldest()

	// Six entries, sequence numbers 0 to 5, into a log that retains three: the window is
	// [3, 6), so the reader at 0 missed 3. The flag alternates, so the entries it resumes
	// at are 3 (no value), 4 (a value) and 5 (no value), each under its own flag.
	for n := uint16(0); n < 6; n++ {
		if n%2 == 0 {
			blogctxPublishFlagged(t, publisher, blogctxEvent(9, optional.Some(n)), true)
		} else {
			blogctxPublishFlagged(t, publisher, blogctxEvent(9, optional.None[uint16]()), false)
		}
	}
	answer, item, err := blogflagged.Read(log, &slow)
	if err != nil || answer.Kind != broadcastlog.ReadLagged || answer.Skipped != 3 || item != nil {
		t.Fatalf("the lapped reader was answered %+v, %+v, %v, want lagged by 3 and no item", answer, item, err)
	}
	blogctxExpectFlagged(t, log, &slow, 3, blogctxEvent(9, optional.None[uint16]()), false)
	blogctxExpectFlagged(t, log, &slow, 4, blogctxEvent(9, optional.Some(uint16(4))), true)
	blogctxExpectFlagged(t, log, &slow, 5, blogctxEvent(9, optional.None[uint16]()), false)
}

func TestBroadcastLogContextAFlagItsElementContradictsIsRefusedByTheCodecAndNothingIsPublished(t *testing.T) {
	// The codec's Encode checks a flag input against the field it gates before it writes a
	// byte (codec.ErrPresentIfMismatch, as the Rust codec does). The log stores what the
	// codec accepted and judges nothing of its own, so a contradiction never reaches an
	// entry: nothing is published and no sequence number is taken.
	log := blogflagged.NewBroadcastLogConformanceFlagged()
	publisher := blogctxPublisher(t, log)

	cases := []struct {
		name     string
		element  blogflaggedevent.BroadcastLogConformanceFlaggedEvent
		hasValue bool
	}{
		// The value is there and the flag says it is not.
		{"a value under a flag that says it is absent", blogctxEvent(1, optional.Some(uint16(0x0203))), false},
		// The value is absent and the flag says it is there.
		{"an absent value under a flag that says it is there", blogctxEvent(2, optional.None[uint16]()), true},
	}
	for _, c := range cases {
		element := c.element
		_, err := blogflagged.Publish(publisher, &element, blogflagged.BroadcastLogConformanceFlaggedFlags{HasValue: c.hasValue})
		if !errors.Is(err, codec.ErrPresentIfMismatch) {
			t.Fatalf("%s: publish returned %v, want codec.ErrPresentIfMismatch", c.name, err)
		}
		if log.Head() != 0 {
			t.Fatalf("%s: a refused flag publishes nothing, the head is %d", c.name, log.Head())
		}
	}

	// An element and a flag that agree are published as the first entry.
	agreeing := blogctxEvent(1, optional.Some(uint16(0x0203)))
	if seq := blogctxPublishFlagged(t, publisher, agreeing, true); seq != 0 {
		t.Fatalf("the first agreeing entry is number %d, want 0: the refusals took no number", seq)
	}
	reader := log.ReaderOldest()
	blogctxExpectFlagged(t, log, &reader, 0, agreeing, true)
}

func TestBroadcastLogContextTheTagIsStoredInTheContextWordAndTheArmsBytesFollowIt(t *testing.T) {
	log := blogchosen.NewBroadcastLogConformanceChosen()
	publisher := blogctxPublisher(t, log)
	// The wire form of an arm carries no tag: the small arm is its one byte and the large arm
	// its two bytes little-endian, and the context word is the tag the publisher gave.
	blogctxPublishChosen(t, publisher, blogctxSmall(9), 1)
	blogctxPublishChosen(t, publisher, blogctxLarge(0x0102), 0)

	reader := log.ReaderOldest()
	blogctxExpectWords(t, "entry 0", blogctxRawEntry(t, log, &reader), []uint64{1, 1, 0x09})
	blogctxExpectWords(t, "entry 1", blogctxRawEntry(t, log, &reader), []uint64{2, 0, 0x0102})
}

func TestBroadcastLogContextAReaderGetsBackTheTagAnEntryWasPublishedUnder(t *testing.T) {
	log := blogchosen.NewBroadcastLogConformanceChosen()
	publisher := blogctxPublisher(t, log)
	reader := log.ReaderOldest()

	// The wire form of an arm carries no tag, so which arm a frame is comes from the tag
	// stored beside it.
	blogctxPublishChosen(t, publisher, blogctxSmall(9), 1)
	blogctxPublishChosen(t, publisher, blogctxLarge(0x0102), 0)
	blogctxPublishChosen(t, publisher, blogctxSmall(255), 1)

	type expected struct {
		tag     byte
		small   uint8
		large   uint16
		isSmall bool
	}
	for seq, want := range []expected{
		{tag: 1, small: 9, isSmall: true},
		{tag: 0, large: 0x0102},
		{tag: 1, small: 255, isSmall: true},
	} {
		answer, item, err := blogchosen.Read(log, &reader)
		if err != nil || answer.Kind != broadcastlog.ReadItem || answer.Seq != uint64(seq) || item == nil {
			t.Fatalf("entry %d: %+v, %+v, %v, want the item of that sequence number", seq, answer, item, err)
		}
		if item.Tag != want.tag {
			t.Fatalf("entry %d came back under tag %d, want %d", seq, item.Tag, want.tag)
		}
		arms := item.Element.Body
		if want.isSmall {
			if arms.BroadcastLogConformanceChoiceSmall == nil || arms.BroadcastLogConformanceChoiceLarge != nil ||
				arms.BroadcastLogConformanceChoiceSmall.Payload != want.small {
				t.Fatalf("entry %d is %+v, want the small arm holding %d", seq, arms, want.small)
			}
		} else {
			if arms.BroadcastLogConformanceChoiceLarge == nil || arms.BroadcastLogConformanceChoiceSmall != nil ||
				arms.BroadcastLogConformanceChoiceLarge.Payload != want.large {
				t.Fatalf("entry %d is %+v, want the large arm holding %d", seq, arms, want.large)
			}
		}
	}
	if answer, item, err := blogchosen.Read(log, &reader); err != nil || answer.Kind != broadcastlog.ReadEmpty || item != nil {
		t.Fatalf("everything was read: %+v, %+v, %v", answer, item, err)
	}
}

func TestBroadcastLogContextATagThatSelectsNoArmIsTheCodecsErrorAndTheLogDoesNotJudgeIt(t *testing.T) {
	// The log stores the tag it is given and does not know which tags the variant has. A
	// tag that selects no arm is stored, and the typed read is the codec's error for it
	// (the variant has no catch-all arm): no item, and the reader has moved past the entry.
	log := blogchosen.NewBroadcastLogConformanceChosen()
	publisher := blogctxPublisher(t, log)
	blogctxPublishChosen(t, publisher, blogctxSmall(9), 7)

	reader := log.ReaderOldest()
	answer, item, err := blogchosen.Read(log, &reader)
	if !errors.Is(err, codec.ErrNeedMoreBytes) || item != nil {
		t.Fatalf("a tag of no arm: %+v, %v, want codec.ErrNeedMoreBytes and no item", item, err)
	}
	if answer.Kind != broadcastlog.ReadItem || reader.Cursor() != 1 {
		t.Fatalf("answer %+v, cursor %d, want the item and a reader past it", answer, reader.Cursor())
	}
}
