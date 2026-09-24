// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include <string>
#include <vector>

namespace SCE {

/**
 * @brief What the interactive visualizer is told about one transition a
 *        microstep considered
 *
 * `StateMachine::getLastEnabledTransitions()` returns one of these for every
 * transition the last selection enabled, and `getLastOptimalTransitions()` for
 * the ones that survived §scxml-D-removeConflictingTransitions — so a host can
 * show which transition preempted which, and what each one's exit set was.
 *
 * A description, not an input: the microstep itself runs on
 * `SCE::Core::EnabledTransition`, and these are written from it after the
 * selection, read off the configuration the transitions were about to exit.
 */
struct TransitionDescriptorString {
    std::string source;                // Source state ID
    std::string target;                // First target as written; empty for a targetless transition
    std::string event;                 // Event name that selected it; empty for an eventless selection
    std::vector<std::string> exitSet;  // Appendix D's computeExitSet for this transition alone
    int transitionIndex = 0;           // Position in the source state's transition list
    bool hasActions = false;           // Whether the transition carries executable content
    bool isInternal = false;           // Written `type="internal"`
    bool isExternal = false;           // Its exit set leaves a `<parallel>`
    bool isTargetless = false;         // No `target`: exits and enters nothing
};

}  // namespace SCE
