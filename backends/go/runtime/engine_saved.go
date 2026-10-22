// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

import (
	"fmt"
	"slices"
	"sort"
)

// Saving a `datamodel="sce-static"` machine and restoring it (SCE Accepted
// Subset §2.15, docs/adr/0045).
//
// The steps every generated Save and Restore take alike; the generated machine
// adds only its own variables, its histories and its running children between
// them. The Go twin of `sce_rust_runtime::saved_state::{save, upgrade, enter,
// judge}` and of the saved-state half of Kotlin's `StateMachineEngine`.

// SavedPolicy is what a generated policy of a document that saves adds to
// StatePolicy: what only the document knows about the sessions and invocations
// a saved state names. Every policy a generated Save and Restore are written for
// has these methods; a document that starts no child has them answer that it
// has none.
type SavedPolicy[S comparable, E comparable] interface {
	StatePolicy[S, E]

	// OwnSessionID is this session's id, `_sessionid`, which a saved state names
	// it by. Empty for a policy never given one.
	OwnSessionID() string

	// BindSessionID makes id this session's own: a restore of a format-2 state
	// carries on under the id it was saved with, which every child's id and a
	// peer's address name.
	BindSessionID(id string)

	// RunningInvokes is the id the document gives each `<invoke>` whose child
	// session is running, in document order (§scxml-6.4).
	RunningInvokes() []string

	// InvokeStarts is how many times each `<invoke>` of the document has started
	// in this session, for each started at least once, in document order.
	InvokeStarts() []SavedInvokeStart

	// InvokeOwner is the state that holds the `<invoke>` id, and false for an id
	// the document does not invoke.
	InvokeOwner(id string) (S, bool)

	// HostInvokeOwner is the state that holds the host-run `<invoke>` of this
	// type and id, and false for one the document does not have a host invoker
	// run (§scxml-6.4.1).
	HostInvokeOwner(processorType, id string) (S, bool)

	// JudgeChild refuses child, a running child a format-2 state lists, unless
	// its session is a state of the document the child names, to any depth
	// (docs/adr/0045, section 3, step 1). It builds nothing.
	JudgeChild(child SavedChild) error

	// ResumeChildren resumes each of children on engine, in the order given, each
	// built as starting it builds it and restored where it stood in place of
	// initialized: no `<onentry>`, no `<data>`, no `<param>` or namelist
	// evaluated, no `idlocation` written, nothing sent. Judged already, so none
	// refuses for what a judgement would have caught.
	ResumeChildren(engine *Engine[S, E], children []SavedChild, wallNowMs int64) error

	// RestartInvokes starts each of ids again from its beginning, as entering its
	// state starts it, which is what a format-1 state asks for of a child it
	// names by id alone.
	RestartInvokes(engine *Engine[S, E], ids []string)

	// SetHistoryValue gives history the states a saved state recorded for it.
	SetHistoryValue(history HistoryID, states []S)
}

// HistoryDecl is a `<history>` of the document, as a generated machine declares
// it: the id a saved state keys it by, the history itself, and whether it is
// deep.
type HistoryDecl struct {
	// ID is the `<history>`'s id in the document.
	ID string
	// History is the history, as the policy names it.
	History HistoryID
	// Deep is whether it records the atomic states below its parent
	// (§scxml-3.10) and not only the parent's children.
	Deep bool
}

// startedHostInvoke is a host-run invocation started and neither completed nor
// cancelled: its token, and the request the host was handed, which a save
// writes.
type startedHostInvoke struct {
	token   uint64
	request HostInvokeRequest
}

// restoredDeadlineKind says what a restored host-run invocation has left of its
// deadline.
type restoredDeadlineKind int

const (
	// restoredNoDeadline is an invocation that had none.
	restoredNoDeadline restoredDeadlineKind = iota
	// restoredDeadlineAt is a deadline that comes due at atMs of the engine's
	// own clock, worked out from the wall-clock moment the saved state holds.
	restoredDeadlineAt
	// restoredDeadlinePassed is one that came due while the machine was away.
	restoredDeadlinePassed
)

// restoredHostInvoke is a host-run invocation a saved state listed, waiting for
// the first macrostep the host drives to start it again.
type restoredHostInvoke struct {
	// request is what the saved machine started it with, marked Restarted.
	request  HostInvokeRequest
	deadline restoredDeadlineKind
	atMs     int64
}

// rearmHostInvoke re-arms a host-run invocation a saved state listed: it starts
// again, from request, at the top of the first macrostep the host drives. A
// restore calls this once per invocation, in the order the state lists them.
func (e *Engine[S, E]) rearmHostInvoke(request HostInvokeRequest, kind restoredDeadlineKind, atMs int64) {
	request.Restarted = true
	e.restoredHostInvokes = append(e.restoredHostInvokes, restoredHostInvoke{
		request: request, deadline: kind, atMs: atMs,
	})
}

// startRestoredHostInvokes starts the invocations a restore re-armed, as the
// macrostep's start would start ones the document had just entered (§scxml-6.4).
//
// A deadline that came due while the machine was away ends the invocation
// without a start: the host is not asked to begin what it would be told to stop
// at once, and the document receives the `error.invoke` the deadline raises. An
// invoker that is not registered by now is the same error a document's own start
// gets.
func (e *Engine[S, E]) startRestoredHostInvokes() {
	if len(e.restoredHostInvokes) == 0 {
		return
	}
	restored := e.restoredHostInvokes
	e.restoredHostInvokes = nil
	for _, host := range restored {
		invokeID := host.request.InvokeID
		// A request SCE sends on the document's behalf (a Mesh request) may already
		// have reached its peer, which would act on it twice if it were sent
		// again, and the router that carried it is gone with the process that
		// saved it. It is not started: the document is told the call was
		// interrupted, and sends a new one if it means to ask again.
		switch {
		case host.deadline == restoredDeadlinePassed:
			e.raiseHostInvokeFailure(invokeID, "deadline")
		case IsReservedType(host.request.ProcessorType):
			e.raiseHostInvokeFailure(invokeID, "interrupted")
		default:
			handler, registered := e.hostInvokers[host.request.ProcessorType]
			if !registered {
				if evt, known := e.policy.GetEventFromName("error.execution"); known {
					e.Raise(NewPlatformError(evt, "<invoke> names an invoker the host declared but never registered"))
				}
				continue
			}
			var deadlineAt *int64
			if host.deadline == restoredDeadlineAt {
				at := host.atMs
				deadlineAt = &at
			}
			e.startHostInvoke(handler, host.request, deadlineAt)
		}
	}
}

// raiseHostInvokeFailure tells the document a host-run invocation failed, as
// `error.invoke.<id>` with the reason as its data: `"deadline"` for a deadline
// that passed, `"interrupted"` when a restore found a call it must not send
// again.
func (e *Engine[S, E]) raiseHostInvokeFailure(invokeID, reason string) {
	metadata := ExternalMetadata("", "")
	metadata.Data = `"` + reason + `"`
	metadata.InvokeID = invokeID
	if meta, ok := e.arrivingEvent(ErrorInvokePrefix+invokeID, metadata); ok {
		e.externalQueue.Raise(meta)
	}
}

// runningHostInvoke is a host-run invocation as a save sees it: the request it
// was started with and the moment of the engine's clock its deadline comes due,
// if it has one.
type runningHostInvoke struct {
	request HostInvokeRequest
	dueAtMs *int64
}

// runningHostInvokes is the host-run invocations that are running, or re-armed
// and waiting to start, for a save: each with the request it was started with and
// the moment of the engine's clock its deadline comes due. Ordered by
// `(type, id)`, so every backend writes them in the same order.
func (e *Engine[S, E]) runningHostInvokes() []runningHostInvoke {
	pending := e.scheduler.Pending()
	now := e.NowMs()
	var running []runningHostInvoke
	for _, started := range e.startedHostInvokes {
		host := runningHostInvoke{request: started.request}
		for _, entry := range pending {
			if deadline := entry.Act.Deadline; deadline != nil && deadline.Token == started.token {
				due := entry.ReadyAtMs
				host.dueAtMs = &due
				break
			}
		}
		running = append(running, host)
	}
	for _, restored := range e.restoredHostInvokes {
		host := runningHostInvoke{request: restored.request}
		switch restored.deadline {
		case restoredDeadlineAt:
			at := restored.atMs
			host.dueAtMs = &at
		case restoredDeadlinePassed:
			host.dueAtMs = &now
		}
		running = append(running, host)
	}
	sort.SliceStable(running, func(i, j int) bool {
		a, b := running[i].request, running[j].request
		if a.ProcessorType != b.ProcessorType {
			return a.ProcessorType < b.ProcessorType
		}
		return a.InvokeID < b.InvokeID
	})
	return running
}

// hostInvokeIsRunning is whether the start token of the host-run invocation
// (type, id) is running: what decides whether a deadline on the scheduler belongs
// to one.
func (e *Engine[S, E]) hostInvokeIsRunning(processorType, invokeID string, token uint64) bool {
	started, running := e.startedHostInvokes[hostInvokeKey{processorType, invokeID}]
	return running && started.token == token
}

// dueOn is the wall-clock moment, in milliseconds since the Unix epoch, a thing
// that comes due at readyAtMs of the engine's clock comes due at, given that
// wallNowMs is what time it is on the wall now and nowMs on the engine's clock:
// none earlier than now, for one already past.
func dueOn(wallNowMs, nowMs, readyAtMs int64) int64 {
	left := readyAtMs - nowMs
	if left < 0 {
		left = 0
	}
	return saturatingAddMs(wallNowMs, left)
}

// SaveSession is engine's saved state, a machine of the document whose saved
// state is bound to shape, with the variables, the histories and the running
// children its generated Save read. The configuration is written in document
// order, and every waiting delayed `<send>` is written as the moment it comes
// due on the wall clock whose reading now is wallNowMs.
//
// A machine any of whose record schemas or enums is past its first revision names
// each such revision in `revisions` and writes the revised shape; any other
// writes the first shape and no `revisions`. The state is format 2: the
// machine's own id, how many times each `<invoke>` has started, and children,
// each running child with its whole state, which the generated machine saved
// through the child's own document, in document order.
//
// Refused for a machine that is not running (never started, or ended at a
// top-level `<final>`, where there is nothing left to resume), for one whose
// last macrostep stopped at the microstep ceiling (its configuration is not a
// settled one, and a restore would resume it as if it were), for one with no
// session id, and when children are not the children the machine is running.
func SaveSession[S comparable, E comparable](
	engine *Engine[S, E],
	shape *SavedShape,
	variables *JSONObject,
	history []SavedHistory,
	children []SavedChild,
	wallNowMs int64,
) (*SavedState, error) {
	policy, saves := engine.policy.(SavedPolicy[S, E])
	if !saves {
		return nil, refuse("this machine has no saved state, so a session it runs cannot be saved")
	}
	if !engine.isRunning {
		return nil, refuse("the machine is not running: it was never started, or it has ended")
	}
	if engine.macrostepTruncated {
		return nil, refuse("the machine's last macrostep stopped at the microstep ceiling, so it does not " +
			"stand at a settled configuration")
	}
	sessionID := policy.OwnSessionID()
	if sessionID == "" {
		return nil, refuse("the machine has no session id, which a saved state names it by")
	}
	// What the generated machine saved is every child it runs, and nothing else:
	// one left out would restore a parent whose child is gone.
	running := policy.RunningInvokes()
	savedIDs := make([]string, len(children))
	for at, child := range children {
		savedIDs[at] = child.ID
	}
	if !slices.Equal(running, savedIDs) {
		return nil, refuse("the machine runs the children %q and the save holds %q, a defect of its generated save",
			running, savedIDs)
	}
	// Read before anything is built, so a delayed send this state cannot carry
	// refuses the save instead of leaving it out.
	pending, err := savePending(engine, policy, wallNowMs)
	if err != nil {
		return nil, err
	}
	// Document order, so the same machine saves the same text on every backend.
	active := append([]S(nil), engine.GetActiveStates()...)
	sort.SliceStable(active, func(i, j int) bool {
		return policy.GetDocumentOrder(active[i]) < policy.GetDocumentOrder(active[j])
	})
	configuration := make([]string, len(active))
	for at, state := range active {
		configuration[at] = policy.GetStateName(state)
	}
	// The revision of each record schema and each enum past its first, whose
	// digest list is empty exactly when it is in its first revision: the record
	// schemas first, then the enums, each in the order the generator gave them.
	var revisions []SavedRevision
	for _, record := range shape.Records {
		if len(record.Revisions) > 0 {
			revisions = append(revisions, SavedRevision{
				Key: record.Event, Revision: record.Revision, Fields: record.Revisions[len(record.Revisions)-1],
			})
		}
	}
	for _, lineage := range shape.Enums {
		if len(lineage.Revisions) > 0 {
			revisions = append(revisions, SavedRevision{
				Key: lineage.Key, Revision: lineage.Revision, Fields: lineage.Revisions[len(lineage.Revisions)-1],
			})
		}
	}
	savedShape := shape.Format2First
	if len(revisions) > 0 {
		savedShape = shape.Format2Revised
	}
	if children == nil {
		children = []SavedChild{}
	}
	state := &SavedState{
		Shape:           savedShape,
		Revisions:       revisions,
		Configuration:   configuration,
		Current:         policy.GetStateName(engine.GetCurrentState()),
		Variables:       variables,
		History:         history,
		Pending:         pending,
		Invokes:         SavedInvokes{Resumed: true, Children: children},
		SessionID:       sessionID,
		InvokeStarts:    policy.InvokeStarts(),
		HostInvokes:     saveHostInvokes(engine, wallNowMs),
		HostInvokeToken: int64(engine.nextHostInvokeToken),
		AutoSendSeq:     int64(engine.autoSendSeq),
	}
	for _, queued := range engine.externalQueue.Snapshot() {
		// §scxml-5.10: a queued event is saved under the name it ARRIVED under, so a
		// restore gives the machine the same `_event.name`.
		name := queued.Metadata.Name
		if name == "" {
			name = policy.GetEventName(queued.Event)
		}
		event := SavedEvent{
			Name:       name,
			Data:       queued.Metadata.Data,
			Type:       queued.Metadata.EventType.String(),
			SendID:     queued.Metadata.SendID,
			Origin:     queued.Metadata.Origin,
			OriginType: queued.Metadata.OriginType,
			InvokeID:   queued.Metadata.InvokeID,
		}
		if token := queued.Metadata.HostInvokeToken; token != nil {
			saved := int64(*token)
			event.HostInvokeToken = &saved
		}
		state.External = append(state.External, event)
	}
	return state, nil
}

// saveHostInvokes is every host-run invocation of engine that is running, or was
// re-armed by a restore and has yet to start, as a saved state holds it: the
// request it was started with and the wall-clock moment its deadline comes due.
func saveHostInvokes[S comparable, E comparable](engine *Engine[S, E], wallNowMs int64) []SavedHostInvoke {
	now := engine.NowMs()
	var hosts []SavedHostInvoke
	for _, running := range engine.runningHostInvokes() {
		host := SavedHostInvoke{
			ProcessorType: running.request.ProcessorType,
			InvokeID:      running.request.InvokeID,
			Src:           running.request.Src,
			Params:        cloneParams(running.request.Params),
			Data:          running.request.EventData,
			Content:       running.request.Content,
		}
		if running.dueAtMs != nil {
			due := dueOn(wallNowMs, now, *running.dueAtMs)
			host.Due = &due
		}
		hosts = append(hosts, host)
	}
	return hosts
}

// savePending is every delayed send of engine still waiting, as a saved state
// holds it: written as the wall-clock moment it comes due, in the order the
// machine would deliver them.
//
// A send to the parent or to a child of this session is held as the act that
// delivers it to the session the restored tree holds there (docs/adr/0045). A
// send the format cannot carry, one routed to a session by its id, refuses the
// save. A document that makes one is generated without the save API, so no
// generated machine reaches this; the refusal is for a machine built by hand,
// where leaving the entry out would restore a machine that never delivers it and
// say nothing.
//
// The deadline of a running host-run invocation is not a send: it is saved with
// the invocation, as its `due` (saveHostInvokes), and left out here. One that
// belongs to no running invocation is refused like a send the format cannot
// carry.
func savePending[S comparable, E comparable](
	engine *Engine[S, E],
	policy SavedPolicy[S, E],
	wallNowMs int64,
) ([]SavedSend, error) {
	now := engine.NowMs()
	var sends []SavedSend
	for _, entry := range engine.scheduler.Pending() {
		act := entry.Act
		due := dueOn(wallNowMs, now, entry.ReadyAtMs)
		var saved SavedAct
		switch {
		case act.Deadline != nil:
			if engine.hostInvokeIsRunning(act.Deadline.ProcessorType, act.Deadline.InvokeID, act.Deadline.Token) {
				continue
			}
			return nil, refuse("the deadline of the invocation '%s' belongs to no invocation this machine is running",
				act.Deadline.InvokeID)
		case act.HostSend != nil:
			saved = SavedAct{
				Kind:          SavedActHost,
				ProcessorType: act.HostSend.ProcessorType,
				Event:         act.HostSend.EventName,
				Target:        act.HostSend.Target,
				Content:       act.HostSend.Content,
				Params:        cloneParams(act.HostSend.Params),
				SendID:        act.HostSend.SendID,
				Data:          act.HostSend.EventData,
				InvokeID:      act.HostSend.InvokeID,
			}
		case act.Route == nil:
			// §scxml-5.10: a send is saved under its member's name. A name the
			// document computed (`<send eventexpr>`) needs a script engine, which no
			// saved machine has, so none is waiting here.
			saved = SavedAct{Kind: SavedActRaise, Event: policy.GetEventName(act.Event), Data: act.Data,
				SendID: act.SendID, Origin: act.Origin}
		case act.Route.Kind == RouteInternalQueue:
			saved = SavedAct{Kind: SavedActInternal, Event: policy.GetEventName(act.Event), Data: act.Data,
				SendID: act.SendID, Origin: act.Origin}
		// docs/adr/0045: the parent and a child of this session are sessions the
		// saved tree holds.
		case act.Route.Kind == RouteParent:
			saved = SavedAct{Kind: SavedActParent, Event: act.Route.EventName, Data: act.Data,
				SendID: act.SendID, Origin: act.Origin}
		case act.Route.Kind == RouteInvocation:
			saved = SavedAct{Kind: SavedActChild, InvokeID: act.Route.InvokeID, Event: act.Route.EventName,
				Data: act.Data, SendID: act.SendID, Origin: act.Origin}
		default:
			return nil, refuse("the delayed send '%s' is routed to the session '%s', a session a saved state does "+
				"not carry", act.SendID, act.Route.SessionID)
		}
		sends = append(sends, SavedSend{Due: due, Act: saved})
	}
	return sends, nil
}

// SaveHistory is what each of declared has recorded in policy, as a saved state
// holds it: state ids in document order (the order one machine writes is every
// backend's), keyed by the history's id and ordered by it. A history that has
// recorded nothing is left out.
func SaveHistory[S comparable, E comparable](policy StatePolicy[S, E], declared []HistoryDecl) []SavedHistory {
	var saved []SavedHistory
	for _, decl := range declared {
		recorded, held := policy.HistoryValue(decl.History)
		if !held {
			continue
		}
		states := append([]S(nil), recorded...)
		sort.SliceStable(states, func(i, j int) bool {
			return policy.GetDocumentOrder(states[i]) < policy.GetDocumentOrder(states[j])
		})
		names := make([]string, len(states))
		for at, state := range states {
			names[at] = policy.GetStateName(state)
		}
		saved = append(saved, SavedHistory{ID: decl.ID, States: names})
	}
	sort.SliceStable(saved, func(i, j int) bool { return saved[i].ID < saved[j].ID })
	return saved
}

// RestoredHistory is what a saved state records for one `<history>`, read as the
// document's own: the history, and the states it recorded.
type RestoredHistory[S comparable] struct {
	History HistoryID
	States  []S
}

// RestoreHistory is what saved records for each of declared, read as the states
// of this document, or the refusal that says which value is not one.
//
// Every value is judged before any is applied, so a state with one bad history
// restores none of them. A history the document does not declare, one named
// twice, and a value its history could not have recorded (a state outside its
// parent, a compound state with two active children) are refused: a resumed
// machine entered through a history must land in a configuration its document
// could have been in.
func RestoreHistory[S comparable, E comparable](
	policy StatePolicy[S, E],
	saved *SavedState,
	declared []HistoryDecl,
) ([]RestoredHistory[S], error) {
	var restored []RestoredHistory[S]
	for at, recorded := range saved.History {
		for _, earlier := range saved.History[:at] {
			if earlier.ID == recorded.ID {
				return nil, refuse("the saved state records the history '%s' twice", recorded.ID)
			}
		}
		decl := -1
		for index, candidate := range declared {
			if candidate.ID == recorded.ID {
				decl = index
				break
			}
		}
		if decl < 0 {
			return nil, refuse("the saved state records the history '%s', which the document does not declare",
				recorded.ID)
		}
		states := make([]S, 0, len(recorded.States))
		for _, name := range recorded.States {
			state, named := policy.GetStateFromName(name)
			if !named {
				return nil, refuse("the history '%s' records '%s', which the document does not name",
					recorded.ID, name)
			}
			states = append(states, state)
		}
		history := declared[decl]
		verdict := ValidateHistory(policy, policy.GetHistoryParent(history.History), history.Deep, states)
		if verdict.Kind != HistoryAccepted {
			return nil, refuse("the history '%s' records %s", recorded.ID, describeHistory(policy, verdict))
		}
		restored = append(restored, RestoredHistory[S]{History: history.History, States: states})
	}
	return restored, nil
}

// describeHistory is a history refusal in the document's own vocabulary.
func describeHistory[S comparable, E comparable](policy StatePolicy[S, E], verdict HistoryRejection[S]) string {
	name := func(state S) string { return policy.GetStateName(state) }
	switch verdict.Kind {
	case HistoryEmpty:
		return "no state, and a history records at least one"
	case HistoryDuplicate:
		return fmt.Sprintf("'%s' twice", name(verdict.State))
	case HistoryNotBelow:
		return fmt.Sprintf("'%s', which is not below its parent '%s'", name(verdict.State), name(verdict.Parent))
	case HistoryNotAChild:
		return fmt.Sprintf("'%s', which is not a child of '%s' (a shallow history records the children)",
			name(verdict.State), name(verdict.Parent))
	case HistoryNotAtomic:
		return fmt.Sprintf("'%s', which has children (a deep history records atomic states)", name(verdict.State))
	default:
		return "states of no configuration: " + verdict.Arity.String()
	}
}

// readSend is a waiting send read against the document: the event it names,
// resolved, so a restore that cannot resolve one refuses before it arms any.
type readSend[E any] struct {
	due   int64
	act   SavedAct
	event E
}

// judgedSession is a saved session read against the document of a policy, before
// anything is built.
type judgedSession[S comparable, E comparable] struct {
	configuration []S
	current       S
	external      []EventWithMetadata[E]
	pending       []readSend[E]
	// restarts is the `<invoke>`s a format-1 state lists, each to be started
	// again.
	restarts []string
}

// JudgeSession refuses saved unless it can be a state of the document of policy,
// building nothing (docs/adr/0045, section 3, step 1): its configuration, its
// queue, its waiting sends, its running and host-run invocations, its count of
// starts, and each running child's session against the document that child runs
// (SavedPolicy.JudgeChild), to any depth.
//
// The shape, the revisions, the history and the variables are the generated
// machine's to judge before this (UpgradeSavedState, RestoreHistory); a generated
// Judge does both, and is what a parent's judge of a tree calls for each child it
// holds.
func JudgeSession[S comparable, E comparable](policy SavedPolicy[S, E], saved *SavedState) error {
	_, err := judgeSession(NewEngine[S, E](policy), policy, saved)
	return err
}

func judgeSession[S comparable, E comparable](
	engine *Engine[S, E],
	policy SavedPolicy[S, E],
	saved *SavedState,
) (*judgedSession[S, E], error) {
	judged := &judgedSession[S, E]{}
	for _, id := range saved.Configuration {
		state, named := policy.GetStateFromName(id)
		if !named {
			return nil, refuse("the document has no state '%s'", id)
		}
		judged.configuration = append(judged.configuration, state)
	}
	current, named := policy.GetStateFromName(saved.Current)
	if !named {
		return nil, refuse("the document has no state '%s'", saved.Current)
	}
	judged.current = current
	if verdict := ValidateConfiguration[S, E](policy, judged.configuration, current); verdict != ConfigurationAccepted {
		return nil, refuse("the saved configuration is refused: %s", verdict)
	}
	// Every queued event is read before anything is entered, so one this document
	// cannot name refuses the restore rather than half of the queue.
	for at, event := range saved.External {
		// The name a queued event arrived under (§scxml-5.10): the document's own
		// event for it, or the one it extends (§scxml-3.12.1).
		resolved, ok := engine.ResolveEventByName(event.Name)
		if !ok {
			return nil, refuse("external[%d] is '%s', which the document does not name", at, event.Name)
		}
		eventType, ok := eventTypeOfName(event.Type)
		if !ok {
			return nil, refuse("external[%d] has the type '%s', which is not an event type", at, event.Type)
		}
		queued := EventWithMetadata[E]{Event: resolved}
		if policy.GetEventName(resolved) != event.Name {
			queued.Metadata.Name = event.Name
		}
		queued.Metadata.Data = event.Data
		queued.Metadata.EventType = eventType
		queued.Metadata.SendID = event.SendID
		queued.Metadata.Origin = event.Origin
		queued.Metadata.OriginType = event.OriginType
		queued.Metadata.InvokeID = event.InvokeID
		if event.HostInvokeToken != nil {
			token := uint64(*event.HostInvokeToken)
			queued.Metadata.HostInvokeToken = &token
		}
		judged.external = append(judged.external, queued)
	}
	// The same for a waiting send: an event this document does not name refuses
	// the restore, not the sends before it.
	for at, send := range saved.Pending {
		read, err := readPendingSend(policy, send, at)
		if err != nil {
			return nil, err
		}
		judged.pending = append(judged.pending, read)
	}
	// The same for a running invocation: one the document does not have, one whose
	// state the saved configuration does not stand in, or one named twice would be
	// a child the saved machine could not have had.
	ids := saved.Invokes.InvokeIDs()
	for at, id := range ids {
		owner, owned := policy.InvokeOwner(id)
		if !owned {
			return nil, refuse("invokes[%d] is '%s', which the document does not invoke", at, id)
		}
		if !slices.Contains(judged.configuration, owner) {
			return nil, refuse("invokes[%d] is '%s', whose state the saved configuration does not stand in", at, id)
		}
		if slices.Contains(ids[:at], id) {
			return nil, refuse("invokes[%d] is '%s', which an earlier entry already names", at, id)
		}
	}
	// A count of starts names an `<invoke>` of the document.
	for _, started := range saved.InvokeStarts {
		if _, owned := policy.InvokeOwner(started.ID); !owned {
			return nil, refuse("invokestarts counts '%s', which the document does not invoke", started.ID)
		}
	}
	if saved.Invokes.Resumed {
		for at, child := range saved.Invokes.Children {
			if err := judgeChildEntry(policy, saved, child, at); err != nil {
				return nil, err
			}
		}
	} else {
		judged.restarts = saved.Invokes.IDs
	}
	// And for a host-run one, by the `(type, id)` the document declares: one it does
	// not have, one whose state the saved configuration does not stand in, or one
	// named twice is a run the saved machine could not have had.
	for at, host := range saved.HostInvokes {
		owner, owned := policy.HostInvokeOwner(host.ProcessorType, host.InvokeID)
		if !owned {
			return nil, refuse("hostinvokes[%d] is '%s' of type '%s', which the document does not have a host "+
				"invoker run", at, host.InvokeID, host.ProcessorType)
		}
		if !slices.Contains(judged.configuration, owner) {
			return nil, refuse("hostinvokes[%d] is '%s', whose state the saved configuration does not stand in",
				at, host.InvokeID)
		}
		for _, earlier := range saved.HostInvokes[:at] {
			if earlier.ProcessorType == host.ProcessorType && earlier.InvokeID == host.InvokeID {
				return nil, refuse("hostinvokes[%d] is '%s', which an earlier entry already names", at, host.InvokeID)
			}
		}
	}
	return judged, nil
}

// eventTypeOfName is the event type a saved state's `type` names.
func eventTypeOfName(name string) (EventType, bool) {
	for _, candidate := range []EventType{EventTypeInternal, EventTypeExternal, EventTypePlatform} {
		if candidate.String() == name {
			return candidate, true
		}
	}
	return EventTypeInternal, false
}

// judgeChildEntry refuses child, the index-th running child of the format-2
// session parent, unless it is the child that session could be running
// (docs/adr/0045, section 3, step 1): its start is the count the session holds
// for its `<invoke>`, its session id is the session's and the id it was started
// under, and its own session is a state of the document it names
// (SavedPolicy.JudgeChild).
func judgeChildEntry[S comparable, E comparable](
	policy SavedPolicy[S, E],
	parent *SavedState,
	child SavedChild,
	index int,
) error {
	what := fmt.Sprintf("invokes[%d]", index)
	if count := parent.InvokeStart(child.ID); child.Start != count {
		return refuse("%s is start %d of '%s', and the session counts %d starts of it: a state re-entered cancels "+
			"the earlier child before it starts the next one", what, child.Start, child.ID, count)
	}
	derived := parent.SessionID + "." + child.InvokeID
	if child.Session.SessionID != derived {
		return refuse("%s.session's id is '%s', and a child of '%s' started under '%s' is '%s'",
			what, child.Session.SessionID, parent.SessionID, child.InvokeID, derived)
	}
	if err := policy.JudgeChild(child); err != nil {
		return RefusalWithin(err, what+".session")
	}
	return nil
}

// readPendingSend is send, the index-th of a saved state's `pending`, with its
// event looked up in the document of policy.
func readPendingSend[S comparable, E comparable](
	policy SavedPolicy[S, E],
	send SavedSend,
	index int,
) (readSend[E], error) {
	read := readSend[E]{due: send.Due, act: send.Act}
	switch send.Act.Kind {
	case SavedActRaise, SavedActInternal:
		event, named := policy.GetEventFromName(send.Act.Event)
		if !named {
			return read, refuse("pending[%d] is '%s', which the document does not name", index, send.Act.Event)
		}
		read.event = event
	// The child is named by the `<invoke>` the target names, which must be one of
	// the document's: a send to any other is one the save would have been refused
	// for.
	case SavedActChild:
		if _, owned := policy.InvokeOwner(send.Act.InvokeID); !owned {
			return read, refuse("pending[%d] is a send to the child '%s', which the document does not invoke",
				index, send.Act.InvokeID)
		}
	}
	return read, nil
}

// EnterSession is a machine of policy, whose variables already hold what saved
// holds, standing at the configuration saved names: the machine a host drives in
// place of one it would Initialize.
//
// No `<onentry>` runs and no `<data>` is evaluated (Engine.EnterAt): the run being
// resumed already did both, and the host calls it caused cannot be made twice. A
// configuration that is not one of this document is refused, and nothing is
// entered.
//
// The machine measures time by clock, which is installed before anything is
// armed: a delayed send armed against one clock and judged against another would
// never come due when it should. wallNowMs is what time it is on the wall clock
// the saved `due`s were written against: a waiting send is armed to come due
// `due - wallNowMs` after now, and one already due is armed to come due now,
// behind the ones due before it.
//
// A format-1 state's running children are started again from their beginning,
// last, so what a child sends as it starts stands behind what was already
// waiting. A format-2 state's are judged with the rest before anything is built,
// and resumed, each where it stood, once this session stands where it did
// (SavedPolicy.ResumeChildren).
func EnterSession[S comparable, E comparable](
	policy SavedPolicy[S, E],
	saved *SavedState,
	clock SceClock,
	wallNowMs int64,
) (*Engine[S, E], error) {
	engine := NewEngine[S, E](policy)
	judged, err := judgeSession(engine, policy, saved)
	if err != nil {
		return nil, err
	}
	// The session carries on under the id it was saved with; a format-1 state holds
	// none, and the machine keeps the one it was created with.
	if id, bound := saved.BoundSessionID(); bound {
		policy.BindSessionID(id)
	}
	engine.SetClock(clock)
	if verdict := engine.EnterAt(judged.configuration, judged.current); verdict != ConfigurationAccepted {
		return nil, refuse("the saved configuration is refused: %s", verdict)
	}
	for _, queued := range judged.external {
		engine.externalQueue.Raise(queued)
	}
	armPending(engine, judged.pending, wallNowMs)
	rearmHostInvokes(engine, saved.HostInvokes, wallNowMs)
	engine.nextHostInvokeToken = uint64(saved.HostInvokeToken)
	engine.autoSendSeq = uint64(saved.AutoSendSeq)
	if saved.Invokes.Resumed {
		if err := policy.ResumeChildren(engine, saved.Invokes.Children, wallNowMs); err != nil {
			return nil, err
		}
	} else {
		// Last, so what a child sends as it starts stands behind what was already
		// waiting: the saved machine's queue was ahead of it.
		policy.RestartInvokes(engine, judged.restarts)
	}
	return engine, nil
}

// rearmHostInvokes re-arms each of hosts on engine, in the order a saved state
// lists them. None starts here: the host registers its invokers on the engine
// this restore returns, so each starts at the top of the first macrostep it
// drives (Engine.startRestoredHostInvokes). A deadline comes due `due -
// wallNowMs` after now on the engine's own clock; one already due ends the
// invocation without a start.
func rearmHostInvokes[S comparable, E comparable](engine *Engine[S, E], hosts []SavedHostInvoke, wallNowMs int64) {
	now := engine.NowMs()
	for _, host := range hosts {
		kind, atMs := restoredNoDeadline, int64(0)
		if host.Due != nil {
			if *host.Due <= wallNowMs {
				kind = restoredDeadlinePassed
			} else {
				kind, atMs = restoredDeadlineAt, saturatingAddMs(now, *host.Due-wallNowMs)
			}
		}
		engine.rearmHostInvoke(HostInvokeRequest{
			ProcessorType: host.ProcessorType,
			InvokeID:      host.InvokeID,
			Src:           host.Src,
			Params:        cloneParams(host.Params),
			EventData:     host.Data,
			Content:       host.Content,
		}, kind, atMs)
	}
}

// armPending arms each of pending, already read, in the order a saved state
// lists them: each comes due `due - wallNowMs` after the engine's now, and one
// that came due while the machine was away comes due now. Entries due at the same
// moment are armed in order and the scheduler delivers those in the order they
// were armed, which is how the saved machine would have.
func armPending[S comparable, E comparable](engine *Engine[S, E], pending []readSend[E], wallNowMs int64) {
	now := engine.NowMs()
	readyAt := func(due int64) int64 {
		wait := due - wallNowMs
		if wait < 0 {
			wait = 0
		}
		return saturatingAddMs(now, wait)
	}
	for _, read := range pending {
		act := read.act
		switch act.Kind {
		case SavedActRaise:
			engine.scheduler.ScheduleEventAt(read.event, readyAt(read.due), act.SendID, act.Data, act.Origin)
		case SavedActInternal:
			engine.scheduler.ScheduleRoutedAt(read.event, readyAt(read.due), act.SendID, act.Data, act.Origin,
				ScheduledRoute{Kind: RouteInternalQueue})
		case SavedActHost:
			engine.scheduler.ScheduleHostSendAt(HostSendRequest{
				ProcessorType: act.ProcessorType,
				EventName:     act.Event,
				Target:        act.Target,
				Content:       act.Content,
				Params:        cloneParams(act.Params),
				SendID:        act.SendID,
				EventData:     act.Data,
				InvokeID:      act.InvokeID,
			}, readyAt(read.due), act.SendID)
		case SavedActParent:
			var none E
			engine.scheduler.ScheduleRoutedAt(none, readyAt(read.due), act.SendID, act.Data, act.Origin,
				ScheduledRoute{Kind: RouteParent, EventName: act.Event})
		case SavedActChild:
			var none E
			engine.scheduler.ScheduleRoutedAt(none, readyAt(read.due), act.SendID, act.Data, act.Origin,
				ScheduledRoute{Kind: RouteInvocation, InvokeID: act.InvokeID, EventName: act.Event})
		}
	}
}
