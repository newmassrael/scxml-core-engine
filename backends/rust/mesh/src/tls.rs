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
//! in from `webpki-roots`), or only the authorities a deployment names ([`Roots::Only`]), for a
//! server under a private one. The operating system's own store is not read: on Android reading it
//! needs the application's JVM and context handed to the library and a Kotlin component added to
//! the application's build, and a dial made before the host handed them over panics inside the
//! library.

use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use std::sync::OnceLock;

use rustls::crypto::CryptoProvider;
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
            let mut store = RootCertStore::empty();
            store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
            let made = client_believing(store)?;
            Ok(PUBLIC.get_or_init(|| made).clone())
        }
        Roots::Only(authorities) => {
            let mut store = RootCertStore::empty();
            for authority in authorities {
                store.add(CertificateDer::from(authority.as_slice()))?;
            }
            client_believing(store)
        }
    }
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
