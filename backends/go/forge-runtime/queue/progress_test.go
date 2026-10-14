// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package queue

import (
	"sync/atomic"
	"testing"
	"time"
)

// A stress test of a queue says how long it waits for the elements to arrive, and
// the question that wait answers is whether the queue is STUCK, not how long a run
// took. A run's length is the speed of the machine it runs on: under the race
// detector the same run takes a tenth of a second or twenty, and on a machine busy
// with other work longer still, with every element still arriving. A fixed
// wall-clock deadline fails such a run, and says "a lost element" of one that was
// only slow.
//
// What separates the two is progress. A queue that has lost an element or lost its
// way delivers nothing more however long it is given, and one that is slow keeps
// delivering. So the limit is how long the run may go with NO operation completing,
// and any operation completing starts it again.

// stuckAfter is how long the queue may complete nothing before a test calls it
// stuck: long enough that no slow machine goes that long without one push or pop
// finishing, and short enough that a lost element fails the test in a minute.
const stuckAfter = 60 * time.Second

// progress counts the operations a test's goroutines have completed.
type progress struct{ done atomic.Uint64 }

// beat records that an operation completed.
func (p *progress) beat() { p.done.Add(1) }

// lookout is what one goroutine remembers of the progress it last saw. It is not
// shared: each goroutine asks for itself, on the paths where it is already
// waiting.
type lookout struct {
	seen  uint64
	since time.Time
	// limit replaces stuckAfter when it is set, so the watchdog itself can be
	// tried in milliseconds.
	limit time.Duration
}

// stuck reports whether nothing has completed for stuckAfter, as this goroutine
// has seen it.
func (l *lookout) stuck(p *progress) bool {
	now := p.done.Load()
	if l.since.IsZero() || now != l.seen {
		l.seen, l.since = now, time.Now()
		return false
	}
	limit := stuckAfter
	if l.limit > 0 {
		limit = l.limit
	}
	return time.Since(l.since) > limit
}

// A run that takes far longer than the limit and keeps completing operations is
// never stuck, and one that stops completing them is, after the limit and not
// before.
func TestTheWatchdogCallsAQueueStuckOnlyWhenNothingCompletes(t *testing.T) {
	var watch progress
	look := lookout{limit: 50 * time.Millisecond}
	if look.stuck(&watch) {
		t.Fatal("a run that has only just begun is not stuck")
	}

	// Slow but moving: ten times the limit in all, an operation in every quarter of it.
	for step := 0; step < 40; step++ {
		time.Sleep(12 * time.Millisecond)
		watch.beat()
		if look.stuck(&watch) {
			t.Fatalf("step %d: a run that keeps completing operations is not stuck", step)
		}
	}

	// Nothing completes any more.
	time.Sleep(10 * time.Millisecond)
	if look.stuck(&watch) {
		t.Fatal("a pause shorter than the limit is not stuck")
	}
	time.Sleep(80 * time.Millisecond)
	if !look.stuck(&watch) {
		t.Fatal("a run in which nothing completes for longer than the limit is stuck")
	}

	// And one operation ends it.
	watch.beat()
	if look.stuck(&watch) {
		t.Fatal("an operation completing starts the wait again")
	}
}
