// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

import (
	"fmt"
	"sort"
	"strconv"
	"strings"
	"time"
)

// A `datamodel="sce-static"` machine's whole state, saved at a macrostep
// boundary and restored into a new process (SCE Accepted Subset §2.15, E17).
//
// What a saved state holds is everything a macrostep boundary holds that the
// document cannot recompute: where the machine is (its configuration and
// current leaf), every variable, what each `<history>` recorded, the delayed
// `<send>`s still waiting, the external queue in order, and, as a tree, every
// running child session with the same (docs/adr/0045).
//
// A delayed send is saved as the moment it comes due on the host's WALL clock,
// milliseconds since the Unix epoch, not as a wait: a wait would start again
// when the process came back, and a timer that ran out while it was dead would
// be late by exactly as long as it was dead.
//
// The format is one JSON document (`sce-saved-state`, version SavedFormat), the
// same on every backend, so what one backend saved another can read. A
// variable is keyed by its document id and written as its `sce:type` says: a
// number, except a 64-bit integer, written as a text so no reader that holds
// numbers as doubles loses its low bits; a byte string as its Latin-1 text; a
// record as an object of its fields; a list as an array.
//
// A saved state is bound to the SHAPE of the document it was saved from, a
// digest the generator computes of every state with its kind and parent and
// every variable with its type, not to the document's text.
//
// Cross-language siblings: `com.sce.runtime.SavedState` (Kotlin) and
// `sce_rust_runtime::saved_state` (Rust).

const (
	// SavedFormat1 is a state that names its running children by id alone.
	SavedFormat1 = 1
	// SavedFormat2 is a tree of sessions, each running child saved inside its
	// parent (docs/adr/0045).
	SavedFormat2 = 2
	// SavedFormat is the newest format version this runtime writes; it reads
	// this one and SavedFormat1.
	SavedFormat = SavedFormat2

	// SavedSchemaStatus is the stability status of the saved-state wire surface,
	// held in lockstep with `x-sce-schema-status` in
	// `schemas/sce-saved-state.v1.schema.json` and
	// `schemas/sce-saved-state.v2.schema.json` (SCE_WIRE_CONTRACTS.md). A flip to
	// "stable" changes all three in one commit.
	SavedSchemaStatus = "pre-release"
)

// sessionAt is what a refusal inside a child session of a tree begins with,
// before the session's path.
const sessionAt = "the session at "

// StateRefusal is why a saved state cannot be restored, or a machine cannot be
// saved.
type StateRefusal struct {
	reason string
}

func (r *StateRefusal) Error() string { return r.reason }

// refuse is a refusal with the reason fmt.Sprintf makes of format and args.
func refuse(format string, args ...any) error {
	return &StateRefusal{reason: fmt.Sprintf(format, args...)}
}

// Refuse is a refusal with the given reason, for generated code to return.
func Refuse(reason string) error {
	return &StateRefusal{reason: reason}
}

// RefusalWithin is err, a refusal of the child session at path of the session
// that reads it (`invokes[0].session`): a tree is refused as a whole, and the
// reason names the session it is about by its path from the root (docs/adr/0045,
// section 2), `invokes[0].session.invokes[1].session` for a grandchild.
func RefusalWithin(err error, path string) error {
	reason := err.Error()
	if strings.HasPrefix(reason, sessionAt) {
		return &StateRefusal{reason: sessionAt + path + "." + strings.TrimPrefix(reason, sessionAt)}
	}
	return &StateRefusal{reason: sessionAt + path + ": " + reason}
}

// SavedHistory is what one `<history>` recorded when its parent was last exited
// (§scxml-3.10): state ids in document order, under the history's id.
type SavedHistory struct {
	ID     string
	States []string
}

// SavedInvokeStart is how many times the `<invoke>` ID has started in a session.
type SavedInvokeStart struct {
	ID    string
	Count int64
}

// SavedInvokes is the running child sessions of a saved session, as the format
// it was written in holds them.
type SavedInvokes struct {
	// Resumed is format 2: each running child is saved with its whole state and a
	// restore resumes it where it stood. In format 1 the text holds the id of
	// each `<invoke>` whose child is running and nothing of the child, so a
	// restore starts each again from its beginning.
	Resumed bool
	// IDs is format 1's running invocations, by the id the document gives them.
	IDs []string
	// Children is format 2's running children, in document order.
	Children []SavedChild
}

// InvokeIDs is the id the document gives each running `<invoke>`, in order.
func (i SavedInvokes) InvokeIDs() []string {
	if !i.Resumed {
		return i.IDs
	}
	ids := make([]string, len(i.Children))
	for at, child := range i.Children {
		ids[at] = child.ID
	}
	return ids
}

// SavedChild is a running child session, saved inside the session that started
// it (docs/adr/0045, section 1).
type SavedChild struct {
	// ID is the id the document gives the `<invoke>` that started it.
	ID string
	// Start is which start of that `<invoke>` this is in its parent, counted
	// from 1: the count the parent's InvokeStarts holds for it.
	Start int64
	// InvokeID is the id the child was started under, as `_event.invokeid` and an
	// `idlocation` variable hold it: carried, not computed again.
	InvokeID string
	// Document is the document the child runs: the child's name for a static
	// `<invoke>`, the candidate's for a hybrid one.
	Document string
	// Session is the child's own state, its running children included.
	Session *SavedState
}

// SavedRevision is the revision of one record schema or enum a state was saved
// at.
type SavedRevision struct {
	// Key is the event the record schema declares, or `enum:` and the enum's
	// document, which names it in `revisions`.
	Key string
	// Revision is the `sce:revision` where the state was saved, 2 or later.
	Revision int
	// Fields is the SHA-256, as lowercase hex, of what that revision declares.
	Fields string
}

// SavedEvent is one event of a saved external queue: its name and the `_event`
// fields a document can read (§scxml-5.10.1). A typed payload is not saved; it
// is lifted again from Data when the event is delivered, the one path every
// other delivery takes.
type SavedEvent struct {
	Name       string
	Data       string
	Type       string
	SendID     string
	Origin     string
	OriginType string
	InvokeID   string
	// HostInvokeToken is the token a host-run invocation's completion or failure
	// was stamped with when it was accepted, nil for any other event. A
	// completion that was accepted and is still queued has to keep the stamp
	// across a save, or the restored machine would refuse the one answer it was
	// waiting for.
	HostInvokeToken *int64
}

// SavedHostInvoke is an `<invoke>` a declared host invoker is running, as it is
// started again (§scxml-6.4.1): every field is what the request the host was
// handed carried.
type SavedHostInvoke struct {
	ProcessorType string
	InvokeID      string
	Src           string
	Params        map[string][]string
	Data          string
	Content       string
	// Due is when its deadline comes due, in milliseconds since the Unix epoch on
	// the wall clock of the host that saved it; nil for an invocation that has
	// none.
	Due *int64
}

// SavedActKind is what a waiting `<send>` does when it comes due.
type SavedActKind string

const (
	// SavedActParent is an event for the session that started this one
	// (`#_parent`).
	SavedActParent SavedActKind = "parent"
	// SavedActChild is an event for a running child of this session
	// (`#_<invokeid>`).
	SavedActChild SavedActKind = "child"
	// SavedActRaise is an event for this session's external queue.
	SavedActRaise SavedActKind = "raise"
	// SavedActInternal is an event for this session's internal queue.
	SavedActInternal SavedActKind = "internal"
	// SavedActHost is a `<send>` a host-served processor performs (§scxml-6.2.5).
	SavedActHost SavedActKind = "host"
)

// SavedAct is what a waiting `<send>` does when it comes due. A delayed send to
// any other session is none of the five kinds, and a document that makes one is
// generated without the save API (§2.15).
type SavedAct struct {
	Kind SavedActKind
	// Event is the event's name, as the receiver resolves it.
	Event string
	// Data is `_event.data`, as the wire carries it.
	Data string
	// SendID is the `<send>`'s id, which a `<cancel>` names.
	SendID string
	// Origin is `_event.origin`.
	Origin string
	// InvokeID names the child a SavedActChild is for, and is
	// `_event.invokeid` of the event a SavedActHost was sent while processing.
	InvokeID string
	// ProcessorType, Target, Content and Params are a SavedActHost's: every field
	// is what the document wrote.
	ProcessorType string
	Target        string
	Content       string
	Params        map[string][]string
}

// SavedSend is one delayed `<send>` that has not been delivered yet.
type SavedSend struct {
	// Due is when it comes due, in milliseconds since the Unix epoch on the wall
	// clock of the host that saved it.
	Due int64
	Act SavedAct
}

// SavedState is a machine's whole state at a macrostep boundary.
type SavedState struct {
	// Shape is the shape of the document the machine was generated from.
	Shape string
	// Configuration is the active configuration, as state ids.
	Configuration []string
	// Current is the current leaf, as a state id: which of a `<parallel>`'s
	// regions the machine last stood in, which the configuration alone cannot
	// say.
	Current string
	// Variables is every variable by document id, in declaration order, each value
	// as the JSON value model holds it.
	Variables *JSONObject
	// History is what each `<history>` recorded, ordered by the history's id. A
	// history that has recorded nothing is absent.
	History []SavedHistory
	// Pending is the delayed `<send>`s still waiting, in the order they would be
	// delivered.
	Pending []SavedSend
	// Invokes is the `<invoke>`s whose child session is running (§scxml-6.4).
	Invokes SavedInvokes
	// HostInvokes is the `<invoke>`s a declared host invoker is running
	// (§scxml-6.4.1), by `(type, id)`.
	HostInvokes []SavedHostInvoke
	// HostInvokeToken is the token the next host-run start receives.
	HostInvokeToken int64
	// AutoSendSeq is how many ids the machine has generated for a
	// `<send idlocation>`.
	AutoSendSeq int64
	// External is the external queue, front first.
	External []SavedEvent
	// Revisions is the revision each record schema and enum past its first was
	// saved at, ordered by key. Empty when every one is in its first revision.
	Revisions []SavedRevision
	// SessionID is the session's own id (`_sessionid`). An XML Nmtoken in format
	// 2; empty in a format-1 state, which holds none.
	SessionID string
	// InvokeStarts is how many times each `<invoke>` of the document has started
	// in this session, for each started at least once, in document order. Empty
	// in a format-1 state.
	InvokeStarts []SavedInvokeStart
}

// Variable is the variable id, or a refusal naming it.
func (s *SavedState) Variable(id string) (any, error) {
	value, held := s.Variables.Get(id)
	if !held {
		return nil, refuse("the saved state has no variable '%s'", id)
	}
	return value, nil
}

// Format is the format this state is of: SavedFormat2 for a tree of sessions,
// SavedFormat1 for a state that names its running children by id alone.
func (s *SavedState) Format() int {
	if s.Invokes.Resumed {
		return SavedFormat2
	}
	return SavedFormat1
}

// BoundSessionID is the id a restore binds the session to: the saved one, for a
// format-2 state, since the session carries on and every child's id and a
// peer's address name it (docs/adr/0045); false for a format-1 state, which
// holds none, so the restored machine keeps the id it was created with.
func (s *SavedState) BoundSessionID() (string, bool) {
	if s.Format() == SavedFormat2 {
		return s.SessionID, true
	}
	return "", false
}

// InvokeStart is how many times the `<invoke>` id had started in this session
// when it was saved: 0 for one that had not, and for every one of a format-1
// state, which holds no count.
func (s *SavedState) InvokeStart(id string) int64 {
	for _, started := range s.InvokeStarts {
		if started.ID == id {
			return started.Count
		}
	}
	return 0
}

// ToJSON is the state as `sce-saved-state` JSON, in the format it is of: the
// root of the text, with its `format`.
func (s *SavedState) ToJSON() string {
	members := NewJSONObject()
	members.Set("format", JSONNumber(strconv.Itoa(s.Format())))
	s.writeSessionMembers(members)
	return WriteJSON(members)
}

// sessionMembers is the members of this session, in the order every backend
// writes them: format 1's, and in format 2 `sessionid` after `shape` and
// `invokestarts` after `invokes`, each running child's session written by the
// same rule.
func (s *SavedState) sessionMembers() *JSONObject {
	members := NewJSONObject()
	s.writeSessionMembers(members)
	return members
}

func (s *SavedState) writeSessionMembers(members *JSONObject) {
	members.Set("shape", s.Shape)
	if s.Invokes.Resumed {
		members.Set("sessionid", s.SessionID)
	}
	members.Set("configuration", stringsToJSON(s.Configuration))
	members.Set("current", s.Current)
	members.Set("variables", s.Variables)
	history := NewJSONObject()
	for _, recorded := range s.History {
		history.Set(recorded.ID, stringsToJSON(recorded.States))
	}
	members.Set("history", history)
	pending := make([]any, len(s.Pending))
	for at, send := range s.Pending {
		pending[at] = send.toJSONValue()
	}
	members.Set("pending", pending)
	if s.Invokes.Resumed {
		children := make([]any, len(s.Invokes.Children))
		for at, child := range s.Invokes.Children {
			children[at] = child.toJSONValue()
		}
		members.Set("invokes", children)
		starts := NewJSONObject()
		for _, started := range s.InvokeStarts {
			starts.Set(started.ID, strconv.FormatInt(started.Count, 10))
		}
		members.Set("invokestarts", starts)
	} else {
		members.Set("invokes", stringsToJSON(s.Invokes.IDs))
	}
	hosts := make([]any, len(s.HostInvokes))
	for at, host := range s.HostInvokes {
		hosts[at] = host.toJSONValue()
	}
	members.Set("hostinvokes", hosts)
	members.Set("hostinvoketoken", strconv.FormatInt(s.HostInvokeToken, 10))
	members.Set("sendseq", strconv.FormatInt(s.AutoSendSeq, 10))
	external := make([]any, len(s.External))
	for at, event := range s.External {
		external[at] = event.toJSONValue()
	}
	members.Set("external", external)
	// Absent, not empty, when no record schema is past its first revision: the
	// text of such a state is the text it was before revisions existed.
	if len(s.Revisions) > 0 {
		revisions := NewJSONObject()
		for _, saved := range s.Revisions {
			revisions.Set(saved.Key, saved.toJSONValue())
		}
		members.Set("revisions", revisions)
	}
}

func stringsToJSON(texts []string) []any {
	out := make([]any, len(texts))
	for at, text := range texts {
		out[at] = text
	}
	return out
}

func (c SavedChild) toJSONValue() *JSONObject {
	members := NewJSONObject()
	members.Set("id", c.ID)
	members.Set("start", strconv.FormatInt(c.Start, 10))
	members.Set("invokeid", c.InvokeID)
	members.Set("document", c.Document)
	members.Set("session", c.Session.sessionMembers())
	return members
}

func (r SavedRevision) toJSONValue() *JSONObject {
	members := NewJSONObject()
	members.Set("revision", JSONNumber(strconv.Itoa(r.Revision)))
	members.Set("fields", r.Fields)
	return members
}

func (e SavedEvent) toJSONValue() *JSONObject {
	members := NewJSONObject()
	members.Set("name", e.Name)
	members.Set("data", e.Data)
	members.Set("type", e.Type)
	members.Set("sendid", e.SendID)
	members.Set("origin", e.Origin)
	members.Set("origintype", e.OriginType)
	members.Set("invokeid", e.InvokeID)
	members.Set("hostinvoketoken", optionalCount(e.HostInvokeToken))
	return members
}

func (h SavedHostInvoke) toJSONValue() *JSONObject {
	members := NewJSONObject()
	members.Set("type", h.ProcessorType)
	members.Set("id", h.InvokeID)
	members.Set("src", h.Src)
	members.Set("params", paramsToJSON(h.Params))
	members.Set("data", h.Data)
	members.Set("content", h.Content)
	members.Set("due", optionalCount(h.Due))
	return members
}

// optionalCount is a count as the text a saved state writes it as, or null.
func optionalCount(count *int64) any {
	if count == nil {
		return nil
	}
	return strconv.FormatInt(*count, 10)
}

// paramsToJSON is params as a saved state writes them: by name, so the text one
// machine writes is every backend's.
func paramsToJSON(params map[string][]string) *JSONObject {
	names := make([]string, 0, len(params))
	for name := range params {
		names = append(names, name)
	}
	sort.Strings(names)
	out := NewJSONObject()
	for _, name := range names {
		out.Set(name, stringsToJSON(params[name]))
	}
	return out
}

func (s SavedSend) toJSONValue() *JSONObject {
	members := NewJSONObject()
	members.Set("due", strconv.FormatInt(s.Due, 10))
	act := s.Act
	members.Set("act", string(act.Kind))
	switch act.Kind {
	case SavedActParent, SavedActRaise, SavedActInternal:
		members.Set("event", act.Event)
		members.Set("data", act.Data)
		members.Set("sendid", act.SendID)
		members.Set("origin", act.Origin)
	case SavedActChild:
		members.Set("invokeid", act.InvokeID)
		members.Set("event", act.Event)
		members.Set("data", act.Data)
		members.Set("sendid", act.SendID)
		members.Set("origin", act.Origin)
	case SavedActHost:
		members.Set("type", act.ProcessorType)
		members.Set("event", act.Event)
		members.Set("target", act.Target)
		members.Set("content", act.Content)
		members.Set("params", paramsToJSON(act.Params))
		members.Set("sendid", act.SendID)
		members.Set("data", act.Data)
		members.Set("invokeid", act.InvokeID)
	default:
		panic("a saved act of a kind no format names")
	}
	return members
}

// SavedStateFromJSON reads `sce-saved-state` JSON, of format 1 or 2. The shape
// is not judged here; the machine that restores it knows its own.
func SavedStateFromJSON(text string) (*SavedState, error) {
	value, err := ParseJSON(text)
	if err != nil {
		return nil, refuse("the saved state is %s", err.Error())
	}
	members, ok := value.(*JSONObject)
	if !ok {
		return nil, refuse("the saved state is not an object")
	}
	written, held := members.Get("format")
	if !held {
		return nil, refuse("the saved state has no 'format'")
	}
	var format int
	switch written {
	case JSONNumber("1"):
		format = SavedFormat1
	case JSONNumber("2"):
		format = SavedFormat2
	default:
		return nil, refuse("the saved state is format %s, and this runtime reads formats %d and %d",
			WriteJSON(written), SavedFormat1, SavedFormat2)
	}
	return savedSessionFromJSON(members, format)
}

// savedSessionFromJSON is the session members hold, of a text of format: the
// root, or a child's `session`, which states no `format` of its own.
func savedSessionFromJSON(members *JSONObject, format int) (*SavedState, error) {
	field := func(name string) (any, error) {
		value, held := members.Get(name)
		if !held {
			return nil, refuse("the saved state has no '%s'", name)
		}
		return value, nil
	}
	text := func(value any, what string) (string, error) {
		written, ok := value.(string)
		if !ok {
			return "", refuse("'%s' is not a text", what)
		}
		return written, nil
	}
	fieldText := func(name string) (string, error) {
		value, err := field(name)
		if err != nil {
			return "", err
		}
		return text(value, name)
	}
	fieldArray := func(name string) ([]any, error) {
		value, err := field(name)
		if err != nil {
			return nil, err
		}
		items, ok := value.([]any)
		if !ok {
			return nil, refuse("'%s' is not an array", name)
		}
		return items, nil
	}

	state := &SavedState{}

	configured, err := fieldArray("configuration")
	if err != nil {
		return nil, err
	}
	for _, item := range configured {
		id, err := text(item, "configuration")
		if err != nil {
			return nil, err
		}
		state.Configuration = append(state.Configuration, id)
	}
	if state.Configuration == nil {
		state.Configuration = []string{}
	}

	variables, err := field("variables")
	if err != nil {
		return nil, err
	}
	variableMembers, ok := variables.(*JSONObject)
	if !ok {
		return nil, refuse("'variables' is not an object")
	}
	state.Variables = variableMembers

	recorded, err := field("history")
	if err != nil {
		return nil, err
	}
	historyMembers, ok := recorded.(*JSONObject)
	if !ok {
		return nil, refuse("'history' is not an object")
	}
	for _, id := range historyMembers.Keys() {
		value, _ := historyMembers.Get(id)
		listed, ok := value.([]any)
		if !ok {
			return nil, refuse("'history.%s' is not an array", id)
		}
		states := []string{}
		for _, item := range listed {
			name, err := text(item, "history."+id)
			if err != nil {
				return nil, err
			}
			states = append(states, name)
		}
		state.History = append(state.History, SavedHistory{ID: id, States: states})
	}

	pending, err := fieldArray("pending")
	if err != nil {
		return nil, err
	}
	for at, item := range pending {
		send, err := savedSendFromJSON(item, fmt.Sprintf("pending[%d]", at), format)
		if err != nil {
			return nil, err
		}
		state.Pending = append(state.Pending, send)
	}

	listed, err := fieldArray("invokes")
	if err != nil {
		return nil, err
	}
	if format == SavedFormat1 {
		state.Invokes = SavedInvokes{IDs: []string{}}
		for _, item := range listed {
			id, err := text(item, "invokes")
			if err != nil {
				return nil, err
			}
			state.Invokes.IDs = append(state.Invokes.IDs, id)
		}
	} else {
		state.Invokes = SavedInvokes{Resumed: true}
		for at, item := range listed {
			child, err := savedChildFromJSON(item, at)
			if err != nil {
				return nil, err
			}
			state.Invokes.Children = append(state.Invokes.Children, child)
		}
	}

	// Format 2's own members: the session's id and its count of starts.
	if format == SavedFormat2 {
		sessionID, err := fieldText("sessionid")
		if err != nil {
			return nil, err
		}
		if SessionIDRefusalOf(sessionID) != "" {
			return nil, refuse("'sessionid' (%s) is not an XML Nmtoken, which a session id is", sessionID)
		}
		state.SessionID = sessionID
		counted, err := field("invokestarts")
		if err != nil {
			return nil, err
		}
		countedMembers, ok := counted.(*JSONObject)
		if !ok {
			return nil, refuse("'invokestarts' is not an object")
		}
		for _, id := range countedMembers.Keys() {
			count, _ := countedMembers.Get(id)
			written, ok := count.(string)
			if !ok {
				return nil, refuse("'invokestarts.%s' is not a text", id)
			}
			n, err := readSavedCount(written, "invokestarts."+id)
			if err != nil {
				return nil, err
			}
			state.InvokeStarts = append(state.InvokeStarts, SavedInvokeStart{ID: id, Count: n})
		}
	}

	hosts, err := fieldArray("hostinvokes")
	if err != nil {
		return nil, err
	}
	for at, item := range hosts {
		host, err := savedHostInvokeFromJSON(item, fmt.Sprintf("hostinvokes[%d]", at))
		if err != nil {
			return nil, err
		}
		state.HostInvokes = append(state.HostInvokes, host)
	}

	for _, counter := range []struct {
		name string
		into *int64
	}{{"hostinvoketoken", &state.HostInvokeToken}, {"sendseq", &state.AutoSendSeq}} {
		written, err := field(counter.name)
		if err != nil {
			return nil, err
		}
		digits, ok := written.(string)
		if !ok {
			return nil, refuse("'%s' is not a text", counter.name)
		}
		n, err := readSavedToken(digits, counter.name)
		if err != nil {
			return nil, err
		}
		*counter.into = n
	}

	external, err := fieldArray("external")
	if err != nil {
		return nil, err
	}
	for at, item := range external {
		event, err := savedEventFromJSON(item, fmt.Sprintf("external[%d]", at))
		if err != nil {
			return nil, err
		}
		state.External = append(state.External, event)
	}

	// Optional, unlike every member above: a state whose record schemas are all
	// in their first revision leaves it out, as every state saved before schemas
	// had revisions does.
	if members.Has("revisions") {
		value, _ := members.Get("revisions")
		named, ok := value.(*JSONObject)
		if !ok {
			return nil, refuse("'revisions' is not an object")
		}
		if named.Len() == 0 {
			return nil, refuse("'revisions' names no schema; a state whose record schemas are all in their first " +
				"revision leaves it out")
		}
		for _, key := range named.Keys() {
			value, _ := named.Get(key)
			revision, err := savedRevisionFromJSON(key, value, "revisions."+key)
			if err != nil {
				return nil, err
			}
			state.Revisions = append(state.Revisions, revision)
		}
	}

	if state.Shape, err = fieldText("shape"); err != nil {
		return nil, err
	}
	if state.Current, err = fieldText("current"); err != nil {
		return nil, err
	}
	return state, nil
}

// savedMemberText is the text member key of members, which what names in a
// refusal.
func savedMemberText(members *JSONObject, what, key string) (string, error) {
	value, held := members.Get(key)
	if !held {
		return "", refuse("'%s' has no '%s'", what, key)
	}
	text, ok := value.(string)
	if !ok {
		return "", refuse("'%s.%s' is not a text", what, key)
	}
	return text, nil
}

// savedMemberObject is value as an object, which what names in a refusal.
func savedMemberObject(value any, what string) (*JSONObject, error) {
	members, ok := value.(*JSONObject)
	if !ok {
		return nil, refuse("'%s' is not an object", what)
	}
	return members, nil
}

func savedChildFromJSON(value any, index int) (SavedChild, error) {
	what := fmt.Sprintf("invokes[%d]", index)
	members, err := savedMemberObject(value, what)
	if err != nil {
		return SavedChild{}, err
	}
	nonEmpty := func(key string) (string, error) {
		written, err := savedMemberText(members, what, key)
		if err != nil {
			return "", err
		}
		if written == "" {
			return "", refuse("'%s.%s' is empty", what, key)
		}
		return written, nil
	}
	startText, err := nonEmpty("start")
	if err != nil {
		return SavedChild{}, err
	}
	start, err := readSavedCount(startText, what+".start")
	if err != nil {
		return SavedChild{}, err
	}
	sessionValue, held := members.Get("session")
	if !held {
		return SavedChild{}, refuse("'%s' has no 'session'", what)
	}
	sessionMembers, ok := sessionValue.(*JSONObject)
	if !ok {
		return SavedChild{}, refuse("'%s.session' is not an object", what)
	}
	if sessionMembers.Has("format") {
		return SavedChild{}, refuse("'%s.session' states a format, which only the root of a tree does", what)
	}
	session, err := savedSessionFromJSON(sessionMembers, SavedFormat2)
	if err != nil {
		return SavedChild{}, RefusalWithin(err, what+".session")
	}
	id, err := nonEmpty("id")
	if err != nil {
		return SavedChild{}, err
	}
	invokeID, err := nonEmpty("invokeid")
	if err != nil {
		return SavedChild{}, err
	}
	document, err := nonEmpty("document")
	if err != nil {
		return SavedChild{}, err
	}
	return SavedChild{ID: id, Start: start, InvokeID: invokeID, Document: document, Session: session}, nil
}

// readSavedCount is a count of starts, as the text a saved state writes it as: a
// whole number from 1 with no leading zero, within a signed 64-bit count, as
// every backend reads one.
func readSavedCount(written, what string) (int64, error) {
	n, ok := parseSavedDigits(written)
	if !ok || n < 1 || strings.HasPrefix(written, "0") {
		return 0, refuse("'%s' (%s) is not a count of starts, a whole number from 1", what, written)
	}
	return n, nil
}

// readSavedToken is a host-run invocation's token, as the text a saved state
// writes it as: digits, within a signed 64-bit count, as every backend reads
// one.
func readSavedToken(written, what string) (int64, error) {
	n, ok := parseSavedDigits(written)
	if !ok {
		return 0, refuse("'%s' (%s) is not a whole number a token can be", what, written)
	}
	return n, nil
}

// readSavedMoment is a moment, as the text a saved state writes it as,
// milliseconds since the Unix epoch, which what (the member, named in a
// refusal) holds: a text of digits, as every 64-bit integer is, and one the
// other backends hold too.
func readSavedMoment(written, what string) (int64, error) {
	n, ok := parseSavedDigits(written)
	if !ok {
		return 0, refuse("'%s' (%s) is not a whole number of milliseconds", what, written)
	}
	return n, nil
}

// parseSavedDigits is written as a signed 64-bit whole number, when it is
// nothing but ASCII digits.
func parseSavedDigits(written string) (int64, bool) {
	if written == "" {
		return 0, false
	}
	for at := 0; at < len(written); at++ {
		if written[at] < '0' || written[at] > '9' {
			return 0, false
		}
	}
	n, err := strconv.ParseInt(written, 10, 64)
	if err != nil {
		return 0, false
	}
	return n, true
}

func savedRevisionFromJSON(key string, value any, what string) (SavedRevision, error) {
	members, err := savedMemberObject(value, what)
	if err != nil {
		return SavedRevision{}, err
	}
	if !members.Has("revision") {
		return SavedRevision{}, refuse("'%s' has no 'revision'", what)
	}
	if !members.Has("fields") {
		return SavedRevision{}, refuse("'%s' has no 'fields'", what)
	}
	revisionValue, _ := members.Get("revision")
	number, ok := revisionValue.(JSONNumber)
	if !ok {
		return SavedRevision{}, refuse("'%s.revision' is not a number", what)
	}
	written := string(number)
	revision, err := strconv.Atoi(written)
	if err != nil || revision < 2 || !isDigits(written) {
		return SavedRevision{}, refuse("'%s.revision' (%s) is not a revision past the first; a schema in its first "+
			"revision is left out", what, written)
	}
	fieldsValue, _ := members.Get("fields")
	fields, ok := fieldsValue.(string)
	if !ok || !isLowerHexDigest(fields) {
		return SavedRevision{}, refuse("'%s.fields' is not a SHA-256 written as lowercase hex", what)
	}
	return SavedRevision{Key: key, Revision: revision, Fields: fields}, nil
}

func isDigits(text string) bool {
	for at := 0; at < len(text); at++ {
		if text[at] < '0' || text[at] > '9' {
			return false
		}
	}
	return text != ""
}

func isLowerHexDigest(text string) bool {
	if len(text) != 64 {
		return false
	}
	for at := 0; at < len(text); at++ {
		c := text[at]
		if (c < '0' || c > '9') && (c < 'a' || c > 'f') {
			return false
		}
	}
	return true
}

func savedEventFromJSON(value any, what string) (SavedEvent, error) {
	members, err := savedMemberObject(value, what)
	if err != nil {
		return SavedEvent{}, err
	}
	var event SavedEvent
	// The token is judged first, as every backend judges it.
	if event.HostInvokeToken, err = savedOptionalMember(members, what, "hostinvoketoken", readSavedToken); err != nil {
		return SavedEvent{}, err
	}
	for _, member := range []struct {
		key  string
		into *string
	}{
		{"name", &event.Name}, {"data", &event.Data}, {"type", &event.Type}, {"sendid", &event.SendID},
		{"origin", &event.Origin}, {"origintype", &event.OriginType}, {"invokeid", &event.InvokeID},
	} {
		if *member.into, err = savedMemberText(members, what, member.key); err != nil {
			return SavedEvent{}, err
		}
	}
	return event, nil
}

// savedOptionalMember is the member key of members, which is null or a text read
// by read.
func savedOptionalMember(
	members *JSONObject, what, key string, read func(string, string) (int64, error),
) (*int64, error) {
	value, held := members.Get(key)
	if !held {
		return nil, refuse("'%s' has no '%s'", what, key)
	}
	switch written := value.(type) {
	case nil:
		return nil, nil
	case string:
		n, err := read(written, what+"."+key)
		if err != nil {
			return nil, err
		}
		return &n, nil
	default:
		return nil, refuse("'%s.%s' is neither a text nor null", what, key)
	}
}

// savedParamsFromJSON is the member `params` of members, read as an object of
// arrays of texts, by name; what names members in a refusal.
func savedParamsFromJSON(members *JSONObject, what string) (map[string][]string, error) {
	value, held := members.Get("params")
	if !held {
		return nil, refuse("'%s' has no 'params'", what)
	}
	params, ok := value.(*JSONObject)
	if !ok {
		return nil, refuse("'%s.params' is not an object", what)
	}
	read := map[string][]string{}
	for _, name := range params.Keys() {
		values, _ := params.Get(name)
		list, ok := values.([]any)
		if !ok {
			return nil, refuse("'%s.params.%s' is not an array", what, name)
		}
		texts := make([]string, 0, len(list))
		for _, item := range list {
			text, ok := item.(string)
			if !ok {
				return nil, refuse("'%s.params.%s' holds a value that is not a text", what, name)
			}
			texts = append(texts, text)
		}
		read[name] = texts
	}
	return read, nil
}

func savedHostInvokeFromJSON(value any, what string) (SavedHostInvoke, error) {
	members, err := savedMemberObject(value, what)
	if err != nil {
		return SavedHostInvoke{}, err
	}
	var host SavedHostInvoke
	text := func(key string, into *string) error {
		written, err := savedMemberText(members, what, key)
		if err != nil {
			return err
		}
		*into = written
		return nil
	}
	if host.Due, err = savedOptionalMember(members, what, "due", readSavedMoment); err != nil {
		return SavedHostInvoke{}, err
	}
	if err = text("type", &host.ProcessorType); err != nil {
		return SavedHostInvoke{}, err
	}
	if err = text("id", &host.InvokeID); err != nil {
		return SavedHostInvoke{}, err
	}
	if err = text("src", &host.Src); err != nil {
		return SavedHostInvoke{}, err
	}
	if host.Params, err = savedParamsFromJSON(members, what); err != nil {
		return SavedHostInvoke{}, err
	}
	if err = text("data", &host.Data); err != nil {
		return SavedHostInvoke{}, err
	}
	if err = text("content", &host.Content); err != nil {
		return SavedHostInvoke{}, err
	}
	return host, nil
}

// savedSendFromJSON is the waiting send value, of a state of format; a send to
// the parent or to a child is an act of format 2 only.
func savedSendFromJSON(value any, what string, format int) (SavedSend, error) {
	members, err := savedMemberObject(value, what)
	if err != nil {
		return SavedSend{}, err
	}
	text := func(key string) (string, error) { return savedMemberText(members, what, key) }
	dueText, err := text("due")
	if err != nil {
		return SavedSend{}, err
	}
	due, err := readSavedMoment(dueText, what+".due")
	if err != nil {
		return SavedSend{}, err
	}
	name, err := text("act")
	if err != nil {
		return SavedSend{}, err
	}
	act := SavedAct{Kind: SavedActKind(name)}
	// The members each act reads, in the order every backend reads them.
	var into []*string
	var keys []string
	switch act.Kind {
	case SavedActParent, SavedActChild:
		if format != SavedFormat2 {
			return SavedSend{}, refuse("'%s.act' is '%s', which is not raise, internal or host, the acts of format 1",
				what, name)
		}
		if act.Kind == SavedActChild {
			keys, into = []string{"invokeid"}, []*string{&act.InvokeID}
		}
		keys = append(keys, "event", "data", "sendid", "origin")
		into = append(into, &act.Event, &act.Data, &act.SendID, &act.Origin)
	case SavedActRaise, SavedActInternal:
		keys = []string{"event", "data", "sendid", "origin"}
		into = []*string{&act.Event, &act.Data, &act.SendID, &act.Origin}
	case SavedActHost:
		keys = []string{"type", "event", "target", "content"}
		into = []*string{&act.ProcessorType, &act.Event, &act.Target, &act.Content}
	default:
		if format == SavedFormat1 {
			return SavedSend{}, refuse("'%s.act' is '%s', which is not raise, internal or host, the acts of format 1",
				what, name)
		}
		return SavedSend{}, refuse("'%s.act' is '%s', which is not raise, internal, host, parent or child", what, name)
	}
	for at, key := range keys {
		if *into[at], err = text(key); err != nil {
			return SavedSend{}, err
		}
	}
	if act.Kind == SavedActHost {
		if act.Params, err = savedParamsFromJSON(members, what); err != nil {
			return SavedSend{}, err
		}
		if act.SendID, err = text("sendid"); err != nil {
			return SavedSend{}, err
		}
		if act.Data, err = text("data"); err != nil {
			return SavedSend{}, err
		}
		if act.InvokeID, err = text("invokeid"); err != nil {
			return SavedSend{}, err
		}
	}
	return SavedSend{Due: due, Act: act}, nil
}

// SavedWallClockMs is the wall clock now, in milliseconds since the Unix epoch:
// what a host that has no clock of its own to give Save and Restore gives them.
func SavedWallClockMs() int64 {
	return time.Now().UnixMilli()
}

// SavedShape is what a machine's saved state is bound to: the document's shape,
// and the revisions of each schema a record variable of it is of (docs/adr/0044).
// The generator computes it, and every saved state is written and judged against
// it.
type SavedShape struct {
	// First is the shape a state names when every record schema it holds is in
	// its first revision.
	First string
	// Revised is the shape a state names when it names a later revision of any
	// record schema. A reader that predates revisions knows only the first shape,
	// so it refuses such a state rather than read it without the fields it does
	// not know.
	Revised string
	// Format2First is First as a format-2 state names it (docs/adr/0045, section
	// 2): each enum-typed variable and record field spelled by its enum's document
	// rather than by the alias it is imported under.
	Format2First string
	// Format2Revised is Revised as a format-2 state names it.
	Format2Revised string
	// Records is each record schema a variable is of, by the event it declares,
	// ordered by it.
	Records []RecordLineage
	// Enums is each enum a value of the state is of, by its key, ordered by it.
	Enums []EnumLineage
}

// EnumLineage is an enum a value of a machine's state is of, as a saved state
// names it (docs/adr/0044).
type EnumLineage struct {
	// Key is `enum:` and the enum's document, which names it in `revisions`.
	Key string
	// Revision is the enum's revision in this machine.
	Revision int
	// Revisions is the digest of what each revision from the second to Revision
	// declares, in order.
	Revisions []string
	// Added is each variant a revision past the first added.
	Added []AddedVariant
	// Holders is where a value of it is held.
	Holders []EnumHolder
}

// AddedVariant is a variant a revision past the first added to an enum.
type AddedVariant struct {
	Name  string
	Since int
}

// EnumHolder is where a value of an EnumLineage is held.
type EnumHolder struct {
	// Variable is the variable's document id.
	Variable string
	// Field is the field of the record the variable holds, for a value held in a
	// record; nil for a variable of the enum.
	Field *string
	// List is whether the variable holds a list of such records rather than one.
	List bool
}

// RecordLineage is a record schema a machine's variables are of, as a saved
// state names it.
type RecordLineage struct {
	// Event is the event the schema declares, which names it in `revisions`.
	Event string
	// Revision is the schema's revision in this machine.
	Revision int
	// Revisions is the digest of what each revision from the second to Revision
	// declares, in order.
	Revisions []string
	// Added is each field a revision past the first added, in the schema's order.
	Added []AddedField
	// Holders is the variables that hold a record of it, or a list of them.
	Holders []RecordHolder
}

// AddedField is a field a revision past the first added to a record schema.
type AddedField struct {
	// ID is the field's id, the member a saved record holds it under.
	ID    string
	Since int
	// Default is its `sce:default` as saved-state JSON.
	Default string
}

// RecordHolder is a variable that holds a record of a RecordLineage.
type RecordHolder struct {
	Variable string
	List     bool
}

// CheckSavedShape refuses saved unless it was saved from a document of this
// shape, judged before any variable is read, so a state saved from another
// document is refused for that and not for whichever variable it happens to
// lack.
func CheckSavedShape(saved *SavedState, shape string) error {
	if saved.Shape != shape {
		return refuse("the saved state is of a document of shape %s, and this machine's is %s", saved.Shape, shape)
	}
	return nil
}

// UpgradeSavedState is saved as a state of the machine whose saved state is
// bound to shape, or the refusal that says why it is not one (docs/adr/0044),
// judged before any variable is read, as CheckSavedShape is.
//
// A state that names no revision is of the first shape; one that does is of the
// revised shape, and each revision it names is of a schema a record variable of
// this machine is of, no later than this machine's revision of it, and declares
// what this machine's revision of that number declares. Such a state is then read
// as one of this machine: each record of a schema saved at an earlier revision
// takes the default of every field a later revision added. A record that already
// holds such a field is refused, since the revision it was saved at does not
// declare it.
func UpgradeSavedState(saved *SavedState, shape *SavedShape) (*SavedState, error) {
	// A format-2 state names an enum by its document and a format-1 state by the
	// alias it is imported under, so each is judged against its own shape
	// (docs/adr/0045, section 2).
	first, revised := shape.First, shape.Revised
	if saved.Format() == SavedFormat2 {
		first, revised = shape.Format2First, shape.Format2Revised
	}
	switch {
	case len(saved.Revisions) == 0:
		if err := CheckSavedShape(saved, first); err != nil {
			return nil, err
		}
	case saved.Shape == first:
		return nil, refuse("the saved state names revisions of its record schemas and has the shape of a state " +
			"that names none")
	default:
		if err := CheckSavedShape(saved, revised); err != nil {
			return nil, err
		}
	}
	for _, named := range saved.Revisions {
		// A record schema by the event it declares, an enum by its key.
		var revision int
		var digests []string
		found := false
		for _, record := range shape.Records {
			if record.Event == named.Key {
				revision, digests, found = record.Revision, record.Revisions, true
				break
			}
		}
		if !found {
			for _, lineage := range shape.Enums {
				if lineage.Key == named.Key {
					revision, digests, found = lineage.Revision, lineage.Revisions, true
					break
				}
			}
		}
		if !found {
			return nil, refuse("'revisions' names '%s', which no record or enum of this machine is", named.Key)
		}
		if named.Revision > revision {
			return nil, refuse("the saved state was saved at revision %d of '%s', later than this machine's %d: a "+
				"machine does not restore what it does not know", named.Revision, named.Key, revision)
		}
		// The digest of revision r is the (r - 2)th: the first has none.
		at := named.Revision - 2
		if at < 0 || at >= len(digests) || digests[at] != named.Fields {
			return nil, refuse("revision %d of '%s' declared other members where the state was saved than it does in "+
				"this machine", named.Revision, named.Key)
		}
	}
	revisionOf := func(key string) int {
		for _, named := range saved.Revisions {
			if named.Key == key {
				return named.Revision
			}
		}
		return 1
	}
	// An enum value saved at an earlier revision of its enum is one of that
	// revision's variants. Judged on the values as saved, before a record is
	// grown by defaults, which are this machine's.
	for _, lineage := range shape.Enums {
		at := revisionOf(lineage.Key)
		var later []AddedVariant
		for _, variant := range lineage.Added {
			if variant.Since > at {
				later = append(later, variant)
			}
		}
		if len(later) == 0 {
			continue
		}
		for _, holder := range lineage.Holders {
			if err := checkEnumHolder(saved, lineage, holder, at, later); err != nil {
				return nil, err
			}
		}
	}
	variables := NewJSONObject()
	for _, id := range saved.Variables.Keys() {
		value, _ := saved.Variables.Get(id)
		variables.Set(id, value)
	}
	for _, record := range shape.Records {
		at := revisionOf(record.Event)
		var added []AddedField
		for _, field := range record.Added {
			if field.Since > at {
				added = append(added, field)
			}
		}
		if len(added) == 0 {
			continue
		}
		for _, holder := range record.Holders {
			value, held := variables.Get(holder.Variable)
			if !held {
				continue
			}
			if !holder.List {
				grown, err := grownRecord(value, holder.Variable, record, at, added)
				if err != nil {
					return nil, err
				}
				variables.Set(holder.Variable, grown)
				continue
			}
			items, isList := value.([]any)
			if !isList {
				continue
			}
			grownItems := make([]any, len(items))
			for index, item := range items {
				grown, err := grownRecord(item, fmt.Sprintf("%s[%d]", holder.Variable, index), record, at, added)
				if err != nil {
					return nil, err
				}
				grownItems[index] = grown
			}
			variables.Set(holder.Variable, grownItems)
		}
	}
	upgraded := *saved
	upgraded.Variables = variables
	return &upgraded, nil
}

// checkEnumHolder refuses a value holder holds, in saved, that names one of
// later, the variants of lineage added after at, the revision it was saved at.
// What is not a text where one is held is left for the variable's own read to
// refuse.
func checkEnumHolder(
	saved *SavedState, lineage EnumLineage, holder EnumHolder, at int, later []AddedVariant,
) error {
	value, held := saved.Variables.Get(holder.Variable)
	if !held {
		return nil
	}
	// Each value the holder holds, with what names it in a refusal.
	type heldValue struct {
		what  string
		value any
	}
	var values []heldValue
	switch {
	case holder.Field == nil:
		values = append(values, heldValue{holder.Variable, value})
	case !holder.List:
		if record, ok := value.(*JSONObject); ok && record.Has(*holder.Field) {
			member, _ := record.Get(*holder.Field)
			values = append(values, heldValue{holder.Variable + "." + *holder.Field, member})
		}
	default:
		items, _ := value.([]any)
		for index, item := range items {
			if record, ok := item.(*JSONObject); ok && record.Has(*holder.Field) {
				member, _ := record.Get(*holder.Field)
				values = append(values, heldValue{fmt.Sprintf("%s[%d].%s", holder.Variable, index, *holder.Field), member})
			}
		}
	}
	for _, held := range values {
		name, isText := held.value.(string)
		if !isText {
			continue
		}
		for _, variant := range later {
			if variant.Name == name {
				return refuse("'%s' holds '%s', which revision %d of '%s' does not declare: revision %d added it",
					held.what, name, at, lineage.Key, variant.Since)
			}
		}
	}
	return nil
}

// grownRecord is the saved record value, of record saved at revision at, with
// the default of each of added, the fields later revisions added; what names it
// in a refusal. A value that is not an object is returned as it is, for the
// record's own read to refuse.
func grownRecord(value any, what string, record RecordLineage, at int, added []AddedField) (any, error) {
	members, ok := value.(*JSONObject)
	if !ok {
		return value, nil
	}
	grown := NewJSONObject()
	for _, name := range members.Keys() {
		member, _ := members.Get(name)
		grown.Set(name, member)
	}
	for _, field := range added {
		if grown.Has(field.ID) {
			return nil, refuse("'%s' holds '%s', which revision %d of '%s' does not declare: revision %d added it",
				what, field.ID, at, record.Event, field.Since)
		}
		fallback, err := ParseJSON(field.Default)
		if err != nil {
			return nil, refuse("the default of '%s' of '%s' is %s", field.ID, record.Event, err.Error())
		}
		grown.Set(field.ID, fallback)
	}
	return grown, nil
}
