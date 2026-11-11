// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The POSIX implementation of `sce_directory_make` (sce/directory.h): the
// directory the runner program of a generated test suite writes its logs to,
// made where it is missing, with each directory above it, as `mkdir -p` makes
// one (docs/SCE_TTCN3_LANE.md §4).

#define _POSIX_C_SOURCE 200809L

#include "sce/directory.h"

#include <errno.h>
#include <stddef.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>

// The longest path made. A longer one is refused as the system refuses it.
#define SCE_DIRECTORY_PATH_MAX 4096u

// Whether `path` names a directory now.
static int sce_directory_is(const char *path) {
    struct stat held;
    return stat(path, &held) == 0 && S_ISDIR(held.st_mode);
}

// `path`, made, every directory above it being there already: 0 where it is a
// directory once this returns, the `errno` of the failure otherwise.
static int sce_directory_make_one(const char *path) {
    if (mkdir(path, (mode_t)0777) == 0) {
        return 0;
    }
    const int failed = errno;
    if (failed == EEXIST && sce_directory_is(path)) {
        return 0;
    }
    return failed;
}

int sce_directory_make(const char *path) {
    char held[SCE_DIRECTORY_PATH_MAX];
    const size_t n = strlen(path);
    if (n == 0u) {
        return ENOENT;
    }
    if (n >= sizeof(held)) {
        return ENAMETOOLONG;
    }
    memcpy(held, path, n + 1u);
    // Each directory above it, from the top down: the path up to each `/` that
    // does not start it.
    for (size_t at = 1u; at < n; ++at) {
        if (held[at] != '/') {
            continue;
        }
        held[at] = '\0';
        const int failed = sce_directory_make_one(held);
        held[at] = '/';
        if (failed != 0) {
            return failed;
        }
    }
    return sce_directory_make_one(held);
}
