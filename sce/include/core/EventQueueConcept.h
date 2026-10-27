// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include <type_traits>
#include <utility>

namespace SCE::Core {

/**
 * @brief §scxml-3.13: Event queue adapter contract
 *
 * Formalizes the duck-typed interface required by EventProcessingAlgorithms;
 * the AOT engine's `AOTEventQueue` satisfies it. The Interpreter's main event
 * loop takes its events straight off the raiser instead.
 *
 * Required methods, on a `Q &q`:
 * - q.hasEvents()  : Check if queue has pending events, convertible to bool
 * - q.popNext()    : Pop and return next event (return type varies per engine)
 *
 * A C++17 trait, so it means the same thing in a consumer's C++17 translation
 * unit and in SCE's C++20 ones (docs/adr/0059).
 */
template <typename Q, typename = void> struct EventQueueAdapterTrait : std::false_type {};

template <typename Q>
struct EventQueueAdapterTrait<
    Q, std::void_t<decltype(std::declval<Q &>().hasEvents()), decltype(std::declval<Q &>().popNext())>>
    : std::is_convertible<decltype(std::declval<Q &>().hasEvents()), bool> {};

}  // namespace SCE::Core
