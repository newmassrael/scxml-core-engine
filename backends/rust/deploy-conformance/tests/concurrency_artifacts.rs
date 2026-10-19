// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The LinkBus and the cooperative scheduler SCE writes for a machine with
//! links (SCE Protocol-Synthesis RFC §synth-5-N), compiled and called. Until
//! this crate existed they were compared as text, so a template edit that
//! stopped them compiling passed every test.

use sce_deploy_conformance::{mcu_node_link_bus, mcu_node_scheduler};
use sce_link_runtime::StubLink;
use tokio::sync::mpsc;

#[test]
fn the_scheduler_carries_the_declared_budgets_and_visits_links_in_order() {
    assert_eq!(mcu_node_scheduler::TICK_PERIOD_US, 1000);
    assert_eq!(mcu_node_scheduler::PER_LINK_BUDGET_US, 500);
    assert_eq!(mcu_node_scheduler::LINK_ORDER, &["udp_scout", "udp_data"]);

    let mut scout = StubLink::new();
    let mut data = StubLink::new();
    mcu_node_scheduler::tick(&mut scout, &mut data);
}

#[tokio::test]
async fn a_link_bus_routes_each_send_to_its_own_link() {
    let (scout_tx, mut scout_rx) = mpsc::channel(4);
    let (data_tx, mut data_rx) = mpsc::channel(4);
    let bus = mcu_node_link_bus::McuNodeLinkBus::new(scout_tx, data_tx);

    bus.send_udp_data(vec![1, 2, 3]).await.expect("send data");
    bus.send_udp_scout(vec![9]).await.expect("send scout");

    assert_eq!(data_rx.recv().await, Some(vec![1, 2, 3]));
    assert_eq!(scout_rx.recv().await, Some(vec![9]));
}
