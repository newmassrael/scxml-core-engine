// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The network transport of the standard host features' HTTP operation
//! (docs/SCE_HOST_FEATURES.md, section 5) for a server: one exchange over
//! HTTP/1.1, plain or over rustls with the operating system's certificate
//! verifier, judged first by a [`DestinationPolicy`] (docs/adr/0109, D4).
//!
//! * **Where it connects.** The policy admits the target's scheme, host and
//!   port, resolves the name once, and answers the addresses it admitted; the
//!   transport connects to one of those and to nothing else, so a name that
//!   rebinds between the check and the connect is never resolved again. The
//!   `Host` header and the TLS server name are the URL's host, so the server
//!   and its certificate are those of the name. Each request is judged anew.
//! * **What it follows.** Nothing: a `3xx` is answered as it came, with its
//!   `location`, and no proxy is read from the environment or used.
//! * **How long it waits.** A connection, a TLS handshake, the response's head
//!   and the whole exchange each have a bound ([`Timeouts`]); one that passes
//!   fails `network`.
//! * **How much it reads.** A body past [`NetworkSettings::max_response_bytes`]
//!   or a head past [`NetworkSettings::max_header_bytes`] fails
//!   `response_too_large`, and nothing of it is kept.
//! * **What it writes.** The request's own header lines, and `host` and
//!   `content-length`, which it writes itself. A request that names one of the
//!   fields that frame or route the message ([`TRANSPORT_FIELDS`]) is not sent
//!   and fails `network`.
//!
//! The response's fields are given sorted by name: hyper keeps them in a map.
//! The transport runs its own tokio runtime and blocks the calling thread on
//! it, so it is called from a thread that is not one of a tokio runtime's.

use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use http_body_util::{BodyExt, Full, LengthLimitError, Limited};
use hyper::client::conn::http1;
use hyper::header::HOST;
use hyper::{Method, Request, Uri};
use hyper_util::rt::TokioIo;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::ClientConfig;
use sce_rust_host_features::destination::{Destination, DestinationPolicy, Scheme};
use sce_rust_host_features::{
    HttpRequest, HttpResponse, Transport, TransportFailure, HTTP_BODY_MAX_BYTES,
};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;

/// The fields that frame or route a message, which the transport writes or
/// leaves out itself and a request may not name (compared in lower case).
pub const TRANSPORT_FIELDS: [&str; 7] = [
    "host",
    "content-length",
    "transfer-encoding",
    "connection",
    "upgrade",
    "expect",
    "te",
];

/// The smallest head buffer hyper reads a response's head in.
const HEAD_BUFFER_FLOOR: usize = 8192;

/// A request as hyper sends it.
type Outgoing = Request<Full<Bytes>>;

/// How long each part of an exchange may take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timeouts {
    /// Opening a TCP connection to one address.
    pub connect: Duration,
    /// The TLS handshake of an `https` exchange.
    pub tls: Duration,
    /// From the request being sent to the response's head being read.
    pub header: Duration,
    /// The whole exchange from its first connection to the end of the body.
    /// The policy resolves a name before it, on the resolver's own bound.
    pub total: Duration,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self {
            connect: Duration::from_secs(10),
            tls: Duration::from_secs(10),
            header: Duration::from_secs(30),
            total: Duration::from_secs(60),
        }
    }
}

/// What a transport is built with besides its policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkSettings {
    pub timeouts: Timeouts,
    /// The most bytes of a response's body read; the default is the bound of
    /// the result record's `body`.
    pub max_response_bytes: usize,
    /// The most bytes of a response's head read. hyper reads a head in a
    /// buffer of at least 8192 bytes, so a smaller bound reads as 8192.
    pub max_header_bytes: usize,
    /// Authorities (each a DER certificate) believed beside those the
    /// operating system believes: a deployment's private authority. Refused on
    /// Android, where the platform's store is the only one the verifier reads.
    pub extra_authorities: Vec<Vec<u8>>,
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            timeouts: Timeouts::default(),
            max_response_bytes: HTTP_BODY_MAX_BYTES,
            max_header_bytes: 65536,
            extra_authorities: Vec::new(),
        }
    }
}

/// Why a transport could not be built.
#[derive(Debug)]
pub enum NetworkTransportError {
    /// The TLS configuration was refused: an authority that does not parse,
    /// or a verifier the platform could not give.
    Tls(rustls::Error),
    /// The transport's runtime could not be started.
    Runtime(std::io::Error),
}

/// The network transport of section 5, built with the policy every exchange
/// is judged by; there is no transport without one.
pub struct NetworkTransport {
    policy: DestinationPolicy,
    settings: NetworkSettings,
    tls: Arc<ClientConfig>,
    runtime: tokio::runtime::Runtime,
}

impl NetworkTransport {
    /// A transport that reaches what `policy` admits, on `settings`.
    pub fn new(
        policy: DestinationPolicy,
        settings: NetworkSettings,
    ) -> Result<Self, NetworkTransportError> {
        let tls = client_tls(&settings.extra_authorities).map_err(NetworkTransportError::Tls)?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(NetworkTransportError::Runtime)?;
        Ok(Self {
            policy,
            settings,
            tls,
            runtime,
        })
    }

    /// The destination the policy admits for `request`, and the request as it
    /// is sent there; nothing is opened yet.
    fn judge(&self, request: &HttpRequest) -> Result<(Destination, Outgoing), TransportFailure> {
        // The operation accepted the URL as an absolute http or https one with
        // a host; one hyper cannot read (a character outside ASCII) is not sent.
        let uri: Uri = request.url.parse().map_err(|_| TransportFailure::Network)?;
        if request
            .headers
            .iter()
            .any(|(name, _)| TRANSPORT_FIELDS.contains(&name.to_ascii_lowercase().as_str()))
        {
            return Err(TransportFailure::Network);
        }
        let scheme = uri.scheme_str().ok_or(TransportFailure::Network)?;
        let host = uri.host().ok_or(TransportFailure::Network)?;
        let port = match uri.port_u16() {
            Some(port) => port,
            None => Scheme::parse(scheme)
                .ok_or(TransportFailure::DestinationRefused)?
                .default_port(),
        };
        let destination = self
            .policy
            .admit(scheme, host, port)
            .map_err(|why| why.failure())?;
        // `Host` names the URL's host, with the port only when the URL wrote one.
        let host_field = match uri.port() {
            Some(written) => format!("{host}:{}", written.as_str()),
            None => host.to_string(),
        };
        let outgoing = outgoing(request, &uri, &host_field)?;
        Ok((destination, outgoing))
    }

    /// Carry `outgoing` to `destination`: connect, shake hands, send, read.
    async fn carry(
        &self,
        destination: Destination,
        outgoing: Outgoing,
    ) -> Result<HttpResponse, TransportFailure> {
        let tcp = self.connect(&destination).await?;
        match destination.scheme {
            Scheme::Http => self.exchange_on(tcp, outgoing).await,
            Scheme::Https => {
                let name = ServerName::try_from(destination.host.clone())
                    .map_err(|_| TransportFailure::Network)?;
                let connector = tokio_rustls::TlsConnector::from(Arc::clone(&self.tls));
                let stream =
                    tokio::time::timeout(self.settings.timeouts.tls, connector.connect(name, tcp))
                        .await
                        .map_err(|_| TransportFailure::Network)?
                        .map_err(|_| TransportFailure::Network)?;
                self.exchange_on(stream, outgoing).await
            }
        }
    }

    /// A connection to the first of the admitted addresses that answers, and
    /// to no other address.
    async fn connect(&self, destination: &Destination) -> Result<TcpStream, TransportFailure> {
        for address in &destination.addresses {
            let attempt =
                tokio::time::timeout(self.settings.timeouts.connect, TcpStream::connect(*address))
                    .await;
            if let Ok(Ok(tcp)) = attempt {
                let _ = tcp.set_nodelay(true);
                return Ok(tcp);
            }
        }
        Err(TransportFailure::Network)
    }

    /// Send `outgoing` on `io` and read the whole response, within the bounds.
    async fn exchange_on<S>(
        &self,
        io: S,
        outgoing: Outgoing,
    ) -> Result<HttpResponse, TransportFailure>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let (mut sender, connection) = http1::Builder::new()
            .max_buf_size(self.settings.max_header_bytes.max(HEAD_BUFFER_FLOOR))
            .handshake(TokioIo::new(io))
            .await
            .map_err(|_| TransportFailure::Network)?;
        // The connection is driven beside the exchange and dropped with it.
        let _driver = Driver(tokio::spawn(async move {
            let _ = connection.await;
        }));
        sender
            .ready()
            .await
            .map_err(|_| TransportFailure::Network)?;
        let response =
            tokio::time::timeout(self.settings.timeouts.header, sender.send_request(outgoing))
                .await
                .map_err(|_| TransportFailure::Network)?
                .map_err(|why| {
                    if why.is_parse_too_large() {
                        TransportFailure::ResponseTooLarge
                    } else {
                        TransportFailure::Network
                    }
                })?;
        let status = response.status().as_u16();
        let mut headers: Vec<(String, String)> = response
            .headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_string(),
                    String::from_utf8_lossy(value.as_bytes()).into_owned(),
                )
            })
            .collect();
        // Stable, so the values of one name keep the order received.
        headers.sort_by(|a, b| a.0.cmp(&b.0));
        let body = Limited::new(response.into_body(), self.settings.max_response_bytes)
            .collect()
            .await
            .map_err(|why| {
                if why.downcast_ref::<LengthLimitError>().is_some() {
                    TransportFailure::ResponseTooLarge
                } else {
                    TransportFailure::Network
                }
            })?
            .to_bytes()
            .to_vec();
        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
}

/// The task that drives a connection, ended when the exchange that opened it
/// ends, whether it completed, failed or passed its bound.
struct Driver(tokio::task::JoinHandle<()>);

impl Drop for Driver {
    fn drop(&mut self) {
        self.0.abort();
    }
}

impl Transport for NetworkTransport {
    fn exchange(
        &mut self,
        request: &HttpRequest,
    ) -> Result<Option<HttpResponse>, TransportFailure> {
        // The policy resolves before the exchange's bound starts: the system
        // resolver keeps its own, and a call into it is not interrupted.
        let (destination, outgoing) = self.judge(request)?;
        let total = self.settings.timeouts.total;
        self.runtime
            .block_on(async {
                tokio::time::timeout(total, self.carry(destination, outgoing))
                    .await
                    .unwrap_or(Err(TransportFailure::Network))
            })
            .map(Some)
    }
}

/// The request as hyper sends it: the origin form of the target, `Host`, the
/// request's own fields, and its body.
fn outgoing(
    request: &HttpRequest,
    uri: &Uri,
    host_field: &str,
) -> Result<Outgoing, TransportFailure> {
    let method =
        Method::from_bytes(request.method.as_bytes()).map_err(|_| TransportFailure::Network)?;
    let target = uri
        .path_and_query()
        .map(|path| path.as_str())
        .filter(|path| !path.is_empty())
        .unwrap_or("/");
    let mut builder = Request::builder()
        .method(method)
        .uri(target)
        .header(HOST, host_field);
    for (name, value) in &request.headers {
        builder = builder.header(name.as_str(), value.as_str());
    }
    builder
        .body(Full::new(Bytes::from(request.body.clone())))
        .map_err(|_| TransportFailure::Network)
}

/// The client's TLS: the platform's verifier with `extra` beside it, offering
/// HTTP/1.1 only.
fn client_tls(extra: &[Vec<u8>]) -> Result<Arc<ClientConfig>, rustls::Error> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let extra: Vec<CertificateDer<'static>> = extra
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
    let mut config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_no_client_auth();
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(Arc::new(config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_authority_that_is_not_a_certificate_is_refused_at_construction() {
        let refused = NetworkTransport::new(
            DestinationPolicy::public_internet(),
            NetworkSettings {
                extra_authorities: vec![b"not a certificate".to_vec()],
                ..NetworkSettings::default()
            },
        );
        assert!(matches!(refused, Err(NetworkTransportError::Tls(_))));
    }

    #[test]
    fn the_target_is_sent_in_origin_form_with_the_host_of_the_url() {
        let request = HttpRequest {
            method: "POST".to_string(),
            url: "http://user@api.example.com:8080/items?page=2#top".to_string(),
            headers: vec![("X-Trace".to_string(), "7".to_string())],
            body: "payload".to_string(),
        };
        let uri: Uri = request.url.parse().expect("a URL");
        let sent = outgoing(&request, &uri, "api.example.com:8080").expect("a request");
        assert_eq!(*sent.method(), Method::POST);
        assert_eq!(sent.uri().to_string(), "/items?page=2");
        assert_eq!(sent.headers()[HOST], "api.example.com:8080");
        assert_eq!(sent.headers()["x-trace"], "7");
        let bare: Uri = "https://api.example.com".parse().expect("a URL");
        let sent = outgoing(&request, &bare, "api.example.com").expect("a request");
        assert_eq!(sent.uri().to_string(), "/");
    }
}
