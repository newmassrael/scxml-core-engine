// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §16.8.1 — distributed conformance driver.
//
// Per-test invocation: spawns every partition binary via fork+exec,
// collects their exit codes, and compares the aggregate verdict
// against the known-correct W3C single-process outcome. The W3C IRP
// `conf:pass` / `conf:fail` final-state convention is the §16.2
// property-1 equivalence witness for this seed — test417 has no
// `<log>` elements, so the property-6 output-multiset check reduces
// to "did every partition reach a success verdict in its local
// configuration?". Property 1 equivalence for the Root is decided
// inside the Root's `run()` (checks SM reaches `<final id="pass">`);
// the NonRoot's success is "wire-21 was dispatched". The driver
// composes these local verdicts into the overall §16.8 PASS.
//
// Partition launch order: the NonRoot partition (worker) launches
// first so it creates the outbound wire-21 shm segment in
// `Mode::Create` before the Root tries `Mode::Open`. The Root retries
// open on every `pumpWire21Inbound()` poll, so this ordering is a CI
// noise optimisation rather than a correctness requirement — mirrors
// the 50 ms launch gap in `test_mesh_partition_rule12_e2e.cpp`.
//
// Cleanup contract: when a partition process exits (success or
// failure) its `TransportRouter` dtor `shm_unlink`s the wire-21
// segment. The driver additionally `waitpid`s every spawned child to
// avoid zombies under ctest -j. If a partition crashes before dtor,
// the segment survives until the OS reboots — `/dev/shm/sce_p21_*`
// can be purged manually without impacting other tests because
// SCE_MESH §16.8 seed partitions are per-test-named
// (`test417_worker_test417_main`), so no cross-test namespace
// collision is possible.

#include <cerrno>
#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <thread>

#include <sys/wait.h>
#include <unistd.h>

#ifndef SCE_W3C_DIST_TEST_ID
#error "SCE_W3C_DIST_TEST_ID must be defined by CMake"
#endif
#ifndef SCE_W3C_DIST_MAIN_BIN
#error "SCE_W3C_DIST_MAIN_BIN must be defined by CMake (absolute path to main-partition binary)"
#endif
#ifndef SCE_W3C_DIST_MAIN_PARTITION
#error "SCE_W3C_DIST_MAIN_PARTITION must be defined by CMake (main-partition name from deploy.yaml)"
#endif
#ifndef SCE_W3C_DIST_WORKER_BIN
#error "SCE_W3C_DIST_WORKER_BIN must be defined by CMake (absolute path to worker-partition binary)"
#endif
#ifndef SCE_W3C_DIST_WORKER_PARTITION
#error "SCE_W3C_DIST_WORKER_PARTITION must be defined by CMake (worker-partition name from deploy.yaml)"
#endif
#ifndef SCE_W3C_DIST_DEPLOY_DIR
#error "SCE_W3C_DIST_DEPLOY_DIR must be defined by CMake (build-tree directory holding deploy.yaml)"
#endif

namespace {

pid_t spawn_partition(const char *binary, const char *test_id, const char *partition,
                      const char *deploy_dir) {
    pid_t pid = ::fork();
    if (pid < 0) {
        std::perror("fork");
        return -1;
    }
    if (pid == 0) {
        // execv argv: mutable per POSIX but the kernel never modifies
        // the strings, so `const_cast` is safe. Mirrors the pattern in
        // `test_mesh_partition_rule12_e2e.cpp`.
        char arg_tid[] = "--test-id";
        char arg_par[] = "--partition";
        char arg_dep[] = "--deploy-dir";
        char *const argv[] = {const_cast<char *>(binary),
                              arg_tid,
                              const_cast<char *>(test_id),
                              arg_par,
                              const_cast<char *>(partition),
                              arg_dep,
                              const_cast<char *>(deploy_dir),
                              nullptr};
        ::execv(binary, argv);
        std::fprintf(stderr, "execv(%s) failed: %s\n", binary, std::strerror(errno));
        std::_Exit(127);
    }
    return pid;
}

struct ChildResult {
    pid_t pid;
    int exit_code;
    bool normal_exit;
};

ChildResult wait_child(pid_t pid) {
    ChildResult r{pid, -1, false};
    int status = 0;
    if (::waitpid(pid, &status, 0) < 0) {
        std::fprintf(stderr, "waitpid(%d) failed: %s\n", static_cast<int>(pid),
                     std::strerror(errno));
        return r;
    }
    if (WIFEXITED(status)) {
        r.normal_exit = true;
        r.exit_code = WEXITSTATUS(status);
    } else if (WIFSIGNALED(status)) {
        std::fprintf(stderr, "child %d killed by signal %d\n", static_cast<int>(pid),
                     WTERMSIG(status));
    }
    return r;
}

}  // namespace

int main() {
    const char *test_id = SCE_W3C_DIST_TEST_ID;
    const char *deploy_dir = SCE_W3C_DIST_DEPLOY_DIR;

    // Launch worker first (creates shm segment), then main (opens it).
    pid_t worker_pid = spawn_partition(SCE_W3C_DIST_WORKER_BIN, test_id,
                                       SCE_W3C_DIST_WORKER_PARTITION, deploy_dir);
    if (worker_pid < 0) {
        return 200;
    }
    std::this_thread::sleep_for(std::chrono::milliseconds(50));

    pid_t main_pid = spawn_partition(SCE_W3C_DIST_MAIN_BIN, test_id,
                                     SCE_W3C_DIST_MAIN_PARTITION, deploy_dir);
    if (main_pid < 0) {
        // Reap the worker already in flight before we exit — ctest's
        // process table should not have to carry our zombies.
        (void)wait_child(worker_pid);
        return 201;
    }

    ChildResult main_res = wait_child(main_pid);
    ChildResult worker_res = wait_child(worker_pid);

    bool ok = main_res.normal_exit && main_res.exit_code == 0 && worker_res.normal_exit &&
              worker_res.exit_code == 0;

    if (ok) {
        std::fprintf(stdout,
                     "SCE Mesh §16.8 distributed conformance: test%s PASS (main=0 worker=0)\n",
                     test_id);
        return 0;
    }

    std::fprintf(stderr,
                 "SCE Mesh §16.8 distributed conformance: test%s FAIL "
                 "(main_normal=%d main_code=%d worker_normal=%d worker_code=%d)\n",
                 test_id, static_cast<int>(main_res.normal_exit), main_res.exit_code,
                 static_cast<int>(worker_res.normal_exit), worker_res.exit_code);
    return 1;
}
