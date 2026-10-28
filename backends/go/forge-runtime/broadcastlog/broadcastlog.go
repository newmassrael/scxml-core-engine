// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Package broadcastlog is the Go row of the bounded broadcast log: one
// publisher, any number of readers, the oldest entry forgotten when the log is
// full (ADR 0062), the runtime half of `sce:kind="broadcast-log"`.
//
// The contract is the sequential log of ADR 0062 section 2. h entries have
// been published, only the last Retention are retained, and a reader owns a
// cursor: Log.Read returns the entry at the cursor, or says the log has
// nothing new, or says how many entries the reader missed and moves it to the
// oldest retained. Publishing never fails and never waits, and neither does a
// read: both are wait-free, in a number of steps that depends on the entry's
// width and on nothing the other side does.
//
// This is a port of the reference row, the Rust file
// backends/rust/forge-runtime/src/broadcast_log.rs, access for access: the same
// stores and loads in the same order, the same checks, the same busy and done
// stamps. Go's sync/atomic is
// sequentially consistent, so each access is at least as strong as the Rust
// one and the two fences of the protocol have nothing left to order; the
// comments say which Rust order every access stands for, and the sequence is
// kept so the two rows read side by side.
//
// The protocol (ADR 0062 section 4). A slot holds a stamp and the entry's
// words. The publisher writes a slot as stamp := busy(n), the words, stamp :=
// done(n), then head := n + 1. A reader holding cursor r loads head and
// answers Empty at the head and Lagged outside the window; otherwise it loads
// the stamp of slot r mod S, copies the words, and loads the stamp again: if
// it is still done(r) the copy is entry r; if not, the publisher has reused the
// slot and the reader reloads head and answers Lagged. This is a sequence
// lock; the words are atomics, so there is no data race for the race detector
// to find.
//
// The slot count is Retention + 1. The slot being overwritten is in flux and
// cannot be promised to a reader: with Retention slots a reader could find the
// entry the specification retains already being overwritten and be told it
// lagged by nothing (docs/formal/BroadcastLog.tla finds that state).
//
// The stamp is a sequence number, not a flag. It carries n, so a reader
// comparing against done(r) cannot mistake a later lap for the entry it wants.
// It is 64 bits and the sequence stays below Horizon.
package broadcastlog

import (
	"math"
	"sync/atomic"
)

// Horizon is the most entries a log can publish: the stamps are 2n + 2, which
// must fit a uint64. Past it a publish would corrupt the stamps, so
// Publisher.Publish stops there with a panic instead of publishing. It is 292
// years at a billion publishes per second.
const Horizon uint64 = (math.MaxUint64 - 2) / 2

// busy is the stamp a slot holds while the entry n is being written into it.
func busy(n uint64) uint64 { return 2*n + 1 }

// done is the stamp a slot holds once the entry n is in it.
func done(n uint64) uint64 { return 2*n + 2 }

// saturatingSub is a - b, or zero when b is above a.
func saturatingSub(a, b uint64) uint64 {
	if b > a {
		return 0
	}
	return a - b
}

// ReadKind says which of the three answers a read gave.
type ReadKind uint8

const (
	// ReadItem: the entry Read.Seq was copied into the caller's buffer, and the
	// reader now holds Seq + 1.
	ReadItem ReadKind = iota + 1
	// ReadEmpty: the reader is at the head, nothing newer is published. Its
	// cursor is unchanged.
	ReadEmpty
	// ReadLagged: the reader had fallen behind the window and missed
	// Read.Skipped >= 1 entries. It now holds the oldest retained entry, so its
	// next read returns that. The caller's buffer is unspecified.
	ReadLagged
)

func (k ReadKind) String() string {
	switch k {
	case ReadItem:
		return "item"
	case ReadEmpty:
		return "empty"
	case ReadLagged:
		return "lagged"
	default:
		return "invalid"
	}
}

// Read is what a read found. Seq is meaningful for ReadItem and Skipped for
// ReadLagged; the other field is zero.
type Read struct {
	Kind    ReadKind
	Seq     uint64
	Skipped uint64
}

// Reader is a reader's position, owned by the reader and shared with nothing.
//
// Opening a reader costs the log nothing: the publisher does not know how many
// readers exist, which is what keeps publishing wait-free. A Reader is a value
// to be kept by the goroutine that reads with it; a copy is a second reader
// that starts where the first stood.
type Reader struct {
	cursor uint64
	lost   uint64
}

// Cursor is the sequence number this reader reads next.
func (r Reader) Cursor() uint64 { return r.cursor }

// Lost is the entries this reader has been told it missed, summed over its
// reads.
func (r Reader) Lost() uint64 { return r.lost }

// cacheLine is the width the head is padded to. It is a layout choice and says
// nothing about correctness.
const cacheLine = 64

// paddedUint64 keeps an atomic counter on a cache line of its own: the
// publisher writes the head on every publish and every reader loads it.
type paddedUint64 struct {
	_ [cacheLine]byte
	v atomic.Uint64
	_ [cacheLine]byte
}

// Log is a bounded log whose entries are Words() 64-bit words, kept in Slots()
// slots and retaining the last Slots() - 1 entries. It must not be copied
// after first use; New returns it by pointer.
type Log struct {
	// head is the number of entries published. Written only by the publisher,
	// loaded by every reader.
	head paddedUint64
	// publisherTaken says whether a Publisher exists.
	publisherTaken atomic.Bool

	words int
	slots uint64
	// stride is the cells a slot takes: its stamp, then its words, so a read
	// touches one run of memory.
	stride int
	cells  []atomic.Uint64
}

// New returns an empty log of entries of words 64-bit words in slots slots,
// retaining slots - 1 entries. It panics when slots is below two (a log retains
// at least one entry, and the slot being overwritten is not one of the
// retained), when words is negative, or when the storage would not be
// addressable.
func New(words, slots int) *Log {
	if slots < 2 {
		panic("broadcastlog: a log retains at least one entry, so it has at least two slots")
	}
	if words < 0 {
		panic("broadcastlog: an entry cannot have a negative number of words")
	}
	if words > math.MaxInt/slots-1 {
		panic("broadcastlog: the log's storage is not addressable")
	}
	stride := words + 1
	return &Log{
		words:  words,
		slots:  uint64(slots),
		stride: stride,
		cells:  make([]atomic.Uint64, slots*stride),
	}
}

// Words is the width of an entry, in 64-bit words.
func (l *Log) Words() int { return l.words }

// Slots is the number of slots, one more than the entries retained.
func (l *Log) Slots() int { return int(l.slots) }

// Retention is the entries a reader may rely on.
func (l *Log) Retention() uint64 { return l.slots - 1 }

// slot is the cells of the slot sequence number n lands in: the stamp first,
// then the words.
func (l *Log) slot(n uint64) []atomic.Uint64 {
	base := int(n%l.slots) * l.stride
	return l.cells[base : base+l.stride]
}

// Head is the number of entries published so far. Rust: an acquire load.
func (l *Log) Head() uint64 {
	return l.head.v.Load()
}

// Publisher returns the publishing side, or false while another exists: two
// publishers would break the stamp protocol silently. Release the handle to
// give the place back; its successor continues the sequence.
func (l *Log) Publisher() (*Publisher, bool) {
	// Rust: a swap with acquire order.
	if l.publisherTaken.Swap(true) {
		return nil, false
	}
	return &Publisher{
		log: l,
		// Rust: an acquire load.
		next: l.head.v.Load(),
	}, true
}

// ReaderOldest is a reader at the oldest retained entry.
func (l *Log) ReaderOldest() Reader {
	return Reader{cursor: saturatingSub(l.Head(), l.Retention())}
}

// ReaderHead is a reader at the head: it sees only what is published after
// this call.
func (l *Log) ReaderHead() Reader {
	return Reader{cursor: l.Head()}
}

// ReaderAt is a reader at the sequence number seq, or at the head when seq is
// past it.
func (l *Log) ReaderAt(seq uint64) Reader {
	return Reader{cursor: min(seq, l.Head())}
}

// Read reads the entry at reader's cursor into out, which must be Words() long.
// A bounded number of steps whatever the publisher is doing: a copy that the
// publisher overwrote is not retried, it is the answer (ReadLagged). out is
// unspecified unless the answer is ReadItem.
func (l *Log) Read(reader *Reader, out []uint64) Read {
	if len(out) != l.words {
		panic("broadcastlog: the buffer is not as long as an entry")
	}
	r := reader.cursor
	// Rust: an acquire load.
	h := l.head.v.Load()
	// A reader's cursor never passes the head of the log it was opened on.
	if r >= h {
		return Read{Kind: ReadEmpty}
	}
	if h-r >= l.slots {
		return l.lagged(reader, h)
	}
	// r was in the window when head was loaded, so its slot was filled before
	// that load. It holds r still unless the publisher has since begun writing
	// r + Slots().
	cells := l.slot(r)
	// Rust: an acquire load.
	first := cells[0].Load()
	if first == done(r) {
		for k := range out {
			// Rust: relaxed loads.
			out[k] = cells[1+k].Load()
		}
		// Rust: an acquire fence keeps the copy before the second stamp load,
		// which is what makes an unchanged stamp mean an unchanged copy.
		// Nothing is left to order here.
		//
		// The second load is an acquire for the other reason: if it sees the
		// stamp of a later lap, the reload of head below must see the publish
		// that began that lap (found by the Rust row's loom model
		// two_readers_each_hold_their_own_cursor). Rust: an acquire load.
		if cells[0].Load() == first {
			reader.cursor = r + 1
			return Read{Kind: ReadItem, Seq: r}
		}
	}
	// The slot was reused while it was being read, so the publisher has
	// published through r + Retention and head shows it. Rust: an acquire load.
	h = l.head.v.Load()
	return l.lagged(reader, h)
}

// lagged moves reader to the oldest entry retained when the head is h, and
// says how far that was. skipped is at least one on every path that reaches it:
// the range test above gives h - cursor >= Slots(), and the reload after a
// failed validation sees a head past r + Retention.
func (l *Log) lagged(reader *Reader, h uint64) Read {
	oldest := saturatingSub(h, l.Retention())
	skipped := saturatingSub(oldest, reader.cursor)
	reader.cursor = oldest
	reader.lost = saturatingAdd(reader.lost, skipped)
	return Read{Kind: ReadLagged, Skipped: skipped}
}

// saturatingAdd is a + b, or the largest uint64 when the sum does not fit.
func saturatingAdd(a, b uint64) uint64 {
	if a > math.MaxUint64-b {
		return math.MaxUint64
	}
	return a + b
}

// Publisher is the publishing side of a Log. There is one at a time, and it is
// for one goroutine at a time.
type Publisher struct {
	log *Log
	// next is the sequence number of the next entry: this side's own count, so
	// publishing reads no shared state.
	next uint64
}

// Publish publishes entry, which must be Words() long, and returns the
// sequence number it took. Never fails and never waits.
//
// It panics when the handle was released, when entry is not as long as an
// entry, and past Horizon entries, which is not reached in practice.
func (p *Publisher) Publish(entry []uint64) uint64 {
	l := p.log
	if l == nil {
		panic("broadcastlog: the publisher was released")
	}
	if len(entry) != l.words {
		panic("broadcastlog: the entry is not as long as the log's entries")
	}
	n := p.next
	if n >= Horizon {
		panic("broadcastlog: the log has published its horizon of entries")
	}
	cells := l.slot(n)
	// Rust: a release store, for the reader that sees this stamp and then
	// reloads head: it must find head at n or later, and the store of head to n
	// ended the previous publish, in this goroutine's order before this one.
	cells[0].Store(busy(n))
	// Rust: a release fence, so that the stamp above comes before the words.
	// Sequentially consistent stores are already in program order.
	for k, word := range entry {
		// Rust: relaxed stores.
		cells[1+k].Store(word)
	}
	// Rust: a release store.
	cells[0].Store(done(n))
	// The linearization point. Rust: a release store, so that a reader that
	// loads this index sees the slot's stamp and words.
	l.head.v.Store(n + 1)
	p.next = n + 1
	return n
}

// Release gives the publisher place back. The handle is not usable afterwards;
// releasing it again does nothing.
func (p *Publisher) Release() {
	if p.log != nil {
		// Rust: a release store.
		p.log.publisherTaken.Store(false)
		p.log = nil
	}
}
