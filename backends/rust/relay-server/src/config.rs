// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a relay is configured with: one TOML file, and the command line over it.
//!
//! ```toml
//! [listen]
//! plain = ["127.0.0.1:8080"]     # ws://, behind a proxy that terminates TLS or on a private network
//! tls = ["0.0.0.0:443"]          # wss://, with the certificate below
//!
//! [tls]
//! certificate_chain = "/etc/sce-relay/chain.pem"
//! private_key = "/etc/sce-relay/key.pem"
//!
//! [store]
//! directory = "/var/lib/sce-relay"
//! slots = 1
//!
//! [authority]
//! key_file = "/etc/sce-relay/authority.key"
//!
//! [connections]
//! max = 512
//! handshake_seconds = 10
//! idle_seconds = 300
//! shutdown_grace_seconds = 10
//!
//! [limits]
//! max_lists = 10000
//! ```
//!
//! A file holds paths and numbers and never a secret: the keys are files of their own
//! ([`crate::secret`]). A relative path in a file is read from the file's directory, one on the
//! command line from the working directory. A section or a key the relay does not read is refused
//! by name, so that a word mistyped is not a setting silently left at its default. What the command
//! line gives replaces what the file gives (a list of addresses as a whole), and a limit given in
//! both is the command line's.

use std::fmt;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use sce_rust_relay::serve::Bounds;
use sce_rust_relay::Limits;
use toml_edit::{Item, TableLike};

use crate::slots::MOST_SLOTS;

/// Why a configuration was not taken: what is wrong and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError(pub String);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ConfigError {}

/// The certificate chain a relay presents and the private key that goes with it, as PEM files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsFiles {
    pub chain: PathBuf,
    pub key: PathBuf,
}

/// What a relay is configured with, every value checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Where it listens for `ws://`.
    pub plain: Vec<SocketAddr>,
    /// Where it listens for `wss://`, with [`Config::tls_files`].
    pub tls: Vec<SocketAddr>,
    pub tls_files: Option<TlsFiles>,
    /// The store's directory, and the number of slots it is laid out for.
    pub store: PathBuf,
    pub slots: usize,
    /// The file of the authority's key.
    pub authority_key: PathBuf,
    /// What each connection is held to.
    pub bounds: Bounds,
    /// How long a stop waits for open connections to finish.
    pub grace: Duration,
    /// What each device is held to.
    pub limits: Limits,
}

/// What a file or a command line says, before it is checked as a whole: `None` where it says
/// nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Settings {
    pub plain: Option<Vec<SocketAddr>>,
    pub tls: Option<Vec<SocketAddr>>,
    pub tls_chain: Option<PathBuf>,
    pub tls_key: Option<PathBuf>,
    pub store: Option<PathBuf>,
    pub slots: Option<u64>,
    pub authority_key: Option<PathBuf>,
    pub max_connections: Option<u64>,
    pub handshake_seconds: Option<u64>,
    pub idle_seconds: Option<u64>,
    pub grace_seconds: Option<u64>,
    /// Limits by name, in the order they were said: a later one of a name wins.
    pub limits: Vec<(String, u64)>,
}

/// The limits a configuration may set, by the names of [`Limits`]' fields.
pub const LIMIT_NAMES: [&str; 10] = [
    "burst",
    "refill_per_second",
    "write_cost",
    "max_request_bytes",
    "max_feed",
    "max_range",
    "max_records_per_list",
    "max_lists",
    "max_group_message_bytes",
    "max_key_packages_per_node",
];

/// The sections of a configuration file.
const SECTIONS: [&str; 6] = [
    "listen",
    "tls",
    "store",
    "authority",
    "connections",
    "limits",
];

const DEFAULT_SLOTS: u64 = 1;
const DEFAULT_HANDSHAKE_SECONDS: u64 = 10;
const DEFAULT_IDLE_SECONDS: u64 = 300;
const DEFAULT_GRACE_SECONDS: u64 = 10;

impl Settings {
    /// What the configuration file `file`, whose text is `text`, says.
    pub fn from_toml(text: &str, file: &Path) -> Result<Self, ConfigError> {
        let base = file.parent().unwrap_or_else(|| Path::new(""));
        let reader = Reader { text, file, base };
        let document = toml_edit::Document::parse(text).map_err(|error| {
            let line = error.span().map_or(String::new(), |span| {
                format!(" line {}", reader.line(span.start))
            });
            ConfigError(format!(
                "{}{line}: not TOML: {}",
                file.display(),
                error.message()
            ))
        })?;
        let mut settings = Settings::default();
        for (section, item) in document.as_table().iter() {
            let table = item
                .as_table_like()
                .filter(|_| SECTIONS.contains(&section))
                .ok_or_else(|| {
                    reader.at(
                        item,
                        format!(
                            "`{section}` is not a section of the relay's; they are [{}]",
                            SECTIONS.join("], [")
                        ),
                    )
                })?;
            for (key, value) in TableLike::iter(table) {
                let name = format!("[{section}] {key}");
                match (section, key) {
                    ("listen", "plain") => settings.plain = Some(reader.addresses(value, &name)?),
                    ("listen", "tls") => settings.tls = Some(reader.addresses(value, &name)?),
                    ("tls", "certificate_chain") => {
                        settings.tls_chain = Some(reader.path(value, &name)?)
                    }
                    ("tls", "private_key") => settings.tls_key = Some(reader.path(value, &name)?),
                    ("store", "directory") => settings.store = Some(reader.path(value, &name)?),
                    ("store", "slots") => settings.slots = Some(reader.number(value, &name)?),
                    ("authority", "key_file") => {
                        settings.authority_key = Some(reader.path(value, &name)?)
                    }
                    ("connections", "max") => {
                        settings.max_connections = Some(reader.number(value, &name)?)
                    }
                    ("connections", "handshake_seconds") => {
                        settings.handshake_seconds = Some(reader.number(value, &name)?)
                    }
                    ("connections", "idle_seconds") => {
                        settings.idle_seconds = Some(reader.number(value, &name)?)
                    }
                    ("connections", "shutdown_grace_seconds") => {
                        settings.grace_seconds = Some(reader.number(value, &name)?)
                    }
                    ("limits", limit) if LIMIT_NAMES.contains(&limit) => settings
                        .limits
                        .push((limit.to_string(), reader.number(value, &name)?)),
                    _ => {
                        return Err(
                            reader.at(value, format!("{name} is not a setting the relay reads"))
                        )
                    }
                }
            }
        }
        Ok(settings)
    }

    /// These settings with `over` said over them: what `over` says wins, and its limits come after
    /// these.
    pub fn over(self, over: Settings) -> Settings {
        let mut limits = self.limits;
        limits.extend(over.limits);
        Settings {
            plain: over.plain.or(self.plain),
            tls: over.tls.or(self.tls),
            tls_chain: over.tls_chain.or(self.tls_chain),
            tls_key: over.tls_key.or(self.tls_key),
            store: over.store.or(self.store),
            slots: over.slots.or(self.slots),
            authority_key: over.authority_key.or(self.authority_key),
            max_connections: over.max_connections.or(self.max_connections),
            handshake_seconds: over.handshake_seconds.or(self.handshake_seconds),
            idle_seconds: over.idle_seconds.or(self.idle_seconds),
            grace_seconds: over.grace_seconds.or(self.grace_seconds),
            limits,
        }
    }

    /// The configuration these settings make, every value checked and the rest at its default.
    pub fn finish(self) -> Result<Config, ConfigError> {
        let plain = self.plain.unwrap_or_default();
        let tls = self.tls.unwrap_or_default();
        if plain.is_empty() && tls.is_empty() {
            return Err(error(
                "the relay listens nowhere: give [listen] plain or tls (--listen, --listen-tls)",
            ));
        }
        let mut taken: Vec<&SocketAddr> = Vec::new();
        for address in plain.iter().chain(&tls) {
            if address.port() != 0 && taken.contains(&address) {
                return Err(ConfigError(format!(
                    "{address} is listened on twice: an address is either plain or tls, once"
                )));
            }
            taken.push(address);
        }
        let tls_files = match (self.tls_chain, self.tls_key) {
            (Some(chain), Some(key)) => Some(TlsFiles { chain, key }),
            (None, None) => None,
            _ => {
                return Err(error(
                    "a certificate chain goes with its private key: give both [tls] \
                     certificate_chain and private_key (--tls-chain, --tls-key)",
                ))
            }
        };
        match (tls.is_empty(), tls_files.is_some()) {
            (false, false) => {
                return Err(error(
                    "[listen] tls needs a certificate: give [tls] certificate_chain and \
                     private_key (--tls-chain, --tls-key)",
                ))
            }
            (true, true) => {
                return Err(error(
                    "a certificate is given and nothing listens with TLS: give [listen] tls \
                     (--listen-tls), or take the certificate away",
                ))
            }
            _ => {}
        }
        let store = self.store.ok_or_else(|| {
            error("the store's directory is not given: [store] directory (--store)")
        })?;
        let authority_key = self.authority_key.ok_or_else(|| {
            error("the authority's key is not given: [authority] key_file (--authority-key)")
        })?;
        let slots = between(
            "[store] slots",
            self.slots.unwrap_or(DEFAULT_SLOTS),
            1,
            MOST_SLOTS as u64,
        )?;
        let max_connections = between(
            "[connections] max",
            self.max_connections
                .unwrap_or(Bounds::default().max_connections as u64),
            1,
            Bounds::MOST_CONNECTIONS as u64,
        )?;
        let handshake = between(
            "[connections] handshake_seconds",
            self.handshake_seconds.unwrap_or(DEFAULT_HANDSHAKE_SECONDS),
            1,
            3600,
        )?;
        let idle = between(
            "[connections] idle_seconds",
            self.idle_seconds.unwrap_or(DEFAULT_IDLE_SECONDS),
            1,
            86_400,
        )?;
        let grace = between(
            "[connections] shutdown_grace_seconds",
            self.grace_seconds.unwrap_or(DEFAULT_GRACE_SECONDS),
            0,
            3600,
        )?;
        let mut limits = Limits::default();
        for (name, value) in &self.limits {
            set_limit(&mut limits, name, *value)?;
        }
        Ok(Config {
            plain,
            tls,
            tls_files,
            store,
            slots: slots as usize,
            authority_key,
            bounds: Bounds {
                max_message: limits.max_request_bytes,
                max_connections: max_connections as usize,
                handshake: Duration::from_secs(handshake),
                idle: Duration::from_secs(idle),
            },
            grace: Duration::from_secs(grace),
            limits,
        })
    }
}

fn error(message: &str) -> ConfigError {
    ConfigError(message.to_string())
}

/// `value` of `name` when it is from `least` to `most`.
fn between(name: &str, value: u64, least: u64, most: u64) -> Result<u64, ConfigError> {
    if (least..=most).contains(&value) {
        Ok(value)
    } else {
        Err(ConfigError(format!(
            "{name} is {value}, and is from {least} to {most}"
        )))
    }
}

/// Set the limit `name` of `limits` to `value`, when it is one and the value is one it can be.
fn set_limit(limits: &mut Limits, name: &str, value: u64) -> Result<(), ConfigError> {
    let at_least = |least: u64| -> Result<u64, ConfigError> {
        if value < least {
            Err(ConfigError(format!(
                "[limits] {name} is {value}, and is at least {least}"
            )))
        } else {
            Ok(value)
        }
    };
    let too_large = || ConfigError(format!("[limits] {name} is {value}, which is too large"));
    let narrow = |least: u64| -> Result<u32, ConfigError> {
        u32::try_from(at_least(least)?).map_err(|_| too_large())
    };
    let wide = |least: u64| -> Result<usize, ConfigError> {
        usize::try_from(at_least(least)?).map_err(|_| too_large())
    };
    match name {
        "burst" => limits.burst = narrow(1)?,
        "refill_per_second" => limits.refill_per_second = narrow(0)?,
        "write_cost" => limits.write_cost = narrow(1)?,
        "max_request_bytes" => limits.max_request_bytes = wide(1024)?,
        "max_feed" => limits.max_feed = narrow(1)?,
        "max_range" => limits.max_range = at_least(1)?,
        "max_records_per_list" => limits.max_records_per_list = wide(1)?,
        "max_lists" => limits.max_lists = wide(1)?,
        "max_group_message_bytes" => limits.max_group_message_bytes = wide(1)?,
        "max_key_packages_per_node" => limits.max_key_packages_per_node = wide(1)?,
        other => {
            return Err(ConfigError(format!(
                "`{other}` is not a limit; the limits are {}",
                LIMIT_NAMES.join(", ")
            )))
        }
    }
    Ok(())
}

/// A limit as the command line gives it, `name=value`.
pub fn parse_limit(text: &str) -> Result<(String, u64), String> {
    let (name, value) = text
        .split_once('=')
        .ok_or_else(|| format!("`{text}` is not name=value"))?;
    if !LIMIT_NAMES.contains(&name) {
        return Err(format!(
            "`{name}` is not a limit; the limits are {}",
            LIMIT_NAMES.join(", ")
        ));
    }
    let value = value
        .parse::<u64>()
        .map_err(|_| format!("the value of {name} is not a whole number: `{value}`"))?;
    Ok((name.to_string(), value))
}

/// What reads the values of one file, and says where one is wrong.
struct Reader<'a> {
    text: &'a str,
    file: &'a Path,
    base: &'a Path,
}

impl Reader<'_> {
    /// The line of the byte at `offset` of the text.
    fn line(&self, offset: usize) -> usize {
        self.text
            .get(..offset)
            .map_or(0, |before| before.matches('\n').count())
            + 1
    }

    /// An error about `item`, at its line when the parser kept it.
    fn at(&self, item: &Item, what: String) -> ConfigError {
        let line = item.span().map_or(String::new(), |span| {
            format!(" line {}", self.line(span.start))
        });
        ConfigError(format!("{}{line}: {what}", self.file.display()))
    }

    fn number(&self, item: &Item, name: &str) -> Result<u64, ConfigError> {
        item.as_integer()
            .and_then(|number| u64::try_from(number).ok())
            .ok_or_else(|| self.at(item, format!("{name} is a whole number, 0 or more")))
    }

    fn path(&self, item: &Item, name: &str) -> Result<PathBuf, ConfigError> {
        let text = item
            .as_str()
            .filter(|text| !text.is_empty())
            .ok_or_else(|| self.at(item, format!("{name} is a path, in quotes")))?;
        Ok(self.base.join(text))
    }

    fn addresses(&self, item: &Item, name: &str) -> Result<Vec<SocketAddr>, ConfigError> {
        let shape = || {
            self.at(
                item,
                format!(
                    "{name} is a list of addresses with their ports, like \
                     [\"127.0.0.1:8080\", \"[::]:443\"]"
                ),
            )
        };
        let array = item.as_array().ok_or_else(shape)?;
        array
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .and_then(|text| text.parse::<SocketAddr>().ok())
                    .ok_or_else(shape)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = "/etc/sce-relay/relay.toml";

    fn read(text: &str) -> Result<Settings, ConfigError> {
        Settings::from_toml(text, Path::new(FILE))
    }

    fn address(text: &str) -> SocketAddr {
        text.parse().unwrap()
    }

    const WHOLE: &str = r#"
# A relay behind a proxy, and on its own TLS port.
[listen]
plain = ["127.0.0.1:8080"]
tls = ["0.0.0.0:8443", "[::]:8443"]

[tls]
certificate_chain = "chain.pem"
private_key = "/etc/private/key.pem"

[store]
directory = "/var/lib/sce-relay"
slots = 4

[authority]
key_file = "authority.key"

[connections]
max = 64
handshake_seconds = 5
idle_seconds = 60
shutdown_grace_seconds = 3

[limits]
burst = 10
max_lists = 50
max_request_bytes = 4096
"#;

    #[test]
    fn a_whole_file_is_read_with_its_paths_from_its_directory() {
        let config = read(WHOLE).unwrap().finish().unwrap();
        assert_eq!(config.plain, vec![address("127.0.0.1:8080")]);
        assert_eq!(
            config.tls,
            vec![address("0.0.0.0:8443"), address("[::]:8443")]
        );
        assert_eq!(
            config.tls_files,
            Some(TlsFiles {
                chain: PathBuf::from("/etc/sce-relay/chain.pem"),
                key: PathBuf::from("/etc/private/key.pem"),
            })
        );
        assert_eq!(config.store, PathBuf::from("/var/lib/sce-relay"));
        assert_eq!(config.slots, 4);
        assert_eq!(
            config.authority_key,
            PathBuf::from("/etc/sce-relay/authority.key")
        );
        assert_eq!(
            config.bounds,
            Bounds {
                max_message: 4096,
                max_connections: 64,
                handshake: Duration::from_secs(5),
                idle: Duration::from_secs(60),
            }
        );
        assert_eq!(config.grace, Duration::from_secs(3));
        assert_eq!(
            config.limits,
            Limits {
                burst: 10,
                max_lists: 50,
                max_request_bytes: 4096,
                ..Limits::default()
            }
        );
    }

    #[test]
    fn what_a_file_leaves_out_is_at_its_default() {
        let config = read(
            "[listen]\nplain = [\"127.0.0.1:0\"]\n[store]\ndirectory = \"s\"\n\
             [authority]\nkey_file = \"k\"\n",
        )
        .unwrap()
        .finish()
        .unwrap();
        assert_eq!(config.slots, 1);
        assert_eq!(config.limits, Limits::default());
        assert_eq!(config.bounds, Bounds::default());
        assert_eq!(config.grace, Duration::from_secs(10));
        assert_eq!(config.tls_files, None);
    }

    #[test]
    fn the_command_line_is_said_over_the_file() {
        let file = read(WHOLE).unwrap();
        let line = Settings {
            plain: Some(vec!["127.0.0.1:9000".parse().unwrap()]),
            store: Some(PathBuf::from("elsewhere")),
            limits: vec![("burst".to_string(), 99)],
            ..Settings::default()
        };
        let config = file.over(line).finish().unwrap();
        assert_eq!(config.plain, vec![address("127.0.0.1:9000")]);
        assert_eq!(config.tls.len(), 2, "what the command line did not say");
        assert_eq!(config.store, PathBuf::from("elsewhere"));
        assert_eq!(config.limits.burst, 99, "the command line's limit wins");
        assert_eq!(config.limits.max_lists, 50, "the file's other limits stay");
    }

    /// The error each wrong file gives, and a word it must say.
    #[test]
    fn a_file_that_is_wrong_is_refused_by_name_and_line() {
        let wrong = [
            ("[listen]\nplian = []\n", "[listen] plian is not a setting"),
            ("[lisen]\nplain = []\n", "`lisen` is not a section"),
            ("[lisen]\n", "`lisen` is not a section"),
            ("store = \"x\"\n", "`store` is not a section"),
            (
                "[store]\nslots = \"four\"\n",
                "[store] slots is a whole number",
            ),
            ("[store]\nslots = -1\n", "[store] slots is a whole number"),
            ("[store]\ndirectory = 7\n", "[store] directory is a path"),
            ("[store]\ndirectory = \"\"\n", "[store] directory is a path"),
            ("[listen]\nplain = \"127.0.0.1:1\"\n", "a list of addresses"),
            (
                "[listen]\nplain = [\"localhost:1\"]\n",
                "a list of addresses",
            ),
            ("[limits]\nbursts = 1\n", "[limits] bursts is not a setting"),
            ("[listen\n", "not TOML"),
            ("[store]\nslots = 1\nslots = 2\n", "not TOML"),
        ];
        for (text, said) in wrong {
            let refused = read(text).unwrap_err();
            assert!(refused.0.contains(said), "{text:?}: {refused}");
            assert!(refused.0.starts_with(FILE), "{refused}");
        }
        let refused = read("[store]\n\n\nslots = true\n").unwrap_err();
        assert!(refused.0.contains("line 4"), "{refused}");
    }

    #[test]
    fn a_configuration_that_does_not_hold_together_is_refused_and_says_why() {
        let base = || Settings {
            plain: Some(vec!["127.0.0.1:8080".parse().unwrap()]),
            store: Some(PathBuf::from("s")),
            authority_key: Some(PathBuf::from("k")),
            ..Settings::default()
        };
        assert!(base().finish().is_ok());
        let tls = |settings: Settings| Settings {
            tls: Some(vec!["127.0.0.1:8443".parse().unwrap()]),
            ..settings
        };
        let cases: Vec<(Settings, &str)> = vec![
            (
                Settings {
                    plain: None,
                    ..base()
                },
                "listens nowhere",
            ),
            (
                Settings {
                    store: None,
                    ..base()
                },
                "store's directory is not given",
            ),
            (
                Settings {
                    authority_key: None,
                    ..base()
                },
                "authority's key is not given",
            ),
            (tls(base()), "[listen] tls needs a certificate"),
            (
                Settings {
                    tls_chain: Some(PathBuf::from("c")),
                    ..tls(base())
                },
                "goes with its private key",
            ),
            (
                Settings {
                    tls_chain: Some(PathBuf::from("c")),
                    tls_key: Some(PathBuf::from("k")),
                    ..base()
                },
                "nothing listens with TLS",
            ),
            (
                Settings {
                    plain: Some(vec![
                        "127.0.0.1:8080".parse().unwrap(),
                        "127.0.0.1:8080".parse().unwrap(),
                    ]),
                    ..base()
                },
                "listened on twice",
            ),
            (
                Settings {
                    slots: Some(0),
                    ..base()
                },
                "[store] slots is 0",
            ),
            (
                Settings {
                    max_connections: Some(0),
                    ..base()
                },
                "[connections] max is 0",
            ),
            (
                Settings {
                    idle_seconds: Some(0),
                    ..base()
                },
                "[connections] idle_seconds is 0",
            ),
            (
                Settings {
                    limits: vec![("burst".to_string(), 0)],
                    ..base()
                },
                "[limits] burst is 0, and is at least 1",
            ),
            (
                Settings {
                    limits: vec![("burst".to_string(), u64::from(u32::MAX) + 1)],
                    ..base()
                },
                "too large",
            ),
            (
                Settings {
                    limits: vec![("max_request_bytes".to_string(), 100)],
                    ..base()
                },
                "at least 1024",
            ),
        ];
        for (settings, said) in cases {
            let refused = settings.clone().finish().unwrap_err();
            assert!(refused.0.contains(said), "{settings:?}: {refused}");
        }
        // Two listeners on port 0 are two ports the system picks.
        let twice = Settings {
            plain: Some(vec![
                "127.0.0.1:0".parse().unwrap(),
                "127.0.0.1:0".parse().unwrap(),
            ]),
            ..base()
        };
        assert!(twice.finish().is_ok());
        let refills_nothing = Settings {
            limits: vec![("refill_per_second".to_string(), 0)],
            ..base()
        };
        assert_eq!(
            refills_nothing.finish().unwrap().limits.refill_per_second,
            0
        );
    }

    #[test]
    fn a_limit_on_the_command_line_is_a_name_and_a_whole_number() {
        assert_eq!(parse_limit("burst=5"), Ok(("burst".to_string(), 5)));
        assert!(parse_limit("burst").unwrap_err().contains("name=value"));
        assert!(parse_limit("bust=5").unwrap_err().contains("not a limit"));
        assert!(parse_limit("burst=-1")
            .unwrap_err()
            .contains("whole number"));
        for name in LIMIT_NAMES {
            assert!(parse_limit(&format!("{name}=1")).is_ok(), "{name}");
        }
    }

    #[test]
    fn every_limit_a_configuration_names_is_one_it_sets() {
        for (at, name) in LIMIT_NAMES.iter().enumerate() {
            let mut limits = Limits::default();
            let value = 2000 + at as u64;
            set_limit(&mut limits, name, value).unwrap();
            assert_ne!(limits, Limits::default(), "{name} set nothing");
        }
    }
}
