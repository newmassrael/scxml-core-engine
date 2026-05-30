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
//     StaticExecutionEngine::isGlobalFinalState so WorkerSessionHost can
//     emit wire-18 `InvokeDone` and retire the session.
//
// Engine requirements (satisfied by StaticExecutionEngine<Policy>):
//   - Default-constructible. (Machines with non-default ctors — e.g. Hardware
//     bindings — are not yet supported as invoke targets; a future Session F
//     refinement will introduce a codegen hook for engine construction.)
//   - Provides `initialize()`, `tick()`, `isGlobalFinalState()`,
//     `setMeshSendCallback(...)`, and `raiseExternal(const std::string&,
//     const std::string&)`.

#pragma once

#include "common/Uuid.h"
#include "mesh/IChildSession.h"
#include "mesh/MeshEnvelope.h"
#include "mesh/PatternKind.h"

#include <array>
#include <cstdint>
#include <functional>
#include <string>
#include <utility>

namespace SCE::Mesh {

/// Publishes a child→parent envelope (wire-16 ChildEvent or wire-18 InvokeDone)
/// via the worker's transport router. Returns true on acceptance. Signature
/// matches the shape WorkerSessionHost::makeWire16Publisher() binds.
using ChildToParentPublisher = std::function<bool(const MeshEnvelope& env)>;

template <typename Engine>
class ChildSessionAdapter : public IChildSession {
public:
    ChildSessionAdapter(std::string machine_name,
                        std::string session_id,
                        std::array<uint8_t, 16> invoke_uuid,
                        std::string invoke_id_string,
                        std::string parent_peer,
                        ChildToParentPublisher wire16_publisher)
        : machine_name_(std::move(machine_name)),
          session_id_(std::move(session_id)),
          invoke_uuid_(invoke_uuid),
          invoke_id_string_(std::move(invoke_id_string)),
          parent_peer_(std::move(parent_peer)),
          wire16_(std::move(wire16_publisher)) {
        // Intercept <send target="#_parent"> on the engine and emit wire-16.
        // The child's mesh-send callback returns true when the send has been
        // accepted — preventing fall-through to the external queue (W3C
        // graceful-degrade path). Any other target returns false so the
        // engine's default handling (external queue or http send) applies.
        engine_.setMeshSendCallback(
            [this](const std::string& target,
                   const std::string& eventName,
                   const std::string& data,
                   const std::string& sendId,
                   const std::string& /*invokeId*/) -> bool {
                if (target != "#_parent") return false;
                if (cancelled_) return true;  // drop post-cancel sends
                MeshEnvelope env{};
                env.id = SCE::uuid::v7();
                env.source = machine_name_;
                env.type = eventName;
                env.pattern = PatternKind::ChildEvent;
                env.data.assign(data.begin(), data.end());
                env.invoke_id = invoke_uuid_;
                env.child_session_id = session_id_;
                if (!sendId.empty()) env.subject = sendId;
                return wire16_(env);
            });
    }

    Engine& engine() { return engine_; }

    void tick() override {
        if (cancelled_) return;
        engine_.tick();
    }

    bool raiseExternal(const std::string& eventName,
                       const std::string& data,
                       const std::string& /*sendId*/) override {
        if (cancelled_) return false;
        // Engine's string-based raiseExternal handles graceful-degrade for
        // unknown events (§scxml-6.4) and converts to the Event enum via
        // PolicyType::getEventFromName. sendId preservation parity with
        // local invoke is documented in IChildSession.h.
        engine_.raiseExternal(eventName, data);
        return true;
    }

    bool isFinal() const override {
        return engine_.isGlobalFinalState();
    }

    std::string getDonedata() override {
        // W3C SCXML 5.5 + 6.3.1: surface the `<donedata>` payload that the
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

    const std::string& sessionId() const override { return session_id_; }
    const std::string& machineName() const override { return machine_name_; }
    const std::string& parentPeer() const override { return parent_peer_; }
    const std::string& invokeIdString() const override { return invoke_id_string_; }
    const std::array<uint8_t, 16>& invokeUuid() const override { return invoke_uuid_; }

    bool cancelled() const { return cancelled_; }

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
