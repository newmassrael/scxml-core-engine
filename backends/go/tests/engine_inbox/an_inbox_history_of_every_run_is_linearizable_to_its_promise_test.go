// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package engine_inbox

// docs/adr/0147, D3: the door of docs/adr/0084 is a concurrent object, and every
// run of it a history. These cases run the Go engine's inbox on real goroutines,
// record every post each poster made and every event the owner put on the
// machine's external queue, with the end of the machine's run, and hold the record
// linearizable to the promise docs/formal/InboxService.tla states. The Go sibling
// of the Rust lane's an_inbox_history_of_every_run_is_linearizable_to_its_promise.rs,
// the C11 machine's test_a_run_of_the_inbox_is_written_as_its_history.c and the
// generated C++ engine's inbox_history_recorder.cpp.
//
// The histories are judged by `sce-codegen check-inbox-history`, the command every
// runtime's inbox histories are judged by, so this lane carries no checker of its
// own: each case writes its runs, in the form every runtime writes
// (tests/forge/conformance/inbox_history.schema.json), into a directory of its own
// and hands the files to the binary SCE_CODEGEN names. Without the variable nothing
// can judge them, and the cases are skipped by name; the Go gate
// (scripts/gates/w3c-go.sh) names it and holds that they passed.
//
// What enters the external queue is told by the engine's inbox port
// (Engine.SetInboxPort): the engine tells it when it begins to put each event
// there, ahead of the drain before it, and once the event entered, saying whether
// its inbox held the event. The drain, the order it keeps and the inbox are the
// engine's; these cases read the clock around them.
//
// A run, on the probe machine the cases beside this file drive: a first poster
// posts `ping`s, and an ending of `job` through the tree's directory by the
// machine's own session id, while the owner stands between two drives. The owner
// then raises a `ping` of its own, whose drain applies them all first; while the
// first of them enters, a post is made that the drain did not read, which may
// follow the event. Then several goroutines post typed and named `ping`s, and now
// and then an ending naming a session the tree does not hold, while the owner
// drives the machine, raises `ping`s of its own and stops it once half the posts
// are made. Last, once the posters are done, a `ping` through the machine's handle
// and an ending naming the machine through the directory. Every input and every
// own event carries its value as its data, which no other of the run has.
//
// The engine closes the inbox when its run ends (Engine.Stop): what it holds then
// is applied, a later post through its handle is refused as ended, and the machine
// leaves the tree's directory, so a later post naming it there is refused as
// unknown. Every history says `closes: true`. The inbox has no bound, so no
// history gives a capacity, and no post is refused as full.
//
// The end of the run is recorded around Stop, from before the call to the instant
// the inbox is known to have closed: the first event the port is told inside the
// call is the first input of the drain that follows the close, so the end returns
// at the reading taken there, or after the call when the port is told none. That
// drain's enqueues follow the end, as an input taken before the end enters after
// it.

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strconv"
	"sync"
	"sync/atomic"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
)

const (
	// historyPosters goroutines post beside the owner, historyPerPoster posts
	// each in one run.
	historyPosters   = 4
	historyPerPoster = 48
	// historyFillerPings is how many `ping`s the first poster posts before the
	// owner's first own event.
	historyFillerPings = 8
	// historyRuns runs are recorded: the schedules that matter are a small
	// fraction of those a machine gives.
	historyRuns = 8
	// historyOwnPerRun own events the owner raises at most in one run, the first
	// of them historyOwnBase, past every input a poster posts.
	historyOwnPerRun        = 24
	historyOwnBase   uint64 = 1_000_000
	// historySession is the number the history names the machine's session by,
	// and historyAbsent the session the tree does not hold, whose id is
	// historyAbsentID.
	historySession  uint64 = 0
	historyAbsent   uint64 = 1
	historyAbsentID        = "inbox-history-absent"
	// The inputs of the posts no poster of the run's middle makes: the one the
	// drain before the first own event did not read, and the two made once the
	// run ended.
	historyBehindInput        uint64 = 900_000
	historyLateHandleInput    uint64 = 900_001
	historyLateAddressedInput uint64 = 900_002

	// The doors a post goes through (docs/adr/0147, D1), as the history spells
	// them.
	doorAddressed = "addressed"
	doorHandle    = "handle"
)

// historyPost is one post a poster observed, in the JSON form.
type historyPost struct {
	Call     string `json:"call"`
	Session  uint64 `json:"session"`
	Input    uint64 `json:"input"`
	Door     string `json:"door"`
	Outcome  string `json:"outcome"`
	Reason   string `json:"reason,omitempty"`
	Invoked  uint64 `json:"invoked"`
	Returned uint64 `json:"returned"`
}

// historyOwnerOperation is one operation the owner observed, in the JSON form:
// an enqueue names its item and its source, an end neither.
type historyOwnerOperation struct {
	Call     string  `json:"call"`
	Session  uint64  `json:"session"`
	Item     *uint64 `json:"item,omitempty"`
	Source   string  `json:"source,omitempty"`
	Invoked  uint64  `json:"invoked"`
	Returned uint64  `json:"returned"`
}

// inboxHistory is one run, in the JSON form.
type inboxHistory struct {
	Version  int                     `json:"version"`
	Sessions []uint64                `json:"sessions"`
	Closes   bool                    `json:"closes"`
	Owner    []historyOwnerOperation `json:"owner"`
	Posters  [][]historyPost         `json:"posters"`
}

// historyClock is the one clock every goroutine of a run reads, immediately
// before and immediately after each operation it records: one read-modify-write
// chain on one atomic orders the readings with the calls between them, so
// "returned before invoked" in the record means it in the run, and no two
// readings are equal.
type historyClock struct {
	readings atomic.Uint64
}

func (c *historyClock) tick() uint64 {
	return c.readings.Add(1)
}

// recordInboxPost posts with post, which answers whether the inbox took the
// input, recorded as a post of input value to session through door with the
// clock read immediately around it. A post through the machine's handle is
// refused only once its inbox closed, and one through the directory only for
// the session it names.
func recordInboxPost(clock *historyClock, session, value uint64, door string, post func() bool) historyPost {
	record := historyPost{Call: "post", Session: session, Input: value, Door: door}
	record.Invoked = clock.tick()
	taken := post()
	record.Returned = clock.tick()
	switch {
	case taken:
		record.Outcome = "taken"
	case door == doorHandle:
		record.Outcome, record.Reason = "refused", "ended"
	default:
		record.Outcome, record.Reason = "refused", "unknown"
	}
	return record
}

// historyRecorder is what the owner records: the engine's inbox port, and the
// end of the run. Written on the owner only.
type historyRecorder struct {
	clock *historyClock
	owner []historyOwnerOperation
	// begun is the reading taken when the engine began to put the event it is
	// telling on the queue: for an event of the engine's own at [0], for an
	// input from the inbox at [1]. Two, because the inputs the drain before an
	// event of the engine's own applies are told between that event's two
	// calls.
	begun [2]uint64
	// ownOpen is set while an event of the engine's own is being put on the
	// queue: the port's first call of it came and its second has not.
	ownOpen bool
	// ending is set while the owner's call that ends the run is under way,
	// which began at endInvoked.
	ending     bool
	endInvoked uint64
	// behind runs once the next input the engine applies from its inbox has
	// entered: inside the drain that applies it, after that drain read the
	// inbox.
	behind  func()
	failure string
}

// told is the inbox port.
func (r *historyRecorder) told(enqueue sce.InboxEnqueue[probeEvent]) {
	at := 0
	if enqueue.Posted {
		at = 1
	}
	if !enqueue.Entered {
		reading := r.clock.tick()
		if r.ending {
			// Inside Stop: the inbox closed before the drain that applies what
			// it held began, so the end returns here and the drain follows it.
			if !enqueue.Posted {
				r.fail("an event of the engine's own was put on its queue while its run ended, before its " +
					"inbox closed, where the end of the run cannot be read")
			}
			r.closeEnd(reading)
			reading = r.clock.tick()
		}
		r.begun[at] = reading
		if !enqueue.Posted {
			r.ownOpen = true
		}
		return
	}
	value, err := strconv.ParseUint(enqueue.Event.Metadata.Data, 10, 64)
	if err != nil {
		r.fail(fmt.Sprintf("an event entered the external queue with no value as its data: %q", enqueue.Event.Metadata.Data))
		return
	}
	source := "own"
	if enqueue.Posted {
		source = "posted"
	}
	// docs/adr/0147, D3: an input the drain before an event of the engine's own
	// applies left the inbox at that drain's read, which comes after the
	// event's first call, so its enqueue is invoked at the event's reading.
	// Read at its own first call, a post that returned between the drain's read
	// and that call would be held to enter ahead of the event, which the drain
	// did not read.
	invoked := r.begun[0]
	if enqueue.Posted && !r.ownOpen {
		invoked = r.begun[1]
	}
	r.owner = append(r.owner, historyOwnerOperation{
		Call: "enqueue", Session: historySession, Item: &value, Source: source,
		Invoked: invoked, Returned: r.clock.tick(),
	})
	if !enqueue.Posted {
		r.ownOpen = false
	}
	if enqueue.Posted && r.behind != nil {
		post := r.behind
		r.behind = nil
		post()
	}
}

// end calls stop, the owner's call that ends the run, recorded as the end.
func (r *historyRecorder) end(stop func()) {
	r.ending = true
	r.endInvoked = r.clock.tick()
	stop()
	if r.ending {
		r.closeEnd(r.clock.tick())
	}
}

func (r *historyRecorder) closeEnd(returned uint64) {
	r.ending = false
	r.owner = append(r.owner, historyOwnerOperation{
		Call: "end", Session: historySession, Invoked: r.endInvoked, Returned: returned,
	})
}

func (r *historyRecorder) fail(why string) {
	if r.failure == "" {
		r.failure = why
	}
}

// postAside runs post on a goroutine of its own, as a poster does, and answers
// what it recorded once it returned.
func postAside(post func() historyPost) historyPost {
	recorded := make(chan historyPost, 1)
	go func() { recorded <- post() }()
	return <-recorded
}

// recordInboxRun is one run of the probe's inbox, as the file's comment says.
func recordInboxRun(t *testing.T) inboxHistory {
	t.Helper()
	pr := newProbe()
	if err := pr.engine.Initialize(); err != nil {
		t.Fatal(err)
	}
	token := pr.latestToken(t)
	sessionID := pr.policy.sessionID
	clock := &historyClock{}
	recorder := &historyRecorder{clock: clock}
	pr.engine.SetInboxPort(recorder.told)
	inbox := pr.engine.Inbox()
	valueText := func(value uint64) string { return strconv.FormatUint(value, 10) }

	// The first poster, while the owner stands between two drives: `ping`s
	// through the machine's handle, and the ending of `job` through the
	// directory.
	filler := make(chan []historyPost, 1)
	go func() {
		var posts []historyPost
		for k := 0; k < historyFillerPings; k++ {
			value := uint64(k) + 1
			posts = append(posts, recordInboxPost(clock, historySession, value, doorHandle, func() bool {
				return inbox.RaiseExternal(probePing, valueText(value))
			}))
		}
		value := uint64(historyFillerPings) + 1
		posts = append(posts, recordInboxPost(clock, historySession, value, doorAddressed, func() bool {
			return inbox.EndHostInvoke(sessionID, ending(token, true, valueText(value)))
		}))
		filler <- posts
	}()
	first := <-filler

	// An event of the owner's own behind what the first poster's posts left in
	// the inbox, so every run enters one; and a post made while the first input
	// its drain applies enters, which that drain did not read.
	var behind []historyPost
	recorder.behind = func() {
		behind = append(behind, postAside(func() historyPost {
			return recordInboxPost(clock, historySession, historyBehindInput, doorHandle, func() bool {
				return inbox.RaiseExternal(probePing, valueText(historyBehindInput))
			})
		}))
	}
	own := historyOwnBase
	pr.engine.RaiseExternal(probePing, valueText(own))
	own++

	// The posters, and the owner: a drive, an own event now and then, and the
	// stop once half the posts are made.
	base := uint64(historyFillerPings) + 2
	logs := make([][]historyPost, historyPosters)
	var (
		running  atomic.Int32
		made     atomic.Int32
		finished sync.WaitGroup
	)
	running.Store(historyPosters)
	release := make(chan struct{})
	for poster := 0; poster < historyPosters; poster++ {
		finished.Add(1)
		go func() {
			defer finished.Done()
			<-release
			for k := 0; k < historyPerPoster; k++ {
				value := base + uint64(poster*historyPerPoster+k)
				var record historyPost
				switch k % 6 {
				case 5:
					record = recordInboxPost(clock, historyAbsent, value, doorAddressed, func() bool {
						return inbox.EndHostInvoke(historyAbsentID, ending(0, true, valueText(value)))
					})
				case 1, 3:
					record = recordInboxPost(clock, historySession, value, doorHandle, func() bool {
						return inbox.RaiseExternalByName("ping", valueText(value))
					})
				default:
					record = recordInboxPost(clock, historySession, value, doorHandle, func() bool {
						return inbox.RaiseExternal(probePing, valueText(value))
					})
				}
				logs[poster] = append(logs[poster], record)
				made.Add(1)
				runtime.Gosched()
			}
			running.Add(-1)
		}()
	}
	close(release)

	ended := false
	for round := 1; running.Load() > 0; round++ {
		pr.engine.Tick()
		if !ended && round%3 == 0 && own < historyOwnBase+historyOwnPerRun {
			pr.engine.RaiseExternal(probePing, valueText(own))
			own++
		}
		if !ended && int(made.Load())*2 >= historyPosters*historyPerPoster {
			recorder.end(pr.engine.Stop)
			ended = true
		}
		runtime.Gosched()
	}
	finished.Wait()
	if !ended {
		recorder.end(pr.engine.Stop)
	}

	// Once the run ended, and the machine left the tree's directory: a post
	// through its handle, refused as ended, and one naming it through the
	// directory, refused as unknown.
	late := []historyPost{
		postAside(func() historyPost {
			return recordInboxPost(clock, historySession, historyLateHandleInput, doorHandle, func() bool {
				return inbox.RaiseExternal(probePing, valueText(historyLateHandleInput))
			})
		}),
		postAside(func() historyPost {
			return recordInboxPost(clock, historySession, historyLateAddressedInput, doorAddressed, func() bool {
				return inbox.EndHostInvoke(sessionID, ending(token, true, valueText(historyLateAddressedInput)))
			})
		}),
	}
	pr.engine.SetInboxPort(nil)

	if recorder.failure != "" {
		t.Fatal(recorder.failure)
	}
	posters := [][]historyPost{first, behind}
	posters = append(posters, logs...)
	posters = append(posters, late)
	// A poster that made no post is an empty list in the history, not `null`,
	// which encoding/json writes for a nil slice and the format refuses.
	for i := range posters {
		if posters[i] == nil {
			posters[i] = []historyPost{}
		}
	}
	return inboxHistory{
		Version:  2,
		Sessions: []uint64{historySession},
		Closes:   true,
		Owner:    recorder.owner,
		Posters:  posters,
	}
}

// postedEnqueues is the place among the owner's operations of each enqueue of
// an input a post took, in the owner's program order.
func postedEnqueues(history inboxHistory) []int {
	var places []int
	for at, op := range history.Owner {
		if op.Call == "enqueue" && op.Source == "posted" {
			places = append(places, at)
		}
	}
	return places
}

// assertInboxRunExercised holds history to what a run is for: every input a post
// took entered, posts were taken through both doors and refused as ended and as
// unknown, an own event entered, the post behind the first drain was made, and
// the run ended once.
func assertInboxRunExercised(t *testing.T, what string, history inboxHistory) {
	t.Helper()
	count := func(keep func(historyPost) bool) int {
		n := 0
		for _, posts := range history.Posters {
			for _, post := range posts {
				if keep(post) {
					n++
				}
			}
		}
		return n
	}
	taken := count(func(post historyPost) bool { return post.Outcome == "taken" })
	if entered := len(postedEnqueues(history)); entered != taken {
		t.Fatalf("%s: %d inputs entered the queue of the %d posts taken", what, entered, taken)
	}
	for _, want := range []struct {
		name string
		keep func(historyPost) bool
	}{
		{"taken through the directory", func(post historyPost) bool {
			return post.Outcome == "taken" && post.Door == doorAddressed
		}},
		{"refused as ended", func(post historyPost) bool { return post.Reason == "ended" }},
		{"refused as unknown", func(post historyPost) bool { return post.Reason == "unknown" }},
	} {
		if count(want.keep) == 0 {
			t.Fatalf("%s: no post was %s", what, want.name)
		}
	}
	if len(history.Posters[1]) != 1 {
		t.Fatalf("%s: no post was made while the drain before the first own event applied what it read", what)
	}
	owns, ends := 0, 0
	for _, op := range history.Owner {
		switch {
		case op.Call == "end":
			ends++
		case op.Source == "own":
			owns++
		}
	}
	if owns == 0 || ends != 1 {
		t.Fatalf("%s: %d events of the engine's own entered and the run ended %d times", what, owns, ends)
	}
}

// historyCodegen is the sce-codegen SCE_CODEGEN names, whose
// `check-inbox-history` judges the runs; the case is skipped without it.
func historyCodegen(t *testing.T) string {
	t.Helper()
	codegen := os.Getenv("SCE_CODEGEN")
	if codegen == "" {
		t.Skip("SCE_CODEGEN names the sce-codegen whose check-inbox-history judges the runs; it is unset")
	}
	return codegen
}

// writeInboxHistory writes history as dir/name.json and answers the path.
func writeInboxHistory(t *testing.T, dir, name string, history inboxHistory) string {
	t.Helper()
	text, err := json.Marshal(history)
	if err != nil {
		t.Fatalf("%s: %v", name, err)
	}
	path := filepath.Join(dir, name+".json")
	if err := os.WriteFile(path, append(text, '\n'), 0o644); err != nil {
		t.Fatalf("cannot write %s: %v", path, err)
	}
	return path
}

// judgeInboxHistories hands files to `sce-codegen check-inbox-history`, and
// answers what it said and whether it found every one linearizable.
func judgeInboxHistories(t *testing.T, codegen string, files ...string) (string, bool) {
	t.Helper()
	out, err := exec.Command(codegen, append([]string{"check-inbox-history"}, files...)...).CombinedOutput()
	if err == nil {
		return string(out), true
	}
	var refused *exec.ExitError
	if !errors.As(err, &refused) {
		t.Fatalf("%s could not be run: %v", codegen, err)
	}
	return string(out), false
}

func TestEveryRunOfTheInboxIsLinearizableToItsPromise(t *testing.T) {
	codegen := historyCodegen(t)
	dir := t.TempDir()
	var files []string
	for run := 0; run < historyRuns; run++ {
		history := recordInboxRun(t)
		assertInboxRunExercised(t, fmt.Sprintf("run %d", run), history)
		files = append(files, writeInboxHistory(t, dir, fmt.Sprintf("inbox_go_run%02d", run), history))
	}
	if out, linearizable := judgeInboxHistories(t, codegen, files...); !linearizable {
		t.Fatalf("a run of the inbox is not linearizable to docs/formal/InboxService.tla:\n%s", out)
	}
}

// docs/adr/0147, D3: a post that returns after the drain before an event of the
// owner's own has read the inbox, and before the event enters, may linearize
// after the event, which enters ahead of it. The event's enqueue is invoked
// before its drain, so the post overlaps it and the run is linearizable. Read at
// the push instead, the event's enqueue is invoked after the post returned, the
// post is held to enter ahead of the event, and the same run is refused.
func TestAnOwnEventIsInvokedBeforeTheDrainItMakes(t *testing.T) {
	codegen := historyCodegen(t)
	dir := t.TempDir()
	history := recordInboxRun(t)
	assertInboxRunExercised(t, "the run", history)
	if out, linearizable := judgeInboxHistories(t, codegen, writeInboxHistory(t, dir, "before_the_drain", history)); !linearizable {
		t.Fatalf("the run with its own events invoked before their drains is refused:\n%s", out)
	}

	// Every reading doubled, so a reading fits between two of them, and the first
	// own event's enqueue invoked just after the post behind its drain returned,
	// as at its push.
	atThePush := inboxHistory{Version: history.Version, Sessions: history.Sessions, Closes: history.Closes}
	for _, op := range history.Owner {
		op.Invoked, op.Returned = 2*op.Invoked, 2*op.Returned
		atThePush.Owner = append(atThePush.Owner, op)
	}
	for _, posts := range history.Posters {
		doubled := []historyPost{}
		for _, post := range posts {
			post.Invoked, post.Returned = 2*post.Invoked, 2*post.Returned
			doubled = append(doubled, post)
		}
		atThePush.Posters = append(atThePush.Posters, doubled)
	}
	postReturned := atThePush.Posters[1][0].Returned
	moved := false
	for at, op := range atThePush.Owner {
		if op.Call != "enqueue" || op.Source != "own" || *op.Item != historyOwnBase {
			continue
		}
		if !(op.Invoked < postReturned && postReturned < op.Returned) {
			t.Fatalf("the post behind the drain did not return inside the own event's enqueue: %+v", op)
		}
		atThePush.Owner[at].Invoked = postReturned + 1
		moved = true
	}
	if !moved {
		t.Fatalf("the first own event never entered")
	}
	if out, linearizable := judgeInboxHistories(t, codegen, writeInboxHistory(t, dir, "at_the_push", atThePush)); linearizable {
		t.Fatalf("the run with the first own event read at its push was judged linearizable:\n%s", out)
	}
}

// The checker refuses a recorded run of this lane that lost an enqueue: the
// recording drops nothing the cases above would then pass on.
func TestARecordedRunWithAnEnqueueLeftOutIsRefused(t *testing.T) {
	codegen := historyCodegen(t)
	history := recordInboxRun(t)
	places := postedEnqueues(history)
	if len(places) == 0 {
		t.Fatalf("the run entered no input a post took")
	}
	last := places[len(places)-1]
	item := *history.Owner[last].Item
	history.Owner = append(history.Owner[:last:last], history.Owner[last+1:]...)
	if out, linearizable := judgeInboxHistories(t, codegen, writeInboxHistory(t, t.TempDir(), "left_out", history)); linearizable {
		t.Fatalf("the run with input %d's enqueue left out, an input taken and never entered, was judged "+
			"linearizable:\n%s", item, out)
	}
}

// The checker refuses a recorded run of this lane whose owner entered two inputs
// of one poster in the reverse of the order that poster posted them in: the order
// the runtime keeps is what the cases above hold.
func TestARecordedRunWithTwoInputsOfOnePosterEnteredInReverseIsRefused(t *testing.T) {
	codegen := historyCodegen(t)
	history := recordInboxRun(t)
	posterOf := map[uint64]int{}
	for who, posts := range history.Posters {
		for _, post := range posts {
			posterOf[post.Input] = who
		}
	}
	places := postedEnqueues(history)
	first, second := -1, -1
	for i := 0; i < len(places) && first < 0; i++ {
		for _, other := range places[i+1:] {
			if posterOf[*history.Owner[places[i]].Item] == posterOf[*history.Owner[other].Item] {
				first, second = places[i], other
				break
			}
		}
	}
	if first < 0 {
		t.Fatalf("no poster's posts entered at least twice")
	}
	a, b := *history.Owner[first].Item, *history.Owner[second].Item
	history.Owner[first].Item, history.Owner[second].Item = &b, &a
	if out, linearizable := judgeInboxHistories(t, codegen, writeInboxHistory(t, t.TempDir(), "reversed", history)); linearizable {
		t.Fatalf("the run with inputs %d and %d of one poster entered in reverse was judged linearizable:\n%s",
			a, b, out)
	}
}
