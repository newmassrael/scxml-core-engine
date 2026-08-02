// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-14
//
// SCE Mesh §14 — first-party `partitions:` ctest fixture.
//
// Until this fixture landed, every `partitions:` rule (rules 6-10 in
// `sce-build/src/mesh/deploy.rs`, rules 1/2/11 in
// `sce-build/src/mesh/partitions.rs`) was covered exclusively by Rust
// unit tests. No tree-level deploy.yaml declared a `partitions:` block,
// so the cross-stage path
//
//     parse_deploy
//       → validate_partitions_schema       (rules 6, 7, 8, 9, 10)
//       → validate_partitions_against_scxml (rules 1, 2, 11)
//       → compile_mesh_transport
//
// was never exercised by the C++ build. A regression that broke any
// of those gates without breaking the Rust-side unit suite (e.g., a
// CLI wiring that bypassed `validate_partitions_against_scxml`, or a
// codegen change that surfaced a different error before partition
// validation could run) would have shipped silently.
//
// The fixture closes that gap. `deploy_partitions.yaml` declares two
// partitions covering the two `<parallel>` regions of
// `motor_partition.scxml`. The build-time pass condition is twofold:
//
//   (a) `sce-codegen --deploy` exits 0 — proves the §14 rule chain
//       accepted the deploy graph, the SCXML, and their cross-product
//       coverage. A regression in any rule's accept path here turns
//       this CMake target red.
//
//   (b) The generated `motor_partition_sm.h` compiles into this TU —
//       proves the SCXML codegen also produced a valid header (the
//       `--deploy` invocation regenerates the SM header as a side
//       effect, per the existing mesh_compile_test pattern).
//
// The `motor_partition` machine has no `bindings:`, so
// `compile_mesh_transport` reaches its early-return at
// `sce-build/src/lib.rs:1092` after partition validation passes and
// emits no `transport.h`. That's intentional — this fixture's job is
// to exercise the §14 path, not to add another transport-router
// instance to the C++ surface (the existing mesh_*_compile_*
// fixtures already cover every transport binding combination).

#include "motor_partition_sm.h"

#include <cstdio>

int main() {
    SCE::Generated::motor_partition::motor_partition sm;
    (void)sm;
    std::printf("SCE Mesh \u00a714 partitions: PASS\n");
    return 0;
}
