// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

namespace SCE {

enum class Type {
    ATOMIC,    // State with no child states
    COMPOUND,  // State with child states
    PARALLEL,  // Parallel state
    FINAL,     // Final state
    HISTORY,   // History pseudo-state
    INITIAL    // Initial pseudo-state
};

enum class HistoryType {
    NONE,     // Not a history state
    SHALLOW,  // Shallow history
    DEEP      // Deep history
};

}  // namespace SCE
