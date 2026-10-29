// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package broadcastlog

// The Go arm's contract tests of the broadcast log (RFC §synth-5-S; ADR 0062 section 2): the
// unit tests of the Rust row (backends/rust/forge-runtime/src/broadcast_log.rs)
// ported, then a stress with one publisher goroutine and several reader
// goroutines whose every answer is held against the contract. The stress is
// written for `go test -race`, which is how the gate runs it.

import (
	"math"
	"runtime"
	"sync"
	"testing"
)

const (
	// testWords and testSlots are the Rust tests' shape: two words an entry,
	// three slots, so a retention of two.
	testWords = 2
	testSlots = 3
)

func newTestLog() *Log { return New(testWords, testSlots) }

func entry(n uint64) []uint64 { return []uint64{n, n} }

// readOnce reads with a buffer that holds a value no test publishes, so an
// answer that did not copy shows.
func readOnce(log *Log, reader *Reader) (Read, []uint64) {
	out := []uint64{math.MaxUint64, math.MaxUint64}
	return log.Read(reader, out), out
}

func wantItem(t *testing.T, log *Log, reader *Reader, seq uint64, words []uint64) {
	t.Helper()
	got, out := readOnce(log, reader)
	if got != (Read{Kind: ReadItem, Seq: seq}) {
		t.Fatalf("read answered %+v, want the item %d", got, seq)
	}
	for k, want := range words {
		if out[k] != want {
			t.Fatalf("item %d: word %d is %d, want %d", seq, k, out[k], want)
		}
	}
}

func wantKind(t *testing.T, log *Log, reader *Reader, want Read) {
	t.Helper()
	got, _ := readOnce(log, reader)
	if got != want {
		t.Fatalf("read answered %+v, want %+v", got, want)
	}
}

func mustPublisher(t *testing.T, log *Log) *Publisher {
	t.Helper()
	publisher, ok := log.Publisher()
	if !ok {
		t.Fatal("the publisher place is free")
	}
	return publisher
}

// panics reports whether f panicked.
func panics(f func()) (did bool) {
	defer func() { did = recover() != nil }()
	f()
	return false
}

func TestAnEmptyLogIsEmptyToEveryReader(t *testing.T) {
	log := newTestLog()
	reader := log.ReaderOldest()
	wantKind(t, log, &reader, Read{Kind: ReadEmpty})
	if reader.Cursor() != 0 {
		t.Fatalf("an empty read moved the cursor to %d", reader.Cursor())
	}
	if log.Head() != 0 {
		t.Fatalf("head of an empty log is %d", log.Head())
	}
}

func TestEntriesComeBackInOrderWithTheirWords(t *testing.T) {
	log := newTestLog()
	publisher := mustPublisher(t, log)
	reader := log.ReaderOldest()
	if seq := publisher.Publish(entry(10)); seq != 0 {
		t.Fatalf("first publish took %d", seq)
	}
	if seq := publisher.Publish(entry(11)); seq != 1 {
		t.Fatalf("second publish took %d", seq)
	}
	wantItem(t, log, &reader, 0, entry(10))
	wantItem(t, log, &reader, 1, entry(11))
	wantKind(t, log, &reader, Read{Kind: ReadEmpty})
	if reader.Lost() != 0 {
		t.Fatalf("a reader that missed nothing lost %d", reader.Lost())
	}
}

func TestTheWindowIsExactlyTheRetention(t *testing.T) {
	// Three slots retain two entries.
	log := newTestLog()
	if log.Retention() != 2 || log.Slots() != 3 || log.Words() != 2 {
		t.Fatalf("shape is retention %d, slots %d, words %d", log.Retention(), log.Slots(), log.Words())
	}
	publisher := mustPublisher(t, log)
	for n := uint64(0); n < 3; n++ {
		publisher.Publish(entry(n))
	}
	// h = 3: the window is [1, 3), so a reader at 0 is one behind it.
	behind := log.ReaderAt(0)
	wantKind(t, log, &behind, Read{Kind: ReadLagged, Skipped: 1})
	if behind.Cursor() != 1 || behind.Lost() != 1 {
		t.Fatalf("a lagged reader holds %d having lost %d, want 1 and 1", behind.Cursor(), behind.Lost())
	}
	wantItem(t, log, &behind, 1, entry(1))
	// A reader at the oldest retained entry lost nothing.
	oldest := log.ReaderOldest()
	if oldest.Cursor() != 1 {
		t.Fatalf("the oldest retained entry is 1, the reader holds %d", oldest.Cursor())
	}
	wantItem(t, log, &oldest, 1, entry(1))
}

func TestReadersDoNotShareACursorOrALoss(t *testing.T) {
	log := newTestLog()
	publisher := mustPublisher(t, log)
	for n := uint64(0); n < 5; n++ {
		publisher.Publish(entry(n))
	}
	a := log.ReaderAt(0)
	b := log.ReaderAt(3)
	wantKind(t, log, &a, Read{Kind: ReadLagged, Skipped: 3})
	wantItem(t, log, &b, 3, entry(3))
	if a.Lost() != 3 || b.Lost() != 0 {
		t.Fatalf("losses are %d and %d, want 3 and 0", a.Lost(), b.Lost())
	}
	if b.Cursor() != 4 {
		t.Fatalf("b holds %d, want 4", b.Cursor())
	}
}

func TestAReaderAtTheHeadSeesOnlyWhatComesAfter(t *testing.T) {
	log := newTestLog()
	publisher := mustPublisher(t, log)
	publisher.Publish(entry(0))
	reader := log.ReaderHead()
	wantKind(t, log, &reader, Read{Kind: ReadEmpty})
	publisher.Publish(entry(1))
	wantItem(t, log, &reader, 1, entry(1))
	// A position past the head is clamped to it.
	if cursor := log.ReaderAt(99).Cursor(); cursor != 2 {
		t.Fatalf("a reader asked for 99 holds %d, want the head 2", cursor)
	}
}

func TestThereIsOnePublisherAndItsSuccessorContinuesTheSequence(t *testing.T) {
	log := newTestLog()
	first := mustPublisher(t, log)
	if second, ok := log.Publisher(); ok || second != nil {
		t.Fatal("a second publisher is refused")
	}
	first.Publish(entry(0))
	first.Release()
	second := mustPublisher(t, log)
	if seq := second.Publish(entry(1)); seq != 1 {
		t.Fatalf("the successor took %d, want 1", seq)
	}
}

func TestPublishingPastTheWindowKeepsTheNewestEntries(t *testing.T) {
	log := newTestLog()
	publisher := mustPublisher(t, log)
	for n := uint64(0); n < 100; n++ {
		publisher.Publish(entry(n))
	}
	reader := log.ReaderOldest()
	wantItem(t, log, &reader, 98, entry(98))
	wantItem(t, log, &reader, 99, entry(99))
	wantKind(t, log, &reader, Read{Kind: ReadEmpty})
}

func TestAReaderThatStoppedIsToldHowManyItMissedAndKeepsTheSum(t *testing.T) {
	log := newTestLog()
	publisher := mustPublisher(t, log)
	reader := log.ReaderHead()
	for n := uint64(0); n < 10; n++ {
		publisher.Publish(entry(n))
	}
	// h = 10, the window is [8, 10), the reader holds 0.
	wantKind(t, log, &reader, Read{Kind: ReadLagged, Skipped: 8})
	wantItem(t, log, &reader, 8, entry(8))
	for n := uint64(10); n < 20; n++ {
		publisher.Publish(entry(n))
	}
	// h = 20, the window is [18, 20), the reader holds 9: nine are gone.
	wantKind(t, log, &reader, Read{Kind: ReadLagged, Skipped: 9})
	if reader.Lost() != 17 || reader.Cursor() != 18 {
		t.Fatalf("the reader lost %d and holds %d, want 17 and 18", reader.Lost(), reader.Cursor())
	}
}

func TestACopyOfAReaderIsASecondReaderThatStartsWhereTheFirstStood(t *testing.T) {
	log := newTestLog()
	publisher := mustPublisher(t, log)
	publisher.Publish(entry(0))
	publisher.Publish(entry(1))
	first := log.ReaderOldest()
	second := first
	wantItem(t, log, &first, 0, entry(0))
	if second.Cursor() != 0 {
		t.Fatalf("reading with the first moved its copy to %d", second.Cursor())
	}
	wantItem(t, log, &second, 0, entry(0))
}

func TestARefusedConstructionOrCallPanics(t *testing.T) {
	for _, slots := range []int{-1, 0, 1} {
		if !panics(func() { New(2, slots) }) {
			t.Errorf("a log of %d slots was built", slots)
		}
	}
	if !panics(func() { New(-1, 3) }) {
		t.Error("a log of entries of negative width was built")
	}
	if !panics(func() { New(math.MaxInt, 2) }) {
		t.Error("a log whose storage cannot be addressed was built")
	}
	log := newTestLog()
	publisher := mustPublisher(t, log)
	if !panics(func() { publisher.Publish([]uint64{1}) }) {
		t.Error("an entry of the wrong width was published")
	}
	reader := log.ReaderOldest()
	if !panics(func() { log.Read(&reader, make([]uint64, 3)) }) {
		t.Error("a read into a buffer of the wrong width was answered")
	}
	publisher.Release()
	publisher.Release()
	if !panics(func() { publisher.Publish(entry(0)) }) {
		t.Error("a released publisher published")
	}
	if log.Head() != 0 {
		t.Errorf("a refused call moved the head to %d", log.Head())
	}
}

func TestAnEntryOfNoWordsIsStillAnEntry(t *testing.T) {
	log := New(0, 2)
	publisher := mustPublisher(t, log)
	reader := log.ReaderOldest()
	publisher.Publish(nil)
	publisher.Publish(nil)
	publisher.Publish(nil)
	if got := log.Read(&reader, nil); got != (Read{Kind: ReadLagged, Skipped: 2}) {
		t.Fatalf("a reader two behind answered %+v", got)
	}
	if got := log.Read(&reader, nil); got != (Read{Kind: ReadItem, Seq: 2}) {
		t.Fatalf("the next read answered %+v", got)
	}
}

func TestTheStampsAreTheSequenceNumberAndFitUntilTheHorizon(t *testing.T) {
	if busy(0) != 1 || done(0) != 2 || busy(7) != 15 || done(7) != 16 {
		t.Fatalf("stamps of 0 and 7 are %d %d and %d %d", busy(0), done(0), busy(7), done(7))
	}
	last := Horizon - 1
	if done(last) != 2*Horizon || done(last) >= math.MaxUint64-1 || busy(last) >= done(last) {
		t.Fatalf("the stamps of the last entry, %d and %d, do not fit", busy(last), done(last))
	}
}

func TestPublishingPastTheHorizonPanicsAndTheLastEntryBeforeItWorks(t *testing.T) {
	log := newTestLog()
	publisher := mustPublisher(t, log)
	publisher.next = Horizon
	if !panics(func() { publisher.Publish(entry(0)) }) {
		t.Fatal("an entry past the horizon was published")
	}
	if log.Head() != 0 {
		t.Fatalf("a refused publish moved the head to %d", log.Head())
	}
	publisher.next = Horizon - 1
	if seq := publisher.Publish(entry(5)); seq != Horizon-1 {
		t.Fatalf("the last entry took %d, want %d", seq, Horizon-1)
	}
	if log.Head() != Horizon {
		t.Fatalf("head is %d after the last entry, want %d", log.Head(), Horizon)
	}
}

// stressShape is a log shape the stress runs: its words an entry and its slots.
type stressShape struct{ words, slots int }

const (
	stressPublishes = 5000
	stressReaders   = 4
)

// A publisher goroutine publishes entries whose every word is the sequence
// number it will take, while several reader goroutines read every one of them.
// Whatever interleaving the machine gives, each answer must hold:
//
//   - an Item is a whole entry: every word equals its sequence number, so a
//     copy that mixed two laps is caught;
//   - an Item is the entry at the reader's cursor, and the sequence numbers a
//     reader sees strictly increase;
//   - a Lagged skips at least one and moves the cursor by exactly what it says;
//   - an Empty leaves the cursor alone;
//   - head never goes back;
//   - nothing is unaccounted for: when the reader holds the head, the entries it
//     read plus the entries it was told it lost are all of them.
//
// Readers open before the publisher starts, so each reader's total is the
// whole run and the last check is exact. The small retentions make the readers
// lag often; the wide one lets them read in long runs.
func TestEveryAnswerOfAConcurrentRunIsAWholeEntryInOrder(t *testing.T) {
	for _, shape := range []stressShape{{1, 2}, {2, 3}, {4, 5}, {3, 17}} {
		runStress(t, shape)
	}
}

func runStress(t *testing.T, shape stressShape) {
	t.Helper()
	log := New(shape.words, shape.slots)
	readers := make([]Reader, stressReaders)
	for i := range readers {
		readers[i] = log.ReaderOldest()
	}
	start := make(chan struct{})
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
		for n := uint64(0); n < stressPublishes; n++ {
			for k := range words {
				words[k] = n
			}
			if seq := publisher.Publish(words); seq != n {
				t.Errorf("shape %+v: publish %d took the sequence number %d", shape, n, seq)
				return
			}
			if n%16 == 15 {
				runtime.Gosched()
			}
		}
	}()

	for i := range readers {
		wg.Add(1)
		go func(who int) {
			defer wg.Done()
			reader := &readers[who]
			out := make([]uint64, shape.words)
			<-start
			var items, lost, lastHead uint64
			next := uint64(0)
			seen := false
			for reader.Cursor() < stressPublishes {
				before := reader.Cursor()
				result := log.Read(reader, out)
				switch result.Kind {
				case ReadItem:
					if result.Seq != before || reader.Cursor() != before+1 {
						t.Errorf("shape %+v reader %d: item %d at cursor %d left it at %d",
							shape, who, result.Seq, before, reader.Cursor())
						return
					}
					for k, word := range out {
						if word != result.Seq {
							t.Errorf("shape %+v reader %d: item %d has word %d equal to %d, a torn entry",
								shape, who, result.Seq, k, word)
							return
						}
					}
					if seen && result.Seq < next {
						t.Errorf("shape %+v reader %d: item %d after %d, not increasing",
							shape, who, result.Seq, next-1)
						return
					}
					seen, next = true, result.Seq+1
					items++
				case ReadEmpty:
					if reader.Cursor() != before {
						t.Errorf("shape %+v reader %d: empty moved the cursor", shape, who)
						return
					}
					runtime.Gosched()
				case ReadLagged:
					if result.Skipped < 1 || reader.Cursor() != before+result.Skipped {
						t.Errorf("shape %+v reader %d: lagged %d at cursor %d left it at %d",
							shape, who, result.Skipped, before, reader.Cursor())
						return
					}
					lost += result.Skipped
				default:
					t.Errorf("shape %+v reader %d: answer %+v", shape, who, result)
					return
				}
				head := log.Head()
				if head < lastHead || head > stressPublishes || reader.Cursor() > head {
					t.Errorf("shape %+v reader %d: head %d after %d, cursor %d", shape, who, head, lastHead, reader.Cursor())
					return
				}
				lastHead = head
			}
			if items+lost != stressPublishes || reader.Lost() != lost {
				t.Errorf("shape %+v reader %d: read %d, lost %d (reader says %d), of %d",
					shape, who, items, lost, reader.Lost(), stressPublishes)
			}
		}(i)
	}

	close(start)
	wg.Wait()
	if log.Head() != stressPublishes {
		t.Errorf("shape %+v: head is %d after the run, want %d", shape, log.Head(), stressPublishes)
	}
}
