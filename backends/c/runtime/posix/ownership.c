// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The host's definition of the ownership seam (`sce/sample.h`), where a
// violation of a sample's borrow contract goes (docs/adr/0110, D2).
//
// The violation is written to `stderr` as one line,
// `sce ownership violation: <what>`, and the process aborts. A host has a
// debugger and a core dump to show where, and a borrow that went wrong is not
// one the program can be trusted to go on from. The tests that watch a
// violation read this line and the abort, so the spelling is a contract of this
// file and not of the header.
//
// It is the same in every build: `NDEBUG` chooses nothing here, so the
// defensive layer that ships in release reports what it finds.

#include <stdio.h>
#include <stdlib.h>

#include "sce/sample.h"

void _sce_ownership_trap(const char *what) {
    (void)fprintf(stderr, "sce ownership violation: %s\n", what);
    abort();
}
