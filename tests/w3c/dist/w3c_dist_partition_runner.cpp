// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §mesh-16.8.1 — distributed conformance partition runner main.
//
// Each `w3c_dist_<test_id>_<partition>` binary is this source file
// compiled together with exactly one `Test<seed>_<partition>.h`
// header — selected via the `SCE_W3C_DIST_PARTITION_HEADER`
// preprocessor define the CMake macro injects per target. The
// CRTP registrar at the header's namespace scope pushes one entry
// into `PartitionRegistry` during static init; `main` looks up the
// matching entry by (--test-id, --partition) and returns the
// partition's verdict as the process exit code.
//
// Exit code contract:
//   0     : partition reached success verdict (§mesh-16.2 property 1 locally).
//   11-19 : partition-local failure (see individual partition's `run()`
//           documentation; e.g. Test417_Main returns 11 on <final id="fail">).
//   2     : CLI argument parse failure.
//   3     : (test_id, partition) not found in registry — indicates the
//           CMake macro wired the wrong SCE_W3C_DIST_PARTITION_HEADER or
//           the manifest drifted from the linked CRTP registrations.
//
// `--deploy-dir` is accepted but not yet load-bearing. It is threaded
// into the partition's `run()` for forward compatibility with
// partitions that need runtime access to the deploy.yaml (e.g. to
// resolve `custom_tcp` `listen:` addresses when that transport's
// wire-21 emitter lands); Test417's shm transport takes its channel
// names from the codegen-baked constants and ignores the argument.

#include "PartitionRunner.h"

#ifndef SCE_W3C_DIST_PARTITION_HEADER
#error "SCE_W3C_DIST_PARTITION_HEADER must be defined by the CMake macro that builds this TU"
#endif

#include SCE_W3C_DIST_PARTITION_HEADER

#include <cstdio>
#include <cstring>
#include <string>

namespace {

struct CliArgs {
    std::string test_id;
    std::string partition;
    std::string deploy_dir;
};

bool parse_args(int argc, char **argv, CliArgs &out) {
    for (int i = 1; i < argc; ++i) {
        const char *a = argv[i];
        if (std::strcmp(a, "--test-id") == 0 && i + 1 < argc) {
            out.test_id = argv[++i];
        } else if (std::strcmp(a, "--partition") == 0 && i + 1 < argc) {
            out.partition = argv[++i];
        } else if (std::strcmp(a, "--deploy-dir") == 0 && i + 1 < argc) {
            out.deploy_dir = argv[++i];
        } else {
            std::fprintf(stderr, "unknown or incomplete argument: %s\n", a);
            return false;
        }
    }
    return !out.test_id.empty() && !out.partition.empty();
}

}  // namespace

int main(int argc, char **argv) {
    CliArgs args;
    if (!parse_args(argc, argv, args)) {
        std::fprintf(stderr, "usage: w3c_dist_<testid>_<partition> --test-id <id> --partition <name> "
                             "[--deploy-dir <path>]\n");
        return 2;
    }

    auto partition = SCE::W3C::Dist::PartitionRegistry::instance().create(args.test_id, args.partition);
    if (!partition) {
        std::fprintf(stderr,
                     "no partition registered for (test_id=%s, partition=%s). "
                     "The binary was built with a CRTP header that does not register this key.\n",
                     args.test_id.c_str(), args.partition.c_str());
        return 3;
    }
    return partition->run(args.deploy_dir);
}
