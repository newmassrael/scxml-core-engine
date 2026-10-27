// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Consumer probe for the C++ an installed SCE generates.
//
// CMakeLists.txt generates probe_generated.scxml with the INSTALLED
// SCECodegen.cmake and sce-codegen, links SCE::sce_base and nothing else, and
// compiles this file at C++17 with warnings as errors. The generated header
// includes the Forge C++ runtime (<sce/forge/algorithm.h>,
// <sce/forge/optional.h>), so the build fails if the install does not ship
// those headers or the tier does not carry their include directory, and the
// run fails if the checked arithmetic they hold does not refuse an overflow.

#include "probe_generated_sm.h"

#include <cstdio>
#include <string>

// docs/adr/0059, section 1: a consumer that links SCE is never raised above
// C++17. CMake compiles a target at the highest standard any of its usage
// requirements names, so a tier that declared cxx_std_20 would raise this file
// without a word; the standard it was actually compiled at is asked here.
#if defined(_MSVC_LANG)
static_assert(_MSVC_LANG == 201703L, "SCE raised its consumer above C++17");
#else
static_assert(__cplusplus == 201703L, "SCE raised its consumer above C++17");
#endif

int main() {
    SCE::Generated::probe_generated::probe_generated machine;
    machine.initialize();
    machine.step();

    // 250 + 3 is 253; 253 + 3 does not fit a uint8, so the second assignment is
    // refused and error.execution is counted.
    machine.raiseExternal(std::string("up"));
    machine.step();
    machine.raiseExternal(std::string("up"));
    machine.step();
    // Taken only when level is 253 and one refusal was counted.
    machine.raiseExternal(std::string("check"));
    machine.step();

    if (!machine.isInFinalState()) {
        std::fputs("probe_generated: the machine did not reach `done`, so the checked "
                   "addition of the installed Forge runtime did not refuse 253 + 3\n",
                   stderr);
        return 1;
    }
    std::puts("probe_generated: generated C++ built and ran against the installed SCE");
    return 0;
}
