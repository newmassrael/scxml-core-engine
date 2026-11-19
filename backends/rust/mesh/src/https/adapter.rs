// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! This crate's adapter of the HTTPS port (SCE_MESH.md §mesh-22.6): hyper over rustls, with the
//! operating system's certificate verifier and the platform's proxy settings.
//!
//! * **Trust.** A server's certificate is verified by `rustls-platform-verifier`, which asks the
//!   operating system (the Windows certificate store, the Apple trust settings, the Android
//!   platform, or on another Unix the system's certificate bundle). An authority the user's
//!   machine trusts, such as the one a network that inspects TLS signs with, is trusted here too.
//!   On Android the host initialises the verifier with the application's context before the
//!   first exchange, as that crate documents, and no additional authority can be added there.
//! * **Proxy.** The proxy is the platform's: `HTTPS_PROXY`, `https_proxy`, `ALL_PROXY` and
//!   `NO_PROXY` everywhere, and the system's own settings on macOS and Windows. An `https://`
//!   server behind a proxy is reached through a `CONNECT` tunnel, with the proxy's basic
//!   credentials when its URL carries them. An `http://` server (a loopback test) is reached
//!   directly.
//! * **HTTP.** HTTP/2 when the server offers it in the TLS handshake, HTTP/1.1 otherwise. A
//!   connection is kept for the next exchange to the same origin: an HTTP/2 one is shared by
//!   concurrent exchanges, an HTTP/1.1 one is taken by one exchange at a time.
//!
//! The adapter runs its own tokio runtime and blocks the calling thread on it, so it is called
//! from a thread that is not one of a tokio runtime's (the binding's client threads are not).

use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use std::sync::Mutex;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::client::conn::{http1, http2};
use hyper::header::HOST;
use hyper::{Method, Request, Uri};
use hyper_util::client::proxy::matcher::Matcher;
use hyper_util::rt::{TokioExecutor, TokioIo};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use rustls::ClientConfig;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use super::{HttpsFailure, HttpsRequest, HttpsResponse, HttpsTransport};

/// Which proxy the adapter reaches an `https://` server through.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ProxyPolicy {
    /// The platform's settings (§mesh-22.6): the proxy variables, and the operating system's
    /// own settings where it keeps them.
    #[default]
    System,
    /// This proxy (`http://<host>:<port>`, with `user:password@` for basic credentials) for every
    /// server, whatever the platform says: for a test, or a host that takes its proxy from a
    /// configuration of its own.
    Through(String),
    /// No proxy.
    Direct,
}

/// What an adapter is built with.
#[derive(Clone, Default)]
pub struct AdapterSettings {
    /// Authorities (each a DER certificate) believed beside those the operating system believes:
    /// a deployment's private authority, or a test's. Refused on Android, where the platform's
    /// store is the only one the verifier reads.
    pub extra_authorities: Vec<Vec<u8>>,
    /// Which proxy reaches a server.
    pub proxy: ProxyPolicy,
    /// The certificate chain (each DER, the client's own first) and its private key (DER) a
    /// client of the `mtls` authenticator presents; `None` for the other two.
    pub client_identity: Option<(Vec<Vec<u8>>, Vec<u8>)>,
}

/// Why an adapter could not be built.
#[derive(Debug)]
pub enum AdapterError {
    /// The TLS configuration was refused: an authority or an identity that does not parse, or a
    /// verifier the platform could not give.
    Tls(rustls::Error),
    /// The adapter's runtime could not be started.
    Runtime(std::io::Error),
}

/// A connection kept for the next exchange to its origin.
enum Sender {
    One(http1::SendRequest<Full<Bytes>>),
    Two(http2::SendRequest<Full<Bytes>>),
}

impl Sender {
    fn is_closed(&self) -> bool {
        match self {
            Sender::One(sender) => sender.is_closed(),
            Sender::Two(sender) => sender.is_closed(),
        }
    }
}

/// The reference adapter of the HTTPS port for Rust on every operating system (§mesh-22.6).
pub struct HyperTransport {
    runtime: tokio::runtime::Runtime,
    tls: Arc<ClientConfig>,
    proxies: Option<Matcher>,
    /// Connections kept, by origin (`scheme://authority`).
    kept: Mutex<Vec<(String, Sender)>>,
}

impl HyperTransport {
    /// An adapter with `settings`.
    pub fn new(settings: AdapterSettings) -> Result<HyperTransport, AdapterError> {
        let tls = client_tls(&settings).map_err(AdapterError::Tls)?;
        let proxies = match &settings.proxy {
            ProxyPolicy::System => Some(Matcher::from_system()),
            ProxyPolicy::Through(proxy) => Some(Matcher::builder().https(proxy.as_str()).build()),
            ProxyPolicy::Direct => None,
        };
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("sce-mesh-https")
            .enable_all()
            .build()
            .map_err(AdapterError::Runtime)?;
        Ok(HyperTransport {
            runtime,
            tls,
            proxies,
            kept: Mutex::new(Vec::new()),
        })
    }

    /// A kept connection to `origin` that is still open. An HTTP/2 one stays kept, since others
    /// may share it; an HTTP/1.1 one is taken, and put back when its exchange ends.
    fn kept_for(&self, origin: &str) -> Option<Sender> {
        let mut kept = self.kept.lock().expect(KEPT);
        kept.retain(|(_, sender)| !sender.is_closed());
        let at = kept.iter().position(|(held, _)| held == origin)?;
        let shared = match &kept[at].1 {
            Sender::Two(sender) => Some(sender.clone()),
            Sender::One(_) => None,
        };
        Some(match shared {
            Some(sender) => Sender::Two(sender),
            None => kept.swap_remove(at).1,
        })
    }

    fn keep(&self, origin: String, sender: Sender) {
        let mut kept = self.kept.lock().expect(KEPT);
        if let Sender::Two(_) = sender {
            if kept
                .iter()
                .any(|(held, held_sender)| *held == origin && matches!(held_sender, Sender::Two(_)))
            {
                return;
            }
        }
        kept.push((origin, sender));
    }

    async fn carry(&self, request: HttpsRequest) -> Result<HttpsResponse, HttpsFailure> {
        let uri: Uri = request
            .url
            .parse()
            .map_err(|why| HttpsFailure::Other(alloc::format!("not a URL: {why}")))?;
        let scheme = uri.scheme_str().unwrap_or_default().to_string();
        let authority = uri
            .authority()
            .map(|authority| authority.as_str().to_string())
            .ok_or_else(|| HttpsFailure::Other("a URL without a host".to_string()))?;
        let origin = alloc::format!("{scheme}://{authority}");

        if let Some(sender) = self.kept_for(&origin) {
            match send(sender, &uri, &authority, &request).await {
                Ok((response, sender)) => {
                    self.keep(origin, sender);
                    return Ok(response);
                }
                // The kept connection had closed before the request was written: nothing reached
                // the server, so a new connection carries it.
                Err(Failed::NotWritten) => {}
                Err(Failed::After(failure)) => return Err(failure),
            }
        }
        let sender = self.connect(&uri, &scheme).await?;
        match send(sender, &uri, &authority, &request).await {
            Ok((response, sender)) => {
                self.keep(origin, sender);
                Ok(response)
            }
            Err(Failed::NotWritten) => Err(HttpsFailure::Other(
                "the connection closed before the request was written".to_string(),
            )),
            Err(Failed::After(failure)) => Err(failure),
        }
    }

    /// A new connection to the server `uri` names, through the proxy when one intercepts it.
    async fn connect(&self, uri: &Uri, scheme: &str) -> Result<Sender, HttpsFailure> {
        let host = uri
            .host()
            .ok_or_else(|| HttpsFailure::Other("a URL without a host".to_string()))?;
        let bare_host = host.trim_start_matches('[').trim_end_matches(']');
        let secure = match scheme {
            "https" => true,
            "http" => false,
            other => return Err(HttpsFailure::Other(alloc::format!("no scheme {other}"))),
        };
        let port = uri.port_u16().unwrap_or(if secure { 443 } else { 80 });
        // §mesh-22.6: the server is reached through the proxy the platform's settings name, by a
        // `CONNECT` tunnel, and a tunnel the proxy refuses is reported as `Proxy`.
        let proxy = if secure {
            self.proxies
                .as_ref()
                .and_then(|proxies| proxies.intercept(uri))
        } else {
            None
        };
        let tcp = match proxy {
            None => TcpStream::connect((bare_host, port))
                .await
                .map_err(|_| HttpsFailure::Connect)?,
            Some(proxy) => {
                let through = proxy.uri();
                if through.scheme_str() != Some("http") {
                    // A proxy reached by TLS or by SOCKS is a proxy this adapter does not speak.
                    return Err(HttpsFailure::Proxy);
                }
                let proxy_host = through.host().ok_or(HttpsFailure::Proxy)?;
                let proxy_host = proxy_host.trim_start_matches('[').trim_end_matches(']');
                let proxy_port = through.port_u16().unwrap_or(80);
                let mut tcp = TcpStream::connect((proxy_host, proxy_port))
                    .await
                    .map_err(|_| HttpsFailure::Connect)?;
                let credentials = proxy
                    .basic_auth()
                    .and_then(|value| value.to_str().ok())
                    .map(str::to_string);
                tunnel(
                    &mut tcp,
                    &alloc::format!("{host}:{port}"),
                    credentials.as_deref(),
                )
                .await?;
                tcp
            }
        };
        let _ = tcp.set_nodelay(true);
        if !secure {
            let (sender, connection) = http1::handshake(TokioIo::new(tcp))
                .await
                .map_err(|why| HttpsFailure::Other(why.to_string()))?;
            tokio::spawn(async move {
                let _ = connection.await;
            });
            return Ok(Sender::One(sender));
        }
        let name = ServerName::try_from(bare_host.to_string())
            .map_err(|_| HttpsFailure::Other(alloc::format!("not a server name: {host}")))?;
        let stream = tokio_rustls::TlsConnector::from(Arc::clone(&self.tls))
            .connect(name, tcp)
            .await
            .map_err(|_| HttpsFailure::Tls)?;
        let two = stream.get_ref().1.alpn_protocol() == Some(b"h2".as_slice());
        if two {
            let (sender, connection) = http2::handshake(TokioExecutor::new(), TokioIo::new(stream))
                .await
                .map_err(|why| HttpsFailure::Other(why.to_string()))?;
            tokio::spawn(async move {
                let _ = connection.await;
            });
            Ok(Sender::Two(sender))
        } else {
            let (sender, connection) = http1::handshake(TokioIo::new(stream))
                .await
                .map_err(|why| HttpsFailure::Other(why.to_string()))?;
            tokio::spawn(async move {
                let _ = connection.await;
            });
            Ok(Sender::One(sender))
        }
    }
}

const KEPT: &str = "the kept connections are never left mid-update";

impl HttpsTransport for HyperTransport {
    fn exchange(&self, request: HttpsRequest) -> Result<HttpsResponse, HttpsFailure> {
        let bound = request.timeout;
        self.runtime.block_on(async {
            tokio::time::timeout(bound, self.carry(request))
                .await
                .unwrap_or(Err(HttpsFailure::Timeout))
        })
    }
}

/// How sending on a connection failed.
enum Failed {
    /// Before the request was written: the connection had closed.
    NotWritten,
    /// After: the server may have the request.
    After(HttpsFailure),
}

/// Send `request` on `sender` and read the whole response, returning the connection for the next
/// exchange.
async fn send(
    sender: Sender,
    uri: &Uri,
    authority: &str,
    request: &HttpsRequest,
) -> Result<(HttpsResponse, Sender), Failed> {
    let target = match &sender {
        // HTTP/1.1 sends the origin form with a `Host` header; HTTP/2 the absolute form, from
        // which `:authority` is taken.
        Sender::One(_) => uri
            .path_and_query()
            .map(|path| path.as_str().to_string())
            .unwrap_or_else(|| "/".to_string()),
        Sender::Two(_) => uri.to_string(),
    };
    let mut builder = Request::builder().method(Method::POST).uri(target);
    if matches!(sender, Sender::One(_)) {
        builder = builder.header(HOST, authority);
    }
    for (name, value) in &request.headers {
        builder = builder.header(name.as_str(), value.as_str());
    }
    let outgoing = builder
        .body(Full::new(Bytes::from(request.body.clone())))
        .map_err(|why| Failed::After(HttpsFailure::Other(why.to_string())))?;
    let (answered, sender) = match sender {
        Sender::One(mut one) => {
            if one.ready().await.is_err() {
                return Err(Failed::NotWritten);
            }
            (one.send_request(outgoing).await, Sender::One(one))
        }
        Sender::Two(mut two) => {
            if two.ready().await.is_err() {
                return Err(Failed::NotWritten);
            }
            (two.send_request(outgoing).await, Sender::Two(two))
        }
    };
    let response = answered.map_err(|why| {
        if why.is_canceled() {
            Failed::NotWritten
        } else {
            Failed::After(HttpsFailure::Other(why.to_string()))
        }
    })?;
    let status = response.status().as_u16();
    let headers = response
        .headers()
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_string(),
                String::from_utf8_lossy(value.as_bytes()).into_owned(),
            )
        })
        .collect();
    let body = response
        .into_body()
        .collect()
        .await
        .map_err(|why| Failed::After(HttpsFailure::Other(why.to_string())))?
        .to_bytes()
        .to_vec();
    Ok((
        HttpsResponse {
            status,
            headers,
            body,
        },
        sender,
    ))
}

/// Open a `CONNECT` tunnel to `target` (`host:port`) on `tcp`, a connection to the proxy, with
/// `credentials` as `Proxy-Authorization`. Anything but a `2xx` answer is a refused tunnel.
async fn tunnel(
    tcp: &mut TcpStream,
    target: &str,
    credentials: Option<&str>,
) -> Result<(), HttpsFailure> {
    let mut head = alloc::format!("CONNECT {target} HTTP/1.1\r\nHost: {target}\r\n");
    if let Some(credentials) = credentials {
        head.push_str(&alloc::format!("Proxy-Authorization: {credentials}\r\n"));
    }
    head.push_str("\r\n");
    tcp.write_all(head.as_bytes())
        .await
        .map_err(|_| HttpsFailure::Proxy)?;
    // The answer's head, read a byte at a time so that nothing after it (the server's first TLS
    // bytes, were the proxy to forward any) is read here.
    const MOST: usize = 8192;
    let mut answer = Vec::new();
    while !answer.ends_with(b"\r\n\r\n") {
        if answer.len() >= MOST {
            return Err(HttpsFailure::Proxy);
        }
        let byte = tcp.read_u8().await.map_err(|_| HttpsFailure::Proxy)?;
        answer.push(byte);
    }
    let status_line = answer.split(|b| *b == b'\r').next().unwrap_or_default();
    let mut words = status_line.split(|b| *b == b' ');
    let version = words.next().unwrap_or_default();
    let code = words.next().unwrap_or_default();
    if version.starts_with(b"HTTP/1.") && code.len() == 3 && code.first() == Some(&b'2') {
        Ok(())
    } else {
        Err(HttpsFailure::Proxy)
    }
}

/// The client's TLS: the platform's verifier with `extra_authorities` beside it, the identity of
/// an `mtls` client, and HTTP/2 offered before HTTP/1.1.
fn client_tls(settings: &AdapterSettings) -> Result<Arc<ClientConfig>, rustls::Error> {
    // §mesh-22.6: the adapter believes the authorities the operating system believes, so a
    // network that inspects TLS is honoured exactly where the user's machine trusts it.
    let provider = crate::tls::provider();
    let extra: Vec<CertificateDer<'static>> = settings
        .extra_authorities
        .iter()
        .map(|der| CertificateDer::from(der.clone()))
        .collect();
    #[cfg(not(target_os = "android"))]
    let verifier =
        rustls_platform_verifier::Verifier::new_with_extra_roots(extra, Arc::clone(&provider))?;
    #[cfg(target_os = "android")]
    let verifier = {
        if !extra.is_empty() {
            return Err(rustls::Error::General(
                "an additional authority cannot be believed on Android".to_string(),
            ));
        }
        rustls_platform_verifier::Verifier::new(Arc::clone(&provider))?
    };
    let builder = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier));
    let mut config = match &settings.client_identity {
        None => builder.with_no_client_auth(),
        Some((chain, key)) => {
            let chain = chain
                .iter()
                .map(|der| CertificateDer::from(der.clone()))
                .collect();
            let key = PrivateKeyDer::try_from(key.clone())
                .map_err(|why| rustls::Error::General(why.to_string()))?;
            builder.with_client_auth_cert(chain, key)?
        }
    };
    config.alpn_protocols = alloc::vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(Arc::new(config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::net::TcpListener;

    /// How long a test waits for what it expects.
    const PATIENCE: Duration = Duration::from_secs(5);

    /// A server on a loopback port that answers one HTTP/1.1 request with `answer`, and the head
    /// of the request it read.
    fn one_answer(answer: &'static [u8]) -> (String, std::sync::mpsc::Receiver<String>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            use std::io::{Read, Write};
            let (mut stream, _) = listener.accept().unwrap();
            let mut seen = Vec::new();
            let mut byte = [0u8; 1];
            while !seen.ends_with(b"\r\n\r\n") && stream.read(&mut byte).unwrap_or(0) == 1 {
                seen.push(byte[0]);
            }
            let _ = tx.send(String::from_utf8_lossy(&seen).into_owned());
            let _ = stream.write_all(answer);
        });
        (alloc::format!("http://{address}"), rx)
    }

    fn request(url: String, timeout: Duration) -> HttpsRequest {
        HttpsRequest {
            url,
            headers: alloc::vec![("Sce-Mesh-Ack".to_string(), "3".to_string())],
            body: Vec::new(),
            timeout,
        }
    }

    fn adapter() -> HyperTransport {
        HyperTransport::new(AdapterSettings {
            proxy: ProxyPolicy::Direct,
            ..AdapterSettings::default()
        })
        .unwrap()
    }

    #[test]
    fn a_plain_exchange_carries_the_headers_and_returns_the_response() {
        let (base, seen) = one_answer(
            b"HTTP/1.1 202 Accepted\r\nSce-Mesh-First: 9\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
        );
        let response = adapter()
            .exchange(request(alloc::format!("{base}/sce-mesh/1/phone"), PATIENCE))
            .unwrap();
        assert_eq!(response.status, 202);
        assert_eq!(response.header("sce-mesh-first"), Some("9"));
        assert_eq!(response.body, b"ok");
        let head = seen.recv_timeout(PATIENCE).unwrap();
        assert!(
            head.starts_with("POST /sce-mesh/1/phone HTTP/1.1\r\n"),
            "{head}"
        );
        assert!(
            head.to_ascii_lowercase().contains("sce-mesh-ack: 3\r\n"),
            "{head}"
        );
    }

    #[test]
    fn a_port_nobody_listens_on_is_a_connect_failure() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        let refused = adapter().exchange(request(alloc::format!("http://{address}/"), PATIENCE));
        assert_eq!(refused, Err(HttpsFailure::Connect));
    }

    #[test]
    fn an_exchange_past_its_bound_is_a_timeout() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        // Accepts and never answers.
        std::thread::spawn(move || {
            let held = listener.accept();
            std::thread::sleep(Duration::from_secs(3));
            drop(held);
        });
        let began = std::time::Instant::now();
        let late = adapter().exchange(request(
            alloc::format!("http://{address}/"),
            Duration::from_millis(200),
        ));
        assert_eq!(late, Err(HttpsFailure::Timeout));
        assert!(began.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn a_tunnel_is_open_only_on_a_2xx_answer() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            for (answer, open) in [
                (&b"HTTP/1.1 200 Connection established\r\n\r\n"[..], true),
                (&b"HTTP/1.0 200 OK\r\nVia: proxy\r\n\r\n"[..], true),
                (
                    &b"HTTP/1.1 407 Proxy Authentication Required\r\n\r\n"[..],
                    false,
                ),
                (&b"SSH-2.0-OpenSSH\r\n\r\n"[..], false),
            ] {
                let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
                let address = listener.local_addr().unwrap();
                let proxy = tokio::spawn(async move {
                    let (mut stream, _) = listener.accept().await.unwrap();
                    let mut seen = Vec::new();
                    while !seen.ends_with(b"\r\n\r\n") {
                        seen.push(stream.read_u8().await.unwrap());
                    }
                    stream.write_all(answer).await.unwrap();
                    String::from_utf8(seen).unwrap()
                });
                let mut tcp = TcpStream::connect(address).await.unwrap();
                let opened = tunnel(&mut tcp, "cal.example:443", Some("Basic dTpw")).await;
                assert_eq!(
                    opened.is_ok(),
                    open,
                    "{:?}",
                    String::from_utf8_lossy(answer)
                );
                let head = proxy.await.unwrap();
                assert_eq!(
                    head,
                    "CONNECT cal.example:443 HTTP/1.1\r\nHost: cal.example:443\r\n\
                     Proxy-Authorization: Basic dTpw\r\n\r\n"
                );
            }
        });
    }

    #[test]
    fn an_authority_that_is_not_a_certificate_is_refused_at_construction() {
        let refused = HyperTransport::new(AdapterSettings {
            extra_authorities: alloc::vec![b"not a certificate".to_vec()],
            ..AdapterSettings::default()
        });
        assert!(matches!(refused, Err(AdapterError::Tls(_))));
    }
}
