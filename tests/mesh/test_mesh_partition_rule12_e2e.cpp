// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §14 rule 12 / §16.5 — inter-partition wire-21 driver test.
//
// Forks two child processes that each `execv` a partition-specific
// binary:
//
//   motor_right (NonRoot) — drives `<final id="right_done">` and emits
//                           a wire-21 `ParallelRegionDone` envelope to
//                           motor_left over a per-partition-pair shm
//                           channel (`/sce_p21_motor_right_motor_left`).
//   motor_left  (Root)    — drives `<final id="left_done">`, drains the
//                           inbound shm channel, raises
//                           `done.state.root` via the §16.5 tracker,
//                           and asserts the SM enters `<final
//                           id="all_done">`.
//
// Driver waits for both to exit and asserts both returned 0. Either
// child's failure surfaces as a non-zero ctest result; the driver's
// own setup failures (fork / exec / waitpid) use a disjoint exit-code
// space (200+) so the source of the failure is unambiguous.
//
// MESH_PARTITION_RULE12_LEFT_BIN / RIGHT_BIN come from CMake via
// `target_compile_definitions` → `$<TARGET_FILE:...>`. They are
// absolute paths into the build tree, so the harness is location-
// agnostic and survives `ctest --output-on-failure` from any cwd.

#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <thread>

#include <sys/wait.h>
#include <unistd.h>

#ifndef MESH_PARTITION_RULE12_LEFT_BIN
#error "MESH_PARTITION_RULE12_LEFT_BIN must be defined by CMake"
#endif
#ifndef MESH_PARTITION_RULE12_RIGHT_BIN
#error "MESH_PARTITION_RULE12_RIGHT_BIN must be defined by CMake"
#endif

namespace {

pid_t spawn(const char* binary) {
    pid_t pid = ::fork();
    if (pid < 0) {
        std::perror("fork");
        return -1;
    }
    if (pid == 0) {
        // Argv must be a non-const char*[] for execv; cast away const
        // since the kernel does not modify the strings.
        char* argv[] = {const_cast<char*>(binary), nullptr};
        ::execv(binary, argv);
        // Unreachable on success; print and abort on exec failure.
        std::fprintf(stderr, "execv(%s) failed: %s\n", binary, std::strerror(errno));
        std::_Exit(127);
    }
    return pid;
}

bool wait_child(pid_t pid, const char* tag) {
    int status = 0;
    if (::waitpid(pid, &status, 0) < 0) {
        std::fprintf(stderr, "waitpid(%s) failed: %s\n", tag, std::strerror(errno));
        return false;
    }
    if (!WIFEXITED(status)) {
        std::fprintf(stderr, "%s did not exit normally (status=%d)\n", tag, status);
        return false;
    }
    int rc = WEXITSTATUS(status);
    if (rc != 0) {
        std::fprintf(stderr, "%s exited with code %d\n", tag, rc);
        return false;
    }
    std::fprintf(stdout, "%s: PASS\n", tag);
    return true;
}

}  // namespace

int main() {
    // Spawn the NonRoot first so it creates the shm segment before the
    // Root opens it. The Root retries open on every pump, so this
    // ordering is an optimization, not a correctness requirement, but
    // it eliminates the (tiny) startup race noise from CI logs.
    pid_t right_pid = spawn(MESH_PARTITION_RULE12_RIGHT_BIN);
    if (right_pid < 0) {
        return 200;
    }
    // 50 ms is enough for the right side to construct the router and
    // call `Mode::Create` on the channel — well under the right side's
    // 2-second hold-alive window. Mirrors the brake/motor harness's
    // 50 ms launch gap (test_mesh_shm_runtime.cpp).
    std::this_thread::sleep_for(std::chrono::milliseconds(50));

    pid_t left_pid = spawn(MESH_PARTITION_RULE12_LEFT_BIN);
    if (left_pid < 0) {
        // Right is already running — reap it so we don't leave a
        // zombie behind for ctest's process table. Exit code is the
        // driver-space 201 either way.
        (void)wait_child(right_pid, "motor_right");
        return 201;
    }

    bool left_ok = wait_child(left_pid, "motor_left");
    bool right_ok = wait_child(right_pid, "motor_right");

    if (left_ok && right_ok) {
        std::printf("SCE Mesh §14 rule 12 inter-partition wire-21: PASS\n");
        return 0;
    }
    std::fprintf(stderr,
                 "SCE Mesh §14 rule 12 inter-partition wire-21: FAIL "
                 "(left_ok=%d right_ok=%d)\n",
                 static_cast<int>(left_ok), static_cast<int>(right_ok));
    return 1;
}
