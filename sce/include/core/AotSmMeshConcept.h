// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

// SCE Mesh — AOT-emitted state machine integration contract for mesh
// transport code generation.
//
// The mesh `TransportRouter` template (emitted by
// `tools/codegen/templates/mesh/cpp/mesh_transport.h.jinja2`) takes a
// `SenderEngine` template parameter that must expose:
//   • `using PolicyType = ...`            // ShmChannel drain / dispatchEnvelope
//   • `using EventWithMetadata = ...`     // wire-15 invoke completion routing
//   • `using Event = ...`                 // enum with Done_invoke / Error_invoke entries
// These are emitted by `tools/codegen/templates/state_machine.jinja2` on
// the generated SM class. Until this concept existed, the two templates
// reached the same contract by Jinja-literal mirror — drift between
// them surfaced only as compile errors in user-facing generated code
// (or worse, silent miscompile if a typedef was renamed on one side
// and the other side picked up an ambient name).
//
// `AotSmMeshIntegration` lifts the mirror contract into a typed
// requirement. The mesh template's `TransportRouter` class body
// `static_assert`s the concept on `SenderEngine`, so any drift between
// the AOT SM emitter and the mesh transport emitter is caught at the
// `TransportRouter<MyEngine>` instantiation site, in the project's own
// compile rather than downstream user code.
//
// C++17 gating mirrors `EventQueueConcept.h` / `StatePolicyConcepts.h` —
// C++20 builds get the typed concept check; C++17 builds fall back to
// duck typing via the existing Jinja-literal contract. Per
// `cpp17_compat.md` memory: SCE_core lowered to cxx_std_17 with
// concepts guarded.

#if __cpp_concepts >= 202002L
#include <concepts>

namespace SCE::Core {

/// AOT-emitted state machine integrating with mesh transport must
/// expose three nested typenames. The mesh template parameterises on
/// the engine type and reaches into these typedefs for ShmChannel
/// drain, envelope dispatch, and invoke-completion event construction.
template <typename T>
concept AotSmMeshIntegration = requires {
    typename T::PolicyType;
    typename T::EventWithMetadata;
    typename T::Event;
};

}  // namespace SCE::Core

#else  // C++17 fallback

namespace SCE::Core {
// C++17: no concept check — the AOT SM contract is duck-typed via the
// existing Jinja-literal mirror between state_machine.jinja2 and
// mesh_transport.h.jinja2. Drift surfaces as a compile error at the
// user's TransportRouter<MyEngine> instantiation site.
}  // namespace SCE::Core

#endif  // __cpp_concepts >= 202002L
