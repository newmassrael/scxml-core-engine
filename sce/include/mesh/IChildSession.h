// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh IChildSession — abstract interface for child state-machine
// sessions hosted by a worker binary (SCE_MESH.md §mesh-9.6).
//
// Each remote `<invoke type="scxml" src="#peer">` round-trip creates one
// IChildSession instance on the child's device. The session owns the
// AOT-generated StaticExecutionEngine for that child document and mediates
// the wire-14/15/16/17/18/19 lifecycle:
//
//   wire-14 InvokeStart  (P→C)  → WorkerSessionHost instantiates a session
//   wire-15 InvokeStarted (C→P) → session publishes its URI back
//   wire-16 ChildEvent   (C→P)  → session's <send target="#_parent"> hook
//   wire-17 ParentEvent  (P→C)  → WorkerSessionHost::onWire17 → raiseExternal
//   wire-18 InvokeDone   (C→P)  → WorkerSessionHost detects isFinal, emits
//   wire-19 InvokeCancel (P→C)  → WorkerSessionHost::onWire19 → cancel()
//
// The registry (`WorkerSessionHost`) owns sessions through
// `std::unique_ptr<IChildSession>` so concrete Engine types do not leak
// into transport-layer code. Concrete sessions are constructed by a
// compile-time factory switch emitted by
// `tools/codegen/templates/mesh/cpp/worker_session_host.jinja2`.

#pragma once

#include <array>
#include <cstdint>
#include <string>

namespace SCE::Mesh {

class IChildSession {
public:
    virtual ~IChildSession() = default;

    /// Advance the child's macrostep once. Called from the worker's tick loop
    /// at the parent's cadence (SCE_MESH.md §mesh-9.6.1, §scxml-6.4.1). Implementation
    /// delegates to Engine::tick() which drains the child's internal/external
    /// queues and runs entry/exit/transition actions.
    virtual void tick() = 0;

    /// Deliver a parent→child event (wire-17 `ParentEvent`) into the child's
    /// external queue. Returns false when the event name is not declared on
    /// the child's machine (graceful degrade per §scxml-6.4 — child silently
    /// ignores autoforwarded events it does not recognize).
    ///
    /// The `sendId` parameter is reserved for future `_event.sendid`
    /// preservation; the current implementation mirrors the local-invoke
    /// autoforward path (invoke_methods.jinja2:287) and does not propagate
    /// sendid into the child. Promoting full `_event.sendid` forwarding is
    /// a separate landing shared with the local-invoke path.
    virtual bool raiseExternal(const std::string& eventName,
                                const std::string& data,
                                const std::string& sendId) = 0;

    /// Return true when the child has reached a top-level `<final>` state
    /// (StaticExecutionEngine::isGlobalFinalState). WorkerSessionHost polls
    /// this after each `tick()` and, on first true, emits a wire-18
    /// `InvokeDone` envelope and retires the session.
    virtual bool isFinal() const = 0;

    /// Retrieve the donedata payload (raw bytes) for the child's completion.
    /// Empty when no `<donedata>` was declared on the reached `<final>`.
    /// Called once, after `isFinal()` first reports true.
    virtual std::string getDonedata() = 0;

    /// Invoked when the parent has left the invoking state. The session
    /// stops emitting further wire-16 events; WorkerSessionHost removes it
    /// from the registry immediately after. `cancel()` must be idempotent —
    /// a parent may send wire-19 redundantly.
    virtual void cancel() = 0;

    /// Child session URI (SCE_MESH.md §mesh-9.6.1 L1410:
    /// `<parent_device>:<parent_machine>:<invoke_id>`). Stamped into every
    /// wire-16 envelope's `child_session_id` field for finalize / autoforward
    /// matching on the parent (§mesh-9.6.3 L1463).
    virtual const std::string& sessionId() const = 0;

    /// Deploy.yaml machine name hosted by this session.
    virtual const std::string& machineName() const = 0;

    /// Parent peer name (deploy.yaml machine name of the invoker). Used by
    /// WorkerSessionHost to route outbound wire-16/18 envelopes back to the
    /// parent that originated the wire-14.
    virtual const std::string& parentPeer() const = 0;

    /// SCXML-side invoke id from the parent (§scxml-3.12.1 format,
    /// `stateid.platformid.index`). Mirrored into every outbound wire-16/18
    /// envelope so the parent's `activeInvokes_[...]` lookup matches.
    virtual const std::string& invokeIdString() const = 0;

    /// Invoke UUID (16-byte wire form). Parent side keyed the correlation
    /// with this UUID; child-side adapter stamps it into every outbound
    /// wire-16/18 envelope.
    virtual const std::array<uint8_t, 16>& invokeUuid() const = 0;
};

}  // namespace SCE::Mesh
