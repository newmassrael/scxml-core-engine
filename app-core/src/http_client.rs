// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A minimal HTTP/1.1 client for the servers a person runs.
//!
//! What it is for: one request to a server the person named (a model server on this computer, or
//! one reached through a tunnel), whose answer is a JSON document. What it holds to:
//!
//! - **A request ends when it is told to or when its time is up**, however the server behaves. A
//!   model server may say nothing for minutes while it works, so waiting is done in short slices of
//!   the socket's own read, and the word to stop is looked at between them; a stopped request closes
//!   its connection, so the server is not left computing for nobody.
//! - **What comes back is bounded.** A header and a body each have a most, so a server that never
//!   stops does not fill the memory of the application.
//! - **Nothing of the person's is sent that they did not give.** The only header that is not about
//!   the request itself is the credential, and only when there is one.
//!
//! - **A server is who its address says it is.** Over `https://` the server's certificate is checked
//!   against the roots of the web (`webpki-roots`) and against the name in the address, and a server
//!   that fails either is not talked to ([`HttpError::Certificate`]), whatever the person would
//!   like: what is sent to it is a specification, and a person who typed an address is trusting
//!   that name and no other. A server whose certificate a private authority made is not one of
//!   these yet; it is reached over a tunnel or a proxy of the person's own that listens on this
//!   computer over `http://`.
//!
//! What it does not do: redirects, compression, keeping a connection. A connection is one request
//! (`Connection: close`). `http://` stays what it was: plain, which is for this computer and for a
//! network the person trusts, and a screen that registers an address says which it is
//! ([`Endpoint::is_loopback`], [`Endpoint::is_tls`]).

use std::io::{self, Read, Write};
use std::net::{IpAddr, TcpStream, ToSocketAddrs};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use rustls::pki_types::ServerName;
use rustls::{CertificateError, ClientConfig, ClientConnection, RootCertStore, StreamOwned};

use crate::runner::Cancel;

/// How long one slice of waiting on the socket is: how late a word to stop is noticed.
const SLICE: Duration = Duration::from_millis(100);

/// The most a connection is given to be made.
const CONNECT_WITHIN: Duration = Duration::from_secs(5);

/// The most of the head (the status line and the headers) that is read.
const HEAD_MAX: usize = 64 * 1024;

/// The most of one line of a chunked body's framing.
const LINE_MAX: usize = 8 * 1024;

/// Where a server is, as a person typed it: `http(s)://host[:port][/path]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    tls: bool,
    host: String,
    port: u16,
    /// The path the address carries, without a trailing slash (`/v1`), or nothing.
    base: String,
}

/// Why an address is not one a request can be made to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EndpointError {
    /// Not `http(s)://host[:port][/path]`; says what is wrong.
    Malformed(String),
}

impl std::fmt::Display for EndpointError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EndpointError::Malformed(why) => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for EndpointError {}

impl Endpoint {
    /// The server an address names.
    pub fn parse(address: &str) -> Result<Endpoint, EndpointError> {
        let bad = |why: &str| EndpointError::Malformed(why.to_string());
        let (tls, rest) = match address.strip_prefix("https://") {
            Some(rest) => (true, rest),
            None => (
                false,
                address
                    .strip_prefix("http://")
                    .ok_or_else(|| bad("the address is not http(s)://host[:port][/path]"))?,
            ),
        };
        let (authority, path) = match rest.find('/') {
            Some(at) => (&rest[..at], &rest[at..]),
            None => (rest, ""),
        };
        let (host, port) = if let Some(inner) = authority.strip_prefix('[') {
            // An IPv6 address is in brackets, and the port, when there is one, follows them.
            let end = inner
                .find(']')
                .ok_or_else(|| bad("an IPv6 address is closed with ]"))?;
            let after = &inner[end + 1..];
            let port = match after.strip_prefix(':') {
                Some(port) => Some(port),
                None if after.is_empty() => None,
                None => return Err(bad("what follows the IPv6 address is a :port")),
            };
            (&inner[..end], port)
        } else {
            match authority.rsplit_once(':') {
                Some((host, port)) => (host, Some(port)),
                None => (authority, None),
            }
        };
        if host.is_empty() {
            return Err(bad("the address names no host"));
        }
        let port = match port {
            None if tls => 443,
            None => 80,
            Some(port) => port
                .parse::<u16>()
                .ok()
                .filter(|port| *port != 0)
                .ok_or_else(|| bad("the port is a number from 1 to 65535"))?,
        };
        Ok(Endpoint {
            tls,
            host: host.to_string(),
            port,
            base: path.trim_end_matches('/').to_string(),
        })
    }

    /// Whether the server is reached over TLS (`https://`), and so is known by its certificate and
    /// is not read by the network on the way.
    pub fn is_tls(&self) -> bool {
        self.tls
    }

    /// The `Host` header: the host, bracketed when it is an IPv6 address, and the port.
    fn authority(&self) -> String {
        if self.host.contains(':') {
            format!("[{}]:{}", self.host, self.port)
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }

    /// Whether this is this computer (a loopback address or `localhost`), where what is sent does
    /// not leave it. A tunnel to another computer looks the same, which is why a connection also
    /// carries a name a person gave it.
    pub fn is_loopback(&self) -> bool {
        self.host.eq_ignore_ascii_case("localhost")
            || self
                .host
                .parse::<IpAddr>()
                .is_ok_and(|address| address.is_loopback())
    }
}

/// One request.
#[derive(Debug, Clone, Copy)]
pub struct Request<'a> {
    pub method: &'a str,
    /// Appended to the path the address carries: `/models`, `/chat/completions`.
    pub path: &'a str,
    /// The credential, sent as `Authorization: Bearer` when there is one.
    pub bearer: Option<&'a str>,
    /// The JSON document sent, when there is one.
    pub json: Option<&'a [u8]>,
}

/// What came back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub body: Vec<u8>,
}

/// Why a request has no answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpError {
    /// It was told to stop, and the connection was closed.
    Cancelled,
    /// Its time was up, and the connection was closed.
    TimedOut,
    /// The server could not be reached; says why.
    Connect(String),
    /// The server is there and is not who its address says (or whom this build trusts): its
    /// certificate was refused. Says why. Nothing was sent to it.
    Certificate(String),
    /// The connection broke or the server said something that is not HTTP; says what.
    Broken(String),
    /// The answer is longer than the most that is read.
    TooLarge,
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpError::Cancelled => f.write_str("it was stopped"),
            HttpError::TimedOut => f.write_str("the server did not answer in time"),
            HttpError::Connect(why) => write!(f, "the server could not be reached: {why}"),
            HttpError::Certificate(why) => {
                write!(f, "the server's certificate was refused: {why}")
            }
            HttpError::Broken(why) => write!(f, "the connection to the server broke: {why}"),
            HttpError::TooLarge => f.write_str("the server's answer is larger than is read"),
        }
    }
}

impl std::error::Error for HttpError {}

/// What ends a request before the server answers.
struct Control<'a> {
    deadline: Instant,
    cancel: &'a Cancel,
}

impl Control<'_> {
    /// Whether the request is to go on. When it is not, the error ends [`send`], which drops the
    /// connection: that close is how the server is told that nobody is waiting for what it is
    /// computing, so nothing else is done to the socket here, and nothing may keep it open past
    /// the end of a request (a second handle to it would).
    fn check(&self) -> Result<(), HttpError> {
        if self.cancel.is_cancelled() {
            Err(HttpError::Cancelled)
        } else if Instant::now() >= self.deadline {
            Err(HttpError::TimedOut)
        } else {
            Ok(())
        }
    }
}

/// Make `request` to `endpoint` and read the answer: at most `max_body` bytes of it, and not past
/// `deadline`, and not after `cancel` is set. A server over `https://` is one whose certificate
/// the roots of the web vouch for.
pub fn send(
    endpoint: &Endpoint,
    request: Request<'_>,
    deadline: Instant,
    cancel: &Cancel,
    max_body: usize,
) -> Result<Response, HttpError> {
    send_trusting(&web_trust(), endpoint, request, deadline, cancel, max_body)
}

/// The same, with the certificates `trust` vouches for in place of the web's: what a test that
/// runs a server of its own, with a certificate of its own, needs.
pub(crate) fn send_trusting(
    trust: &Arc<ClientConfig>,
    endpoint: &Endpoint,
    request: Request<'_>,
    deadline: Instant,
    cancel: &Cancel,
    max_body: usize,
) -> Result<Response, HttpError> {
    let control = Control { deadline, cancel };
    let mut stream = connect(endpoint, trust, &control)?;
    stream
        .write_all(&head_of(endpoint, &request))
        .and_then(|()| match request.json {
            Some(body) => stream.write_all(body),
            None => Ok(()),
        })
        .and_then(|()| stream.flush())
        .map_err(|e| HttpError::Broken(format!("the request could not be sent: {e}")))?;
    let mut reader = Reader::new(stream, control);
    let (status, headers) = reader.head()?;
    let body = reader.body(status, &headers, max_body)?;
    Ok(Response { status, body })
}

fn head_of(endpoint: &Endpoint, request: &Request<'_>) -> Vec<u8> {
    let mut head = format!(
        "{} {}{} HTTP/1.1\r\nHost: {}\r\nUser-Agent: sce-workbench\r\nAccept: application/json\r\n\
         Connection: close\r\n",
        request.method,
        endpoint.base,
        request.path,
        endpoint.authority()
    );
    if let Some(key) = request.bearer {
        head.push_str(&format!("Authorization: Bearer {key}\r\n"));
    }
    if let Some(body) = request.json {
        head.push_str(&format!(
            "Content-Type: application/json\r\nContent-Length: {}\r\n",
            body.len()
        ));
    }
    head.push_str("\r\n");
    head.into_bytes()
}

/// The roots of the web as what a client trusts, made once.
fn web_trust() -> Arc<ClientConfig> {
    static TRUST: OnceLock<Arc<ClientConfig>> = OnceLock::new();
    Arc::clone(TRUST.get_or_init(|| {
        trusting(RootCertStore::from_iter(
            webpki_roots::TLS_SERVER_ROOTS.iter().cloned(),
        ))
    }))
}

/// A client that trusts the servers `roots` vouch for. The provider is named and not installed for
/// the process: a library does not decide that for the program it is in.
pub(crate) fn trusting(roots: RootCertStore) -> Arc<ClientConfig> {
    Arc::new(
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .expect("the provider has the protocol versions this build enables")
            .with_root_certificates(roots)
            .with_no_client_auth(),
    )
}

/// What a request is written to and read from: the socket, or the socket under TLS.
enum Transport {
    Plain(TcpStream),
    Tls(Box<StreamOwned<ClientConnection, TcpStream>>),
}

impl Read for Transport {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        match self {
            Transport::Plain(stream) => stream.read(buffer),
            // A server that closes without saying so (TLS's own `close_notify`) ends what it sends
            // as a plain one does. A body that its length or its chunks frame is still found cut
            // short by that framing; only a body that has none runs to the end of the connection.
            Transport::Tls(stream) => match stream.read(buffer) {
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => Ok(0),
                other => other,
            },
        }
    }
}

impl Write for Transport {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        match self {
            Transport::Plain(stream) => stream.write(bytes),
            Transport::Tls(stream) => stream.write(bytes),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Transport::Plain(stream) => stream.flush(),
            Transport::Tls(stream) => stream.flush(),
        }
    }
}

fn connect(
    endpoint: &Endpoint,
    trust: &Arc<ClientConfig>,
    control: &Control<'_>,
) -> Result<Transport, HttpError> {
    let tcp = reach(endpoint, control)?;
    if endpoint.tls {
        handshake(endpoint, trust, tcp, control)
    } else {
        Ok(Transport::Plain(tcp))
    }
}

/// The TLS handshake, in the slices a read is made in: a server that accepts the connection and
/// says nothing is waited for until the request is told to stop or its time is up, as an answer
/// is, and not for as long as the system's own timeouts allow.
fn handshake(
    endpoint: &Endpoint,
    trust: &Arc<ClientConfig>,
    mut tcp: TcpStream,
    control: &Control<'_>,
) -> Result<Transport, HttpError> {
    let name = ServerName::try_from(endpoint.host.clone()).map_err(|_| {
        HttpError::Connect(format!(
            "`{}` is not a name a certificate can be made out to",
            endpoint.host
        ))
    })?;
    let mut connection = ClientConnection::new(Arc::clone(trust), name)
        .map_err(|e| HttpError::Connect(e.to_string()))?;
    while connection.is_handshaking() {
        control.check()?;
        match connection.complete_io(&mut tcp) {
            Ok(_) => {}
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock
                        | io::ErrorKind::TimedOut
                        | io::ErrorKind::Interrupted
                ) => {}
            Err(e) => return Err(handshake_failure(e)),
        }
    }
    Ok(Transport::Tls(Box::new(StreamOwned::new(connection, tcp))))
}

/// What a handshake that did not finish says, in the words of the person's side of it.
fn handshake_failure(error: io::Error) -> HttpError {
    let tls = error
        .get_ref()
        .and_then(|inner| inner.downcast_ref::<rustls::Error>());
    match tls {
        Some(rustls::Error::InvalidCertificate(why)) => HttpError::Certificate(match why {
            CertificateError::UnknownIssuer | CertificateError::BadSignature => {
                "it is signed by an authority this application does not trust (a server whose \
                 certificate a private authority made is reached through a tunnel or a proxy of \
                 your own over http)"
                    .to_string()
            }
            CertificateError::NotValidForName | CertificateError::NotValidForNameContext { .. } => {
                "it is not made out to the name in the address".to_string()
            }
            CertificateError::Expired | CertificateError::ExpiredContext { .. } => {
                "it has expired".to_string()
            }
            CertificateError::NotValidYet | CertificateError::NotValidYetContext { .. } => {
                "it is not valid yet (check this computer's clock)".to_string()
            }
            other => format!("{other:?}"),
        }),
        Some(other) => HttpError::Broken(format!("the TLS handshake failed: {other}")),
        None if error.kind() == io::ErrorKind::UnexpectedEof => HttpError::Broken(
            "the server closed the connection during the TLS handshake: is the address https?"
                .to_string(),
        ),
        None => HttpError::Broken(format!("the TLS handshake failed: {error}")),
    }
}

fn reach(endpoint: &Endpoint, control: &Control<'_>) -> Result<TcpStream, HttpError> {
    let addresses = (endpoint.host.as_str(), endpoint.port)
        .to_socket_addrs()
        .map_err(|e| HttpError::Connect(format!("the address does not resolve ({e})")))?;
    let mut last = None;
    for address in addresses {
        let left = control.deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(HttpError::TimedOut);
        }
        if control.cancel.is_cancelled() {
            return Err(HttpError::Cancelled);
        }
        match TcpStream::connect_timeout(&address, CONNECT_WITHIN.min(left)) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(SLICE))
                    .and_then(|()| stream.set_write_timeout(Some(CONNECT_WITHIN)))
                    .and_then(|()| stream.set_nodelay(true))
                    .map_err(|e| HttpError::Connect(e.to_string()))?;
                return Ok(stream);
            }
            Err(e) => last = Some(e),
        }
    }
    Err(HttpError::Connect(last.map_or_else(
        || "the address names nothing".to_string(),
        |e| e.to_string(),
    )))
}

/// A connection read in slices, with what has been read and what of it has been used.
struct Reader<'a> {
    stream: Transport,
    control: Control<'a>,
    buffer: Vec<u8>,
    used: usize,
}

type Headers = Vec<(String, String)>;

impl<'a> Reader<'a> {
    fn new(stream: Transport, control: Control<'a>) -> Self {
        Reader {
            stream,
            control,
            buffer: Vec::new(),
            used: 0,
        }
    }

    /// Read some more. `0` is the end of what the server sends.
    fn fill(&mut self) -> Result<usize, HttpError> {
        let mut chunk = [0u8; 16 * 1024];
        loop {
            self.control.check()?;
            match self.stream.read(&mut chunk) {
                Ok(count) => {
                    self.buffer.extend_from_slice(&chunk[..count]);
                    return Ok(count);
                }
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock
                            | io::ErrorKind::TimedOut
                            | io::ErrorKind::Interrupted
                    ) => {}
                Err(e) => return Err(HttpError::Broken(e.to_string())),
            }
        }
    }

    fn unused(&self) -> &[u8] {
        &self.buffer[self.used..]
    }

    /// The status and the headers.
    fn head(&mut self) -> Result<(u16, Headers), HttpError> {
        let end = loop {
            if let Some(at) = find(self.unused(), b"\r\n\r\n") {
                break at;
            }
            if self.unused().len() > HEAD_MAX {
                return Err(HttpError::Broken(
                    "the head of the answer is longer than is read".to_string(),
                ));
            }
            if self.fill()? == 0 {
                return Err(HttpError::Broken(
                    "the server closed the connection before it answered".to_string(),
                ));
            }
        };
        let head = String::from_utf8_lossy(&self.unused()[..end]).into_owned();
        self.used += end + 4;
        let mut lines = head.split("\r\n");
        let status = lines
            .next()
            .and_then(|line| {
                let mut parts = line.split_whitespace();
                parts
                    .next()
                    .filter(|version| version.starts_with("HTTP/"))?;
                parts.next()?.parse::<u16>().ok()
            })
            .ok_or_else(|| HttpError::Broken("the answer is not HTTP".to_string()))?;
        let headers = lines
            .filter_map(|line| line.split_once(':'))
            .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_string()))
            .collect();
        Ok((status, headers))
    }

    fn body(&mut self, status: u16, headers: &Headers, max: usize) -> Result<Vec<u8>, HttpError> {
        let header = |name: &str| {
            headers
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.as_str())
        };
        if status == 204 || status == 304 || (100..200).contains(&status) {
            return Ok(Vec::new());
        }
        if header("transfer-encoding").is_some_and(|v| v.to_ascii_lowercase().contains("chunked")) {
            return self.chunked(max);
        }
        match header("content-length") {
            Some(length) => {
                let length: usize = length.parse().map_err(|_| {
                    HttpError::Broken("the answer's length is not a number".to_string())
                })?;
                if length > max {
                    return Err(HttpError::TooLarge);
                }
                self.take(length)
            }
            None => self.until_closed(max),
        }
    }

    /// Exactly `count` bytes.
    fn take(&mut self, count: usize) -> Result<Vec<u8>, HttpError> {
        while self.unused().len() < count {
            if self.fill()? == 0 {
                return Err(HttpError::Broken(
                    "the connection closed before the whole answer came".to_string(),
                ));
            }
        }
        let body = self.unused()[..count].to_vec();
        self.used += count;
        Ok(body)
    }

    /// Everything the server sends until it closes the connection.
    fn until_closed(&mut self, max: usize) -> Result<Vec<u8>, HttpError> {
        loop {
            if self.unused().len() > max {
                return Err(HttpError::TooLarge);
            }
            if self.fill()? == 0 {
                break;
            }
        }
        let body = self.unused().to_vec();
        self.used = self.buffer.len();
        Ok(body)
    }

    /// One line of a chunked body's framing, without its line end.
    fn line(&mut self) -> Result<String, HttpError> {
        loop {
            if let Some(at) = find(self.unused(), b"\r\n") {
                let line = String::from_utf8_lossy(&self.unused()[..at]).into_owned();
                self.used += at + 2;
                return Ok(line);
            }
            if self.unused().len() > LINE_MAX {
                return Err(HttpError::Broken(
                    "a line of the answer is too long".to_string(),
                ));
            }
            if self.fill()? == 0 {
                return Err(HttpError::Broken(
                    "the connection closed inside the answer".to_string(),
                ));
            }
        }
    }

    fn chunked(&mut self, max: usize) -> Result<Vec<u8>, HttpError> {
        let mut body = Vec::new();
        loop {
            let line = self.line()?;
            // A chunk's size is in hex, and may be followed by extensions after a `;`.
            let size = line.split(';').next().unwrap_or("").trim();
            let size = usize::from_str_radix(size, 16)
                .map_err(|_| HttpError::Broken("a chunk's size is not a number".to_string()))?;
            if size == 0 {
                // Trailers, if there are any, end at an empty line.
                while !self.line()?.is_empty() {}
                return Ok(body);
            }
            if body.len() + size > max {
                return Err(HttpError::TooLarge);
            }
            body.extend(self.take(size)?);
            if !self.line()?.is_empty() {
                return Err(HttpError::Broken(
                    "a chunk is not followed by its end".to_string(),
                ));
            }
        }
    }
}

/// Where `needle` first is in `haystack`.
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;

    fn endpoint(address: &str) -> Endpoint {
        Endpoint::parse(address).unwrap()
    }

    #[test]
    fn an_address_is_a_host_a_port_and_the_path_it_carries() {
        for (address, host, port, base) in [
            ("http://127.0.0.1:11434/v1", "127.0.0.1", 11434, "/v1"),
            ("http://127.0.0.1:11434/v1/", "127.0.0.1", 11434, "/v1"),
            ("http://localhost", "localhost", 80, ""),
            ("http://localhost/", "localhost", 80, ""),
            (
                "http://model-box.lan:9090/api/openai/v1",
                "model-box.lan",
                9090,
                "/api/openai/v1",
            ),
            ("http://[::1]:1234/v1", "::1", 1234, "/v1"),
            ("http://[::1]/v1", "::1", 80, "/v1"),
        ] {
            assert_eq!(
                endpoint(address),
                Endpoint {
                    tls: false,
                    host: host.to_string(),
                    port,
                    base: base.to_string(),
                },
                "{address}"
            );
        }
    }

    #[test]
    fn an_address_that_is_not_one_is_refused_and_says_what_is_wrong() {
        for address in [
            "ftp://x",
            "127.0.0.1:11434",
            "http://",
            "http://:80",
            "http://host:0",
            "http://host:70000",
            "http://host:port",
            "http://[::1",
            "http://[::1]x",
        ] {
            let refused = Endpoint::parse(address).unwrap_err();
            assert!(
                matches!(refused, EndpointError::Malformed(_)),
                "{address}: {refused:?}"
            );
        }
    }

    #[test]
    fn https_is_an_address_that_is_reached_over_tls_and_has_a_port_of_its_own() {
        for (address, host, port, base) in [
            ("https://example.test/v1", "example.test", 443, "/v1"),
            ("https://example.test:8443/v1/", "example.test", 8443, "/v1"),
            ("https://[::1]/", "::1", 443, ""),
        ] {
            assert_eq!(
                endpoint(address),
                Endpoint {
                    tls: true,
                    host: host.to_string(),
                    port,
                    base: base.to_string(),
                },
                "{address}"
            );
        }
        assert!(endpoint("https://example.test").is_tls());
        assert!(!endpoint("http://example.test").is_tls());
    }

    #[test]
    fn this_computer_is_told_from_another_by_the_address() {
        for address in [
            "http://127.0.0.1:1",
            "http://localhost:1",
            "http://LOCALHOST",
            "http://[::1]:1",
        ] {
            assert!(endpoint(address).is_loopback(), "{address}");
        }
        for address in [
            "http://192.168.0.5:1",
            "http://model-box.lan",
            "http://10.0.0.1",
        ] {
            assert!(!endpoint(address).is_loopback(), "{address}");
        }
    }

    /// One request (as bytes) read from `stream`: its head, and the body the head says follows.
    /// What was read when the stream ends or fails is what there is.
    fn read_request(stream: &mut impl Read) -> Vec<u8> {
        let mut request = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            let count = stream.read(&mut chunk).unwrap_or(0);
            request.extend_from_slice(&chunk[..count]);
            if let Some(at) = find(&request, b"\r\n\r\n") {
                let head = String::from_utf8_lossy(&request[..at]).to_ascii_lowercase();
                let length = head
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length:"))
                    .and_then(|value| value.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                if request.len() >= at + 4 + length {
                    break;
                }
            }
            if count == 0 {
                break;
            }
        }
        request
    }

    /// A server that reads one request (as bytes) and answers with `answer`, sent as it says.
    fn serve(
        answer: impl FnOnce(&mut TcpStream) + Send + 'static,
    ) -> (String, mpsc::Receiver<Vec<u8>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}/v1", listener.local_addr().unwrap());
        let (seen, requests) = mpsc::channel();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let _ = seen.send(read_request(&mut stream));
            answer(&mut stream);
        });
        (address, requests)
    }

    fn within(seconds: u64) -> Instant {
        Instant::now() + Duration::from_secs(seconds)
    }

    fn get(address: &str, path: &str) -> Result<Response, HttpError> {
        send(
            &endpoint(address),
            Request {
                method: "GET",
                path,
                bearer: None,
                json: None,
            },
            within(10),
            &Cancel::new(),
            1024 * 1024,
        )
    }

    #[test]
    fn an_answer_with_a_length_is_read_whole() {
        let (address, _) = serve(|s| {
            s.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 11\r\n\r\n{\"ok\":true}").unwrap();
        });

        let got = get(&address, "/models").unwrap();

        assert_eq!(got.status, 200);
        assert_eq!(got.body, b"{\"ok\":true}");
    }

    #[test]
    fn an_answer_in_chunks_is_put_together_and_its_extensions_and_trailers_are_ignored() {
        let (address, _) = serve(|s| {
            s.write_all(
                // A chunk's size is in hex: `a` is ten and `10` is sixteen, which a size read as
                // decimal gets wrong (`a` is no number, and `10` is ten and not sixteen).
                b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\na;ext=1\r\n0123456789\r\n10\r\n0123456789abcdef\r\n0\r\nX-Trailer: 1\r\n\r\n",
            )
            .unwrap();
        });

        let got = get(&address, "/models").unwrap();

        assert_eq!(got.body, b"01234567890123456789abcdef");
    }

    #[test]
    fn an_answer_with_no_length_is_what_comes_until_the_server_closes() {
        let (address, _) = serve(|s| {
            s.write_all(b"HTTP/1.1 200 OK\r\n\r\nhello there").unwrap();
        });

        assert_eq!(get(&address, "/models").unwrap().body, b"hello there");
    }

    #[test]
    fn an_answer_that_is_not_a_success_is_still_an_answer_with_its_body() {
        let (address, _) = serve(|s| {
            s.write_all(
                b"HTTP/1.1 404 Not Found\r\nContent-Length: 17\r\n\r\n{\"error\":\"nope.\"}",
            )
            .unwrap();
        });

        let got = get(&address, "/nothing").unwrap();

        assert_eq!(got.status, 404);
        assert_eq!(got.body, b"{\"error\":\"nope.\"}");
    }

    #[test]
    fn the_request_names_the_host_the_path_and_the_length_and_carries_a_key_only_when_there_is_one()
    {
        let (address, requests) = serve(|s| {
            s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                .unwrap();
        });

        send(
            &endpoint(&address),
            Request {
                method: "POST",
                path: "/chat/completions",
                bearer: Some("sk-secret"),
                json: Some(b"{\"model\":\"m\"}"),
            },
            within(10),
            &Cancel::new(),
            1024,
        )
        .unwrap();

        let sent = String::from_utf8(requests.recv().unwrap()).unwrap();
        assert!(
            sent.starts_with("POST /v1/chat/completions HTTP/1.1\r\n"),
            "{sent}"
        );
        assert!(sent.contains("\r\nHost: 127.0.0.1:"), "{sent}");
        assert!(
            sent.contains("\r\nAuthorization: Bearer sk-secret\r\n"),
            "{sent}"
        );
        assert!(sent.contains("\r\nContent-Length: 13\r\n"), "{sent}");
        assert!(sent.contains("\r\nConnection: close\r\n"), "{sent}");
        assert!(sent.ends_with("{\"model\":\"m\"}"), "{sent}");
    }

    #[test]
    fn a_request_with_no_key_sends_no_authorization() {
        let (address, requests) = serve(|s| {
            s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                .unwrap();
        });

        get(&address, "/models").unwrap();

        let sent = String::from_utf8(requests.recv().unwrap())
            .unwrap()
            .to_ascii_lowercase();
        assert!(!sent.contains("authorization"), "{sent}");
    }

    #[test]
    fn a_server_that_does_not_answer_is_waited_for_until_it_is_told_to_stop_and_then_closed() {
        let (closed_tx, closed_rx) = mpsc::channel();
        let (address, _) = serve(move |s| {
            // Says nothing, and notes when the other end goes away: a close, or a reset. A read
            // that only timed out is the other end still being there.
            s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
            let mut byte = [0u8; 1];
            let closed = match s.read(&mut byte) {
                Ok(count) => count == 0,
                Err(e) => matches!(
                    e.kind(),
                    io::ErrorKind::ConnectionReset
                        | io::ErrorKind::ConnectionAborted
                        | io::ErrorKind::BrokenPipe
                ),
            };
            let _ = closed_tx.send(closed);
        });
        let cancel = Cancel::new();
        let stopper = {
            let cancel = cancel.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(300));
                cancel.cancel();
            })
        };

        let started = Instant::now();
        let got = send(
            &endpoint(&address),
            Request {
                method: "GET",
                path: "/models",
                bearer: None,
                json: None,
            },
            within(30),
            &cancel,
            1024,
        );
        stopper.join().unwrap();

        assert_eq!(got, Err(HttpError::Cancelled));
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "{:?}",
            started.elapsed()
        );
        // The server saw the connection close: it is not left working for nobody.
        assert!(closed_rx.recv_timeout(Duration::from_secs(5)).unwrap());
    }

    #[test]
    fn a_server_that_takes_longer_than_the_time_given_is_stopped_at_that_time() {
        let (address, _) = serve(|s| {
            thread::sleep(Duration::from_secs(3));
            let _ = s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");
        });

        let started = Instant::now();
        let got = send(
            &endpoint(&address),
            Request {
                method: "GET",
                path: "/models",
                bearer: None,
                json: None,
            },
            Instant::now() + Duration::from_millis(400),
            &Cancel::new(),
            1024,
        );

        assert_eq!(got, Err(HttpError::TimedOut));
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "{:?}",
            started.elapsed()
        );
    }

    #[test]
    fn an_answer_larger_than_the_most_read_is_refused_by_its_length_and_when_it_has_none() {
        let (address, _) = serve(|s| {
            s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 5000\r\n\r\n")
                .unwrap();
        });
        let by_length = send(
            &endpoint(&address),
            Request {
                method: "GET",
                path: "/m",
                bearer: None,
                json: None,
            },
            within(10),
            &Cancel::new(),
            1000,
        );
        let (address, _) = serve(|s| {
            let _ = s.write_all(b"HTTP/1.1 200 OK\r\n\r\n");
            let _ = s.write_all(&vec![b'x'; 5000]);
        });
        let to_the_end = send(
            &endpoint(&address),
            Request {
                method: "GET",
                path: "/m",
                bearer: None,
                json: None,
            },
            within(10),
            &Cancel::new(),
            1000,
        );

        assert_eq!(by_length, Err(HttpError::TooLarge));
        assert_eq!(to_the_end, Err(HttpError::TooLarge));
    }

    #[test]
    fn a_server_that_is_not_there_is_said_so_and_a_server_that_is_not_http_is_said_so() {
        // A port nothing listens on. Not one that was bound and let go of: tests run side by side,
        // and another's server may be given that port between the letting go and the asking. Port 1
        // is below the range the system hands out.
        let nobody = get("http://127.0.0.1:1/v1", "/models").unwrap_err();
        let (address, _) = serve(|s| {
            s.write_all(b"SSH-2.0-OpenSSH\r\n\r\n").unwrap();
        });
        let not_http = get(&address, "/models").unwrap_err();

        assert!(matches!(nobody, HttpError::Connect(_)), "{nobody:?}");
        assert!(
            nobody.to_string().contains("could not be reached"),
            "{nobody}"
        );
        assert_eq!(
            not_http,
            HttpError::Broken("the answer is not HTTP".to_string())
        );
    }

    #[test]
    fn a_server_that_closes_before_it_answers_is_said_to_have() {
        let (address, _) = serve(|_| {});

        let got = get(&address, "/models").unwrap_err();

        assert!(matches!(got, HttpError::Broken(_)), "{got:?}");
    }

    // ---- over TLS ------------------------------------------------------------------------

    use rcgen::{BasicConstraints, CertificateParams, IsCa, KeyPair, KeyUsagePurpose, SanType};
    use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
    use rustls::{ServerConfig, ServerConnection};
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// An authority and the certificate it made out to some names, made now.
    struct Pki {
        root: CertificateDer<'static>,
        leaf: CertificateDer<'static>,
        key: Vec<u8>,
    }

    /// The certificate of a server named `names`, valid from `from` to `until` (years), made by an
    /// authority of a name of its own: two authorities with one name are told apart by their
    /// signatures only, which is another way to fail.
    fn pki_valid(names: Vec<SanType>, from: i32, until: i32) -> Pki {
        static MADE: AtomicUsize = AtomicUsize::new(0);
        let name = format!("test authority {}", MADE.fetch_add(1, Ordering::Relaxed));
        pki_by(names, from, until, &name)
    }

    /// The same, made by the authority that is named `authority`: another that bears the same name
    /// is a look-alike.
    fn pki_by(names: Vec<SanType>, from: i32, until: i32, authority: &str) -> Pki {
        let mut authority_params = CertificateParams::new(Vec::new()).unwrap();
        authority_params
            .distinguished_name
            .push(rcgen::DnType::CommonName, authority.to_string());
        let mut authority = authority_params;
        authority.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        authority.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        let authority_key = KeyPair::generate().unwrap();
        let authority = authority.self_signed(&authority_key).unwrap();
        let mut leaf = CertificateParams::new(Vec::new()).unwrap();
        leaf.subject_alt_names = names;
        leaf.not_before = rcgen::date_time_ymd(from, 1, 1);
        leaf.not_after = rcgen::date_time_ymd(until, 1, 1);
        let leaf_key = KeyPair::generate().unwrap();
        let leaf = leaf
            .signed_by(&leaf_key, &authority, &authority_key)
            .unwrap();
        Pki {
            root: authority.der().clone(),
            leaf: leaf.der().clone(),
            key: leaf_key.serialize_der(),
        }
    }

    fn pki(names: Vec<SanType>) -> Pki {
        pki_valid(names, 2000, 2999)
    }

    fn localhost() -> Vec<SanType> {
        vec![SanType::DnsName("localhost".try_into().unwrap())]
    }

    /// A client that trusts the servers `root` vouches for, and nobody else.
    fn trusting_only(root: &CertificateDer<'static>) -> Arc<ClientConfig> {
        let mut roots = RootCertStore::empty();
        roots.add(root.clone()).unwrap();
        trusting(roots)
    }

    type TlsStream = StreamOwned<ServerConnection, TcpStream>;

    /// A server that speaks TLS with `pki`'s certificate, reads one request, and answers with
    /// `answer`. The address is `https://localhost:port/v1` unless `by_ip` says otherwise.
    fn serve_tls(
        pki: &Pki,
        by_ip: bool,
        answer: impl FnOnce(&mut TlsStream) + Send + 'static,
    ) -> (String, mpsc::Receiver<Vec<u8>>) {
        let config = Arc::new(
            ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_safe_default_protocol_versions()
                .unwrap()
                .with_no_client_auth()
                .with_single_cert(
                    vec![pki.leaf.clone()],
                    PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(pki.key.clone())),
                )
                .unwrap(),
        );
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let host = if by_ip { "127.0.0.1" } else { "localhost" };
        let address = format!("https://{host}:{port}/v1");
        let (seen, requests) = mpsc::channel();
        thread::spawn(move || {
            let (tcp, _) = listener.accept().unwrap();
            tcp.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut stream = StreamOwned::new(ServerConnection::new(config).unwrap(), tcp);
            let _ = seen.send(read_request(&mut stream));
            answer(&mut stream);
        });
        (address, requests)
    }

    fn get_trusting(
        trust: &Arc<ClientConfig>,
        address: &str,
        deadline: Instant,
        cancel: &Cancel,
    ) -> Result<Response, HttpError> {
        send_trusting(
            trust,
            &endpoint(address),
            Request {
                method: "GET",
                path: "/models",
                bearer: None,
                json: None,
            },
            deadline,
            cancel,
            1024 * 1024,
        )
    }

    const OK_WITH_LENGTH: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\n\r\n{\"ok\":true}";

    /// What a server says when the request reaches it. A client that refused the certificate has
    /// sent an alert and no request, and a server that answers it cannot: that is not a failure of
    /// the server, so it is not unwrapped.
    fn says_ok(stream: &mut TlsStream) {
        let _ = stream.write_all(OK_WITH_LENGTH);
    }

    #[test]
    fn an_answer_over_tls_is_read_as_it_is_over_http_and_the_request_is_the_same() {
        let pki = pki(localhost());
        let (address, requests) = serve_tls(&pki, false, says_ok);

        let got = get_trusting(
            &trusting_only(&pki.root),
            &address,
            within(10),
            &Cancel::new(),
        )
        .unwrap();

        assert_eq!(got.status, 200);
        assert_eq!(got.body, b"{\"ok\":true}");
        // What the server read, once it was decrypted: a request, with the name that is in the
        // certificate as its host.
        let sent = String::from_utf8(requests.recv().unwrap()).unwrap();
        assert!(sent.starts_with("GET /v1/models HTTP/1.1\r\n"), "{sent}");
        assert!(sent.contains("\r\nHost: localhost:"), "{sent}");
    }

    #[test]
    fn a_server_known_by_its_address_is_known_by_the_address_in_its_certificate() {
        let pki = pki(vec![SanType::IpAddress("127.0.0.1".parse().unwrap())]);
        let (address, _) = serve_tls(&pki, true, says_ok);

        let got = get_trusting(
            &trusting_only(&pki.root),
            &address,
            within(10),
            &Cancel::new(),
        );

        assert_eq!(got.unwrap().status, 200);
    }

    #[test]
    fn a_server_whose_certificate_an_authority_not_trusted_made_is_refused_and_is_sent_nothing() {
        let pki = pki(localhost());
        let stranger = self::pki(localhost());
        let (address, requests) = serve_tls(&pki, false, says_ok);

        let got = get_trusting(
            &trusting_only(&stranger.root),
            &address,
            within(10),
            &Cancel::new(),
        )
        .unwrap_err();

        let HttpError::Certificate(why) = &got else {
            panic!("expected a certificate refused, got {got:?}");
        };
        assert!(
            why.contains("authority this application does not trust"),
            "{why}"
        );
        // The server read no request, because none was sent: it was the handshake that failed.
        assert!(requests.recv().unwrap().is_empty());
    }

    #[test]
    fn an_authority_that_bears_the_name_of_a_trusted_one_is_refused_in_the_same_words() {
        // Told apart from the one that is trusted by its signature only: the chain names an issuer
        // the client has, and the issuer did not sign it.
        let trusted = pki_by(localhost(), 2000, 2999, "the same name");
        let look_alike = pki_by(localhost(), 2000, 2999, "the same name");
        let (address, _) = serve_tls(&look_alike, false, says_ok);

        let got = get_trusting(
            &trusting_only(&trusted.root),
            &address,
            within(10),
            &Cancel::new(),
        )
        .unwrap_err();

        let HttpError::Certificate(why) = &got else {
            panic!("expected a certificate refused, got {got:?}");
        };
        assert!(
            why.contains("authority this application does not trust"),
            "{why}"
        );
    }

    #[test]
    fn a_certificate_made_out_to_another_name_is_refused() {
        // The server is known by its address, and the certificate is for `localhost`.
        let pki = pki(localhost());
        let (address, _) = serve_tls(&pki, true, says_ok);

        let got = get_trusting(
            &trusting_only(&pki.root),
            &address,
            within(10),
            &Cancel::new(),
        )
        .unwrap_err();

        let HttpError::Certificate(why) = &got else {
            panic!("expected a certificate refused, got {got:?}");
        };
        assert!(why.contains("not made out to the name"), "{why}");
    }

    #[test]
    fn a_certificate_that_has_run_out_is_refused() {
        let pki = pki_valid(localhost(), 2001, 2002);
        let (address, _) = serve_tls(&pki, false, says_ok);

        let got = get_trusting(
            &trusting_only(&pki.root),
            &address,
            within(10),
            &Cancel::new(),
        )
        .unwrap_err();

        let HttpError::Certificate(why) = &got else {
            panic!("expected a certificate refused, got {got:?}");
        };
        assert!(why.contains("expired"), "{why}");
    }

    #[test]
    fn the_roots_of_the_web_do_not_vouch_for_a_certificate_of_a_test() {
        // What `send` trusts: not an authority made a moment ago.
        let pki = pki(localhost());
        let (address, _) = serve_tls(&pki, false, says_ok);

        let got = send(
            &endpoint(&address),
            Request {
                method: "GET",
                path: "/models",
                bearer: None,
                json: None,
            },
            within(10),
            &Cancel::new(),
            1024,
        )
        .unwrap_err();

        assert!(matches!(got, HttpError::Certificate(_)), "{got:?}");
    }

    #[test]
    fn a_server_that_does_not_speak_tls_is_said_not_to() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!(
            "https://localhost:{}/v1",
            listener.local_addr().unwrap().port()
        );
        thread::spawn(move || {
            let (mut tcp, _) = listener.accept().unwrap();
            let _ = tcp.write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n");
        });

        let got = get_trusting(
            &trusting_only(&pki(localhost()).root),
            &address,
            within(10),
            &Cancel::new(),
        )
        .unwrap_err();

        let HttpError::Broken(why) = &got else {
            panic!("expected a broken connection, got {got:?}");
        };
        assert!(why.contains("TLS handshake"), "{why}");
    }

    #[test]
    fn a_server_that_takes_the_connection_and_says_nothing_is_waited_for_in_the_handshake_only_as_long_as_told(
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!(
            "https://localhost:{}/v1",
            listener.local_addr().unwrap().port()
        );
        let (held_tx, held_rx) = mpsc::channel();
        thread::spawn(move || {
            // Holds every connection it is given, open, until the test is over.
            let mut held = Vec::new();
            while let Ok((tcp, _)) = listener.accept() {
                held.push(tcp);
                let _ = held_tx.send(());
            }
        });
        let trust = trusting_only(&pki(localhost()).root);

        let cancel = Cancel::new();
        let stopper = {
            let cancel = cancel.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(300));
                cancel.cancel();
            })
        };
        let started = Instant::now();
        let stopped = get_trusting(&trust, &address, within(30), &cancel);
        stopper.join().unwrap();
        let stopped_in = started.elapsed();
        let started = Instant::now();
        let late = get_trusting(
            &trust,
            &address,
            Instant::now() + Duration::from_millis(400),
            &Cancel::new(),
        );

        assert_eq!(stopped, Err(HttpError::Cancelled));
        assert!(stopped_in < Duration::from_secs(5), "{stopped_in:?}");
        assert_eq!(late, Err(HttpError::TimedOut));
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "{:?}",
            started.elapsed()
        );
        assert!(held_rx.try_recv().is_ok(), "the server was never reached");
    }

    #[test]
    fn a_server_that_closes_without_saying_so_ends_an_answer_that_has_no_length_and_not_one_that_has(
    ) {
        let pki = pki(localhost());
        let trust = trusting_only(&pki.root);
        let (address, _) = serve_tls(&pki, false, |s| {
            s.write_all(b"HTTP/1.1 200 OK\r\n\r\nhello there").unwrap();
            s.conn.send_close_notify();
            let _ = s.flush();
        });
        let said_so = get_trusting(&trust, &address, within(10), &Cancel::new());
        let (address, _) = serve_tls(&pki, false, |s| {
            // No close_notify: the stream is dropped, and the connection closes under TLS.
            s.write_all(b"HTTP/1.1 200 OK\r\n\r\nhello there").unwrap();
            let _ = s.flush();
        });
        let without = get_trusting(&trust, &address, within(10), &Cancel::new());
        let (address, _) = serve_tls(&pki, false, |s| {
            s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 20\r\n\r\nonly five")
                .unwrap();
            let _ = s.flush();
        });
        let cut = get_trusting(&trust, &address, within(10), &Cancel::new());

        assert_eq!(said_so.unwrap().body, b"hello there");
        assert_eq!(without.unwrap().body, b"hello there");
        // A length that was not met is a cut answer, whether or not the server said it closed.
        assert!(matches!(cut, Err(HttpError::Broken(_))), "{cut:?}");
    }
}
