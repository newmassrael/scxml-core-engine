// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package conformance

// The broadcast-log kind's generated packages (RFC §synth-5-S, ADR 0062), compiled and used.
//
// sce-build's tests read what the generator writes for a log and look for text
// in it, which shows the text is what the template says and nothing about
// whether the names in it are the names sce-forge-runtime/broadcastlog and the
// element's codec have, or whether the bytes it packs into a slot read back as
// the element that was published. Here the packages the generator writes for
// three logs are compiled into this test over the element packages they
// import, and the typed Publish and Read they define are used, on one
// goroutine: a rename in the runtime, a width the template computes wrongly,
// or a byte packed into the wrong word stops the build or fails an assertion.
// What the log does under several goroutines is broadcastlog's own tests; this
// file holds what the generated package adds.
//
// The packages come from tests/forge/resources/broadcast_log_conformance_*.scxml,
// written by generate.sh into the same tree as the numerical fixtures. A log is
// generated from its document set and never alone, because a slot is sized from
// the element document's largest encoded size.

import (
	"bytes"
	"errors"
	"math"
	"testing"

	"github.com/newmassrael/sce-forge-runtime/broadcastlog"
	"github.com/newmassrael/sce-forge-runtime/codec"
	blogbounded "github.com/newmassrael/sce-forge-runtime/conformance/generated/broadcast_log_conformance_bounded"
	blogevent "github.com/newmassrael/sce-forge-runtime/conformance/generated/broadcast_log_conformance_event"
	blogliteral "github.com/newmassrael/sce-forge-runtime/conformance/generated/broadcast_log_conformance_literal"
	blogrecord "github.com/newmassrael/sce-forge-runtime/conformance/generated/broadcast_log_conformance_record"
)

// The generated alias is the runtime's log, and the constructor builds one.
var _ *broadcastlog.Log = blogliteral.NewBroadcastLogConformanceLiteral()

// What the generated package states is checked when this file is compiled: a
// constant the template computes wrongly stops the build. A negative constant
// is not a uint, so each pair holds the value from both sides.
const (
	_ = uint(blogliteral.Retention - 3)
	_ = uint(3 - blogliteral.Retention)
	_ = uint(blogliteral.Words - 2)
	_ = uint(2 - blogliteral.Words)
)

// blogEventRead is one typed read of the literal log: the runtime's answer, the
// decoded element, and the error, so that a test names what it expects of each.
type blogEventRead struct {
	answer  broadcastlog.Read
	element *blogevent.BroadcastLogConformanceEvent
	err     error
}

func blogReadEvent(log *blogliteral.BroadcastLogConformanceLiteral, reader *broadcastlog.Reader) blogEventRead {
	answer, element, err := blogliteral.Read(log, reader)
	return blogEventRead{answer: answer, element: element, err: err}
}

func blogNewLiteralPublisher(t *testing.T, log *blogliteral.BroadcastLogConformanceLiteral) *broadcastlog.Publisher {
	t.Helper()
	publisher, ok := log.Publisher()
	if !ok {
		t.Fatal("a fresh log hands out its publisher")
	}
	return publisher
}

func blogPublishEvent(t *testing.T, publisher *broadcastlog.Publisher, element blogevent.BroadcastLogConformanceEvent) uint64 {
	t.Helper()
	seq, err := blogliteral.Publish(publisher, &element)
	if err != nil {
		t.Fatalf("publishing %+v: %v", element, err)
	}
	return seq
}

// ─── The contract the packages state ───

func TestBroadcastLogLiteralStatesItsRetentionAndSizesItsSlotsFromItsElement(t *testing.T) {
	// <sce:retention entries="3"/>.
	if blogliteral.Retention != 3 {
		t.Fatalf("Retention = %d, want the document's 3", blogliteral.Retention)
	}
	// One more than the retention: the slot being overwritten is in flux.
	if blogliteral.Slots != 4 {
		t.Fatalf("Slots = %d, want 3 + 1 = 4", blogliteral.Slots)
	}
	// The element is an 8-bit id and a 16-bit value: 1 + 2 = 3 bytes.
	if blogliteral.MaxEncodedBytes != 3 {
		t.Fatalf("MaxEncodedBytes = %d, want 1 + 2 = 3", blogliteral.MaxEncodedBytes)
	}
	// One length word, then (3 + 7) / 8 = 1 word of bytes: 1 + 1 = 2.
	if blogliteral.Words != 2 {
		t.Fatalf("Words = %d, want 1 + 1 = 2", blogliteral.Words)
	}
	if blogliteral.DeclaredProgress != "wait-free" {
		t.Fatalf("DeclaredProgress = %q, want wait-free", blogliteral.DeclaredProgress)
	}
}

func TestBroadcastLogBoundedSizesItsSlotsFromTheLargestRecord(t *testing.T) {
	// <sce:retention entries="2"/>, so a slot count of 2 + 1 = 3.
	if blogbounded.Retention != 2 || blogbounded.Slots != 3 {
		t.Fatalf("Retention, Slots = %d, %d, want 2, 3", blogbounded.Retention, blogbounded.Slots)
	}
	// A one-byte tag and a payload of at most eight bytes: 1 + 8 = 9 bytes.
	if blogbounded.MaxEncodedBytes != 9 {
		t.Fatalf("MaxEncodedBytes = %d, want 1 + 8 = 9", blogbounded.MaxEncodedBytes)
	}
	// One length word, then (9 + 7) / 8 = 2 words of bytes: 1 + 2 = 3.
	if blogbounded.Words != 3 {
		t.Fatalf("Words = %d, want 1 + 2 = 3", blogbounded.Words)
	}
}

func TestBroadcastLogTheLogItDefinesIsTheRuntimesLogOfTheStatedSize(t *testing.T) {
	log := blogliteral.NewBroadcastLogConformanceLiteral()
	if log.Words() != blogliteral.Words || log.Slots() != blogliteral.Slots {
		t.Fatalf("the log has %d words in %d slots, the package states %d in %d",
			log.Words(), log.Slots(), blogliteral.Words, blogliteral.Slots)
	}
	if log.Retention() != uint64(blogliteral.Retention) {
		t.Fatalf("the log retains %d, the package states %d", log.Retention(), blogliteral.Retention)
	}
}

// ─── The log it defines is the log the runtime has ───

func TestBroadcastLogEntriesComeBackWholeAndInOrderThroughTheTypedFunctions(t *testing.T) {
	log := blogliteral.NewBroadcastLogConformanceLiteral()
	publisher := blogNewLiteralPublisher(t, log)
	reader := log.ReaderOldest()
	if got := blogReadEvent(log, &reader); got.err != nil || got.answer.Kind != broadcastlog.ReadEmpty || got.element != nil {
		t.Fatalf("a new log is empty: %+v", got)
	}

	// Values that fill every byte of the packed word and differ in each.
	sent := []blogevent.BroadcastLogConformanceEvent{
		{SensorId: 1, Value: 0x0102},
		{SensorId: 2, Value: 0xFFEE},
		{SensorId: 255, Value: 0},
	}
	for n := range sent {
		if seq := blogPublishEvent(t, publisher, sent[n]); seq != uint64(n) {
			t.Fatalf("entry %d took sequence number %d: an entry takes the next one", n, seq)
		}
	}
	for n := range sent {
		got := blogReadEvent(log, &reader)
		if got.err != nil || got.answer.Kind != broadcastlog.ReadItem || got.answer.Seq != uint64(n) {
			t.Fatalf("read %d: %+v, want the item of sequence number %d", n, got, n)
		}
		if got.element == nil || *got.element != sent[n] {
			t.Fatalf("entry %d came back as %+v, want %+v: whole, in the order published", n, got.element, sent[n])
		}
	}
	if got := blogReadEvent(log, &reader); got.err != nil || got.answer.Kind != broadcastlog.ReadEmpty || got.element != nil {
		t.Fatalf("everything was read: %+v", got)
	}
	if reader.Lost() != 0 {
		t.Fatalf("a reader that kept up lost %d, want 0", reader.Lost())
	}
}

func TestBroadcastLogPacksTheLengthThenTheBytesLittleEndianEightToAWord(t *testing.T) {
	log := blogliteral.NewBroadcastLogConformanceLiteral()
	publisher := blogNewLiteralPublisher(t, log)
	blogPublishEvent(t, publisher, blogevent.BroadcastLogConformanceEvent{SensorId: 1, Value: 0x0102})
	blogPublishEvent(t, publisher, blogevent.BroadcastLogConformanceEvent{SensorId: 2, Value: 0xFFEE})

	// The element is little-endian: the id, then the low byte of the value, then
	// its high byte. 1, 0x02, 0x01 packs to 0x01<<16 | 0x02<<8 | 0x01 = 0x010201,
	// and 2, 0xEE, 0xFF to 0xFF<<16 | 0xEE<<8 | 0x02 = 0xFFEE02.
	want := [][]uint64{{3, 0x010201}, {3, 0xFFEE02}}
	reader := log.ReaderOldest()
	for n, words := range want {
		out := make([]uint64, log.Words())
		answer := log.Read(&reader, out)
		if answer.Kind != broadcastlog.ReadItem || answer.Seq != uint64(n) {
			t.Fatalf("read %d: %+v, want the item of sequence number %d", n, answer, n)
		}
		for k := range words {
			if out[k] != words[k] {
				t.Fatalf("entry %d word %d = %#x, want %#x", n, k, out[k], words[k])
			}
		}
	}
}

func TestBroadcastLogAReaderThePublisherLapsIsToldHowManyItMissedAndResumesAtTheOldest(t *testing.T) {
	log := blogliteral.NewBroadcastLogConformanceLiteral()
	publisher := blogNewLiteralPublisher(t, log)
	slow := log.ReaderOldest()

	// Six entries, sequence numbers 0 to 5, into a log that retains three: the
	// window is [6 - 3, 6) = [3, 6), so the reader at 0 missed 3 - 0 = 3.
	for n := uint16(0); n < 6; n++ {
		blogPublishEvent(t, publisher, blogevent.BroadcastLogConformanceEvent{SensorId: 9, Value: n})
	}
	lagged := blogReadEvent(log, &slow)
	if lagged.err != nil || lagged.answer.Kind != broadcastlog.ReadLagged || lagged.answer.Skipped != 3 || lagged.element != nil {
		t.Fatalf("the lapped reader was answered %+v, want lagged by 3 and no element", lagged)
	}
	if slow.Cursor() != 3 || slow.Lost() != 3 {
		t.Fatalf("cursor, lost = %d, %d, want 3, 3: it resumes at the oldest entry held", slow.Cursor(), slow.Lost())
	}

	for n := uint16(3); n < 6; n++ {
		got := blogReadEvent(log, &slow)
		want := blogevent.BroadcastLogConformanceEvent{SensorId: 9, Value: n}
		if got.err != nil || got.answer.Kind != broadcastlog.ReadItem || got.answer.Seq != uint64(n) || got.element == nil || *got.element != want {
			t.Fatalf("after the lag, read of %d: %+v (element %+v), want the item %+v", n, got, got.element, want)
		}
	}
	if got := blogReadEvent(log, &slow); got.answer.Kind != broadcastlog.ReadEmpty {
		t.Fatalf("everything held was read: %+v", got)
	}
	if slow.Lost() != 3 {
		t.Fatalf("reading what is held lost more: %d, want 3", slow.Lost())
	}
}

func TestBroadcastLogTwoReadersKeepSeparateCursors(t *testing.T) {
	log := blogliteral.NewBroadcastLogConformanceLiteral()
	publisher := blogNewLiteralPublisher(t, log)
	blogPublishEvent(t, publisher, blogevent.BroadcastLogConformanceEvent{SensorId: 1, Value: 10})
	blogPublishEvent(t, publisher, blogevent.BroadcastLogConformanceEvent{SensorId: 2, Value: 20})

	// Opened at the oldest entry held (0, as nothing was forgotten), at sequence
	// number 1, and at the head, which is 2.
	first := log.ReaderOldest()
	second := log.ReaderAt(1)
	late := log.ReaderHead()

	one := blogevent.BroadcastLogConformanceEvent{SensorId: 1, Value: 10}
	two := blogevent.BroadcastLogConformanceEvent{SensorId: 2, Value: 20}
	expectItem := func(name string, got blogEventRead, seq uint64, want blogevent.BroadcastLogConformanceEvent) {
		t.Helper()
		if got.err != nil || got.answer.Kind != broadcastlog.ReadItem || got.answer.Seq != seq || got.element == nil || *got.element != want {
			t.Fatalf("%s: %+v (element %+v), want sequence number %d, %+v", name, got, got.element, seq, want)
		}
	}

	expectItem("first reader", blogReadEvent(log, &first), 0, one)
	if first.Cursor() != 1 || second.Cursor() != 1 {
		t.Fatalf("cursors %d and %d: reading with one reader moves no other", first.Cursor(), second.Cursor())
	}
	expectItem("second reader", blogReadEvent(log, &second), 1, two)
	expectItem("first reader again", blogReadEvent(log, &first), 1, two)
	if got := blogReadEvent(log, &first); got.answer.Kind != broadcastlog.ReadEmpty {
		t.Fatalf("the first reader has everything: %+v", got)
	}
	if got := blogReadEvent(log, &second); got.answer.Kind != broadcastlog.ReadEmpty {
		t.Fatalf("the second reader has everything: %+v", got)
	}

	// A reader opened at the head sees only what is published after it.
	if got := blogReadEvent(log, &late); got.answer.Kind != broadcastlog.ReadEmpty {
		t.Fatalf("a reader at the head sees nothing published before it: %+v", got)
	}
	blogPublishEvent(t, publisher, blogevent.BroadcastLogConformanceEvent{SensorId: 3, Value: 30})
	expectItem("late reader", blogReadEvent(log, &late), 2, blogevent.BroadcastLogConformanceEvent{SensorId: 3, Value: 30})
}

// ─── What the element's bound does ───

func TestBroadcastLogAnElementLargerThanItsBoundIsRefusedByTheCodecAndNotTruncated(t *testing.T) {
	log := blogbounded.NewBroadcastLogConformanceBounded()
	publisher, ok := log.Publisher()
	if !ok {
		t.Fatal("a fresh log hands out its publisher")
	}

	// 1 + 20 = 21 bytes and 1 + 9 = 10 bytes into a log whose entries hold at
	// most 9: the nearest above the bound is refused as the far above is.
	for _, size := range []int{20, 9} {
		tooLong := bytes.Repeat([]byte{0xAA}, size)
		_, err := blogbounded.Publish(publisher, &blogrecord.BroadcastLogConformanceRecord{Tag: 1, Payload: tooLong})
		if !errors.Is(err, codec.ErrBufferOverflow) {
			t.Fatalf("a payload of %d bytes past its declared bound: error %v, want codec.ErrBufferOverflow", size, err)
		}
		if log.Head() != 0 {
			t.Fatalf("a refused element publishes nothing, the head is %d", log.Head())
		}
	}

	// Exactly the bound, and under it: the length word keeps what was written.
	full := []byte{1, 2, 3, 4, 5, 6, 7, 8}
	short := []byte{7, 7, 7}
	if seq, err := blogbounded.Publish(publisher, &blogrecord.BroadcastLogConformanceRecord{Tag: 0x5A, Payload: full}); err != nil || seq != 0 {
		t.Fatalf("a payload of exactly the bound: sequence number %d, error %v, want 0, nil", seq, err)
	}
	if seq, err := blogbounded.Publish(publisher, &blogrecord.BroadcastLogConformanceRecord{Tag: 0x5B, Payload: short}); err != nil || seq != 1 {
		t.Fatalf("a payload under the bound: sequence number %d, error %v, want 1, nil", seq, err)
	}

	// The words that were stored. The first record is 0x5A, 1 to 8: 9 bytes, so
	// the length is 9, the first word holds bytes 0 to 7 with byte 0 lowest
	// (0x070605040302015A) and the second holds byte 8, which is 8. The second is
	// 0x5B, 7, 7, 7: 4 bytes, one word of 0x07<<24 | 0x07<<16 | 0x07<<8 | 0x5B =
	// 0x0707075B and a second word that is zero.
	want := [][]uint64{{9, 0x070605040302015A, 8}, {4, 0x0707075B, 0}}
	raw := log.ReaderOldest()
	for n, words := range want {
		out := make([]uint64, log.Words())
		answer := log.Read(&raw, out)
		if answer.Kind != broadcastlog.ReadItem || answer.Seq != uint64(n) {
			t.Fatalf("raw read %d: %+v, want the item of sequence number %d", n, answer, n)
		}
		for k := range words {
			if out[k] != words[k] {
				t.Fatalf("entry %d word %d = %#x, want %#x", n, k, out[k], words[k])
			}
		}
	}

	// The typed read hands the records back whole. Each is decoded from a buffer
	// of its own, so the payload of the first is still what it was after the
	// second has been read.
	reader := log.ReaderOldest()
	answer, firstRecord, err := blogbounded.Read(log, &reader)
	if err != nil || answer.Kind != broadcastlog.ReadItem || firstRecord == nil {
		t.Fatalf("first typed read: %+v, %v, %v", answer, firstRecord, err)
	}
	answer, secondRecord, err := blogbounded.Read(log, &reader)
	if err != nil || answer.Kind != broadcastlog.ReadItem || secondRecord == nil {
		t.Fatalf("second typed read: %+v, %v, %v", answer, secondRecord, err)
	}
	if firstRecord.Tag != 0x5A || !bytes.Equal(firstRecord.Payload, full) {
		t.Fatalf("the first record is %+v, want tag 0x5A and payload %v, still, after the second was read", firstRecord, full)
	}
	if secondRecord.Tag != 0x5B || !bytes.Equal(secondRecord.Payload, short) {
		t.Fatalf("the second record is %+v, want tag 0x5B and payload %v", secondRecord, short)
	}
	if answer, element, err := blogbounded.Read(log, &reader); err != nil || answer.Kind != broadcastlog.ReadEmpty || element != nil {
		t.Fatalf("both records were read: %+v, %v, %v", answer, element, err)
	}
}

func TestBroadcastLogAStoredLengthAboveTheBoundIsAnErrorAndNeverASlice(t *testing.T) {
	// The runtime takes any words, so an entry whose length word is not one this
	// log wrote (a corrupt slot, a log of another size) can be offered to the
	// typed read. A length of MaxEncodedBytes + 1 = 4 is the nearest; the largest
	// uint64 is the one a conversion to int would wrap.
	for _, stored := range []uint64{blogliteral.MaxEncodedBytes + 1, math.MaxUint64} {
		log := blogliteral.NewBroadcastLogConformanceLiteral()
		publisher := blogNewLiteralPublisher(t, log)
		entry := make([]uint64, blogliteral.Words)
		entry[0] = stored
		publisher.Publish(entry)

		reader := log.ReaderOldest()
		got := blogReadEvent(log, &reader)
		if !errors.Is(got.err, codec.ErrLengthOutOfRange) || got.element != nil {
			t.Fatalf("stored length %d: error %v, element %+v, want codec.ErrLengthOutOfRange and none", stored, got.err, got.element)
		}
		if got.answer.Kind != broadcastlog.ReadItem || reader.Cursor() != 1 {
			t.Fatalf("stored length %d: answer %+v, cursor %d, want the item and a reader past it", stored, got.answer, reader.Cursor())
		}
	}
}

func TestBroadcastLogAStoredLengthTooShortForTheElementIsTheCodecsError(t *testing.T) {
	// The element needs 3 bytes, and a length of 2 is within the bound: the
	// decode reports the frame short, no element is returned, and the reader has
	// moved past the entry.
	log := blogliteral.NewBroadcastLogConformanceLiteral()
	publisher := blogNewLiteralPublisher(t, log)
	entry := make([]uint64, blogliteral.Words)
	entry[0] = 2
	entry[1] = 0x0201
	publisher.Publish(entry)

	reader := log.ReaderOldest()
	got := blogReadEvent(log, &reader)
	if !errors.Is(got.err, codec.ErrNeedMoreBytes) || got.element != nil {
		t.Fatalf("error %v, element %+v, want codec.ErrNeedMoreBytes and none", got.err, got.element)
	}
	if reader.Cursor() != 1 {
		t.Fatalf("cursor %d, want 1: the reader moved past the entry", reader.Cursor())
	}
}
