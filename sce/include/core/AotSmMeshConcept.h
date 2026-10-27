// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
// the generated SM class. Until this contract existed, the two templates
// reached the same contract by Jinja-literal mirror — drift between
// them surfaced only as compile errors in user-facing generated code
// (or worse, silent miscompile if a typedef was renamed on one side
// and the other side picked up an ambient name).
//
// `AotSmMeshIntegrationTrait` lifts the mirror contract into a typed
// requirement. The mesh template's `TransportRouter` class body
// `static_assert`s the trait on `SenderEngine`, so any drift between
// the AOT SM emitter and the mesh transport emitter is caught at the
// `TransportRouter<MyEngine>` instantiation site, in the project's own
// compile rather than downstream user code.
//
// A C++17 trait, so the check is the same in a consumer's C++17 translation
// unit and in SCE's C++20 ones (docs/adr/0059).

#include <type_traits>

namespace SCE::Core {

/// AOT-emitted state machine integrating with mesh transport must
/// expose three nested typenames. The mesh template parameterises on
/// the engine type and reaches into these typedefs for ShmChannel
/// drain, envelope dispatch, and invoke-completion event construction.
template <typename T, typename = void> struct AotSmMeshIntegrationTrait : std::false_type {};

template <typename T>
struct AotSmMeshIntegrationTrait<T,
                                 std::void_t<typename T::PolicyType, typename T::EventWithMetadata, typename T::Event>>
    : std::true_type {};

}  // namespace SCE::Core
