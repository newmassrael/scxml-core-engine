// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. AGPL-3.0 + Linking Exception: Free for unmodified use (see LICENSE-EXCEPTION.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $100 cumulative
//   Enterprise: $500 cumulative
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include <cstdint>

namespace SCE::Core {

/**
 * @brief How many links an `error.*` chain may have before an engine stops
 *        feeding it
 *
 * §scxml-3.12.2 says what to do with an error event nothing matches. It does not
 * say what to do when something *does* match it and that handler fails too: the
 * failure raises the same error, the same transition answers it, and the machine
 * has no way out. Nothing in the specification bounds that, so the number is the
 * engine's to choose, and both C++ engines choose this one.
 *
 * A hundred links is far past any repair strategy a document plausibly spells (a
 * handler that tries a fallback, then a second one, is three) and far short of a
 * number a host would wait through. In the Interpreter it also caps recursion:
 * executable content runs a nested drain, so each link is a stack frame.
 */
inline constexpr uint32_t MAX_ERROR_CASCADE_DEPTH = 100;

/**
 * @brief How many microsteps one macrostep may take before an engine stops
 *        taking them
 *
 * The specification defines a macrostep as a chain of microsteps ending in a
 * configuration where nothing is enabled by NULL and the internal queue is
 * empty, and its Principles and Constraints say in as many words that such a
 * chain need not exist: *"A microstep always terminates. A macrostep may not. A
 * macrostep that does not terminate may be said to consist of an infinitely long
 * sequence of microsteps. This is currently allowed."*
 *
 * So the ceiling is not conformance: it is an engine declining a document the
 * specification permits, which is exactly why the decline has to be visible
 * (`truncatedMacrosteps`).
 *
 * One budget for the whole inner loop, not one per branch. Appendix D's loop
 * takes a microstep on an eventless transition *or* on an internal event, and a
 * document alternating the two is one chain, not two: budgeting the branches
 * separately leaves that chain unbounded, which is what a per-call counter on the
 * eventless branch alone did until 2026-08-20.
 *
 * Ten times `MAX_ERROR_CASCADE_DEPTH`, and deliberately not equal to it. This is
 * the backstop; the cascade ceiling is a diagnostic that names the error a
 * handler keeps failing on, and a backstop that fires first makes that diagnostic
 * unreachable. Measured 2026-08-20: with both at a hundred, a handler that raises
 * one event of its own per link (two microsteps a link, which is what a document
 * that logs before it fails looks like) was cut at fifty links by this ceiling
 * and `errorCascadeEvents()` never moved. The factor of ten is the headroom that
 * keeps the specific report reachable for a handler raising up to eight events a
 * link; a busier one is cut here instead, which is coarser but still reported.
 */
inline constexpr uint32_t MAX_MACROSTEP_MICROSTEPS = 1000;

// The relation above is a property of the two numbers and not of either one, so
// it is checked where they are written. Moving one without the other is the
// edit this stops.
static_assert(MAX_MACROSTEP_MICROSTEPS == 10 * MAX_ERROR_CASCADE_DEPTH,
              "the microstep ceiling is ten times the error-cascade depth, so the cascade report stays reachable");

}  // namespace SCE::Core
