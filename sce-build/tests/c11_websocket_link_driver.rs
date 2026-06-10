//! Spec C11 follow-up — WebSocket link driver (`websocket_tcp`)
//! deploy.yaml allowlist.
//!
//! watching-zenoh RFC §synth-8 Q8 line 3747 names `websocket_tcp` as one
//! of the six core-shipped drivers; §synth-5-C row 4 (line 770) names the
//! `websocket` link class with "TCP + WebSocket framing" semantics.
//! Spec §synth-7 item C11 ("Serial + WebSocket link drivers", line 3626)
//! commits this driver. The serial sub-scope landed via `4f1c8bfa`
//! (2026-05-14, see `c11_serial_link_driver.rs`); this file covers
//! the WebSocket sub-scope.
//!
//! SCE-side support pinned here: the `KNOWN_DRIVERS`
//! baseline in [`sce_build::mesh::deploy::validate_links`] gains
//! the `("websocket_tcp", 40)` entry. Floor `40` matches `lwip_tcp`
//! — `websocket_tcp` runs over IPv4 + TCP and inherits that
//! encapsulation overhead. The per-frame WebSocket header (RFC 6455
//! 2-14 bytes) is application-protocol framing and lives in the
//! §synth-5-B framer codec, not in the driver MTU floor — same layer
//! split that lets `serial_uart` carry floor 0.
//!
//! The driver↔class cross-validator (e.g. rejecting
//! `class=websocket` + `driver=lwip_tcp`) is a separate surface,
//! covered by `c11_driver_class_cross_validator.rs` on a 4×4
//! (4 drivers × 4 classes) matrix; this file pins only the
//! allowlist entry.

use sce_build::mesh::deploy::parse_deploy_str;
use sce_build::mesh::error::DeployError;

/// Standard MCU deploy.yaml prelude (mirror of
/// `c11_serial_link_driver.rs` and `c13_links_block.rs`). Carries
/// one machine with platform + scheduler + memory; only the
/// `links:` block changes per-test.
fn deploy_prelude_with_links(links_yaml: &str) -> String {
    format!(
        r#"
version: "1.0"
topology:
  mcu_device:
    machines:
      mcu_node:
        source: mcu_node.scxml
        platform:
          class: mcu
          os: bare_metal
          has_dcache: true
          dcache_line_size: 32
          has_speculative_prefetch: false
          core_count: 1
          clock_freq_mhz: 400
          memcpy_cycles_per_byte: 1.0
        scheduler:
          kind: cooperative
          tick_period_us: 1000
          worker_stack_budget: 4096
          worker_slot_budget_us: 200
          keepalive_jitter_budget_us: 5000
        memory:
          sram_regions:
            sram1: {{ base: 0x08000000, size: 65536, attr: [dma_coherent, cacheable] }}
          dma_channels: [DW0_CH0]
        links:
{links_yaml}
"#,
    )
}

/// Happy path — `driver: websocket_tcp` with only the required
/// `bind` + `driver` fields parses and validates. Verifies the new
/// allowlist entry is reachable; optional fields default to `None`
/// just like other drivers.
#[test]
fn websocket_tcp_driver_accepted_minimal() {
    let yaml = deploy_prelude_with_links(
        r#"          ws_control:
            bind: "0.0.0.0:8080"
            driver: websocket_tcp
"#,
    );
    let cfg = parse_deploy_str(&yaml).expect("websocket_tcp minimal binding parses");
    let link = cfg
        .topology
        .get("mcu_device")
        .and_then(|d| d.machines.get("mcu_node"))
        .and_then(|m| m.links.get("ws_control"))
        .expect("ws_control link entry");
    assert_eq!(link.bind, "0.0.0.0:8080");
    assert_eq!(link.driver, "websocket_tcp");
    assert_eq!(link.mtu_bytes, None);
    assert_eq!(link.expected_p99_bytes, None);
}

/// `websocket_tcp` with a realistic WS payload size (512 bytes)
/// parses cleanly. Above the 40-byte TCP-encapsulation floor, so
/// `link/mtu-below-driver-floor` does not fire.
#[test]
fn websocket_tcp_with_realistic_mtu_passes() {
    let yaml = deploy_prelude_with_links(
        r#"          ws_data:
            bind: "0.0.0.0:8080"
            driver: websocket_tcp
            mtu_bytes: 512
"#,
    );
    let cfg = parse_deploy_str(&yaml).expect("websocket_tcp + realistic mtu parses");
    let link = &cfg.topology["mcu_device"].machines["mcu_node"].links["ws_data"];
    assert_eq!(link.driver, "websocket_tcp");
    assert_eq!(link.mtu_bytes, Some(512));
}

/// Negative — `mtu_bytes: 30` is below `websocket_tcp`'s floor of
/// 40 (IPv4 + TCP encapsulation). Fires `link/mtu-below-driver-
/// floor`. Symmetric inversion of `serial_uart_below_ip_stack_
/// floor_passes`: that test proved serial_uart's floor=0 is
/// permissive; this test proves websocket_tcp's floor=40 IS
/// enforced. Pins the TCP-encapsulation-only semantic against
/// silent regression — if someone later inflates the floor to
/// `40 + 2` (counting the minimum WS frame header), this test
/// stays correct (30 < 40 < 42) but a follow-up author would need
/// to refresh the doc-comment in `validate_links` to explain the
/// new layer-of-overhead inclusion.
#[test]
fn websocket_tcp_below_tcp_floor_fires() {
    let yaml = deploy_prelude_with_links(
        r#"          ws_short:
            bind: "0.0.0.0:8080"
            driver: websocket_tcp
            mtu_bytes: 30
"#,
    );
    let err = parse_deploy_str(&yaml).expect_err("mtu below websocket_tcp floor rejects");
    let DeployError::LinkMtuBelowDriverFloor {
        machine,
        link_name,
        driver,
        declared_mtu,
        driver_floor,
    } = err
    else {
        panic!("expected LinkMtuBelowDriverFloor, got {err:?}");
    };
    assert_eq!(machine, "mcu_node");
    assert_eq!(link_name, "ws_short");
    assert_eq!(driver, "websocket_tcp");
    assert_eq!(declared_mtu, 30);
    assert_eq!(driver_floor, 40);
}

/// Negative — unknown driver name typo. The candidate set returned
/// in the error must include `websocket_tcp` in sorted alphabetical
/// position alongside `lwip_tcp` + `lwip_udp` + `serial_uart`. This
/// pins the new allowlist entry's visibility to authors hitting
/// `link-driver-unknown` so they discover the WebSocket driver
/// naturally.
#[test]
fn unknown_websocket_typo_lists_websocket_tcp_in_candidates() {
    let yaml = deploy_prelude_with_links(
        r#"          ws_typo:
            bind: "0.0.0.0:8080"
            driver: websocket_xyz
"#,
    );
    let err = parse_deploy_str(&yaml).expect_err("typo'd driver rejects");
    let DeployError::LinkDriverUnknown {
        machine,
        link_name,
        driver,
        candidates,
        ..
    } = err
    else {
        panic!("expected LinkDriverUnknown, got {err:?}");
    };
    assert_eq!(machine, "mcu_node");
    assert_eq!(link_name, "ws_typo");
    assert_eq!(driver, "websocket_xyz");
    // Sorted KNOWN_DRIVERS baseline after spec C11 WebSocket
    // follow-up: lwip_tcp / lwip_udp / serial_uart / websocket_tcp.
    assert_eq!(
        candidates,
        vec![
            "lwip_tcp".to_string(),
            "lwip_udp".to_string(),
            "serial_uart".to_string(),
            "websocket_tcp".to_string(),
        ]
    );
}
