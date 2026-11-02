// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// The files a relay is deployed with (`deploy/`) are held to the program: the configuration is one the
// relay takes once the unit has given it its keys; the unit runs `serve` with that configuration, hands
// it the keys it loads as credentials, waits for its readiness, gives it time to stop within its grace,
// and does not start again a relay whose store or configuration is wrong; and the firewall opens the
// port the relay listens on with TLS.

use std::path::{Path, PathBuf};
use std::time::Duration;

use sce_rust_relay_server::config::{Config, Settings};
use sce_rust_relay_server::exit;

const CONFIGURATION: &str = include_str!("../deploy/relay.toml");
const UNIT: &str = include_str!("../deploy/sce-relay.service");
const FIREWALL: &str = include_str!("../deploy/sce-relay.xml");

/// The values the unit gives `key` (`Key=value` lines), in order.
fn unit_values(key: &str) -> Vec<&'static str> {
    UNIT.lines()
        .filter_map(|line| line.strip_prefix(key)?.strip_prefix('='))
        .collect()
}

/// The configuration as the unit makes it: the file, and the keys from the credentials directory.
fn configured() -> Config {
    let file = Settings::from_toml(CONFIGURATION, Path::new("/etc/sce-relay/relay.toml")).unwrap();
    let unit = Settings {
        authority_key: Some(PathBuf::from(
            "/run/credentials/sce-relay.service/authority.key",
        )),
        tls_key: Some(PathBuf::from("/run/credentials/sce-relay.service/tls.key")),
        ..Settings::default()
    };
    file.over(unit).finish().unwrap()
}

#[test]
fn the_configuration_is_one_the_relay_takes_with_the_keys_the_unit_gives() {
    let config = configured();
    assert_eq!(config.store, PathBuf::from("/var/lib/sce-relay"));
    assert_eq!(
        config.tls,
        vec!["[::]:443".parse::<std::net::SocketAddr>().unwrap()]
    );
    assert!(config.plain.is_empty());
    assert!(config.tls_files.is_some());
}

#[test]
fn the_unit_runs_the_relay_as_a_service_that_says_when_it_is_ready() {
    assert_eq!(unit_values("Type"), vec!["notify"]);
    let start = unit_values("ExecStart");
    assert_eq!(start.len(), 1);
    let words: Vec<&str> = start[0].split_whitespace().collect();
    assert_eq!(
        &words[1..4],
        ["serve", "--config", "/etc/sce-relay/relay.toml"]
    );

    // Each key the relay is pointed at is a credential the unit loads.
    let credentials: Vec<&str> = unit_values("LoadCredential")
        .into_iter()
        .map(|credential| credential.split_once(':').unwrap().0)
        .collect();
    for flag in ["--authority-key", "--tls-key"] {
        let at = words.iter().position(|word| *word == flag).unwrap();
        let name = words[at + 1].strip_prefix("%d/").unwrap();
        assert!(
            credentials.contains(&name),
            "{flag} {name}: {credentials:?}"
        );
    }

    // The store is the state directory the unit makes.
    let state = unit_values("StateDirectory");
    assert_eq!(
        configured().store,
        Path::new("/var/lib").join(state[0]),
        "the store is the unit's state directory"
    );
}

#[test]
fn the_unit_does_not_start_again_a_relay_that_cannot_start_and_lets_one_stop() {
    let kept: Vec<u8> = unit_values("RestartPreventExitStatus")[0]
        .split_whitespace()
        .map(|status| status.parse().unwrap())
        .collect();
    assert_eq!(kept, vec![exit::DATA, exit::CONFIG]);
    assert_eq!(unit_values("Restart"), vec!["on-failure"]);

    // The relay waits its grace for connections and as long again for its runtime.
    let stop: u64 = unit_values("TimeoutStopSec")[0].parse().unwrap();
    assert!(
        Duration::from_secs(stop) > configured().grace * 2,
        "TimeoutStopSec={stop}"
    );
}

#[test]
fn the_firewall_opens_the_port_the_relay_listens_on_with_tls() {
    let port = configured().tls[0].port();
    assert!(
        FIREWALL.contains(&format!("<port protocol=\"tcp\" port=\"{port}\"/>")),
        "{FIREWALL}"
    );
}
