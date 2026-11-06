// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE::Saved::SavedState — a `datamodel="sce-static"` machine's whole state, saved at a
// macrostep boundary and restored into a new process (SCE Accepted Subset §2.15, E17).
//
// What a saved state holds is everything a macrostep boundary holds that the document
// cannot recompute: where the machine is (its configuration and current leaf), every
// variable, what each `<history>` recorded, the delayed `<send>`s still waiting, the
// external queue in order, and, as a tree, every running child session with the same
// (docs/adr/0045).
//
// A delayed send is saved as the moment it comes due on the host's WALL clock,
// milliseconds since the Unix epoch, not as a wait: a wait would start again when the
// process came back, and a timer that ran out while it was dead would be late by exactly
// as long as it was dead.
//
// The format is one JSON document (`sce-saved-state`, version kSavedFormat), the same on
// every backend, so what one backend saved another can read. A variable is keyed by its
// document id and written as its `sce:type` says: a number, except a 64-bit integer,
// written as a text so no reader that holds numbers as doubles loses its low bits; a byte
// string as its Latin-1 text; a record as an object of its fields; a list as an array.
//
// A saved state is bound to the SHAPE of the document it was saved from, a digest the
// generator computes of every state with its kind and parent and every variable with its
// type, not to the document's text.
//
// Cross-language siblings: backends/go/runtime/saved_state.go,
// backends/python/runtime/sce_runtime/saved_state.py, com.sce.runtime.SavedState (Kotlin)
// and sce_rust_runtime::saved_state (Rust).

#pragma once

#include "common/IOProcessorHelper.h"
#include "core/EventOrigin.h"
#include "saved/SavedJson.h"
#include "saved/SessionId.h"

#include <algorithm>
#include <chrono>
#include <cstdint>
#include <map>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

namespace SCE::Saved {

/// A state that names its running children by id alone.
inline constexpr int kSavedFormat1 = 1;
/// A tree of sessions, each running child saved inside its parent (docs/adr/0045), with an
/// event's origin stored as two texts.
inline constexpr int kSavedFormat2 = 2;
/// Format 2's tree with an event's origin held as one pair and the sending session's id
/// beside it (docs/adr/0090, section 7).
inline constexpr int kSavedFormat3 = 3;
/// The newest format version this runtime writes; it reads this one and formats 1 and 2.
inline constexpr int kSavedFormat = kSavedFormat3;

/// The stability status of the saved-state wire surface, held in lockstep with
/// `x-sce-schema-status` in `schemas/sce-saved-state.v1.schema.json`,
/// `schemas/sce-saved-state.v2.schema.json` and `schemas/sce-saved-state.v3.schema.json`
/// (SCE_WIRE_CONTRACTS.md). A flip to "stable" changes all four in one commit.
inline constexpr const char *kSavedSchemaStatus = "pre-release";

/// Why a saved state cannot be restored, or a machine cannot be saved.
class StateRefusal : public std::runtime_error {
public:
    using std::runtime_error::runtime_error;
};

namespace detail {
/// What a refusal inside a child session of a tree begins with, before the session's path.
inline constexpr std::string_view kSessionAt = "the session at ";
}  // namespace detail

/// `err`, a refusal of the child session at `path` of the session that reads it
/// (`invokes[0].session`): a tree is refused as a whole, and the reason names the session
/// it is about by its path from the root (docs/adr/0045, section 2),
/// `invokes[0].session.invokes[1].session` for a grandchild.
inline StateRefusal refusalWithin(const std::exception &err, const std::string &path) {
    const std::string reason = err.what();
    if (reason.compare(0, detail::kSessionAt.size(), detail::kSessionAt) == 0) {
        return StateRefusal(std::string(detail::kSessionAt) + path + "." + reason.substr(detail::kSessionAt.size()));
    }
    return StateRefusal(std::string(detail::kSessionAt) + path + ": " + reason);
}

/// A saved wire text's params, by name: an object of arrays of texts. Ordered by name, so
/// the text one machine writes is every backend's.
using SavedParams = std::map<std::string, std::vector<std::string>>;

/// What one `<history>` recorded when its parent was last exited (§scxml-3.10): state ids
/// in document order, under the history's id.
struct SavedHistory {
    std::string id;
    std::vector<std::string> states;
};

/// How many times the `<invoke>` `id` has started in a session.
struct SavedInvokeStart {
    std::string id;
    std::int64_t count = 0;

    bool operator==(const SavedInvokeStart &other) const {
        return id == other.id && count == other.count;
    }
};

struct SavedState;

/// A running child session, saved inside the session that started it (docs/adr/0045,
/// section 1).
struct SavedChild {
    /// The id the document gives the `<invoke>` that started it.
    std::string id;
    /// Which start of that `<invoke>` this is in its parent, counted from 1: the count the
    /// parent's `invokeStarts` holds for it.
    std::int64_t start = 0;
    /// The id the child was started under, as `_event.invokeid` and an `idlocation`
    /// variable hold it: carried, not computed again.
    std::string invokeId;
    /// The document the child runs: the child's name for a static `<invoke>`, the
    /// candidate's for a hybrid one.
    std::string document;
    /// The child's own state, its running children included.
    std::shared_ptr<SavedState> session;
};

/// The running child sessions of a saved session, as the format it was written in holds
/// them. `resumed` is a tree, format 2 or 3: each running child is saved with its whole state
/// and a restore resumes it where it stood. In format 1 the text holds the id of each
/// `<invoke>` whose child is running and nothing of the child, so a restore starts each again
/// from its beginning.
struct SavedInvokes {
    bool resumed = false;
    /// Format 1's running invocations, by the id the document gives them.
    std::vector<std::string> ids;
    /// A tree's running children, in document order.
    std::vector<SavedChild> children;

    /// The id the document gives each running `<invoke>`, in order.
    [[nodiscard]] std::vector<std::string> invokeIds() const {
        if (!resumed) {
            return ids;
        }
        std::vector<std::string> out;
        out.reserve(children.size());
        for (const SavedChild &child : children) {
            out.push_back(child.id);
        }
        return out;
    }
};

/// The revision of one record schema or enum a state was saved at.
struct SavedRevision {
    /// The event the record schema declares, or `enum:` and the enum's document, which
    /// names it in `revisions`.
    std::string key;
    /// The `sce:revision` where the state was saved, 2 or later.
    int revision = 0;
    /// The SHA-256, as lowercase hex, of what that revision declares.
    std::string fields;
};

/// One event of a saved external queue: its name and the `_event` fields a document can
/// read (§scxml-5.10.1). A typed payload is not saved; it is lifted again from `data` when
/// the event is delivered, the one path every other delivery takes.
struct SavedEvent {
    std::string name;
    std::string data;
    std::string type;
    std::string sendId;
    /// `_event.origin` and `_event.origintype` as the one pair the processor that delivered
    /// the event wrote (docs/adr/0090), nullopt for an event no processor stamped.
    std::optional<Core::EventOrigin> origin;
    /// The id of the session that sent the event, as the processor that delivered it from
    /// that session wrote it (the SCXML processor for a local session, the Mesh transport
    /// for a remote child or parent), and empty for any other sender. It is independent of
    /// `origin`'s address, which is the reply address that processor wrote; a session that
    /// sent an event always gave one, so a sender never stands beside no origin.
    std::string sender;
    std::string invokeId;
    /// The token a host-run invocation's completion or failure was stamped with when it was
    /// accepted, nullopt for any other event. A completion that was accepted and is still
    /// queued has to keep the stamp across a save, or the restored machine would refuse the
    /// one answer it was waiting for.
    std::optional<std::int64_t> hostInvokeToken;
};

/// An `<invoke>` a declared host invoker is running, as it is started again (§scxml-6.4.1):
/// every field is what the request the host was handed carried.
struct SavedHostInvoke {
    std::string processorType;
    std::string invokeId;
    std::string src;
    SavedParams params;
    std::string data;
    std::string content;
    /// When its deadline comes due, in milliseconds since the Unix epoch on the wall clock
    /// of the host that saved it; nullopt for an invocation that has none.
    std::optional<std::int64_t> due;
};

/// What a waiting `<send>` does when it comes due.
enum class SavedActKind {
    /// An event for the session that started this one (`#_parent`).
    Parent,
    /// An event for a running child of this session (`#_<invokeid>`).
    Child,
    /// An event for this session's external queue.
    Raise,
    /// An event for this session's internal queue.
    Internal,
    /// A `<send>` a host-served processor performs (§scxml-6.2.5).
    Host,
};

/// The word the format writes for an act.
inline const char *savedActName(SavedActKind kind) {
    switch (kind) {
    case SavedActKind::Parent:
        return "parent";
    case SavedActKind::Child:
        return "child";
    case SavedActKind::Raise:
        return "raise";
    case SavedActKind::Internal:
        return "internal";
    case SavedActKind::Host:
        return "host";
    }
    return "";
}

/// Whether an act of `kind` is a send of the session that saved it, to itself, its parent or a
/// child: the acts that carry the session's SCXML pair and id (docs/adr/0090, section 7).
inline bool savedActIsOwnSend(SavedActKind kind) {
    return kind == SavedActKind::Raise || kind == SavedActKind::Parent || kind == SavedActKind::Child;
}

/// What a waiting `<send>` does when it comes due. A delayed send to any other session is
/// none of the five kinds, and a document that makes one is generated without the save API
/// (§2.15).
struct SavedAct {
    SavedActKind kind = SavedActKind::Raise;
    /// The event's name, as the receiver resolves it.
    std::string event;
    /// `_event.data`, as the wire carries it.
    std::string data;
    /// The `<send>`'s id, which a `<cancel>` names.
    std::string sendId;
    /// The origin the event a Raise, Parent or Child act delivers will carry: the SCXML pair
    /// of the session that sent it. An Internal act's event is an internal event, which has
    /// none, and a Host act holds none (docs/adr/0090, section 7).
    std::optional<Core::EventOrigin> origin;
    /// The id of the session that sent a Raise, Parent or Child act, beside its pair; empty
    /// for any other act.
    std::string sender;
    /// Names the child a Child act is for, and is `_event.invokeid` of the event a Host act
    /// was sent while processing.
    std::string invokeId;
    /// A Host act's: every field is what the document wrote.
    std::string processorType;
    std::string target;
    std::string content;
    SavedParams params;
};

/// One delayed `<send>` that has not been delivered yet. `due` is when it comes due, in
/// milliseconds since the Unix epoch on the wall clock of the host that saved it.
struct SavedSend {
    std::int64_t due = 0;
    SavedAct act;
};

// ── Shape and revisions (docs/adr/0044) ──────────────────────────────────────

/// A variant a revision past the first added to an enum.
struct AddedVariant {
    std::string name;
    int since = 0;
};

/// Where a value of an EnumLineage is held. `field` is the field of the record the
/// variable holds, for a value held in a record, nullopt for a variable of the enum;
/// `list` is whether the variable holds a list of such records rather than one.
struct EnumHolder {
    std::string variable;
    std::optional<std::string> field;
    bool list = false;
};

/// An enum a value of a machine's state is of, as a saved state names it.
struct EnumLineage {
    /// `enum:` and the enum's document, which names it in `revisions`.
    std::string key;
    /// The enum's revision in this machine.
    int revision = 1;
    /// The digest of what each revision from the second to `revision` declares, in order.
    std::vector<std::string> revisions;
    /// Each variant a revision past the first added.
    std::vector<AddedVariant> added;
    /// Where a value of it is held.
    std::vector<EnumHolder> holders;
};

/// A field a revision past the first added to a record schema. `id` is the field's id, the
/// member a saved record holds it under; `defaultText` its `sce:default` as saved-state JSON.
struct AddedField {
    std::string id;
    int since = 0;
    std::string defaultText;
};

/// A field a revision widened from `T` to `optional<T>` (`sce:optional-since`). A record saved
/// before that revision holds it as a value of `T`, so a `null` there is no absence and
/// upgradeSavedState refuses it (docs/adr/0055, section 6). `id` is the field's id, the member a
/// saved record holds it under; `since` the revision that added the field (1 for a field of the
/// first revision), before which a record holds no member for it; `optionalSince` the revision
/// that widened it.
struct WidenedField {
    std::string id;
    int since = 1;
    int optionalSince = 0;
};

/// A variable that holds a record of a RecordLineage.
struct RecordHolder {
    std::string variable;
    bool list = false;
};

/// A record schema a machine's variables are of, as a saved state names it.
struct RecordLineage {
    /// The event the schema declares, which names it in `revisions`.
    std::string event;
    /// The schema's revision in this machine.
    int revision = 1;
    /// The digest of what each revision from the second to `revision` declares, in order.
    std::vector<std::string> revisions;
    /// Each field a revision past the first added, in the schema's order.
    std::vector<AddedField> added;
    /// Each field a revision widened from `T` to `optional<T>`, in the schema's order.
    std::vector<WidenedField> widened;
    /// The variables that hold a record of it, or a list of them.
    std::vector<RecordHolder> holders;
};

/// What a machine's saved state is bound to: the document's shape, and the revisions of each
/// schema a record variable of it is of (docs/adr/0044). The generator computes it, and every
/// saved state is written and judged against it.
struct SavedShape {
    /// The shape a state names when every record schema it holds is in its first revision.
    std::string first;
    /// The shape a state names when it names a later revision of any record schema. A reader
    /// that predates revisions knows only the first shape, so it refuses such a state rather
    /// than read it without the fields it does not know.
    std::string revised;
    /// `first` as a tree, of format 2 or 3, names it (docs/adr/0045, section 2): each enum-typed
    /// variable and record field spelled by its enum's document rather than by the alias it
    /// is imported under.
    std::string format2First;
    /// `revised` as a tree names it.
    std::string format2Revised;
    /// Each record schema a variable is of, by the event it declares, ordered by it.
    std::vector<RecordLineage> records;
    /// Each enum a value of the state is of, by its key, ordered by it.
    std::vector<EnumLineage> enums;
    /// The configuration of a product line the machine was built for, sorted
    /// (SavedState::features); nullopt for a machine of no product line. A machine generated
    /// for every build of one states it per build, under the build's own condition.
    std::optional<std::vector<std::string>> features;
};

/// A machine's whole state at a macrostep boundary.
struct SavedState {
    /// The shape of the document the machine was generated from.
    std::string shape;
    /// The active configuration, as state ids.
    std::vector<std::string> configuration;
    /// The current leaf, as a state id: which of a `<parallel>`'s regions the machine last
    /// stood in, which the configuration alone cannot say.
    std::string current;
    /// Every variable by document id, in declaration order, each value as the JSON value
    /// model holds it.
    JsonObject variables;
    /// What each `<history>` recorded, ordered by the history's id. A history that has
    /// recorded nothing is absent.
    std::vector<SavedHistory> history;
    /// The delayed `<send>`s still waiting, in the order they would be delivered.
    std::vector<SavedSend> pending;
    /// The `<invoke>`s whose child session is running (§scxml-6.4).
    SavedInvokes invokes;
    /// The `<invoke>`s a declared host invoker is running (§scxml-6.4.1), by `(type, id)`.
    std::vector<SavedHostInvoke> hostInvokes;
    /// The token the next host-run start receives.
    std::int64_t hostInvokeToken = 0;
    /// How many ids the machine has generated for a `<send idlocation>`.
    std::int64_t autoSendSeq = 0;
    /// The external queue, front first.
    std::vector<SavedEvent> external;
    /// The revision each record schema and enum past its first was saved at, ordered by key.
    /// Empty when every one is in its first revision.
    std::vector<SavedRevision> revisions;
    /// The configuration of a product line the machine was built for (docs/adr/0057, section
    /// 7): the features it turns on, sorted, each once. nullopt for a machine of no product
    /// line, which is every machine of a document that imports no feature model; an empty list
    /// is the configuration that turns none on. A restore judges it before the shape
    /// (checkSavedFeatures).
    std::optional<std::vector<std::string>> features;
    /// The session's own id (`_sessionid`). An XML Nmtoken in a tree; empty in a format-1
    /// state, which holds none.
    std::string sessionId;
    /// How many times each `<invoke>` of the document has started in this session, for each
    /// started at least once, in document order. Empty in a format-1 state.
    std::vector<SavedInvokeStart> invokeStarts;

    /// The variable `id`, or a refusal naming it.
    [[nodiscard]] const JsonValue &variable(const std::string &id) const {
        const JsonValue *value = variables.get(id);
        if (value == nullptr) {
            throw StateRefusal("the saved state has no variable '" + id + "'");
        }
        return *value;
    }

    /// The format this state is written in: kSavedFormat for a tree of sessions, kSavedFormat1
    /// for a state that names its running children by id alone. A tree read from a format-2
    /// text holds the pairs its origins are read as, and is written as format 3
    /// (docs/adr/0090, section 7).
    [[nodiscard]] int format() const {
        return invokes.resumed ? kSavedFormat : kSavedFormat1;
    }

    /// The id a restore binds the session to: the saved one, for a tree, since the session
    /// carries on and every child's id and a peer's address name it (docs/adr/0045); nullopt
    /// for a format-1 state, which holds none, so the restored machine keeps the id it was
    /// created with.
    [[nodiscard]] std::optional<std::string> boundSessionId() const {
        if (invokes.resumed) {
            return sessionId;
        }
        return std::nullopt;
    }

    /// How many times the `<invoke>` `id` had started in this session when it was saved: 0
    /// for one that had not, and for every one of a format-1 state, which holds no count.
    [[nodiscard]] std::int64_t invokeStart(const std::string &id) const {
        for (const SavedInvokeStart &started : invokeStarts) {
            if (started.id == id) {
                return started.count;
            }
        }
        return 0;
    }

    /// The state as `sce-saved-state` JSON, in the format it is of: the root of the text,
    /// with its `format`.
    [[nodiscard]] std::string toJson() const;

    /// Reads `sce-saved-state` JSON, of format 1, 2 or 3. The shape is not judged here; the
    /// machine that restores it knows its own. Throws StateRefusal when the text is not one.
    [[nodiscard]] static SavedState fromJson(std::string_view text);

    /// The members of this session, in the order every backend writes them: format 1's, and
    /// in a tree `sessionid` after `shape` and `invokestarts` after `invokes`, each running
    /// child's session written by the same rule.
    [[nodiscard]] JsonObject sessionMembers() const;
};

namespace detail {

inline JsonValue stringsToJson(const std::vector<std::string> &texts) {
    JsonArray out;
    out.reserve(texts.size());
    for (const std::string &text : texts) {
        out.emplace_back(text);
    }
    return JsonValue(std::move(out));
}

inline JsonValue paramsToJson(const SavedParams &params) {
    JsonObject out;
    for (const auto &[name, values] : params) {
        out.set(name, stringsToJson(values));
    }
    return JsonValue(std::move(out));
}

inline JsonValue optionalCount(const std::optional<std::int64_t> &count) {
    if (!count.has_value()) {
        return JsonValue();
    }
    return JsonValue(std::to_string(*count));
}

inline JsonValue countText(std::int64_t count) {
    return JsonValue(std::to_string(count));
}

/// An origin as format 3 writes it (docs/adr/0090, section 7): the pair, `processor` then
/// `address`, or `null` for none.
inline JsonValue originToJson(const std::optional<Core::EventOrigin> &origin) {
    if (!origin.has_value()) {
        return JsonValue();
    }
    JsonObject pair;
    pair.set("processor", origin->processor);
    pair.set("address", origin->address);
    return JsonValue(std::move(pair));
}

/// A sender as format 3 writes it: the session's id, or `null` for none.
inline JsonValue senderToJson(const std::string &sender) {
    if (sender.empty()) {
        return JsonValue();
    }
    return JsonValue(sender);
}

/// Writes the members that hold the origin and the sender of a record that carries them. A
/// tree writes format 3's `origin` and `sender`; a format-1 state writes its `origin` text,
/// the sender's id for a session's send and the address otherwise
/// (IOProcessorHelper::storedOriginText), and an event's `origintype` beside it.
inline void setOriginMembers(JsonObject &members, const std::optional<Core::EventOrigin> &origin,
                             const std::string &sender, bool tree, bool withType) {
    if (tree) {
        members.set("origin", originToJson(origin));
        members.set("sender", senderToJson(sender));
        return;
    }
    members.set("origin", IOProcessorHelper::storedOriginText(origin, sender));
    if (withType) {
        members.set("origintype", Core::originProcessorOf(origin));
    }
}

/// The waiting send `send` as the format of a tree (`tree`) or of format 1 writes it.
inline JsonValue sendToJson(const SavedSend &send, bool tree) {
    JsonObject members;
    members.set("due", countText(send.due));
    const SavedAct &act = send.act;
    members.set("act", savedActName(act.kind));
    switch (act.kind) {
    case SavedActKind::Parent:
    case SavedActKind::Raise:
        members.set("event", act.event);
        members.set("data", act.data);
        members.set("sendid", act.sendId);
        setOriginMembers(members, act.origin, act.sender, tree, false);
        break;
    case SavedActKind::Internal:
        members.set("event", act.event);
        members.set("data", act.data);
        members.set("sendid", act.sendId);
        // The event an internal act delivers is an internal event, which has no origin
        // (docs/adr/0090, section 7), so neither member holds one, whatever the act was given.
        setOriginMembers(members, std::nullopt, std::string(), tree, false);
        break;
    case SavedActKind::Child:
        members.set("invokeid", act.invokeId);
        members.set("event", act.event);
        members.set("data", act.data);
        members.set("sendid", act.sendId);
        setOriginMembers(members, act.origin, act.sender, tree, false);
        break;
    case SavedActKind::Host:
        members.set("type", act.processorType);
        members.set("event", act.event);
        members.set("target", act.target);
        members.set("content", act.content);
        members.set("params", paramsToJson(act.params));
        members.set("sendid", act.sendId);
        members.set("data", act.data);
        members.set("invokeid", act.invokeId);
        break;
    }
    return JsonValue(std::move(members));
}

/// The queued event `event` as the format of a tree (`tree`) or of format 1 writes it.
inline JsonValue eventToJson(const SavedEvent &event, bool tree) {
    JsonObject members;
    members.set("name", event.name);
    members.set("data", event.data);
    members.set("type", event.type);
    members.set("sendid", event.sendId);
    setOriginMembers(members, event.origin, event.sender, tree, true);
    members.set("invokeid", event.invokeId);
    members.set("hostinvoketoken", optionalCount(event.hostInvokeToken));
    return JsonValue(std::move(members));
}

inline JsonValue hostInvokeToJson(const SavedHostInvoke &host) {
    JsonObject members;
    members.set("type", host.processorType);
    members.set("id", host.invokeId);
    members.set("src", host.src);
    members.set("params", paramsToJson(host.params));
    members.set("data", host.data);
    members.set("content", host.content);
    members.set("due", optionalCount(host.due));
    return JsonValue(std::move(members));
}

}  // namespace detail

inline JsonObject SavedState::sessionMembers() const {
    JsonObject members;
    members.set("shape", shape);
    if (invokes.resumed) {
        members.set("sessionid", sessionId);
    }
    members.set("configuration", detail::stringsToJson(configuration));
    members.set("current", current);
    members.set("variables", variables);
    JsonObject recorded;
    for (const SavedHistory &entry : history) {
        recorded.set(entry.id, detail::stringsToJson(entry.states));
    }
    members.set("history", std::move(recorded));
    JsonArray waiting;
    for (const SavedSend &send : pending) {
        waiting.push_back(detail::sendToJson(send, invokes.resumed));
    }
    members.set("pending", std::move(waiting));
    if (invokes.resumed) {
        JsonArray children;
        for (const SavedChild &child : invokes.children) {
            JsonObject entry;
            entry.set("id", child.id);
            entry.set("start", detail::countText(child.start));
            entry.set("invokeid", child.invokeId);
            entry.set("document", child.document);
            entry.set("session", child.session->sessionMembers());
            children.emplace_back(std::move(entry));
        }
        members.set("invokes", std::move(children));
        JsonObject starts;
        for (const SavedInvokeStart &started : invokeStarts) {
            starts.set(started.id, detail::countText(started.count));
        }
        members.set("invokestarts", std::move(starts));
    } else {
        members.set("invokes", detail::stringsToJson(invokes.ids));
    }
    JsonArray hosts;
    for (const SavedHostInvoke &host : hostInvokes) {
        hosts.push_back(detail::hostInvokeToJson(host));
    }
    members.set("hostinvokes", std::move(hosts));
    members.set("hostinvoketoken", detail::countText(hostInvokeToken));
    members.set("sendseq", detail::countText(autoSendSeq));
    JsonArray queued;
    for (const SavedEvent &event : external) {
        queued.push_back(detail::eventToJson(event, invokes.resumed));
    }
    members.set("external", std::move(queued));
    // Absent, not empty, when no record schema is past its first revision: the text of such a
    // state is the text it was before revisions existed.
    if (!revisions.empty()) {
        JsonObject named;
        for (const SavedRevision &saved : revisions) {
            JsonObject entry;
            entry.set("revision", JsonNumber{std::to_string(saved.revision)});
            entry.set("fields", saved.fields);
            named.set(saved.key, std::move(entry));
        }
        members.set("revisions", std::move(named));
    }
    // Absent for a machine of no product line, so the text of every other machine is the text
    // it was before configurations were recorded.
    if (features.has_value()) {
        members.set("features", detail::stringsToJson(*features));
    }
    return members;
}

inline std::string SavedState::toJson() const {
    JsonObject root;
    root.set("format", JsonNumber{std::to_string(format())});
    const JsonObject members = sessionMembers();
    for (std::size_t at = 0; at < members.size(); ++at) {
        root.set(members.keys()[at], members.valueAt(at));
    }
    return writeJson(JsonValue(std::move(root)));
}

// ── Reading ──────────────────────────────────────────────────────────────────

namespace detail {

[[noreturn]] inline void refuse(const std::string &reason) {
    throw StateRefusal(reason);
}

/// The text member `key` of `members`, which `what` names in a refusal.
inline std::string memberText(const JsonObject &members, const std::string &what, const std::string &key) {
    const JsonValue *value = members.get(key);
    if (value == nullptr) {
        refuse("'" + what + "' has no '" + key + "'");
    }
    if (!value->isString()) {
        refuse("'" + what + "." + key + "' is not a text");
    }
    return value->asString();
}

/// `value` as an object, which `what` names in a refusal.
inline const JsonObject &memberObject(const JsonValue &value, const std::string &what) {
    if (!value.isObject()) {
        refuse("'" + what + "' is not an object");
    }
    return value.asObject();
}

/// `written` as a signed 64-bit whole number, when it is nothing but ASCII digits.
inline std::optional<std::int64_t> parseSavedDigits(const std::string &written) {
    if (written.empty()) {
        return std::nullopt;
    }
    std::int64_t value = 0;
    for (const char c : written) {
        if (c < '0' || c > '9') {
            return std::nullopt;
        }
        const int digit = c - '0';
        if (value > (INT64_MAX - digit) / 10) {
            return std::nullopt;
        }
        value = value * 10 + digit;
    }
    return value;
}

/// A count of starts, as the text a saved state writes it as: a whole number from 1 with no
/// leading zero, within a signed 64-bit count, as every backend reads one.
inline std::int64_t readSavedCount(const std::string &written, const std::string &what) {
    const auto n = parseSavedDigits(written);
    if (!n.has_value() || *n < 1 || written[0] == '0') {
        refuse("'" + what + "' (" + written + ") is not a count of starts, a whole number from 1");
    }
    return *n;
}

/// A host-run invocation's token, as the text a saved state writes it as: digits, within a
/// signed 64-bit count, as every backend reads one.
inline std::int64_t readSavedToken(const std::string &written, const std::string &what) {
    const auto n = parseSavedDigits(written);
    if (!n.has_value()) {
        refuse("'" + what + "' (" + written + ") is not a whole number a token can be");
    }
    return *n;
}

/// A moment, as the text a saved state writes it as, milliseconds since the Unix epoch: a
/// text of digits, as every 64-bit integer is.
inline std::int64_t readSavedMoment(const std::string &written, const std::string &what) {
    const auto n = parseSavedDigits(written);
    if (!n.has_value()) {
        refuse("'" + what + "' (" + written + ") is not a whole number of milliseconds");
    }
    return *n;
}

inline bool isDigits(const std::string &text) {
    if (text.empty()) {
        return false;
    }
    for (const char c : text) {
        if (c < '0' || c > '9') {
            return false;
        }
    }
    return true;
}

inline bool isLowerHexDigest(const std::string &text) {
    if (text.size() != 64) {
        return false;
    }
    for (const char c : text) {
        if (!((c >= '0' && c <= '9') || (c >= 'a' && c <= 'f'))) {
            return false;
        }
    }
    return true;
}

template <typename Read>
inline std::optional<std::int64_t> optionalMember(const JsonObject &members, const std::string &what,
                                                  const std::string &key, Read read) {
    const JsonValue *value = members.get(key);
    if (value == nullptr) {
        refuse("'" + what + "' has no '" + key + "'");
    }
    if (value->isNull()) {
        return std::nullopt;
    }
    if (value->isString()) {
        return read(value->asString(), what + "." + key);
    }
    refuse("'" + what + "." + key + "' is neither a text nor null");
}

inline SavedParams savedParamsFromJson(const JsonObject &members, const std::string &what) {
    const JsonValue *value = members.get("params");
    if (value == nullptr) {
        refuse("'" + what + "' has no 'params'");
    }
    if (!value->isObject()) {
        refuse("'" + what + ".params' is not an object");
    }
    SavedParams read;
    const JsonObject &params = value->asObject();
    for (std::size_t at = 0; at < params.size(); ++at) {
        const std::string &name = params.keys()[at];
        const JsonValue &values = params.valueAt(at);
        if (!values.isArray()) {
            refuse("'" + what + ".params." + name + "' is not an array");
        }
        std::vector<std::string> texts;
        for (const JsonValue &item : values.asArray()) {
            if (!item.isString()) {
                refuse("'" + what + ".params." + name + "' holds a value that is not a text");
            }
            texts.push_back(item.asString());
        }
        read[name] = std::move(texts);
    }
    return read;
}

inline SavedRevision savedRevisionFromJson(const std::string &key, const JsonValue &value, const std::string &what) {
    const JsonObject &members = memberObject(value, what);
    if (!members.has("revision")) {
        refuse("'" + what + "' has no 'revision'");
    }
    if (!members.has("fields")) {
        refuse("'" + what + "' has no 'fields'");
    }
    const JsonValue &number = *members.get("revision");
    if (!number.isNumber()) {
        refuse("'" + what + ".revision' is not a number");
    }
    const std::string &written = number.asNumber().text;
    // A revision too wide for an int is still a revision past the first, and later than any
    // this machine has.
    long long revision = 0;
    if (isDigits(written)) {
        revision = written.size() < 10 ? std::stoll(written) : 2'000'000'000LL;
    }
    if (revision < 2) {
        refuse("'" + what + ".revision' (" + written +
               ") is not a revision past the first; a schema in its first revision is left out");
    }
    const JsonValue &fields = *members.get("fields");
    if (!fields.isString() || !isLowerHexDigest(fields.asString())) {
        refuse("'" + what + ".fields' is not a SHA-256 written as lowercase hex");
    }
    return SavedRevision{key, static_cast<int>(revision), fields.asString()};
}

/// The `origin` and the `sender` of `what`, a record of format 3, as they are written
/// (docs/adr/0090, section 7): neither is derived from the other. `origin` is `null` or a pair
/// of a `processor` and an `address`, neither empty and with no other member; `sender` is
/// `null` or a session id. Whether the two agree is the caller's to judge, once it has judged
/// what kind of record it reads.
inline StoredOrigin savedOriginPairFromJson(const JsonObject &members, const std::string &what) {
    StoredOrigin read;
    const JsonValue *origin = members.get("origin");
    if (origin == nullptr) {
        refuse("'" + what + "' has no 'origin'");
    }
    if (!origin->isNull()) {
        const std::string where = what + ".origin";
        if (!origin->isObject()) {
            refuse("'" + where + "' is neither null nor a pair of a 'processor' and an 'address'");
        }
        const JsonObject &pair = origin->asObject();
        for (const std::string &key : pair.keys()) {
            if (key != "processor" && key != "address") {
                refuse("'" + where + "' holds '" + key + "', and a pair holds a 'processor' and an 'address' alone");
            }
        }
        std::string processor = memberText(pair, where, "processor");
        if (processor.empty()) {
            refuse("'" + where + ".processor' is empty");
        }
        std::string address = memberText(pair, where, "address");
        if (address.empty()) {
            refuse("'" + where + ".address' is empty");
        }
        read.origin = Core::EventOrigin(std::move(processor), std::move(address));
    }
    const JsonValue *sender = members.get("sender");
    if (sender == nullptr) {
        refuse("'" + what + "' has no 'sender'");
    }
    if (!sender->isNull()) {
        if (!sender->isString()) {
            refuse("'" + what + ".sender' is neither null nor a text");
        }
        if (SessionId::refusalOf(sender->asString()).has_value()) {
            refuse("'" + what + ".sender' (" + sender->asString() + ") is not an XML Nmtoken, which a session id is");
        }
        read.sender = sender->asString();
    }
    return read;
}

/// Refuses `what`, a waiting send of a session, when its origin is not the SCXML pair of its
/// sender: the act is the session's own send, which the SCXML processor delivers with the
/// session's own location as the address (docs/adr/0090, section 7).
inline void checkSenderOrigin(const StoredOrigin &read, const std::string &what) {
    if (!read.sender.empty() && read.origin != IOProcessorHelper::scxmlOrigin(read.sender)) {
        refuse("'" + what + "' has the sender '" + read.sender +
               "', and its origin is not the SCXML pair of that session, whose address is " +
               IOProcessorHelper::scxmlLocation(read.sender));
    }
}

/// The queued event `value`, of a state of `format`. Formats 1 and 2 store the origin as two
/// texts, which the stored-origin reader reads as the pair and the sender they stood for
/// (IOProcessorHelper::readStoredOrigin); format 3 holds both as they are.
inline SavedEvent savedEventFromJson(const JsonValue &value, const std::string &what, int format) {
    const JsonObject &members = memberObject(value, what);
    SavedEvent event;
    // The token is judged first, as every backend judges it.
    event.hostInvokeToken = optionalMember(members, what, "hostinvoketoken", readSavedToken);
    event.name = memberText(members, what, "name");
    event.data = memberText(members, what, "data");
    event.type = memberText(members, what, "type");
    event.sendId = memberText(members, what, "sendid");
    StoredOrigin read;
    if (format == kSavedFormat3) {
        read = savedOriginPairFromJson(members, what);
        // The sender is the session that sent the event and the origin the reply address the
        // processor that carried it wrote, two fields no rule ties together: a Mesh remote
        // child's event names its session beside the endpoint it answers at. A session that
        // sent an event always gave a reply address, so a sender beside no origin is refused.
        if (!read.sender.empty() && !read.origin.has_value()) {
            refuse("'" + what + "' has the sender '" + read.sender +
                   "' and no origin, and a session that sends an event gives a reply address");
        }
    } else {
        const std::string origin = memberText(members, what, "origin");
        read = IOProcessorHelper::readStoredOrigin(origin, memberText(members, what, "origintype"));
    }
    event.origin = std::move(read.origin);
    event.sender = std::move(read.sender);
    event.invokeId = memberText(members, what, "invokeid");
    return event;
}

inline SavedHostInvoke savedHostInvokeFromJson(const JsonValue &value, const std::string &what) {
    const JsonObject &members = memberObject(value, what);
    SavedHostInvoke host;
    host.due = optionalMember(members, what, "due", readSavedMoment);
    host.processorType = memberText(members, what, "type");
    host.invokeId = memberText(members, what, "id");
    host.src = memberText(members, what, "src");
    host.params = savedParamsFromJson(members, what);
    host.data = memberText(members, what, "data");
    host.content = memberText(members, what, "content");
    return host;
}

/// The origin and the sender of `act`, a waiting send of a state of `format` that is not a
/// host act, read from `members` (docs/adr/0090, section 7). A raise, parent or child act is
/// a send of the session that saved it, so it carries that session's SCXML pair and id; an
/// internal act's event is an internal event, which has no origin, and it carries neither.
///
/// Formats 1 and 2 store only `origin`, the sending session's id or its location as one
/// backend stored it, read as that session's pair with the id as its sender
/// (IOProcessorHelper::readStoredSender); an internal act's stored origin is not read. A
/// stored origin that is not empty names the act's session or the text is refused: the
/// sender it is read as must be an XML Nmtoken, as a format-3 `sender` must. The SCXML
/// location prefix alone is one origin this refuses, read as itself, which holds '/'. An
/// empty stored origin is left with no sender here: in format 2 the act is the send of the
/// session that holds it, whose `sessionid` savedSessionFromJson gives it once it is read, and
/// a format-1 text names no session, so the machine that restores it arms it as its own.
inline void savedActOriginFromJson(const JsonObject &members, const std::string &what, int format, SavedAct &act) {
    const bool internal = act.kind == SavedActKind::Internal;
    if (format != kSavedFormat3) {
        const std::string stored = memberText(members, what, "origin");
        if (internal || stored.empty()) {
            return;
        }
        act.sender = IOProcessorHelper::readStoredSender(stored);
        if (!SessionId::isNmtoken(act.sender)) {
            refuse("'" + what + ".origin' is '" + stored + "', which names no session: a " + savedActName(act.kind) +
                   " act is a session's own send, and its stored origin is that session's id or its SCXML location");
        }
        act.origin = IOProcessorHelper::scxmlOrigin(act.sender);
        return;
    }
    StoredOrigin read = savedOriginPairFromJson(members, what);
    if (internal) {
        if (read.origin.has_value() || !read.sender.empty()) {
            refuse("'" + what +
                   "' is an internal act and holds an origin or a sender, and the internal event it delivers "
                   "has neither");
        }
        return;
    }
    if (read.sender.empty()) {
        refuse("'" + what + "' is a " + savedActName(act.kind) +
               " act with no sender, and it is a send of the session that saved it");
    }
    checkSenderOrigin(read, what);
    act.origin = std::move(read.origin);
    act.sender = std::move(read.sender);
}

/// The waiting send `value`, of a state of `format`; a send to the parent or to a child is an
/// act of a tree only.
inline SavedSend savedSendFromJson(const JsonValue &value, const std::string &what, int format) {
    const JsonObject &members = memberObject(value, what);
    SavedSend send;
    send.due = readSavedMoment(memberText(members, what, "due"), what + ".due");
    const std::string name = memberText(members, what, "act");
    SavedAct &act = send.act;
    // The members each act reads, in the order every backend reads them; a session's act reads
    // its origin and sender after them.
    std::vector<std::pair<std::string, std::string *>> reads;
    if (name == "parent" || name == "child") {
        if (format == kSavedFormat1) {
            refuse("'" + what + ".act' is '" + name + "', which is not raise, internal or host, the acts of format 1");
        }
        act.kind = name == "child" ? SavedActKind::Child : SavedActKind::Parent;
        if (act.kind == SavedActKind::Child) {
            reads.emplace_back("invokeid", &act.invokeId);
        }
        reads.emplace_back("event", &act.event);
        reads.emplace_back("data", &act.data);
        reads.emplace_back("sendid", &act.sendId);
    } else if (name == "raise" || name == "internal") {
        act.kind = name == "raise" ? SavedActKind::Raise : SavedActKind::Internal;
        reads.emplace_back("event", &act.event);
        reads.emplace_back("data", &act.data);
        reads.emplace_back("sendid", &act.sendId);
    } else if (name == "host") {
        act.kind = SavedActKind::Host;
        reads.emplace_back("type", &act.processorType);
        reads.emplace_back("event", &act.event);
        reads.emplace_back("target", &act.target);
        reads.emplace_back("content", &act.content);
    } else {
        if (format == kSavedFormat1) {
            refuse("'" + what + ".act' is '" + name + "', which is not raise, internal or host, the acts of format 1");
        }
        refuse("'" + what + ".act' is '" + name + "', which is not raise, internal, host, parent or child");
    }
    for (auto &[key, into] : reads) {
        *into = memberText(members, what, key);
    }
    if (act.kind == SavedActKind::Host) {
        act.params = savedParamsFromJson(members, what);
        act.sendId = memberText(members, what, "sendid");
        act.data = memberText(members, what, "data");
        act.invokeId = memberText(members, what, "invokeid");
    } else {
        savedActOriginFromJson(members, what, format, act);
    }
    return send;
}

inline SavedState savedSessionFromJson(const JsonObject &members, int format);

/// The `index`-th running child of a session of a tree of `format`, whose records the child's
/// are read as: a child session does not state the format again.
inline SavedChild savedChildFromJson(const JsonValue &value, std::size_t index, int format) {
    const std::string what = "invokes[" + std::to_string(index) + "]";
    const JsonObject &members = memberObject(value, what);
    const auto nonEmpty = [&](const std::string &key) {
        std::string written = memberText(members, what, key);
        if (written.empty()) {
            refuse("'" + what + "." + key + "' is empty");
        }
        return written;
    };
    SavedChild child;
    child.start = readSavedCount(nonEmpty("start"), what + ".start");
    const JsonValue *session = members.get("session");
    if (session == nullptr) {
        refuse("'" + what + "' has no 'session'");
    }
    if (!session->isObject()) {
        refuse("'" + what + ".session' is not an object");
    }
    if (session->asObject().has("format")) {
        refuse("'" + what + ".session' states a format, which only the root of a tree does");
    }
    try {
        child.session = std::make_shared<SavedState>(savedSessionFromJson(session->asObject(), format));
    } catch (const StateRefusal &err) {
        throw refusalWithin(err, what + ".session");
    }
    child.id = nonEmpty("id");
    child.invokeId = nonEmpty("invokeid");
    child.document = nonEmpty("document");
    return child;
}

inline SavedState savedSessionFromJson(const JsonObject &members, int format) {
    const auto field = [&](const std::string &name) -> const JsonValue & {
        const JsonValue *value = members.get(name);
        if (value == nullptr) {
            refuse("the saved state has no '" + name + "'");
        }
        return *value;
    };
    const auto textOf = [](const JsonValue &value, const std::string &what) -> const std::string & {
        if (!value.isString()) {
            refuse("'" + what + "' is not a text");
        }
        return value.asString();
    };
    const auto fieldText = [&](const std::string &name) { return textOf(field(name), name); };
    const auto fieldArray = [&](const std::string &name) -> const JsonArray & {
        const JsonValue &items = field(name);
        if (!items.isArray()) {
            refuse("'" + name + "' is not an array");
        }
        return items.asArray();
    };

    SavedState state;
    for (const JsonValue &item : fieldArray("configuration")) {
        state.configuration.push_back(textOf(item, "configuration"));
    }

    const JsonValue &variables = field("variables");
    if (!variables.isObject()) {
        refuse("'variables' is not an object");
    }
    state.variables = variables.asObject();

    const JsonValue &recorded = field("history");
    if (!recorded.isObject()) {
        refuse("'history' is not an object");
    }
    const JsonObject &historyMembers = recorded.asObject();
    for (std::size_t at = 0; at < historyMembers.size(); ++at) {
        const std::string &id = historyMembers.keys()[at];
        const JsonValue &listed = historyMembers.valueAt(at);
        if (!listed.isArray()) {
            refuse("'history." + id + "' is not an array");
        }
        SavedHistory entry;
        entry.id = id;
        for (const JsonValue &item : listed.asArray()) {
            entry.states.push_back(textOf(item, "history." + id));
        }
        state.history.push_back(std::move(entry));
    }

    const JsonArray &pending = fieldArray("pending");
    for (std::size_t at = 0; at < pending.size(); ++at) {
        state.pending.push_back(savedSendFromJson(pending[at], "pending[" + std::to_string(at) + "]", format));
    }

    const JsonArray &listed = fieldArray("invokes");
    if (format == kSavedFormat1) {
        state.invokes.resumed = false;
        for (const JsonValue &item : listed) {
            state.invokes.ids.push_back(textOf(item, "invokes"));
        }
    } else {
        state.invokes.resumed = true;
        for (std::size_t at = 0; at < listed.size(); ++at) {
            state.invokes.children.push_back(savedChildFromJson(listed[at], at, format));
        }
    }

    // A tree's own members: the session's id and its count of starts.
    if (format != kSavedFormat1) {
        const std::string sessionId = fieldText("sessionid");
        if (SessionId::refusalOf(sessionId).has_value()) {
            refuse("'sessionid' (" + sessionId + ") is not an XML Nmtoken, which a session id is");
        }
        state.sessionId = sessionId;
        // docs/adr/0090, section 7: a raise, parent or child act of a format-2 text whose stored
        // origin is empty is read as sent by the session that holds it.
        if (format == kSavedFormat2) {
            for (SavedSend &send : state.pending) {
                SavedAct &act = send.act;
                if (savedActIsOwnSend(act.kind) && act.sender.empty()) {
                    act.sender = sessionId;
                    act.origin = IOProcessorHelper::scxmlOrigin(sessionId);
                }
            }
        }
        // docs/adr/0090, section 7: a format-3 raise, parent or child act carries this session's
        // pair and this session's id, so one that names another session is no act this session
        // saved. Judged here, once the session's id is read, for the root and every child.
        if (format == kSavedFormat3) {
            for (std::size_t at = 0; at < state.pending.size(); ++at) {
                const SavedAct &act = state.pending[at].act;
                if (savedActIsOwnSend(act.kind) && act.sender != sessionId) {
                    refuse("'pending[" + std::to_string(at) + "].sender' is '" + act.sender +
                           "', and this session is '" + sessionId + "': a " + savedActName(act.kind) +
                           " act is this session's own send");
                }
            }
        }
        const JsonValue &counted = field("invokestarts");
        if (!counted.isObject()) {
            refuse("'invokestarts' is not an object");
        }
        const JsonObject &countedMembers = counted.asObject();
        for (std::size_t at = 0; at < countedMembers.size(); ++at) {
            const std::string &id = countedMembers.keys()[at];
            const JsonValue &count = countedMembers.valueAt(at);
            if (!count.isString()) {
                refuse("'invokestarts." + id + "' is not a text");
            }
            state.invokeStarts.push_back(SavedInvokeStart{id, readSavedCount(count.asString(), "invokestarts." + id)});
        }
    }

    const JsonArray &hosts = fieldArray("hostinvokes");
    for (std::size_t at = 0; at < hosts.size(); ++at) {
        state.hostInvokes.push_back(savedHostInvokeFromJson(hosts[at], "hostinvokes[" + std::to_string(at) + "]"));
    }

    for (const std::string name : {"hostinvoketoken", "sendseq"}) {
        const JsonValue &digits = field(name);
        if (!digits.isString()) {
            refuse("'" + name + "' is not a text");
        }
        const std::int64_t n = readSavedToken(digits.asString(), name);
        (name == "hostinvoketoken" ? state.hostInvokeToken : state.autoSendSeq) = n;
    }

    const JsonArray &external = fieldArray("external");
    for (std::size_t at = 0; at < external.size(); ++at) {
        state.external.push_back(savedEventFromJson(external[at], "external[" + std::to_string(at) + "]", format));
    }

    // Optional, unlike every member above: a state whose record schemas are all in their
    // first revision leaves it out, as every state saved before schemas had revisions does.
    if (const JsonValue *named = members.get("revisions")) {
        if (!named->isObject()) {
            refuse("'revisions' is not an object");
        }
        if (named->asObject().size() == 0) {
            refuse("'revisions' names no schema; a state whose record schemas are all in their first revision "
                   "leaves it out");
        }
        for (std::size_t at = 0; at < named->asObject().size(); ++at) {
            const std::string &key = named->asObject().keys()[at];
            state.revisions.push_back(savedRevisionFromJson(key, named->asObject().valueAt(at), "revisions." + key));
        }
    }

    // Optional too: only a machine of a product line records the features it was built with,
    // as a set, so each is named once and in order (byte order, which std::string compares in).
    if (const JsonValue *configured = members.get("features")) {
        if (!configured->isArray()) {
            refuse("'features' is not an array");
        }
        std::vector<std::string> features;
        for (const JsonValue &item : configured->asArray()) {
            features.push_back(textOf(item, "features"));
        }
        for (std::size_t at = 1; at < features.size(); ++at) {
            if (features[at - 1] >= features[at]) {
                refuse("'features' is not in order or names a feature twice; it is a set, sorted");
            }
        }
        state.features = std::move(features);
    }

    state.shape = fieldText("shape");
    state.current = fieldText("current");
    return state;
}

}  // namespace detail

inline SavedState SavedState::fromJson(std::string_view text) {
    JsonValue value;
    try {
        value = parseJson(text);
    } catch (const JsonError &err) {
        throw StateRefusal(std::string("the saved state is ") + err.what());
    }
    if (!value.isObject()) {
        throw StateRefusal("the saved state is not an object");
    }
    const JsonObject &members = value.asObject();
    const JsonValue *written = members.get("format");
    if (written == nullptr) {
        throw StateRefusal("the saved state has no 'format'");
    }
    int format = 0;
    if (written->isNumber() && written->asNumber().text == "1") {
        format = kSavedFormat1;
    } else if (written->isNumber() && written->asNumber().text == "2") {
        format = kSavedFormat2;
    } else if (written->isNumber() && written->asNumber().text == "3") {
        format = kSavedFormat3;
    } else {
        throw StateRefusal("the saved state is format " + writeJson(*written) + ", and this runtime reads formats " +
                           std::to_string(kSavedFormat1) + ", " + std::to_string(kSavedFormat2) + " and " +
                           std::to_string(kSavedFormat3));
    }
    return detail::savedSessionFromJson(members, format);
}

/// The wall clock now, in milliseconds since the Unix epoch: what a host that has no clock of
/// its own to give save and restore gives them.
inline std::int64_t savedWallClockMs() {
    return std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::system_clock::now().time_since_epoch())
        .count();
}

/// Refuses `saved` unless it was saved from a document of this `shape`, judged before any
/// variable is read, so a state saved from another document is refused for that and not for
/// whichever variable it happens to lack.
inline void checkSavedShape(const SavedState &saved, const std::string &shape) {
    if (saved.shape != shape) {
        throw StateRefusal("the saved state is of a document of shape " + saved.shape + ", and this machine's is " +
                           shape);
    }
}

namespace detail {

/// `features` as a saved-state refusal lists them: `[a, b]`, and `[]` for none.
inline std::string featureList(const std::vector<std::string> &features) {
    std::string out = "[";
    for (std::size_t at = 0; at < features.size(); ++at) {
        if (at > 0) {
            out += ", ";
        }
        out += features[at];
    }
    return out + "]";
}

/// Each of `features` that `others` does not name, in the order `features` names them.
inline std::vector<std::string> featuresNotIn(const std::vector<std::string> &features,
                                              const std::vector<std::string> &others) {
    std::vector<std::string> out;
    for (const std::string &feature : features) {
        if (std::find(others.begin(), others.end(), feature) == others.end()) {
            out.push_back(feature);
        }
    }
    return out;
}

}  // namespace detail

/// Refuses `saved` unless it was taken in the configuration `features` of a product line, or
/// both are of none (docs/adr/0057, section 7), judged before the shape, which differs too
/// between builds that differ in states or variables, so a state of another build is refused
/// by the features that differ and not by a digest. A build with fewer features lacks states
/// or data the state may hold, and one with more has data and invocations the saved run never
/// initialised or started: either is refused. Carrying a state across configurations is a
/// migration, which a restore does not do.
inline void checkSavedFeatures(const SavedState &saved, const std::optional<std::vector<std::string>> &features) {
    if (!saved.features.has_value() && !features.has_value()) {
        return;
    }
    if (saved.features.has_value() && features.has_value()) {
        const std::vector<std::string> &taken = *saved.features;
        const std::vector<std::string> &built = *features;
        if (taken == built) {
            return;
        }
        const std::vector<std::string> onlySaved = detail::featuresNotIn(taken, built);
        const std::vector<std::string> onlyBuilt = detail::featuresNotIn(built, taken);
        std::string differ;
        if (!onlySaved.empty()) {
            differ += detail::featureList(onlySaved) + " on only where it was saved";
        }
        if (!onlyBuilt.empty()) {
            differ += (differ.empty() ? "" : ", ") + detail::featureList(onlyBuilt) + " on only in this build";
        }
        throw StateRefusal("the saved state was taken with the features " + detail::featureList(taken) +
                           " on, and this machine is built with " + detail::featureList(built) + ": " + differ);
    }
    if (features.has_value()) {
        throw StateRefusal("the saved state records no features, and this machine is built with the features " +
                           detail::featureList(*features) + " on");
    }
    throw StateRefusal("the saved state was taken with the features " + detail::featureList(*saved.features) +
                       " on, and this machine is built from a document of no feature model");
}

namespace detail {

/// Grows `value`, a saved record of `record` saved at revision `at`, with the default of
/// each of `added`, the fields later revisions added; `what` names it in a refusal. A value
/// that is not an object is returned as it is, for the record's own read to refuse.
inline JsonValue grownRecord(const JsonValue &value, const std::string &what, const RecordLineage &record, int at,
                             const std::vector<const AddedField *> &added) {
    if (!value.isObject()) {
        return value;
    }
    JsonObject grown = value.asObject();
    for (const AddedField *field : added) {
        if (grown.has(field->id)) {
            refuse("'" + what + "' holds '" + field->id + "', which revision " + std::to_string(at) + " of '" +
                   record.event + "' does not declare: revision " + std::to_string(field->since) + " added it");
        }
        try {
            grown.set(field->id, parseJson(field->defaultText));
        } catch (const JsonError &err) {
            refuse("the default of '" + field->id + "' of '" + record.event + "' is " + err.what());
        }
    }
    return JsonValue(std::move(grown));
}

/// Refuses a value `holder` holds, in `saved`, that names one of `later`, the variants of
/// `lineage` added after `at`, the revision it was saved at. What is not a text where one is
/// held is left for the variable's own read to refuse.
inline void checkEnumHolder(const SavedState &saved, const EnumLineage &lineage, const EnumHolder &holder, int at,
                            const std::vector<const AddedVariant *> &later) {
    const JsonValue *value = saved.variables.get(holder.variable);
    if (value == nullptr) {
        return;
    }
    std::vector<std::pair<std::string, const JsonValue *>> values;
    if (!holder.field.has_value()) {
        values.emplace_back(holder.variable, value);
    } else if (!holder.list) {
        if (value->isObject()) {
            if (const JsonValue *member = value->asObject().get(*holder.field)) {
                values.emplace_back(holder.variable + "." + *holder.field, member);
            }
        }
    } else if (value->isArray()) {
        const JsonArray &items = value->asArray();
        for (std::size_t index = 0; index < items.size(); ++index) {
            if (items[index].isObject()) {
                if (const JsonValue *member = items[index].asObject().get(*holder.field)) {
                    values.emplace_back(holder.variable + "[" + std::to_string(index) + "]." + *holder.field, member);
                }
            }
        }
    }
    for (const auto &[what, held] : values) {
        if (!held->isString()) {
            continue;
        }
        for (const AddedVariant *variant : later) {
            if (variant->name == held->asString()) {
                refuse("'" + what + "' holds '" + variant->name + "', which revision " + std::to_string(at) + " of '" +
                       lineage.key + "' does not declare: revision " + std::to_string(variant->since) + " added it");
            }
        }
    }
}

/// Refuses the saved record `value`, of `record` saved at revision `at`, if it holds `null` in a
/// field of `widened`; `what` names it in the refusal. What is not an object is left for the
/// record's own read to refuse.
inline void checkWidenedRecord(const JsonValue &value, const std::string &what, const RecordLineage &record, int at,
                               const std::vector<const WidenedField *> &widened) {
    if (!value.isObject()) {
        return;
    }
    for (const WidenedField *field : widened) {
        const JsonValue *member = value.asObject().get(field->id);
        if (member != nullptr && member->isNull()) {
            refuse("'" + what + "." + field->id + "' is null, which revision " + std::to_string(at) + " of '" +
                   record.event + "' holds as a value: revision " + std::to_string(field->optionalSince) +
                   " widened it");
        }
    }
}

/// Refuses a `null` the record, or each record of the list, `holder` holds in `saved` has in a
/// field of `widened`, the fields of `record` that a revision after `at`, the one it was saved
/// at, widened to optional: at `at` each was a value of `T`. What is not a record where one is
/// held, and a member the record lacks, is left for the variable's own read to refuse.
inline void checkWidenedHolder(const SavedState &saved, const RecordLineage &record, const RecordHolder &holder, int at,
                               const std::vector<const WidenedField *> &widened) {
    const JsonValue *value = saved.variables.get(holder.variable);
    if (value == nullptr) {
        return;
    }
    if (!holder.list) {
        checkWidenedRecord(*value, holder.variable, record, at, widened);
        return;
    }
    if (!value->isArray()) {
        return;
    }
    const JsonArray &items = value->asArray();
    for (std::size_t index = 0; index < items.size(); ++index) {
        checkWidenedRecord(items[index], holder.variable + "[" + std::to_string(index) + "]", record, at, widened);
    }
}

}  // namespace detail

/// `saved` as a state of the machine whose saved state is bound to `shape`, or the refusal
/// that says why it is not one (docs/adr/0044), judged before any variable is read, as
/// checkSavedShape is. The configuration the state was taken in is judged first
/// (checkSavedFeatures).
///
/// A state that names no revision is of the first shape; one that does is of the revised
/// shape, and each revision it names is of a schema a record variable of this machine is of,
/// no later than this machine's revision of it, and declares what this machine's revision of
/// that number declares. Such a state is then read as one of this machine: each record of a
/// schema saved at an earlier revision takes the default of every field a later revision
/// added. A record that already holds such a field is refused, since the revision it was
/// saved at does not declare it, and so is one that holds `null` in a field a later revision
/// widened to optional, since at the revision it was saved at the field was a value of its
/// present type and no build wrote it absent (docs/adr/0055, section 6).
inline SavedState upgradeSavedState(const SavedState &saved, const SavedShape &shape) {
    checkSavedFeatures(saved, shape.features);
    // A tree names an enum by its document and a format-1 state by the alias it is imported
    // under, so each is judged against its own shape (docs/adr/0045, section 2). Format 3 holds
    // format 2's tree, so it is of format 2's shape (docs/adr/0090, section 7).
    const bool tree = saved.format() != kSavedFormat1;
    const std::string &first = tree ? shape.format2First : shape.first;
    const std::string &revised = tree ? shape.format2Revised : shape.revised;
    if (saved.revisions.empty()) {
        checkSavedShape(saved, first);
    } else if (saved.shape == first) {
        throw StateRefusal(
            "the saved state names revisions of its record schemas and has the shape of a state that names none");
    } else {
        checkSavedShape(saved, revised);
    }
    for (const SavedRevision &named : saved.revisions) {
        // A record schema by the event it declares, an enum by its key.
        const std::vector<std::string> *digests = nullptr;
        int revision = 0;
        for (const RecordLineage &record : shape.records) {
            if (record.event == named.key) {
                digests = &record.revisions;
                revision = record.revision;
                break;
            }
        }
        if (digests == nullptr) {
            for (const EnumLineage &lineage : shape.enums) {
                if (lineage.key == named.key) {
                    digests = &lineage.revisions;
                    revision = lineage.revision;
                    break;
                }
            }
        }
        if (digests == nullptr) {
            throw StateRefusal("'revisions' names '" + named.key + "', which no record or enum of this machine is");
        }
        if (named.revision > revision) {
            throw StateRefusal("the saved state was saved at revision " + std::to_string(named.revision) + " of '" +
                               named.key + "', later than this machine's " + std::to_string(revision) +
                               ": a machine does not restore what it does not know");
        }
        // The digest of revision r is the (r - 2)th: the first has none.
        const int at = named.revision - 2;
        if (at < 0 || static_cast<std::size_t>(at) >= digests->size() ||
            (*digests)[static_cast<std::size_t>(at)] != named.fields) {
            throw StateRefusal("revision " + std::to_string(named.revision) + " of '" + named.key +
                               "' declared other members where the state was saved than it does in this machine");
        }
    }
    const auto revisionOf = [&](const std::string &key) {
        for (const SavedRevision &named : saved.revisions) {
            if (named.key == key) {
                return named.revision;
            }
        }
        return 1;
    };
    // An enum value saved at an earlier revision of its enum is one of that revision's
    // variants. Judged on the values as saved, before a record is grown by defaults, which
    // are this machine's.
    for (const EnumLineage &lineage : shape.enums) {
        const int at = revisionOf(lineage.key);
        std::vector<const AddedVariant *> later;
        for (const AddedVariant &variant : lineage.added) {
            if (variant.since > at) {
                later.push_back(&variant);
            }
        }
        if (later.empty()) {
            continue;
        }
        for (const EnumHolder &holder : lineage.holders) {
            detail::checkEnumHolder(saved, lineage, holder, at, later);
        }
    }
    // A record's field saved before the revision that widened it holds a value of `T`, so a
    // `null` there is no absence and the state is not one any build wrote. Judged on the values
    // as saved, before a record is grown by defaults, which are this machine's and may be `null`
    // for a field added absent.
    for (const RecordLineage &record : shape.records) {
        const int at = revisionOf(record.event);
        std::vector<const WidenedField *> held;
        for (const WidenedField &field : record.widened) {
            if (field.since <= at && at < field.optionalSince) {
                held.push_back(&field);
            }
        }
        if (held.empty()) {
            continue;
        }
        for (const RecordHolder &holder : record.holders) {
            detail::checkWidenedHolder(saved, record, holder, at, held);
        }
    }
    SavedState upgraded = saved;
    for (const RecordLineage &record : shape.records) {
        const int at = revisionOf(record.event);
        std::vector<const AddedField *> added;
        for (const AddedField &field : record.added) {
            if (field.since > at) {
                added.push_back(&field);
            }
        }
        if (added.empty()) {
            continue;
        }
        for (const RecordHolder &holder : record.holders) {
            const JsonValue *value = upgraded.variables.get(holder.variable);
            if (value == nullptr) {
                continue;
            }
            if (!holder.list) {
                upgraded.variables.set(holder.variable,
                                       detail::grownRecord(*value, holder.variable, record, at, added));
                continue;
            }
            if (!value->isArray()) {
                continue;
            }
            JsonArray grownItems;
            const JsonArray &items = value->asArray();
            for (std::size_t index = 0; index < items.size(); ++index) {
                grownItems.push_back(detail::grownRecord(
                    items[index], holder.variable + "[" + std::to_string(index) + "]", record, at, added));
            }
            upgraded.variables.set(holder.variable, JsonValue(std::move(grownItems)));
        }
    }
    return upgraded;
}

}  // namespace SCE::Saved
