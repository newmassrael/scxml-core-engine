//! Spec C11 — Serial link driver (`serial_uart`) deploy.yaml allowlist.
//!
//! watching-zenoh RFC §5.C line 729 names `serial_uart` as the lwIP
//! crate's UART driver alongside `lwip_udp` + `lwip_tcp` + `raw_eth`.
//! Spec §7 line 3615 ("Serial + WebSocket link drivers") commits this
//! to Phase C atomic #11.
//!
//! This atomic delivers SCE-side support: the `KNOWN_DRIVERS` baseline
//! in [`sce_build::mesh::deploy::validate_links`] gains the
//! `("serial_uart", 0)` entry. Floor `0` reflects that UART has no
//! IP-stack overhead; the §5.B framer carries the frame-size invariant
//! at the protocol-decoder layer (see deploy.rs doc-comment above the
//! const for the rationale).
//!
//! WebSocket follow-up + driver↔class cross-validator follow-up are
//! tracked separately per RFC §5.1 + §5.2 with explicit re-entry
//! triggers. They are NOT in scope for this atomic — the cross-class
//! pass-through (e.g. `driver: serial_uart` with no class context
//! check at deploy layer) is the pre-existing silently-broken hook
//! that the cross-validator follow-up will close.

use sce_build::mesh::deploy::parse_deploy_str;
use sce_build::mesh::error::DeployError;

/// Standard MCU deploy.yaml prelude (mirror of c13_links_block.rs).
/// Carries one machine with platform + scheduler + memory; only the
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

/// Happy path — `driver: serial_uart` with only the required
/// `bind` + `driver` fields parses and validates. Verifies the new
/// allowlist entry is reachable; the link inherits the same defaults
/// as any other driver entry (mtu_bytes / expected_p99_bytes / etc.
/// all None).
#[test]
fn serial_uart_driver_accepted_minimal() {
    let yaml = deploy_prelude_with_links(
        r#"          serial_console:
            bind: "/dev/ttyUSB0"
            driver: serial_uart
"#,
    );
    let cfg = parse_deploy_str(&yaml).expect("serial_uart minimal binding parses");
    let link = cfg
        .topology
        .get("mcu_device")
        .and_then(|d| d.machines.get("mcu_node"))
        .and_then(|m| m.links.get("serial_console"))
        .expect("serial_console link entry");
    assert_eq!(link.bind, "/dev/ttyUSB0");
    assert_eq!(link.driver, "serial_uart");
    assert_eq!(link.mtu_bytes, None);
    assert_eq!(link.expected_p99_bytes, None);
}

/// `serial_uart` with a realistic UART MTU (256 bytes) parses
/// without firing `link/mtu-below-driver-floor` — confirms the
/// floor=0 path. This is the typical authoring shape for an
/// embedded UART link.
#[test]
fn serial_uart_with_realistic_mtu_passes() {
    let yaml = deploy_prelude_with_links(
        r#"          uart_data:
            bind: "/dev/ttyUSB0"
            driver: serial_uart
            mtu_bytes: 256
"#,
    );
    let cfg = parse_deploy_str(&yaml).expect("serial_uart + realistic mtu parses");
    let link = &cfg.topology["mcu_device"].machines["mcu_node"].links["uart_data"];
    assert_eq!(link.driver, "serial_uart");
    assert_eq!(link.mtu_bytes, Some(256));
}

/// `serial_uart` with `mtu_bytes: 27` — a value that would fire
/// `link/mtu-below-driver-floor` for `lwip_udp` (floor = 28, IPv4
/// minimum header). The same value passes for `serial_uart` because
/// UART has no IP-stack overhead; the floor is 0 by design. This
/// test documents the more-permissive contract of non-IP drivers.
#[test]
fn serial_uart_below_ip_stack_floor_passes() {
    let yaml = deploy_prelude_with_links(
        r#"          uart_short:
            bind: "/dev/ttyUSB0"
            driver: serial_uart
            mtu_bytes: 27
"#,
    );
    let cfg = parse_deploy_str(&yaml)
        .expect("serial_uart accepts mtu_bytes below IP-stack floor (UART has no IP overhead)");
    let link = &cfg.topology["mcu_device"].machines["mcu_node"].links["uart_short"];
    assert_eq!(link.mtu_bytes, Some(27));
}

/// Negative — unknown driver name typo. The candidate set returned
/// in the error must include `serial_uart` in sorted alphabetical
/// position alongside `lwip_tcp` + `lwip_udp`. This pins the new
/// allowlist entry's visibility to authors hitting `link-driver-
/// unknown` so they discover the serial driver naturally.
#[test]
fn unknown_serial_typo_lists_serial_uart_in_candidates() {
    let yaml = deploy_prelude_with_links(
        r#"          uart_typo:
            bind: "/dev/ttyUSB0"
            driver: serial_xyz
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
    assert_eq!(link_name, "uart_typo");
    assert_eq!(driver, "serial_xyz");
    // Sorted KNOWN_DRIVERS baseline after spec C11 serial +
    // WebSocket follow-up: lwip_tcp / lwip_udp / serial_uart /
    // websocket_tcp.
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
