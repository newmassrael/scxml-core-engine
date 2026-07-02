// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §16.5 L3500 — barrier-timeout runtime driver.
//
// Dispatched into two ctest entries by CMake:
//
//   mesh_barrier_timeout_fires_e2e   (argv[1] = "fires")
//     Spawns a silent NonRoot + Root-with-timeout-expectation and
//     asserts both exit 0 — i.e. the timer fired, the scheduler
//     popped `error.communication`, and the SM transitioned into
//     `<final id="timeout_failed">`.
//
//   mesh_barrier_timeout_cancels_e2e (argv[1] = "cancels")
//     Spawns a sending NonRoot + Root-with-convergence-expectation
//     and asserts both exit 0 — i.e. the wire-21 envelope arrived
//     before the 150 ms timer, the tracker cancelled the scheduled
//     event, and the SM transitioned into `<final id="all_done">`.
//
// The cancels mode proves the cancel-wins-race invariant under a
// genuinely racy timeout; the fires mode proves the §16.7 row 6
// raise path is observable. Both modes use the SAME generated code
// (`deploy_partitions_barrier_fast.yaml` with 150 ms timeout) — the
// divergence lives entirely in the NonRoot's argv-selected behaviour
// and the Root's argv-selected assertion.
//
// Child binary paths and argv tokens come from CMake compile-defines
// so the harness remains location-agnostic.

#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>
#include <thread>

#include <sys/wait.h>
#include <unistd.h>

#ifndef MESH_BARRIER_TIMEOUT_ROOT_BIN
#error "MESH_BARRIER_TIMEOUT_ROOT_BIN must be defined by CMake"
#endif
#ifndef MESH_BARRIER_TIMEOUT_RIGHT_BIN
#error "MESH_BARRIER_TIMEOUT_RIGHT_BIN must be defined by CMake"
#endif

namespace {

pid_t spawn_with_arg(const char *binary, const char *arg) {
    pid_t pid = ::fork();
    if (pid < 0) {
        std::perror("fork");
        return -1;
    }
    if (pid == 0) {
        char *argv[] = {
            const_cast<char *>(binary),
            const_cast<char *>(arg),
            nullptr,
        };
        ::execv(binary, argv);
        std::fprintf(stderr, "execv(%s %s) failed: %s\n", binary, arg, std::strerror(errno));
        std::_Exit(127);
    }
    return pid;
}

bool wait_child(pid_t pid, const char *tag) {
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

int main(int argc, char *argv[]) {
    if (argc < 2) {
        std::fprintf(stderr, "driver: usage: %s <fires|cancels>\n", argv[0]);
        return 64;
    }
    const std::string mode = argv[1];
    const bool fires = (mode == "fires");
    const bool cancels = (mode == "cancels");
    if (!fires && !cancels) {
        std::fprintf(stderr, "driver: unknown mode '%s'\n", mode.c_str());
        return 65;
    }

    const char *right_arg = fires ? "silent" : "sends";
    const char *root_arg = fires ? "timeout" : "convergence";

    // Spawn the NonRoot first so the shm outbound is `Mode::Create`'d
    // before the Root starts `Mode::Open`-retrying on pump. 50 ms
    // mirrors the rule-12 harness's launch gap.
    pid_t right_pid = spawn_with_arg(MESH_BARRIER_TIMEOUT_RIGHT_BIN, right_arg);
    if (right_pid < 0) {
        return 200;
    }
    std::this_thread::sleep_for(std::chrono::milliseconds(50));

    pid_t root_pid = spawn_with_arg(MESH_BARRIER_TIMEOUT_ROOT_BIN, root_arg);
    if (root_pid < 0) {
        (void)wait_child(right_pid, "right");
        return 201;
    }

    bool root_ok = wait_child(root_pid, "root");
    bool right_ok = wait_child(right_pid, "right");

    if (root_ok && right_ok) {
        std::printf("SCE Mesh §16.5 barrier-timeout %s: PASS\n", mode.c_str());
        return 0;
    }
    std::fprintf(stderr,
                 "SCE Mesh §16.5 barrier-timeout %s: FAIL "
                 "(root_ok=%d right_ok=%d)\n",
                 mode.c_str(), static_cast<int>(root_ok), static_cast<int>(right_ok));
    return 1;
}
