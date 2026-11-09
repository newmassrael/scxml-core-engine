// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package testprofile

import (
	"fmt"
	"sort"
)

// The run of one test case of the test profile (docs/SCE_TEST_PROFILE.md §2 to
// §5, §12): its entities, its ports and timers, the round scheduler on the
// logical clock, and the test case verdict.
//
// A component's machine is generated from its `test-component` and reaches this
// run only through its ComponentContext (§12.3): the snapshot its branches'
// guards read, the statement each taken branch runs first, the statements of
// the profile, and the doors of a send and of a PTC. The run knows nothing of
// the engine or of the record types, so it is tested on its own. How the state
// is shared and why it takes no lock is the package's documentation.
//
// Cross-language siblings: `sce_rust_runtime::test::TestCaseRun` (Rust),
// `SCE::Test::TestCaseRun` (C++), `com.sce.runtime.test.TestCaseRun` (Kotlin).

// MTCName is the name of the MTC in a configuration; no PTC takes it (§11.6).
const MTCName = "mtc"

// Endpoint is a component port named in a configuration: the component's name
// (`mtc` or a PTC's id) and the port's id.
type Endpoint struct {
	// Component is the component's name.
	Component string
	// Port is the port's id.
	Port string
}

func (e Endpoint) names(component, port string) bool {
	return e.Component == component && e.Port == port
}

// Connection is one `<sce:connect>` pair (-4 §9.10).
type Connection struct {
	A Endpoint
	B Endpoint
}

// Mapping is one `<sce:map>` entry: a component port and its system port
// (-4 §9.32).
type Mapping struct {
	Port       Endpoint
	SystemPort string
}

// Configuration is a `test-configuration` as the run reads it (§11.6).
//
// Generated from the configuration document. Each port takes part in one
// connection or one mapping (§2.2), which the build has checked.
type Configuration struct {
	// MTCType is the MTC's component type.
	MTCType string
	// PTCTypes is each PTC's name and its component type.
	PTCTypes map[string]string
	// PortsOfType is the port ids each component type declares.
	PortsOfType map[string][]string
	// TimersOfType is the timer ids each component type declares.
	TimersOfType map[string][]string
	// Connections are the `<sce:connect>` pairs.
	Connections []Connection
	// Mappings are the `<sce:map>` entries.
	Mappings []Mapping
}

// LogKind is what a line of the run's log records (§12.5): the run as a reader
// follows it, with the removals of PTCs and the cause of an `error` in the
// runtime's words. The test log, which two runtimes compare, is
// TestCaseResult.Records.
type LogKind int

const (
	// LogStart is the MTC created and started.
	LogStart LogKind = iota
	// LogCreate is a PTC created and started.
	LogCreate
	// LogSend is a component's send on a port.
	LogSend
	// LogReceive is a receive branch's removal of the head of a port.
	LogReceive
	// LogTimeout is a timeout branch's taking of a timer.
	LogTimeout
	// LogDone is a PTC that terminated (P10).
	LogDone
	// LogKill is a PTC removed.
	LogKill
	// LogVerdict is a component's setting of its local verdict.
	LogVerdict
	// LogLog is a `<log>`.
	LogLog
	// LogError is a dynamic error that ended the run (P13).
	LogError
)

// String is the kind's name, as the other runtimes name it.
func (k LogKind) String() string {
	switch k {
	case LogStart:
		return "Start"
	case LogCreate:
		return "Create"
	case LogSend:
		return "Send"
	case LogReceive:
		return "Receive"
	case LogTimeout:
		return "Timeout"
	case LogDone:
		return "Done"
	case LogKill:
		return "Kill"
	case LogVerdict:
		return "Verdict"
	case LogLog:
		return "Log"
	case LogError:
		return "Error"
	}
	return fmt.Sprintf("LogKind(%d)", int(k))
}

// LogRecord is one line of the run's log.
type LogRecord struct {
	// At is `NOW` when it was recorded.
	At Instant
	// Component is the component it is of.
	Component string
	// Kind is what it records.
	Kind LogKind
	// Detail is the port and event, the timer, the verdict and reason, the
	// text or the error, by kind; empty for a kind that has none.
	Detail string
}

// TestCaseResult is the outcome of one test case: its verdict, the instant it
// ended at, the lines of its log, and its test log.
type TestCaseResult struct {
	// Verdict is the test case verdict (P14).
	Verdict Verdict
	// EndedAt is `NOW` when the run ended.
	EndedAt Instant
	// Log is the lines of the run's log, for a reader.
	Log []LogRecord
	// Records is the test log (docs/adr/0091, C3): the records of TCI's test
	// logging interface, in the order the run produced them, which
	// CanonicalLog writes.
	Records []TestLogRecord
}

// SystemInput is one input of the SUT: the system port it arrives on and the
// message.
type SystemInput struct {
	SystemPort string
	Message    Message
}

// SystemAdapter is the SUT's side of the mapped ports (§12.2).
//
// Shaped after TRI (ES 201 873-5): Send is `triSend`, and the inputs
// TakeInputsDue returns are what `triEnqueueMsg` would enqueue. The adapter
// owns the codec and the transport; the run sees values only (ADR 0091 C2). The
// instants are the logical clock's, so a recorded trace replays to the same run
// (§3.1).
type SystemAdapter interface {
	// Send is a component's send of message on a port mapped to systemPort.
	Send(systemPort string, message Message, now Instant)
	// NextInput is the instant of the earliest input not yet taken and true,
	// or false when there is none.
	NextInput() (Instant, bool)
	// TakeInputsDue is every input due at or before now, in arrival order.
	TakeInputsDue(now Instant) []SystemInput
}

// ComponentMachine is a component's machine, as the run drives it (§12.3).
//
// Generated from a `test-component`: the backend's statechart machine, with
// each branch a transition on the round's snapshot event.
type ComponentMachine interface {
	// Start enters the initial configuration and completes the first
	// macrostep. The machine keeps context, and every call it makes, now and
	// on each later delivery, goes to it.
	Start(context *ComponentContext)
	// DeliverSnapshot delivers the round's snapshot event: the machine takes
	// the first branch whose guard holds on the snapshot (P3) and completes its
	// macrostep (P4), or takes none.
	DeliverSnapshot()
	// Terminated reports whether the machine has entered a top-level `<final>`
	// (P10).
	Terminated() bool
}

// MachineFactory builds the machine of a component type, or returns a nil
// ComponentMachine for a type it does not make, which ends the run with
// `error` where a component of that type is created.
type MachineFactory func(componentType string) ComponentMachine

// Guard is a test case's guard duration (P11), or none.
type Guard struct {
	limit Duration
	set   bool
}

// GuardOf is the guard of limit microseconds.
func GuardOf(limit Duration) Guard {
	return Guard{limit: limit, set: true}
}

// NoGuard is the guard of a test case that has none.
var NoGuard = Guard{}

// Limit is the guard's duration and true, or false for no guard.
func (g Guard) Limit() (Duration, bool) {
	return g.limit, g.set
}

// componentState is what one component holds in the run.
type componentState struct {
	ports        map[string]*Port
	timers       map[string]*Timer
	doneSnapshot map[string]bool
	verdict      Verdict
	branchTaken  bool
}

func (c *componentState) port(id string) *Port {
	port, ok := c.ports[id]
	if !ok {
		port = &Port{}
		c.ports[id] = port
	}
	return port
}

func (c *componentState) timer(id string) *Timer {
	timer, ok := c.timers[id]
	if !ok {
		timer = &Timer{}
		c.timers[id] = timer
	}
	return timer
}

type entity struct {
	// serial says which creation this is: a PTC removed and created again in
	// one round is a new component, which took no snapshot in it.
	serial uint64
	state  *componentState
	// machine is nil while the run has it out to call it.
	machine    ComponentMachine
	terminated bool
}

// waiting is a component that took a round's snapshot, by name and serial.
type waiting struct {
	name   string
	serial uint64
}

// runState is the state of one run, shared by the run and every component's
// context.
type runState struct {
	configuration    Configuration
	factory          MachineFactory
	adapter          SystemAdapter
	guard            Guard
	moduleParameters *ModuleParameters

	now     Instant
	created uint64
	// entities are by name; a name's place here is not the creation order,
	// order is.
	entities       map[string]*entity
	order          []string
	done           map[string]bool
	removedVerdict Verdict
	mtcTerminated  bool
	// failed says a dynamic error ended the run (P13).
	failed  bool
	log     []LogRecord
	records []TestLogRecord

	// held says a step of the run is inside the state (access).
	held bool
}

// access runs step on run, for component, with the state held for its length;
// a call that finds it held already is re-entrant, and panics.
func access[R any](run *runState, component string, step func(*runState) R) R {
	if run.held {
		panic("re-entrant access to the test run from " + component)
	}
	run.held = true
	defer func() { run.held = false }()
	return step(run)
}

// perform is access for a step that answers nothing.
func perform(run *runState, component string, step func(*runState)) {
	access(run, component, func(r *runState) struct{} {
		step(r)
		return struct{}{}
	})
}

// -- The round (§3.2, §12.4) ------------------------------------------------

// updateAndSnapshot is step 2, the update: inputs and timer expiries due at
// `NOW`. Then step 3, the snapshot: every waiting component at once. It
// answers the components that took it, in creation order, each with its
// serial.
func (r *runState) updateAndSnapshot() []waiting {
	for _, input := range r.adapter.TakeInputsDue(r.now) {
		r.deliverInput(input.SystemPort, input.Message)
	}
	for _, e := range r.entities {
		for _, timer := range e.state.timers {
			timer.Update(r.now)
		}
	}
	var took []waiting
	for _, name := range r.order {
		e, ok := r.entities[name]
		if !ok || e.terminated {
			continue
		}
		for _, port := range e.state.ports {
			port.TakeSnapshot()
		}
		for _, timer := range e.state.timers {
			timer.TakeSnapshot()
		}
		e.state.doneSnapshot = make(map[string]bool, len(r.done))
		for ptc := range r.done {
			e.state.doneSnapshot[ptc] = true
		}
		took = append(took, waiting{name: name, serial: e.serial})
	}
	return took
}

// takeForDelivery is the machine of a component that took this round's
// snapshot, out of its entity to be delivered; nil for one removed or
// terminated earlier in the round, which does not select (§3.2 step 5).
func (r *runState) takeForDelivery(name string, serial uint64) ComponentMachine {
	e, ok := r.entities[name]
	if !ok || e.serial != serial || e.terminated {
		return nil
	}
	e.state.branchTaken = false
	machine := e.machine
	e.machine = nil
	return machine
}

func (r *runState) branchTaken(name string, serial uint64) bool {
	e, ok := r.entities[name]
	return ok && e.serial == serial && e.state.branchTaken
}

// advance is a round in which no branch fired: time advances to the next
// instant something can happen at (§3.3), or the test case can never end
// (P13).
func (r *runState) advance() {
	next, pending := r.adapter.NextInput()
	for _, e := range r.entities {
		if e.terminated {
			continue
		}
		for _, timer := range e.state.timers {
			expiry, running := timer.Expiry()
			if !running {
				continue
			}
			if !pending || expiry < next {
				next = expiry
				pending = true
			}
		}
	}
	guardAt, guarded := r.guard.Limit()
	if guarded && (!pending || guardAt < next) {
		next = guardAt
		pending = true
	}
	if !pending {
		r.fail(MTCName, "no branch can fire and no instant is pending: the test case can never end")
		return
	}
	if next > r.now {
		r.now = next
	}
	if guarded && r.now >= guardAt {
		r.fail(MTCName, "the guard of the test case was reached")
	}
}

// settle puts back a component's machine that has just run. If it terminated,
// P10 applies.
//
// The entity is still there: a component whose machine is running is not
// removed (kill), and nothing else removes one mid-round.
func (r *runState) settle(name string, serial uint64, machine ComponentMachine, terminated bool) {
	e, ok := r.entities[name]
	if !ok || e.serial != serial {
		panic(fmt.Sprintf("the component %s was removed while its machine ran", name))
	}
	e.machine = machine
	if e.terminated || !terminated {
		return
	}
	e.terminated = true
	r.trace(ComponentDone{Component: name, Verdict: e.state.verdict})
	if name == MTCName {
		r.mtcTerminated = true
		return
	}
	r.done[name] = true
	r.record(name, LogDone, "")
}

func (r *runState) ended() bool {
	return r.mtcTerminated || r.failed
}

// finish is the end of the run, the MTC terminated or the run in error: every
// PTC still present is removed, in ascending order of name as the other
// runtimes remove them, and the test case verdict is the worst local verdict
// (P10, P14).
func (r *runState) finish() TestCaseResult {
	ptcs := make([]string, 0, len(r.entities))
	for name := range r.entities {
		if name != MTCName {
			ptcs = append(ptcs, name)
		}
	}
	sort.Strings(ptcs)
	for _, ptc := range ptcs {
		r.removeEntity(ptc)
	}
	verdict := r.removedVerdict
	if mtc, ok := r.entities[MTCName]; ok {
		verdict = Worse(verdict, mtc.state.verdict)
	}
	if r.failed {
		verdict = VerdictError
	}
	r.trace(TestCaseEnded{Verdict: verdict})
	log := make([]LogRecord, len(r.log))
	copy(log, r.log)
	records := make([]TestLogRecord, len(r.records))
	copy(records, r.records)
	return TestCaseResult{Verdict: verdict, EndedAt: r.now, Log: log, Records: records}
}

// -- Entities (§2.1, P9) ----------------------------------------------------

// enter enters a new component of the type componentType into the table, with
// its ports and timers, and logs its creation. It answers the component's
// serial.
func (r *runState) enter(name, componentType string) uint64 {
	r.created++
	state := &componentState{
		ports:  map[string]*Port{},
		timers: map[string]*Timer{},
	}
	for _, port := range r.configuration.PortsOfType[componentType] {
		state.port(port)
	}
	for _, timer := range r.configuration.TimersOfType[componentType] {
		state.timer(timer)
	}
	r.entities[name] = &entity{serial: r.created, state: state}
	r.order = append(r.order, name)
	kind := LogCreate
	if name == MTCName {
		kind = LogStart
	}
	r.record(name, kind, componentType)
	r.trace(ComponentCreated{Component: name, ComponentType: componentType})
	return r.created
}

// ptcToCreate is the type of the PTC ptc that creator creates and true, or
// false, with the dynamic error, when it cannot be created.
func (r *runState) ptcToCreate(creator, ptc string) (string, bool) {
	componentType, declared := r.configuration.PTCTypes[ptc]
	if !declared {
		r.fail(creator, fmt.Sprintf("creates '%s', which the configuration does not declare", ptc))
		return "", false
	}
	if _, present := r.entities[ptc]; present {
		r.fail(creator, fmt.Sprintf("creates '%s', which is already present", ptc))
		return "", false
	}
	delete(r.done, ptc)
	return componentType, true
}

// kill is the kill of the PTC ptc by killer (P9). A component whose machine is
// running, up the call stack of this one, is not removed under it: that is a
// dynamic error.
func (r *runState) kill(killer, ptc string) {
	if e, ok := r.entities[ptc]; ok && e.machine == nil {
		r.fail(killer, fmt.Sprintf("kills '%s', whose machine is running", ptc))
		return
	}
	r.removeEntity(ptc)
}

// removeEntity removes a PTC: its local verdict still counts (-4 §9.29a.2),
// and its ports are gone, so a send to one of them is an error.
func (r *runState) removeEntity(name string) {
	e, ok := r.entities[name]
	if !ok {
		return
	}
	delete(r.entities, name)
	r.removedVerdict = Worse(r.removedVerdict, e.state.verdict)
	r.record(name, LogKill, "")
	kept := r.order[:0]
	for _, present := range r.order {
		if present != name {
			kept = append(kept, present)
		}
	}
	r.order = kept
	delete(r.done, name)
}

// -- Ports (§2.2, P6) -------------------------------------------------------

func (r *runState) send(sender, port string, message Message) {
	r.record(sender, LogSend, port+" "+message.Event)
	r.trace(MessageSent{
		Component:   sender,
		Port:        port,
		MessageType: message.TypeName(),
		Value:       message.CanonicalValue(),
	})
	for _, connection := range r.configuration.Connections {
		var peer Endpoint
		switch {
		case connection.A.names(sender, port):
			peer = connection.B
		case connection.B.names(sender, port):
			peer = connection.A
		default:
			continue
		}
		if e, ok := r.entities[peer.Component]; ok {
			e.state.port(peer.Port).Enqueue(message)
			return
		}
		r.fail(sender, fmt.Sprintf("sends on '%s', whose peer '%s' is not present", port, peer.Component))
		return
	}
	for _, mapping := range r.configuration.Mappings {
		if mapping.Port.names(sender, port) {
			r.adapter.Send(mapping.SystemPort, message, r.now)
			return
		}
	}
	r.fail(sender, fmt.Sprintf("sends on '%s', which is neither connected nor mapped", port))
}

func (r *runState) deliverInput(systemPort string, message Message) {
	for _, mapping := range r.configuration.Mappings {
		if mapping.SystemPort != systemPort {
			continue
		}
		if e, ok := r.entities[mapping.Port.Component]; ok {
			e.state.port(mapping.Port.Port).Enqueue(message)
			return
		}
		r.fail(mapping.Port.Component,
			fmt.Sprintf("an input on system port '%s' found no component", systemPort))
		return
	}
	r.fail(MTCName, fmt.Sprintf("an input on system port '%s', which no port maps", systemPort))
}

// -- A component's own state, through its context ---------------------------

func (r *runState) component(name string, serial uint64) *componentState {
	e, ok := r.entities[name]
	if !ok || e.serial != serial {
		panic(fmt.Sprintf("the component %s was removed, and its context outlived it", name))
	}
	return e.state
}

func (r *runState) receive(name string, serial uint64, port string) Message {
	component := r.component(name, serial)
	component.branchTaken = true
	queue, declared := component.ports[port]
	if !declared {
		panic(fmt.Sprintf("%s receives on '%s', which its component type does not declare", name, port))
	}
	message := queue.RemoveHead()
	r.record(name, LogReceive, port+" "+message.Event)
	r.trace(MessageReceived{
		Component:   name,
		Port:        port,
		MessageType: message.TypeName(),
		Value:       message.CanonicalValue(),
	})
	return message
}

func (r *runState) timeout(name string, serial uint64, timer string) {
	component := r.component(name, serial)
	component.branchTaken = true
	binding, declared := component.timers[timer]
	if !declared {
		panic(fmt.Sprintf("%s times out on '%s', which its component type does not declare", name, timer))
	}
	binding.TimeoutTaken()
	r.record(name, LogTimeout, timer)
	r.trace(TimerTimedOut{Component: name, Timer: timer})
}

func (r *runState) setverdict(name string, serial uint64, verdict Verdict, reason string) {
	component := r.component(name, serial)
	component.verdict = Worse(component.verdict, verdict)
	detail := verdict.String()
	if reason != "" {
		detail += " " + reason
	}
	r.record(name, LogVerdict, detail)
	r.trace(VerdictSet{Component: name, Verdict: verdict, Reason: reason})
}

// -- The verdict and the log (§5, §12.5) ------------------------------------

// fail is a dynamic error (P13): the run ends with `error`. The test log
// records the verdict without why, which is this runtime's own prose.
func (r *runState) fail(component, why string) {
	r.failed = true
	r.record(component, LogError, why)
	r.trace(VerdictSet{Component: component, Verdict: VerdictError})
}

func (r *runState) record(component string, kind LogKind, detail string) {
	r.log = append(r.log, LogRecord{At: r.now, Component: component, Kind: kind, Detail: detail})
}

// trace is a record of the test log, at `NOW`.
func (r *runState) trace(event TestLogEvent) {
	r.records = append(r.records, TestLogRecord{At: r.now, Event: event})
}

// createEntity creates the component name of the type componentType and
// starts its machine, with the state let go while the factory and the machine
// run.
func createEntity(run *runState, name, componentType string) {
	machine := run.factory(componentType)
	if machine == nil {
		perform(run, name, func(r *runState) {
			r.fail(name, fmt.Sprintf(
				"is of the component type '%s', which the factory of machines does not make", componentType))
		})
		return
	}
	serial := access(run, name, func(r *runState) uint64 { return r.enter(name, componentType) })
	machine.Start(&ComponentContext{run: run, name: name, serial: serial})
	terminated := machine.Terminated()
	perform(run, name, func(r *runState) { r.settle(name, serial, machine, terminated) })
}

// round is one round (§3.2, §12.4).
func round(run *runState) {
	took := access(run, MTCName, func(r *runState) []waiting { return r.updateAndSnapshot() })
	// Steps 4 and 5: one component at a time, in creation order, each selects
	// on its own snapshot and runs the macrostep of the branch it took. A
	// component removed or terminated earlier in the round does not select,
	// and nothing does once the MTC has terminated (§3.2).
	anyTaken := false
	for _, w := range took {
		if access(run, w.name, func(r *runState) bool { return r.ended() }) {
			break
		}
		machine := access(run, w.name, func(r *runState) ComponentMachine {
			return r.takeForDelivery(w.name, w.serial)
		})
		if machine == nil {
			continue
		}
		machine.DeliverSnapshot()
		terminated := machine.Terminated()
		taken := access(run, w.name, func(r *runState) bool {
			branch := r.branchTaken(w.name, w.serial)
			r.settle(w.name, w.serial, machine, terminated)
			return branch
		})
		anyTaken = anyTaken || taken
	}
	perform(run, MTCName, func(r *runState) {
		if !r.ended() && !anyTaken {
			r.advance()
		}
	})
}

// ComponentContext is what a component's machine reads and does through the run
// (§12.3).
//
// The machine keeps it from Start on, and so may the host processors the
// machine registers for the doors of a send and of a PTC. Each call takes the
// run's state for its own length and lets it go before it returns, so no call
// holds it across a call into a machine. A call made while the state is held,
// from inside a snapshot reader say, is re-entrant and panics naming the
// component.
type ComponentContext struct {
	run    *runState
	name   string
	serial uint64
}

// Name is the component's name: `mtc`, or the PTC's id.
func (c *ComponentContext) Name() string {
	return c.name
}

// String shows the component the context is of.
func (c *ComponentContext) String() string {
	return fmt.Sprintf("ComponentContext(name=%s, serial=%d, ..)", c.name, c.serial)
}

// -- The snapshot, read by the branches' guards (P2) ------------------------

// WithSnapshot hands read the message the snapshot saw at the head of port, or
// nil. read runs while the run's state is held, so it must not call this
// context: such a call panics.
func (c *ComponentContext) WithSnapshot(port string, read func(head *Message)) {
	perform(c.run, c.name, func(r *runState) {
		queue, declared := r.component(c.name, c.serial).ports[port]
		if !declared {
			read(nil)
			return
		}
		head, saw := queue.Snapshot()
		if !saw {
			read(nil)
			return
		}
		read(&head)
	})
}

// SnapshotPresent reports whether the snapshot saw a message at the head of
// port: the guard of a receive any branch.
func (c *ComponentContext) SnapshotPresent(port string) bool {
	present := false
	c.WithSnapshot(port, func(head *Message) { present = head != nil })
	return present
}

// TimedOut reports whether the snapshot saw timer timed out.
func (c *ComponentContext) TimedOut(timer string) bool {
	return access(c.run, c.name, func(r *runState) bool {
		binding, declared := r.component(c.name, c.serial).timers[timer]
		return declared && binding.SnapshotTimedOut()
	})
}

// Done reports whether the snapshot saw the PTC ptc in `DONE`.
func (c *ComponentContext) Done(ptc string) bool {
	return access(c.run, c.name, func(r *runState) bool {
		return r.component(c.name, c.serial).doneSnapshot[ptc]
	})
}

// -- Taking a branch (P4 step 1), the first statement of each ---------------

// Receive is a receive branch: it removes the head of port, which the
// branch's content reads as `_event.data`.
//
// It panics when the component type declares no port port, or its queue is
// empty: a receive branch is taken only on a snapshot that saw a message
// there.
func (c *ComponentContext) Receive(port string) Message {
	return access(c.run, c.name, func(r *runState) Message { return r.receive(c.name, c.serial, port) })
}

// Timeout is a timeout branch: timer becomes idle.
//
// It panics when the component type declares no timer timer.
func (c *ComponentContext) Timeout(timer string) {
	perform(c.run, c.name, func(r *runState) { r.timeout(c.name, c.serial, timer) })
}

// DoneTaken is a done branch on the PTC ptc: nothing is removed (-4 §9.16);
// the component took a branch in this round.
func (c *ComponentContext) DoneTaken(ptc string) {
	perform(c.run, c.name, func(r *runState) { r.component(c.name, c.serial).branchTaken = true })
}

// -- Statements of the profile ----------------------------------------------

// Setverdict is `<sce:setverdict>` (P12): the local verdict becomes the worse
// of the two, and the verdict and its reason are logged.
func (c *ComponentContext) Setverdict(verdict Verdict, reason string) {
	perform(c.run, c.name, func(r *runState) { r.setverdict(c.name, c.serial, verdict, reason) })
}

// StartTimer is `<sce:start-timer>` (P7), for duration microseconds from
// `NOW`.
func (c *ComponentContext) StartTimer(timer string, duration Duration) {
	perform(c.run, c.name, func(r *runState) {
		r.component(c.name, c.serial).timer(timer).Start(r.now, duration)
		r.trace(TimerStarted{Component: c.name, Timer: timer, Duration: duration})
	})
}

// StopTimer is `<sce:stop-timer>` (P7). The test log records the stop of a
// running timer; the stop of one that is not running changes nothing.
func (c *ComponentContext) StopTimer(timer string) {
	perform(c.run, c.name, func(r *runState) {
		binding := r.component(c.name, c.serial).timer(timer)
		running := binding.Status() == TimerRunning
		binding.Stop()
		if running {
			r.trace(TimerStopped{Component: c.name, Timer: timer})
		}
	})
}

// Log is a `<log>` (P8): recorded in the test log with `NOW` and the
// component.
func (c *ComponentContext) Log(text string) {
	perform(c.run, c.name, func(r *runState) {
		r.record(c.name, LogLog, text)
		r.trace(Logged{Component: c.name, Text: text})
	})
}

// -- Doors (§12.3) ------------------------------------------------------------

// Send is a send on port (P6): to the queue of the connected port, or to the
// system adapter when the port is mapped.
func (c *ComponentContext) Send(port string, message Message) {
	perform(c.run, c.name, func(r *runState) { r.send(c.name, port, message) })
}

// Create is the start of the invocation that creates the PTC ptc (P9): the
// PTC is created and its machine started before this returns.
func (c *ComponentContext) Create(ptc string) {
	var componentType string
	creatable := access(c.run, c.name, func(r *runState) bool {
		var ok bool
		componentType, ok = r.ptcToCreate(c.name, ptc)
		return ok
	})
	if creatable {
		createEntity(c.run, ptc, componentType)
	}
}

// Kill is the cancel of that invocation: the PTC is removed (P9).
func (c *ComponentContext) Kill(ptc string) {
	perform(c.run, c.name, func(r *runState) { r.kill(c.name, ptc) })
}

// -- Module parameters (§11.5, §11.8) ---------------------------------------

// ReadModuleParameters hands read the module parameters of the run, which it
// reads into the variables the component declares as module parameters,
// before the component's machine starts. A value the run was not given
// (ModuleParameters.Of), or one read refuses as not of the type the component
// declares, is the error read returns, and a dynamic error (P13): the run ends
// with `error` and false is returned, so the machine is not started on a value
// made up in its place. read runs with the run's state let go.
func (c *ComponentContext) ReadModuleParameters(read func(given *ModuleParameters) error) bool {
	given := access(c.run, c.name, func(r *runState) *ModuleParameters { return r.moduleParameters.Copy() })
	if err := read(given); err != nil {
		perform(c.run, c.name, func(r *runState) { r.fail(c.name, err.Error()) })
		return false
	}
	return true
}

// Now is `NOW`.
func (c *ComponentContext) Now() Instant {
	return access(c.run, c.name, func(r *runState) Instant { return r.now })
}

// Receives reports whether a receive branch with a template matches the
// snapshot of context (P2, docs/SCE_TEST_PROFILE.md §12.3).
//
// The snapshot saw a message at the head of port, under event, the event name
// of the template's record, whose value is a V, and matches, the template's
// `Matches` bound to its arguments, holds of that value. A message of another
// record, or one whose value is of another type, does not match. matches runs
// while the run's state is held, as a reader of ComponentContext.WithSnapshot
// does.
func Receives[V any](context *ComponentContext, port, event string, matches func(V) bool) bool {
	held := false
	context.WithSnapshot(port, func(head *Message) {
		if head == nil || head.Event != event {
			return
		}
		value, ok := ValueAs[V](*head)
		held = ok && matches(value)
	})
	return held
}

// TestCaseRun is one run of one test case (§2.3, §3, §4.9 to §4.11, §5).
//
// The components run on the calling goroutine, in the order they were created;
// §3.2 shows that the order within a round does not change the result.
type TestCaseRun struct {
	state *runState
	// started says the run has been run. The Rust run is consumed by its
	// `run`; a Go value cannot be, so a second run of it, whose adapter has
	// already been read, is refused.
	started bool
}

// NewTestCaseRun is a run of the test case whose configuration is
// configuration, whose machines factory builds, against adapter.
//
// guard is the test case's guard (P11), and moduleParameters the values of the
// module parameters the components read (§11.8): the suite's, with any the
// caller overrides. They are copied, so a later change to the caller's leaves
// the run as it was built; nil is no module parameter.
func NewTestCaseRun(
	configuration Configuration,
	factory MachineFactory,
	adapter SystemAdapter,
	guard Guard,
	moduleParameters *ModuleParameters,
) *TestCaseRun {
	return &TestCaseRun{state: &runState{
		configuration:    configuration,
		factory:          factory,
		adapter:          adapter,
		guard:            guard,
		moduleParameters: moduleParameters.Copy(),
		entities:         map[string]*entity{},
		done:             map[string]bool{},
	}}
}

// Run runs the test case to its end and answers its verdict and logs.
//
// It panics when this run has been run already.
func (t *TestCaseRun) Run() TestCaseResult {
	if t.started {
		panic("a test case run is run once; build another to run the test case again")
	}
	t.started = true
	perform(t.state, MTCName, func(r *runState) { r.trace(TestCaseStarted{}) })
	createEntity(t.state, MTCName, t.state.configuration.MTCType)
	for !access(t.state, MTCName, func(r *runState) bool { return r.ended() }) {
		round(t.state)
	}
	return access(t.state, MTCName, func(r *runState) TestCaseResult { return r.finish() })
}
