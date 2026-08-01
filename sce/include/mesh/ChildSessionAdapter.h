// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh ChildSessionAdapter — binds a concrete AOT-generated
// StaticExecutionEngine to the IChildSession interface (SCE_MESH.md §mesh-9.6).
//
// One instance per active remote invoke. Owned by WorkerSessionHost's
// registry through std::unique_ptr<IChildSession>. The adapter:
//
//   - Installs a MeshSendCallback on the engine that intercepts
//     <send target="#_parent"> and emits wire-16 `ChildEvent` envelopes.
//   - Forwards tick() / cancel() / raiseExternal calls to the engine.
//   - Advertises the child's top-level `<final>` condition via
//     StaticExecutionEngine::isInFinalState so WorkerSessionHost can
//     emit wire-18 `InvokeDone` and retire the session.
//
// Engine requirements (satisfied by StaticExecutionEngine<Policy>):
//   - Default-constructible. (Machines with non-default ctors — e.g. Hardware
//     bindings — are not yet supported as invoke targets; a future Session F
//     refinement will introduce a codegen hook for engine construction.)
//   - Provides `initialize()`, `tick()`, `isInFinalState()`,
//     `setMeshSendCallback(...)`, and `raiseExternal(const std::string&,
//     const std::string&)`.

#pragma once

#include "common/ForwardedEvent.h"
#include "common/SCXMLConstants.h"
#include "common/Uuid.h"
#include "mesh/IChildSession.h"
#include "mesh/MeshEnvelope.h"
#include "mesh/PatternKind.h"
#include "mesh/PayloadCodec.h"
#include "mesh/RpcStatus.h"

#include <array>
#include <cstdint>
#include <functional>
#include <string>
#include <utility>

namespace SCE::Mesh {

/// Publishes a child→parent envelope (wire-16 ChildEvent or wire-18 InvokeDone)
/// via the worker's transport router. Returns true on acceptance. Signature
/// matches the shape WorkerSessionHost::makeWire16Publisher() binds.
using ChildToParentPublisher = std::function<bool(const MeshEnvelope &env)>;

/// Reason code a worker reports when a child session could not be brought
/// up; see [makeInvokeChildInitFailedEnvelope].
inline constexpr const char *kInvokeChildInitFailed = "INVOKE_CHILD_INIT_FAILED";

/// Build the wire-18 `InvokeDone` envelope a worker publishes when a child
/// session completes.
inline MeshEnvelope makeInvokeDoneEnvelope(const std::string &host_machine_name,
                                           const std::array<uint8_t, 16> &invoke_uuid,
                                           const std::string &invoke_id_string, const std::string &child_session_id,
                                           const std::string &donedata) {
    // Taking `donedata` as a parameter rather than reading it off the session
    // is what lets the SCE_MESH.md §mesh-9.3 failure path reuse this builder:
    // a child that threw during instantiation has no session left to
    // interrogate, but the parent still needs the completion that releases
    // its invoking state.
    MeshEnvelope done{};
    done.id = SCE::uuid::v7();
    done.source = host_machine_name;
    done.type = "sce.invoke.done";
    done.pattern = PatternKind::InvokeDone;
    done.invoke_id = invoke_uuid;
    done.subject = invoke_id_string;
    done.child_session_id = child_session_id;
    done.data.assign(donedata.begin(), donedata.end());
    done.datacontenttype = donedata.empty() ? PayloadCodec::None : PayloadCodec::Json;
    return done;
}

/// Build the wire-20 `InvokeError` envelope for a child that could not be
/// instantiated.
inline MeshEnvelope makeInvokeChildInitFailedEnvelope(const std::string &host_machine_name,
                                                      const std::array<uint8_t, 16> &invoke_uuid,
                                                      const std::string &invoke_id_string,
                                                      const std::string &child_session_id, const std::string &detail) {
    // SCE_MESH.md §mesh-9.3, branch "Child fails during initialization":
    // the parent must observe `error.execution` with this reason. A worker
    // cannot hand an exception across a device boundary, so the failure
    // travels as this envelope and the parent's `dispatchEnvelope` turns it
    // back into the raise an author already handles.
    //
    // The code rides in `rpc_error_message`, which that dispatch surfaces as
    // `_event.data` — the same payload shape the transport-absent local
    // setup fault produces for `INVOKE_SRC_NOT_FOUND`, so an author's
    // `<transition event="error.execution">` reads one shape either way. The
    // structured `_event.data` JSON of §mesh-10.7.1 is a separate landing;
    // until then the code travels in the message text, prefixed rather than
    // buried so a substring match on it is stable.
    MeshEnvelope err{};
    err.id = SCE::uuid::v7();
    err.source = host_machine_name;
    err.type = "sce.invoke.error";
    err.pattern = PatternKind::InvokeError;
    err.invoke_id = invoke_uuid;
    err.subject = invoke_id_string;
    err.child_session_id = child_session_id;
    err.rpc_status = RpcStatus::Internal;
    err.rpc_error_message = std::string(kInvokeChildInitFailed) + ": remote child '" + host_machine_name +
                            "' failed to initialize for invoke '" + invoke_id_string + "'" +
                            (detail.empty() ? std::string() : (" (" + detail + ")"));
    return err;
}

template <typename Engine> class ChildSessionAdapter : public IChildSession {
public:
    ChildSessionAdapter(std::string machine_name, std::string session_id, std::array<uint8_t, 16> invoke_uuid,
                        std::string invoke_id_string, std::string parent_peer, ChildToParentPublisher wire16_publisher)
        : machine_name_(std::move(machine_name)), session_id_(std::move(session_id)), invoke_uuid_(invoke_uuid),
          invoke_id_string_(std::move(invoke_id_string)), parent_peer_(std::move(parent_peer)),
          wire16_(std::move(wire16_publisher)) {
        // Intercept <send target="#_parent"> on the engine and emit wire-16.
        // The child's mesh-send callback returns true when the send has been
        // accepted — preventing fall-through to the external queue (W3C
        // graceful-degrade path). Any other target returns false so the
        // engine's default handling (external queue or http send) applies.
        engine_.setMeshSendCallback([this](const std::string &target, const std::string &eventName,
                                           const std::string &data, const std::string &sendId,
                                           const std::string & /*invokeId*/) -> bool {
            if (target != "#_parent") {
                return false;
            }
            if (cancelled_) {
                return true;  // drop post-cancel sends
            }
            MeshEnvelope env{};
            env.id = SCE::uuid::v7();
            env.source = machine_name_;
            env.type = eventName;
            env.pattern = PatternKind::ChildEvent;
            env.data.assign(data.begin(), data.end());
            env.invoke_id = invoke_uuid_;
            env.child_session_id = session_id_;
            if (!sendId.empty()) {
                env.subject = sendId;
            }
            return wire16_(env);
        });
    }

    Engine &engine() {
        return engine_;
    }

    void tick() override {
        if (cancelled_) {
            return;
        }
        engine_.tick();
    }

    bool raiseExternal(const ::SCE::Common::ForwardedEvent &forwarded) override {
        if (cancelled_) {
            return false;
        }
        // SCE_MESH.md §mesh-9.6.3: a wire-17 `ParentEvent` always surfaces on
        // the child as an external event from the SCXML Event I/O Processor —
        // the remote transport is transparent per §scxml-6.2. The wire-carried
        // fields (name, payload, sendid) arrive verbatim from the parent per
        // §mesh-9.6.5; the processor identity is stamped here so every
        // transport arm's `onWire17` gets it without repeating the constant.
        ::SCE::Common::ForwardedEvent delivered = forwarded;
        delivered.type = "external";
        delivered.originType = SCE::Constants::SCXML_EVENT_PROCESSOR_TYPE;
        // Engine's ForwardedEvent overload handles graceful-degrade for
        // unknown events (§scxml-6.4) via PolicyType::getEventFromName.
        engine_.raiseExternal(delivered);
        return true;
    }

    bool isFinal() const override {
        return engine_.isInFinalState();
    }

    std::string getDonedata() override {
        // §scxml-5.5 + 6.4.3: surface the `<donedata>` payload that the
        // child stashed at top-level `<final>` entry. Shared single source
        // of truth with the local-invoke path — both read from
        // `StaticExecutionEngine::donedataAtFinal`, populated by the
        // `DoneDataHelper::evaluateParams` / `evaluateContent` /
        // `emitContentLiteral` pipeline, each of which now stores
        // canonical JSON (via `EventDataHelper::scriptValueToJsonString`).
        // No wire-side re-serialization needed: the parent's
        // `setPolicyMetadata` re-hydrates `typedData` by parsing this
        // JSON with `jsonStringToScriptValue` — the symmetric counterpart.
        return engine_.donedataAtFinal();
    }

    void cancel() override {
        cancelled_ = true;
    }

    const std::string &sessionId() const override {
        return session_id_;
    }

    const std::string &machineName() const override {
        return machine_name_;
    }

    const std::string &parentPeer() const override {
        return parent_peer_;
    }

    const std::string &invokeIdString() const override {
        return invoke_id_string_;
    }

    const std::array<uint8_t, 16> &invokeUuid() const override {
        return invoke_uuid_;
    }

    bool cancelled() const {
        return cancelled_;
    }

private:
    std::string machine_name_;
    std::string session_id_;
    std::array<uint8_t, 16> invoke_uuid_;
    std::string invoke_id_string_;
    std::string parent_peer_;
    ChildToParentPublisher wire16_;
    Engine engine_;
    bool cancelled_ = false;
};

}  // namespace SCE::Mesh
