// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! `sce-relay`: the relay of a shared list as a server process (`docs/SCE_RELAY_DEPLOY.md`).
//!
//! * `sce-relay serve` runs it: it reads its configuration, its authority's key, its certificate
//!   and its store (every record verified again), takes its ports, says it is ready, answers until
//!   it is told to stop (`SIGTERM`, `SIGINT`), and then stops accepting, lets each connection
//!   finish the request it is answering within the grace it is given, and exits 0.
//! * `sce-relay check` does all of that but take its ports and answer, and exits: a configuration,
//!   a key, a certificate or a store restored from a copy is checked before a relay is started on
//!   it.
//! * `sce-relay keygen` makes the authority's key, a file only its owner may read, and prints
//!   nothing of it.
//! * `sce-relay token issue` issues a token for one device of one list from that key, as the
//!   service that logs a person in issues one.
//!
//! The exit status is one of `sce_rust_relay_server::exit`'s; what happened is one line on stderr
//! for each thing (`sce_rust_relay_server::logline`).

#![forbid(unsafe_code)]

use std::fs;
use std::io::Write;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::{Args, Parser, Subcommand};
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use sce_rust_relay::serve::Server;
use sce_rust_relay::tls::{self, ServerConfig};
use sce_rust_relay::{Authority, FileStore, Shards};
use sce_rust_relay_server::config::{self, Config, Settings, TlsFiles};
use sce_rust_relay_server::exit;
use sce_rust_relay_server::logline::Log;
use sce_rust_relay_server::notify::notify;
use sce_rust_relay_server::secret::{self, SecretError, SecretErrorKind};
use sce_rust_relay_server::slots::{self, StoreErrorKind};
use tokio::net::TcpListener;

/// The relay of a shared list: it keeps what the members of a list sign and gives it back, holds
/// no key of theirs and reads nothing it holds.
#[derive(Parser)]
#[command(name = "sce-relay", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the relay until it is told to stop (SIGTERM or SIGINT).
    Serve(ServeArgs),
    /// Read the configuration, the keys, the certificate and the store as `serve` would, and exit.
    Check(ServeArgs),
    /// Make the authority's key: the file the relay and the service that issues tokens read.
    Keygen(KeygenArgs),
    /// Tokens for devices.
    Token {
        #[command(subcommand)]
        command: TokenCommand,
    },
}

#[derive(Subcommand)]
enum TokenCommand {
    /// Issue a token for one device of one list, printed on standard output.
    Issue(IssueArgs),
}

/// What `serve` and `check` are given: a configuration file, and the command line over it. No
/// secret is given here, only the paths of the files that hold them.
#[derive(Args)]
struct ServeArgs {
    /// The configuration file (TOML).
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,
    /// Listen for ws:// on this address, in place of the file's [listen] plain (repeat for more).
    #[arg(long = "listen", value_name = "ADDRESS")]
    listen: Vec<SocketAddr>,
    /// Listen for wss:// on this address, in place of the file's [listen] tls (repeat for more).
    #[arg(long = "listen-tls", value_name = "ADDRESS")]
    listen_tls: Vec<SocketAddr>,
    /// The certificate chain the relay presents (PEM), its own certificate first.
    #[arg(long, value_name = "FILE")]
    tls_chain: Option<PathBuf>,
    /// The private key of the certificate (PEM), readable by its owner alone.
    #[arg(long, value_name = "FILE")]
    tls_key: Option<PathBuf>,
    /// The store's directory.
    #[arg(long, value_name = "DIRECTORY")]
    store: Option<PathBuf>,
    /// The authority's key file, readable by its owner alone.
    #[arg(long, value_name = "FILE")]
    authority_key: Option<PathBuf>,
    /// The most connections open at once.
    #[arg(long, value_name = "COUNT")]
    max_connections: Option<u64>,
    /// A limit of each device, over the file's [limits] (repeat for more).
    #[arg(long = "limit", value_name = "NAME=VALUE", value_parser = config::parse_limit)]
    limits: Vec<(String, u64)>,
}

impl ServeArgs {
    fn settings(&self) -> Settings {
        Settings {
            plain: (!self.listen.is_empty()).then(|| self.listen.clone()),
            tls: (!self.listen_tls.is_empty()).then(|| self.listen_tls.clone()),
            tls_chain: self.tls_chain.clone(),
            tls_key: self.tls_key.clone(),
            store: self.store.clone(),
            authority_key: self.authority_key.clone(),
            max_connections: self.max_connections,
            limits: self.limits.clone(),
            ..Settings::default()
        }
    }
}

#[derive(Args)]
struct KeygenArgs {
    /// The file to make; there must be none there.
    #[arg(long, value_name = "FILE")]
    out: PathBuf,
    /// The name of the relay its tokens are for, the same in every token it takes.
    #[arg(long, value_name = "NAME")]
    audience: String,
}

#[derive(Args)]
struct IssueArgs {
    /// The authority's key file.
    #[arg(long, value_name = "FILE")]
    key: PathBuf,
    /// The list the device is of.
    #[arg(long, value_name = "NAME")]
    list: String,
    /// The node the list's membership log gave the device.
    #[arg(long, value_name = "NUMBER")]
    node: u64,
    /// How long the token is good for, in seconds.
    #[arg(long, value_name = "SECONDS", default_value_t = 3600)]
    valid_for: u64,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let log = Log::from_environment();
    let status = match cli.command {
        Command::Serve(args) => serve(&log, &args),
        Command::Check(args) => check(&log, &args),
        Command::Keygen(args) => keygen(&log, &args),
        Command::Token {
            command: TokenCommand::Issue(args),
        } => issue(&log, &args),
    };
    ExitCode::from(status)
}

/// Why a command did not do what it was asked: the status it exits with, and the reason it says.
struct Refusal {
    status: u8,
    reason: String,
}

impl Refusal {
    fn new(status: u8, reason: impl Into<String>) -> Self {
        Self {
            status,
            reason: reason.into(),
        }
    }
}

/// Say why, and give the status to exit with.
fn refused(log: &Log, refusal: Refusal) -> u8 {
    log.error(
        "refused",
        &[("status", &refusal.status), ("reason", &refusal.reason)],
    );
    refusal.status
}

/// The time tokens expire by: milliseconds since the Unix epoch.
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

/// What a relay is made of, read and checked before it takes a port.
struct Prepared {
    config: Config,
    authority: Authority,
    tls: Option<Arc<ServerConfig>>,
    hub: Shards<FileStore>,
}

fn prepare(args: &ServeArgs) -> Result<Prepared, Refusal> {
    let config = configure(args)?;
    let authority = read_authority(&config.authority_key)?;
    let tls = config.tls_files.as_ref().map(read_tls).transpose()?;
    let hub = slots::open(
        &config.store,
        config.slots,
        authority.clone(),
        config.limits,
    )
    .map_err(|error| {
        let status = match error.kind {
            StoreErrorKind::Invalid => exit::DATA,
            StoreErrorKind::Unreadable => exit::CONFIG,
        };
        Refusal::new(status, error.message)
    })?;
    Ok(Prepared {
        config,
        authority,
        tls,
        hub,
    })
}

fn configure(args: &ServeArgs) -> Result<Config, Refusal> {
    let refusal = |error: config::ConfigError| Refusal::new(exit::CONFIG, error.0);
    let file = match &args.config {
        Some(path) => {
            let text = fs::read_to_string(path).map_err(|why| {
                Refusal::new(
                    exit::CONFIG,
                    format!(
                        "the configuration at {} cannot be read: {why}",
                        path.display()
                    ),
                )
            })?;
            Settings::from_toml(&text, path).map_err(refusal)?
        }
        None => Settings::default(),
    };
    file.over(args.settings()).finish().map_err(refusal)
}

fn secret_refusal(error: SecretError) -> Refusal {
    let status = match error.kind {
        SecretErrorKind::Exists | SecretErrorKind::Unwritable => exit::CANNOT_CREATE,
        SecretErrorKind::Unreadable | SecretErrorKind::Exposed => exit::CONFIG,
    };
    Refusal::new(status, error.message)
}

fn read_authority(path: &Path) -> Result<Authority, Refusal> {
    let bytes = secret::read(path, "the authority's key").map_err(secret_refusal)?;
    let not_a_key = |why: &dyn std::fmt::Display| {
        Refusal::new(
            exit::CONFIG,
            format!(
                "the authority's key at {} is not one `sce-relay keygen` makes: {why}",
                path.display()
            ),
        )
    };
    let text = String::from_utf8(bytes).map_err(|_| not_a_key(&"it is not text"))?;
    Authority::from_key_text(&text).map_err(|why| not_a_key(&why))
}

fn read_tls(files: &TlsFiles) -> Result<Arc<ServerConfig>, Refusal> {
    let refusal = |why: String| Refusal::new(exit::CONFIG, why);
    let pem = fs::read(&files.chain).map_err(|why| {
        refusal(format!(
            "the certificate chain at {} cannot be read: {why}",
            files.chain.display()
        ))
    })?;
    let chain = CertificateDer::pem_slice_iter(&pem)
        .map(|certificate| certificate.map(|certificate| certificate.to_vec()))
        .collect::<Result<Vec<Vec<u8>>, _>>()
        .map_err(|why| {
            refusal(format!(
                "the certificate chain at {} is not PEM certificates: {why}",
                files.chain.display()
            ))
        })?;
    if chain.is_empty() {
        return Err(refusal(format!(
            "the certificate chain at {} holds no certificate",
            files.chain.display()
        )));
    }
    let pem = secret::read(&files.key, "the certificate's private key").map_err(secret_refusal)?;
    let key = PrivateKeyDer::from_pem_slice(&pem).map_err(|why| {
        refusal(format!(
            "the private key at {} is not a PEM private key: {why}",
            files.key.display()
        ))
    })?;
    tls::server_config(chain, key.secret_der().to_vec()).map_err(|why| {
        refusal(format!(
            "the certificate chain at {} and the private key at {} make no TLS configuration \
             (is the key the first certificate's?): {why}",
            files.chain.display(),
            files.key.display()
        ))
    })
}

fn check(log: &Log, args: &ServeArgs) -> u8 {
    match prepare(args) {
        Ok(prepared) => {
            log.info(
                "checked",
                &[
                    ("store", &prepared.config.store.display()),
                    ("slots", &prepared.config.slots),
                    ("audience", &prepared.authority.audience()),
                    ("tls", &prepared.tls.is_some()),
                ],
            );
            exit::OK
        }
        Err(refusal) => refused(log, refusal),
    }
}

fn serve(log: &Log, args: &ServeArgs) -> u8 {
    let prepared = match prepare(args) {
        Ok(prepared) => prepared,
        Err(refusal) => return refused(log, refusal),
    };
    let grace = prepared.config.grace;
    // A relay's work for one slot is one lock: more threads to wait on it than slots, and some
    // to spare, is all the blocking pool needs.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("sce-relay")
        .max_blocking_threads((2 * prepared.config.slots).max(4))
        .build();
    let runtime = match runtime {
        Ok(runtime) => runtime,
        Err(why) => {
            return refused(
                log,
                Refusal::new(
                    exit::SOFTWARE,
                    format!("the runtime could not start: {why}"),
                ),
            )
        }
    };
    let status = runtime.block_on(run(log, prepared));
    // What a connection still runs past the grace is given up with the process: every record the
    // relay acknowledged is on the disk already.
    runtime.shutdown_timeout(grace);
    status
}

/// Take the ports, say the relay is ready, answer until told to stop, and stop.
async fn run(log: &Log, prepared: Prepared) -> u8 {
    let Prepared {
        config,
        authority,
        tls,
        hub,
    } = prepared;
    // The handlers are in place before the relay says it is ready, so that a stop that comes at
    // once is a stop, not the end of the process.
    let mut stops = match Stops::install() {
        Ok(stops) => stops,
        Err(why) => {
            return refused(
                log,
                Refusal::new(
                    exit::SOFTWARE,
                    format!("the stop signals could not be handled: {why}"),
                ),
            )
        }
    };
    let mut listeners = Vec::new();
    let addresses = config
        .plain
        .iter()
        .map(|address| (address, false))
        .chain(config.tls.iter().map(|address| (address, true)));
    for (address, secure) in addresses {
        match TcpListener::bind(address).await {
            Ok(listener) => listeners.push((listener, secure)),
            Err(why) => {
                return refused(
                    log,
                    Refusal::new(
                        exit::UNAVAILABLE,
                        format!("the relay cannot listen on {address}: {why}"),
                    ),
                )
            }
        }
    }

    let clock: Arc<dyn Fn() -> u64 + Send + Sync> = Arc::new(now_ms);
    let server = Server::new(Arc::new(hub), clock, config.bounds);
    let mut listening = Vec::new();
    for (listener, secure) in listeners {
        let address = listener
            .local_addr()
            .map_or_else(|_| "unknown".to_string(), |address| address.to_string());
        let transport = if secure { "wss" } else { "ws" };
        log.info(
            "listening",
            &[("transport", &transport), ("address", &address)],
        );
        let server = server.clone();
        let tls = if secure { tls.clone() } else { None };
        listening.push(tokio::spawn(
            async move { server.listen(listener, tls).await },
        ));
    }
    log.info(
        "ready",
        &[
            ("audience", &authority.audience()),
            ("slots", &config.slots),
            ("max_connections", &config.bounds.max_connections),
        ],
    );
    if let Err(why) = notify("READY=1\nSTATUS=answering") {
        log.warning("not_notified", &[("reason", &why)]);
    }

    let signal = stops.next().await;
    log.info(
        "stopping",
        &[
            ("signal", &signal),
            ("grace_seconds", &config.grace.as_secs()),
        ],
    );
    if let Err(why) = notify("STOPPING=1\nSTATUS=stopping") {
        log.warning("not_notified", &[("reason", &why)]);
    }
    server.stop().raise();
    for task in listening {
        let _ = task.await;
    }
    let open = server.drain(config.grace).await;
    if open > 0 {
        log.warning("connections_left_open", &[("open", &open)]);
    }
    let counts = server.counts();
    log.info(
        "stopped",
        &[
            ("accepted", &counts.accepted),
            ("refused", &counts.refused),
            ("unopened", &counts.unopened),
            ("requests", &counts.requests),
            ("accept_failures", &counts.accept_failures),
        ],
    );
    exit::OK
}

/// The signals that stop a relay: SIGTERM, which a service manager sends, and SIGINT.
#[cfg(unix)]
struct Stops {
    terminate: tokio::signal::unix::Signal,
    interrupt: tokio::signal::unix::Signal,
}

#[cfg(unix)]
impl Stops {
    fn install() -> std::io::Result<Self> {
        use tokio::signal::unix::{signal, SignalKind};
        Ok(Self {
            terminate: signal(SignalKind::terminate())?,
            interrupt: signal(SignalKind::interrupt())?,
        })
    }

    async fn next(&mut self) -> &'static str {
        tokio::select! {
            _ = self.terminate.recv() => "SIGTERM",
            _ = self.interrupt.recv() => "SIGINT",
        }
    }
}

/// A system with no Unix signals stops a relay by its console's interrupt.
#[cfg(not(unix))]
struct Stops;

#[cfg(not(unix))]
impl Stops {
    fn install() -> std::io::Result<Self> {
        Ok(Self)
    }

    async fn next(&mut self) -> &'static str {
        let _ = tokio::signal::ctrl_c().await;
        "interrupt"
    }
}

fn keygen(log: &Log, args: &KeygenArgs) -> u8 {
    if !Authority::is_audience(&args.audience) {
        return refused(
            log,
            Refusal::new(
                exit::USAGE,
                "the audience is a name of one line, with no space at either end",
            ),
        );
    }
    let authority = match Authority::generate(&args.audience) {
        Ok(authority) => authority,
        Err(why) => {
            return refused(
                log,
                Refusal::new(
                    exit::SOFTWARE,
                    format!("the operating system gave no random bytes: {why}"),
                ),
            )
        }
    };
    let made = secret::create(
        &args.out,
        "the authority's key",
        authority.to_key_text().as_bytes(),
    );
    match made {
        Ok(()) => {
            log.info(
                "key_made",
                &[
                    ("path", &args.out.display()),
                    ("audience", &authority.audience()),
                ],
            );
            exit::OK
        }
        Err(error) => refused(log, secret_refusal(error)),
    }
}

fn issue(log: &Log, args: &IssueArgs) -> u8 {
    if args.list.is_empty() || args.valid_for == 0 {
        return refused(
            log,
            Refusal::new(
                exit::USAGE,
                "a token names a list by a name that is not empty, for a time that is not none",
            ),
        );
    }
    let authority = match read_authority(&args.key) {
        Ok(authority) => authority,
        Err(refusal) => return refused(log, refusal),
    };
    let expires = now_ms().saturating_add(args.valid_for.saturating_mul(1000));
    let token = authority.issue(&args.list, args.node, expires);
    if writeln!(std::io::stdout().lock(), "{token}").is_err() {
        return refused(
            log,
            Refusal::new(exit::IO_ERROR, "the token could not be written out"),
        );
    }
    log.info(
        "token_issued",
        &[
            ("list", &args.list),
            ("node", &args.node),
            ("expires_at_ms", &expires),
        ],
    );
    exit::OK
}
