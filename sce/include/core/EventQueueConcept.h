// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#if __cpp_concepts >= 202002L
#include <concepts>

namespace SCE::Core {

/**
 * @brief W3C SCXML 3.12.1: Event queue adapter contract
 *
 * Formalizes the duck-typed interface required by EventProcessingAlgorithms.
 * Both AOT (AOTEventQueue) and Interpreter (InterpreterEventQueue) adapters
 * must satisfy this concept.
 *
 * Required methods:
 * - bool hasEvents() const  : Check if queue has pending events
 * - auto popNext()          : Pop and return next event (return type varies per engine)
 */
template <typename Q>
concept EventQueueAdapter = requires(Q q) {
    { q.hasEvents() } -> std::convertible_to<bool>;
    { q.popNext() };
};

}  // namespace SCE::Core

#else  // C++17 fallback

namespace SCE::Core {
// C++17: No concept check — duck typing via templates.
// EventProcessingAlgorithms will rely on implicit template requirements.
}  // namespace SCE::Core

#endif  // __cpp_concepts >= 202002L
