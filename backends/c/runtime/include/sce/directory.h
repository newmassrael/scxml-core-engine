// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Making a directory: the one service of a file system the runner program of a
// generated test suite asks of its platform beyond the files of ISO C
// (docs/SCE_TTCN3_LANE.md §4, `--log-dir`, "made where it is missing";
// sce/test/runner.h). ISO C reads and writes a file and makes no directory, and
// this tier holds no POSIX in its headers, so the symbol is resolved at link
// time, as `_sce_clock_now_ms` (sce/clock.h) is, by whichever implementation
// tier the program links:
//
//   - `sce_c_runtime_posix` (backends/c/runtime/posix/directory.c) — POSIX
//     `mkdir`, the reference implementation for a hosted program.
//
//   - A downstream platform tier, for a host that runs the program elsewhere.
//     It must satisfy the contract below.
//
// Only a program that runs a suite's runner references it: a unit that includes
// a generated suite and never calls its `main` links without it.

#ifndef SCE_DIRECTORY_H
#define SCE_DIRECTORY_H

#ifdef __cplusplus
extern "C" {
#endif

// Makes the directory `path` and each directory above it that is missing.
// Returns 0 where `path` is a directory once it returns, whether or not it was
// before; otherwise the `errno` value of the step that failed, which `strerror`
// names.
int sce_directory_make(const char *path);

#ifdef __cplusplus
}
#endif

#endif /* SCE_DIRECTORY_H */
