// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The TLS the WebSocket bindings run on: rustls, with `ring`'s cryptography.
//!
//! Every TLS connection the bindings make or accept is configured here, the Mesh's ([`crate::wss`])
//! and the relay's (`sce_rust_relay::wss` and `serve`), so that which cryptography runs and which
//! authorities a client believes are decided once. The cryptography is passed to rustls explicitly
//! ([`provider`]) rather than read from the process: rustls picks a process-wide default only when
//! exactly one of its providers is compiled in, and a host that links another library asking for
//! the other one would otherwise turn every `wss://` dial into a panic. `ring` was chosen over
//! `aws-lc-rs` (owner decision 2026-11-02) because it is the simpler of the two to cross-build for
//! Android and iOS.
//!
//! A client believes the public certificate authorities ([`Roots::Public`]: Mozilla's set, compiled
//! in from `webpki-roots`), only the authorities a deployment names ([`Roots::Only`]), for a server
//! under a private one, or both ([`Roots::PublicAnd`]), for a deployment that moves its servers
//! from a private authority to a public one, or keeps some under each, while the applications it
//! shipped keep reaching every one of them. A deployment hands its authority out as a file
//! ([`authorities`] reads one in PEM or DER). The operating system's own store is not read: on
//! Android reading it needs the application's JVM and context handed to the library and a Kotlin
//! component added to the application's build, and a dial made before the host handed them over
//! panics inside the library.

use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use std::sync::OnceLock;

use rustls::crypto::CryptoProvider;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::RootCertStore;

pub use rustls::{ClientConfig, Error, ServerConfig};

/// The authorities a client believes a server's certificate from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Roots {
    /// The public certificate authorities: Mozilla's set, as `webpki-roots` compiled it in.
    #[default]
    Public,
    /// These authorities' certificates alone (each DER), for a server under a private authority.
    Only(Vec<Vec<u8>>),
    /// The public certificate authorities and these (each DER): for a deployment whose servers are
    /// under a private authority and a public one, or move from the one to the other, so that an
    /// application it shipped reaches every one of them.
    PublicAnd(Vec<Vec<u8>>),
}

/// The text a PEM file of certificates begins with, after any blank space.
const PEM_BEGIN: &[u8] = b"-----BEGIN";

/// The certificates of the authorities `bytes` holds, each DER: one or more in PEM
/// (`-----BEGIN CERTIFICATE-----`), as an authority is most often handed out and as a mobile
/// application most readily ships it (an Android raw resource, a file of an iOS bundle), or one in
/// DER. Sections of PEM that are not certificates are passed over; PEM that holds no certificate,
/// or that does not decode, is refused, and so is nothing at all. Bytes that are not PEM are taken as
/// one DER certificate, which [`client_config`] refuses when it is not one.
pub fn authorities(bytes: &[u8]) -> Result<Vec<Vec<u8>>, Error> {
    let start = bytes
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    let text = &bytes[start..];
    if text.is_empty() {
        return Err(Error::General("no certificate was given".to_string()));
    }
    if !text.starts_with(PEM_BEGIN) {
        return Ok(alloc::vec![bytes.to_vec()]);
    }
    let certificates = CertificateDer::pem_slice_iter(text)
        .map(|certificate| {
            certificate
                .map(|der| der.as_ref().to_vec())
                .map_err(|why| Error::General(alloc::format!("the PEM does not decode: {why}")))
        })
        .collect::<Result<Vec<Vec<u8>>, Error>>()?;
    if certificates.is_empty() {
        return Err(Error::General("the PEM holds no certificate".to_string()));
    }
    Ok(certificates)
}

/// The cryptography every TLS connection of the bindings runs on: `ring`'s.
pub fn provider() -> Arc<CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

/// A client's configuration believing `roots`. The public one is made once and shared by every
/// connection, so they share its sessions to resume too. An authority that is not a certificate is
/// refused.
pub fn client_config(roots: &Roots) -> Result<Arc<ClientConfig>, Error> {
    match roots {
        Roots::Public => {
            static PUBLIC: OnceLock<Arc<ClientConfig>> = OnceLock::new();
            if let Some(config) = PUBLIC.get() {
                return Ok(config.clone());
            }
            let made = client_believing(public_store())?;
            Ok(PUBLIC.get_or_init(|| made).clone())
        }
        Roots::Only(authorities) => {
            client_believing(with_authorities(RootCertStore::empty(), authorities)?)
        }
        Roots::PublicAnd(authorities) => {
            client_believing(with_authorities(public_store(), authorities)?)
        }
    }
}

/// The public certificate authorities: Mozilla's set, as `webpki-roots` compiled it in.
fn public_store() -> RootCertStore {
    let mut store = RootCertStore::empty();
    store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    store
}

/// `store` with `authorities` (each DER) added; one that is not a certificate is refused.
fn with_authorities(
    mut store: RootCertStore,
    authorities: &[Vec<u8>],
) -> Result<RootCertStore, Error> {
    for authority in authorities {
        store.add(CertificateDer::from(authority.as_slice()))?;
    }
    Ok(store)
}

fn client_believing(store: RootCertStore) -> Result<Arc<ClientConfig>, Error> {
    Ok(Arc::new(
        ClientConfig::builder_with_provider(provider())
            .with_safe_default_protocol_versions()?
            .with_root_certificates(store)
            .with_no_client_auth(),
    ))
}

/// A server's configuration presenting `chain` (each certificate DER, the server's own first) with
/// the private key `key` (DER: PKCS#8, PKCS#1 or SEC1). A key that is none of them, or that is not
/// the first certificate's, is refused.
pub fn server_config(chain: Vec<Vec<u8>>, key: Vec<u8>) -> Result<Arc<ServerConfig>, Error> {
    let key = PrivateKeyDer::try_from(key).map_err(|why| Error::General(why.to_string()))?;
    let chain = chain.into_iter().map(CertificateDer::from).collect();
    let config = ServerConfig::builder_with_provider(provider())
        .with_safe_default_protocol_versions()?
        .with_no_client_auth()
        .with_single_cert(chain, key)?;
    Ok(Arc::new(config))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn certified() -> rcgen::CertifiedKey {
        rcgen::generate_simple_self_signed(alloc::vec!["127.0.0.1".to_string()]).unwrap()
    }

    #[test]
    fn the_public_roots_are_mozillas_and_made_once() {
        let first = client_config(&Roots::Public).unwrap();
        let again = client_config(&Roots::default()).unwrap();
        assert!(Arc::ptr_eq(&first, &again));
        assert!(!webpki_roots::TLS_SERVER_ROOTS.is_empty());
    }

    #[test]
    fn an_authority_that_is_not_a_certificate_is_refused() {
        assert!(client_config(&Roots::Only(alloc::vec![b"not a certificate".to_vec()])).is_err());
        let issued = certified();
        assert!(client_config(&Roots::Only(alloc::vec![issued.cert.der().to_vec()])).is_ok());
    }

    /// `der` as a PEM certificate: standard base64 in lines of 64 between the markers, as a file an
    /// authority is handed out as.
    fn pem_of(der: &[u8]) -> String {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut body = String::new();
        for chunk in der.chunks(3) {
            let n = chunk
                .iter()
                .enumerate()
                .fold(0u32, |n, (i, byte)| n | (u32::from(*byte) << (16 - 8 * i)));
            for i in 0..4 {
                if i <= chunk.len() {
                    body.push(char::from(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize]));
                } else {
                    body.push('=');
                }
            }
        }
        let mut pem = String::from("-----BEGIN CERTIFICATE-----\n");
        for line in body.as_bytes().chunks(64) {
            pem.push_str(core::str::from_utf8(line).unwrap());
            pem.push('\n');
        }
        pem.push_str("-----END CERTIFICATE-----\n");
        pem
    }

    #[test]
    fn a_deployments_authority_is_read_from_pem_or_der() {
        let issued = certified();
        let der = issued.cert.der().to_vec();
        assert_eq!(authorities(&der).unwrap(), alloc::vec![der.clone()]);
        assert_eq!(
            authorities(pem_of(&der).as_bytes()).unwrap(),
            alloc::vec![der.clone()]
        );
        // Two in one file, after blank space, with a section that is no certificate between them.
        let other = certified().cert.der().to_vec();
        let bundle = alloc::format!(
            "\n  {}-----BEGIN PRIVATE KEY-----\nAAAA\n-----END PRIVATE KEY-----\n{}",
            pem_of(&der),
            pem_of(&other)
        );
        assert_eq!(
            authorities(bundle.as_bytes()).unwrap(),
            alloc::vec![der.clone(), other]
        );
        assert!(authorities(b"").is_err(), "nothing");
        assert!(authorities(b" \n\t").is_err(), "blank space");
        assert!(
            authorities(b"-----BEGIN PRIVATE KEY-----\nAAAA\n-----END PRIVATE KEY-----\n").is_err(),
            "PEM with no certificate"
        );
        assert!(
            authorities(b"-----BEGIN CERTIFICATE-----\n!!!!\n-----END CERTIFICATE-----\n").is_err(),
            "PEM that does not decode"
        );
        // Bytes that are not PEM are one certificate in DER, which a configuration judges.
        let not_one = authorities(b"not a certificate").unwrap();
        assert!(client_config(&Roots::Only(not_one)).is_err());
    }

    #[test]
    fn the_public_roots_and_a_deployments_authority_are_believed_together() {
        let issued = certified();
        let der = issued.cert.der().to_vec();
        let both = with_authorities(public_store(), core::slice::from_ref(&der)).unwrap();
        assert_eq!(both.len(), public_store().len() + 1);
        let config = client_config(&Roots::PublicAnd(alloc::vec![der])).unwrap();
        assert!(!Arc::ptr_eq(
            &config,
            &client_config(&Roots::Public).unwrap()
        ));
        assert!(client_config(&Roots::PublicAnd(
            alloc::vec![b"not a certificate".to_vec()]
        ))
        .is_err());
    }

    #[test]
    fn a_server_presents_its_own_key_and_no_other() {
        let issued = certified();
        let chain = alloc::vec![issued.cert.der().to_vec()];
        assert!(server_config(chain.clone(), issued.key_pair.serialize_der()).is_ok());
        assert!(server_config(chain.clone(), certified().key_pair.serialize_der()).is_err());
        assert!(server_config(chain, b"not a key".to_vec()).is_err());
    }

    /// The process's default is not what a binding runs on: a host may install one of its own
    /// (here, `ring` narrowed to one cipher suite), and the bindings' connections still run on the
    /// provider this module names.
    #[test]
    fn a_configuration_runs_on_rings_cryptography_whatever_the_process_installed() {
        let mut narrowed = rustls::crypto::ring::default_provider();
        narrowed.cipher_suites.truncate(1);
        // Installed once per process: a second install is refused, and the first stays.
        let _ = narrowed.install_default();
        let ours = rustls::crypto::ring::default_provider().cipher_suites;
        assert!(ours.len() > 1);

        let client = client_config(&Roots::Only(Vec::new())).unwrap();
        assert_eq!(client.crypto_provider().cipher_suites, ours);
        let issued = certified();
        let server = server_config(
            alloc::vec![issued.cert.der().to_vec()],
            issued.key_pair.serialize_der(),
        )
        .unwrap();
        assert_eq!(server.crypto_provider().cipher_suites, ours);
    }
}
