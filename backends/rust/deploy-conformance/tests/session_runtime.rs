// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The session runtime a deploy declares (SCE Protocol-Synthesis RFC §synth-5-K,
//! Session runtime), as the host that reads it sees it: the const says what was
//! declared, and the runtime the generated constructor builds is the one the
//! declaration names.

use sce_deploy_conformance::{own_thread_node_session_runtime, pool_node_session_runtime};
use tokio::runtime::RuntimeFlavor;

#[test]
fn an_own_thread_deploy_yields_a_const_and_a_runtime_without_workers() {
    use own_thread_node_session_runtime::{
        build_session_runtime, SessionDrive, SessionRuntime, SESSION_RUNTIME,
    };

    assert_eq!(
        SESSION_RUNTIME,
        SessionRuntime {
            drive: SessionDrive::OwnThread,
            rx_busy_poll_us: 0,
        }
    );
    let runtime = build_session_runtime().expect("a current-thread runtime builds");
    assert_eq!(
        runtime.handle().runtime_flavor(),
        RuntimeFlavor::CurrentThread
    );
}

#[test]
fn a_worker_pool_deploy_yields_a_runtime_with_the_declared_workers_and_budget() {
    use pool_node_session_runtime::{
        build_session_runtime, SessionDrive, SessionRuntime, SESSION_RUNTIME,
    };

    assert_eq!(
        SESSION_RUNTIME,
        SessionRuntime {
            drive: SessionDrive::WorkerPool { worker_threads: 3 },
            rx_busy_poll_us: 10,
        }
    );
    let runtime = build_session_runtime().expect("a multi-thread runtime builds");
    assert_eq!(
        runtime.handle().runtime_flavor(),
        RuntimeFlavor::MultiThread
    );
    assert_eq!(runtime.metrics().num_workers(), 3);
}

#[test]
fn a_task_spawned_on_the_runtime_runs_to_completion_on_both_drives() {
    // A runtime that builds but cannot run a task would pass the two tests
    // above. `enable_all` is what lets a session use its timers and sockets.
    let own = own_thread_node_session_runtime::build_session_runtime().expect("build");
    let answer = own.block_on(async {
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        7
    });
    assert_eq!(answer, 7);

    let pool = pool_node_session_runtime::build_session_runtime().expect("build");
    let answer = pool.block_on(async {
        tokio::spawn(async {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            9
        })
        .await
        .expect("the spawned task finishes")
    });
    assert_eq!(answer, 9);
}
