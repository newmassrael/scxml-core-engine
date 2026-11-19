// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Where a transport of the HTTP operation may connect for a server
//! (docs/SCE_HOST_FEATURES.md, section 5, "Destination policy"; docs/adr/0109,
//! D4).
//!
//! A [`DestinationPolicy`] judges a target before anything is sent: whether its
//! scheme, host and port are ones the policy admits, then every address its
//! name resolves to, against the special-purpose ranges of RFC 6890 and the
//! IANA registries it set up ([`special_purpose`]). The policy resolves the
//! name itself and answers the addresses it admitted, and a transport connects
//! to those and to no other: a name that resolves to another address between a
//! check and a connect is never resolved a second time.
//!
//! The policy is data and a resolver; it opens no socket, so a transport on any
//! stack judges its targets with it.

use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs};
use std::sync::Arc;

use crate::TransportFailure;

/// A special-purpose range an address lies in: an address a server's outbound
/// request is not sent to unless the deployment names the range for the
/// destination ([`AllowedDestination::reaching`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpecialPurpose {
    /// `0.0.0.0/32`, `::/128`.
    Unspecified,
    /// `0.0.0.0/8`, "this host on this network".
    ThisNetwork,
    /// `127.0.0.0/8`, `::1/128`.
    Loopback,
    /// RFC 1918: `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`.
    Private,
    /// RFC 6598, the carrier-grade NAT space: `100.64.0.0/10`.
    Shared,
    /// `169.254.0.0/16`, `fe80::/10`.
    LinkLocal,
    /// RFC 4193: `fc00::/7`.
    UniqueLocal,
    /// The IETF protocol assignments: `192.0.0.0/24`, `2001::/23` (Teredo and
    /// the benchmarking and ORCHID blocks among them).
    ProtocolAssignments,
    /// `192.0.2.0/24`, `198.51.100.0/24`, `203.0.113.0/24`, `2001:db8::/32`,
    /// `3fff::/20`.
    Documentation,
    /// RFC 2544: `198.18.0.0/15`.
    Benchmarking,
    /// `224.0.0.0/4`, `ff00::/8`.
    Multicast,
    /// `255.255.255.255/32`.
    Broadcast,
    /// RFC 6666, discard-only: `100::/64`.
    Discard,
    /// RFC 8215, the local-use NAT64 prefix: `64:ff9b:1::/48`.
    LocalTranslation,
    /// Reserved, deprecated, or not allocated for global unicast:
    /// `240.0.0.0/4`, the 6to4 relay anycast `192.88.99.0/24`, and every IPv6
    /// address outside `2000::/3` that no other range names (site-local
    /// `fec0::/10` among them).
    Reserved,
}

/// The IPv4 special-purpose ranges, as network, prefix length and range; the
/// first that holds an address names it, so a range inside another comes first.
const V4_RANGES: [(u32, u32, SpecialPurpose); 17] = [
    (0x0000_0000, 32, SpecialPurpose::Unspecified),
    (0x0000_0000, 8, SpecialPurpose::ThisNetwork),
    (0x0a00_0000, 8, SpecialPurpose::Private),
    (0x6440_0000, 10, SpecialPurpose::Shared),
    (0x7f00_0000, 8, SpecialPurpose::Loopback),
    (0xa9fe_0000, 16, SpecialPurpose::LinkLocal),
    (0xac10_0000, 12, SpecialPurpose::Private),
    (0xc000_0000, 24, SpecialPurpose::ProtocolAssignments),
    (0xc000_0200, 24, SpecialPurpose::Documentation),
    (0xc058_6300, 24, SpecialPurpose::Reserved),
    (0xc0a8_0000, 16, SpecialPurpose::Private),
    (0xc612_0000, 15, SpecialPurpose::Benchmarking),
    (0xc633_6400, 24, SpecialPurpose::Documentation),
    (0xcb00_7100, 24, SpecialPurpose::Documentation),
    (0xe000_0000, 4, SpecialPurpose::Multicast),
    (0xffff_ffff, 32, SpecialPurpose::Broadcast),
    (0xf000_0000, 4, SpecialPurpose::Reserved),
];

/// The IPv6 special-purpose ranges that embed no IPv4 address, as
/// [`V4_RANGES`] lists them. An address outside `2000::/3` that none names is
/// [`SpecialPurpose::Reserved`].
const V6_RANGES: [(u128, u32, SpecialPurpose); 8] = [
    (
        0x0064_ff9b_0001_0000_0000_0000_0000_0000,
        48,
        SpecialPurpose::LocalTranslation,
    ),
    (
        0x0100_0000_0000_0000_0000_0000_0000_0000,
        64,
        SpecialPurpose::Discard,
    ),
    (
        0x2001_0db8_0000_0000_0000_0000_0000_0000,
        32,
        SpecialPurpose::Documentation,
    ),
    (
        0x2001_0000_0000_0000_0000_0000_0000_0000,
        23,
        SpecialPurpose::ProtocolAssignments,
    ),
    (
        0x3fff_0000_0000_0000_0000_0000_0000_0000,
        20,
        SpecialPurpose::Documentation,
    ),
    (
        0xfc00_0000_0000_0000_0000_0000_0000_0000,
        7,
        SpecialPurpose::UniqueLocal,
    ),
    (
        0xfe80_0000_0000_0000_0000_0000_0000_0000,
        10,
        SpecialPurpose::LinkLocal,
    ),
    (
        0xff00_0000_0000_0000_0000_0000_0000_0000,
        8,
        SpecialPurpose::Multicast,
    ),
];

/// The IPv6 forms that carry an IPv4 address in their last 32 bits, each
/// judged by the address it carries: IPv4-mapped (`::ffff:0:0/96`),
/// IPv4-translated (`::ffff:0:0:0/96`), the well-known NAT64 prefix
/// (`64:ff9b::/96`) and IPv4-compatible (`::/96`).
const V6_EMBEDDING_LOW: [u128; 4] = [
    0x0000_0000_0000_0000_0000_ffff_0000_0000,
    0x0000_0000_0000_0000_ffff_0000_0000_0000,
    0x0064_ff9b_0000_0000_0000_0000_0000_0000,
    0x0000_0000_0000_0000_0000_0000_0000_0000,
];

/// 6to4 (`2002::/16`), which carries an IPv4 address in the 32 bits after its
/// prefix.
const SIX_TO_FOUR: u128 = 0x2002_0000_0000_0000_0000_0000_0000_0000;

/// Global unicast, `2000::/3`.
const GLOBAL_UNICAST: u128 = 0x2000_0000_0000_0000_0000_0000_0000_0000;

/// Whether `bits` lies in the IPv4 network `network`/`prefix`.
fn within_v4(bits: u32, network: u32, prefix: u32) -> bool {
    let mask = u32::MAX.checked_shl(32 - prefix).unwrap_or(0);
    bits & mask == network & mask
}

/// Whether `bits` lies in the IPv6 network `network`/`prefix`.
fn within_v6(bits: u128, network: u128, prefix: u32) -> bool {
    let mask = u128::MAX.checked_shl(128 - prefix).unwrap_or(0);
    bits & mask == network & mask
}

/// The special-purpose range `address` lies in, or `None` for an address of
/// the public internet.
///
/// An IPv6 address that carries an IPv4 address (IPv4-mapped, IPv4-translated,
/// IPv4-compatible, 6to4, the well-known NAT64 prefix) is judged by the address
/// it carries, so `::ffff:127.0.0.1`, `64:ff9b::7f00:1` and `2002:7f00:1::` are
/// loopback, and `::ffff:8.8.8.8` is not special.
pub fn special_purpose(address: IpAddr) -> Option<SpecialPurpose> {
    match address {
        IpAddr::V4(v4) => {
            let bits = u32::from(v4);
            V4_RANGES
                .iter()
                .find(|(network, prefix, _)| within_v4(bits, *network, *prefix))
                .map(|(_, _, range)| *range)
        }
        IpAddr::V6(v6) => {
            let bits = u128::from(v6);
            if bits == 0 {
                return Some(SpecialPurpose::Unspecified);
            }
            if bits == 1 {
                return Some(SpecialPurpose::Loopback);
            }
            if V6_EMBEDDING_LOW
                .iter()
                .any(|prefix| within_v6(bits, *prefix, 96))
            {
                // The low 32 bits are the IPv4 address carried.
                return special_purpose(IpAddr::V4(Ipv4Addr::from(bits as u32)));
            }
            if within_v6(bits, SIX_TO_FOUR, 16) {
                return special_purpose(IpAddr::V4(Ipv4Addr::from((bits >> 80) as u32)));
            }
            V6_RANGES
                .iter()
                .find(|(network, prefix, _)| within_v6(bits, *network, *prefix))
                .map(|(_, _, range)| *range)
                .or_else(|| {
                    (!within_v6(bits, GLOBAL_UNICAST, 3)).then_some(SpecialPurpose::Reserved)
                })
        }
    }
}

/// A network: an address and the length of its prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpNetwork {
    address: IpAddr,
    prefix: u8,
}

impl IpNetwork {
    /// The network of `address` with a prefix of `prefix` bits, or `None` for
    /// a prefix longer than the address.
    pub fn new(address: IpAddr, prefix: u8) -> Option<Self> {
        let most = match address {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        };
        (prefix <= most).then_some(Self { address, prefix })
    }

    /// Whether `address` lies in the network. An address of the other family
    /// lies in none, an IPv4-mapped one included.
    pub fn contains(&self, address: IpAddr) -> bool {
        let prefix = u32::from(self.prefix);
        match (self.address, address) {
            (IpAddr::V4(network), IpAddr::V4(address)) => {
                within_v4(u32::from(address), u32::from(network), prefix)
            }
            (IpAddr::V6(network), IpAddr::V6(address)) => {
                within_v6(u128::from(address), u128::from(network), prefix)
            }
            _ => false,
        }
    }
}

/// The schemes the HTTP operation carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    Http,
    Https,
}

impl Scheme {
    /// The scheme a URL names, in lower case, or `None` for another.
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "http" => Some(Scheme::Http),
            "https" => Some(Scheme::Https),
            _ => None,
        }
    }

    /// The port a URL of the scheme that names none is reached on.
    pub fn default_port(self) -> u16 {
        match self {
            Scheme::Http => 80,
            Scheme::Https => 443,
        }
    }
}

/// A host as it is compared: without the brackets of an IPv6 literal and a
/// final dot, in ASCII lower case, and an address literal in its canonical text.
fn normalized(host: &str) -> String {
    let bare = host.strip_prefix('[').unwrap_or(host);
    let bare = bare.strip_suffix(']').unwrap_or(bare);
    if let Ok(literal) = bare.parse::<IpAddr>() {
        return literal.to_string();
    }
    bare.strip_suffix('.').unwrap_or(bare).to_ascii_lowercase()
}

/// The hosts an allow-list entry names.
#[derive(Debug, Clone, PartialEq, Eq)]
enum HostPattern {
    /// One name, or one address literal.
    Exact(String),
    /// Every name below this one (`*.example.com`), and not the name itself.
    Below(String),
}

impl HostPattern {
    fn matches(&self, host: &str) -> bool {
        match self {
            HostPattern::Exact(name) => name == host,
            HostPattern::Below(parent) => host
                .strip_suffix(parent.as_str())
                .and_then(|label| label.strip_suffix('.'))
                .is_some_and(|label| !label.is_empty()),
        }
    }
}

/// One destination an allow-list admits: a scheme, a host pattern and a port,
/// and the special-purpose networks it may reach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllowedDestination {
    scheme: Scheme,
    host: HostPattern,
    port: u16,
    reaches: Vec<IpNetwork>,
}

impl AllowedDestination {
    /// `scheme://host:port`. `host` is a name, `*.` before a name for every
    /// name below it (not the name itself), or an address literal; names are
    /// compared without case and without a final dot. The addresses it
    /// resolves to are still judged: one in a special-purpose range is refused
    /// unless [`Self::reaching`] names a network that holds it.
    pub fn new(scheme: Scheme, host: &str, port: u16) -> Self {
        let host = match host.strip_prefix("*.") {
            Some(parent) => HostPattern::Below(normalized(parent)),
            None => HostPattern::Exact(normalized(host)),
        };
        Self {
            scheme,
            host,
            port,
            reaches: Vec::new(),
        }
    }

    /// The destination may also reach `network`, special-purpose or not: an
    /// internal service the deployment names, and no other.
    pub fn reaching(mut self, network: IpNetwork) -> Self {
        self.reaches.push(network);
        self
    }

    fn matches(&self, scheme: Scheme, host: &str, port: u16) -> bool {
        self.scheme == scheme && self.port == port && self.host.matches(host)
    }
}

/// What resolves a name for a policy.
pub trait Resolve {
    /// The addresses `host` resolves to. An error, or no address, is a name
    /// that did not resolve.
    fn resolve(&self, host: &str) -> io::Result<Vec<IpAddr>>;
}

/// The operating system's resolver (`getaddrinfo` on Unix).
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemResolver;

impl Resolve for SystemResolver {
    fn resolve(&self, host: &str) -> io::Result<Vec<IpAddr>> {
        Ok((host, 0u16)
            .to_socket_addrs()?
            .map(|address| address.ip())
            .collect())
    }
}

/// The resolver a policy holds.
pub type SharedResolver = Arc<dyn Resolve + Send + Sync>;

/// Which targets a policy admits before their addresses are judged.
#[derive(Clone)]
enum Scope {
    AllowList(Vec<AllowedDestination>),
    PublicInternet,
}

/// Where a transport may connect: an allow-list, or any public address over
/// `https`, with every address a name resolves to judged.
#[derive(Clone)]
pub struct DestinationPolicy {
    scope: Scope,
    resolver: SharedResolver,
}

/// A target a policy admitted: the host as it was compared, which a transport
/// names in `Host` and verifies TLS against, its port, and the addresses it
/// may connect to, in the order resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Destination {
    pub scheme: Scheme,
    pub host: String,
    pub port: u16,
    pub addresses: Vec<SocketAddr>,
}

/// Why a policy did not admit a target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inadmissible {
    /// The scheme, host or port is on no entry of the allow-list, or the
    /// scheme is not `https` under [`DestinationPolicy::public_internet`]. The
    /// name was not resolved.
    NotAllowed,
    /// Every address the name resolved to is refused: each, with the range it
    /// lies in.
    AddressesRefused(Vec<(IpAddr, SpecialPurpose)>),
    /// The name resolved to no address.
    Unresolved,
}

impl Inadmissible {
    /// The failure a transport reports it as: `destination_refused`, or
    /// `network` for a name that did not resolve.
    pub fn failure(&self) -> TransportFailure {
        match self {
            Inadmissible::NotAllowed | Inadmissible::AddressesRefused(_) => {
                TransportFailure::DestinationRefused
            }
            Inadmissible::Unresolved => TransportFailure::Network,
        }
    }
}

impl DestinationPolicy {
    /// A policy that admits the destinations `entries` name, and no other,
    /// resolving names with the operating system's resolver.
    pub fn allow_list(entries: Vec<AllowedDestination>) -> Self {
        Self {
            scope: Scope::AllowList(entries),
            resolver: Arc::new(SystemResolver),
        }
    }

    /// A policy that admits any host on any port over `https`, at an address
    /// of the public internet only, resolving names with the operating system's
    /// resolver.
    pub fn public_internet() -> Self {
        Self {
            scope: Scope::PublicInternet,
            resolver: Arc::new(SystemResolver),
        }
    }

    /// The same policy, resolving names with `resolver`.
    pub fn resolving_with(mut self, resolver: SharedResolver) -> Self {
        self.resolver = resolver;
        self
    }

    /// Judge `scheme://host:port`, resolving `host` once when it is a name.
    ///
    /// A target the scope does not admit is refused before its name is
    /// resolved. An address is admitted when it lies in no special-purpose
    /// range, or in a network the matching entry reaches; the others are
    /// dropped, and a name none of whose addresses is admitted is refused.
    pub fn admit(&self, scheme: &str, host: &str, port: u16) -> Result<Destination, Inadmissible> {
        let scheme = Scheme::parse(scheme).ok_or(Inadmissible::NotAllowed)?;
        let host = normalized(host);
        let reaches: &[IpNetwork] = match &self.scope {
            Scope::PublicInternet if scheme == Scheme::Https => &[],
            Scope::PublicInternet => return Err(Inadmissible::NotAllowed),
            Scope::AllowList(entries) => entries
                .iter()
                .find(|entry| entry.matches(scheme, &host, port))
                .ok_or(Inadmissible::NotAllowed)?
                .reaches
                .as_slice(),
        };
        let resolved = match host.parse::<IpAddr>() {
            Ok(literal) => vec![literal],
            Err(_) => self
                .resolver
                .resolve(&host)
                .map_err(|_| Inadmissible::Unresolved)?,
        };
        if resolved.is_empty() {
            return Err(Inadmissible::Unresolved);
        }
        let mut admitted = Vec::new();
        let mut refused = Vec::new();
        for address in resolved {
            match special_purpose(address) {
                Some(range) if !reaches.iter().any(|network| network.contains(address)) => {
                    refused.push((address, range));
                }
                _ => admitted.push(SocketAddr::new(address, port)),
            }
        }
        if admitted.is_empty() {
            return Err(Inadmissible::AddressesRefused(refused));
        }
        Ok(Destination {
            scheme,
            host,
            port,
            addresses: admitted,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn address(text: &str) -> IpAddr {
        text.parse().expect("an address literal")
    }

    /// RFC 6890 and the registries it set up, range by range, with the
    /// addresses at and just past the edge of each, and every IPv6 form that
    /// carries an IPv4 address judged by the address it carries.
    #[test]
    fn every_special_purpose_range_is_classified() {
        use SpecialPurpose::*;
        let table: &[(&str, Option<SpecialPurpose>)] = &[
            ("0.0.0.0", Some(Unspecified)),
            ("0.1.2.3", Some(ThisNetwork)),
            ("0.255.255.255", Some(ThisNetwork)),
            ("1.0.0.0", None),
            ("10.0.0.1", Some(Private)),
            ("10.255.255.255", Some(Private)),
            ("11.0.0.0", None),
            ("100.63.255.255", None),
            ("100.64.0.0", Some(Shared)),
            ("100.127.255.255", Some(Shared)),
            ("100.128.0.0", None),
            ("126.255.255.255", None),
            ("127.0.0.1", Some(Loopback)),
            ("127.255.255.254", Some(Loopback)),
            ("128.0.0.0", None),
            ("169.254.169.254", Some(LinkLocal)),
            ("169.255.0.0", None),
            ("172.15.255.255", None),
            ("172.16.0.1", Some(Private)),
            ("172.31.255.255", Some(Private)),
            ("172.32.0.0", None),
            ("192.0.0.8", Some(ProtocolAssignments)),
            ("192.0.1.0", None),
            ("192.0.2.1", Some(Documentation)),
            ("192.88.99.1", Some(Reserved)),
            ("192.167.255.255", None),
            ("192.168.1.1", Some(Private)),
            ("192.169.0.0", None),
            ("198.17.255.255", None),
            ("198.18.0.1", Some(Benchmarking)),
            ("198.19.255.255", Some(Benchmarking)),
            ("198.20.0.0", None),
            ("198.51.100.7", Some(Documentation)),
            ("203.0.113.9", Some(Documentation)),
            ("223.255.255.255", None),
            ("224.0.0.1", Some(Multicast)),
            ("239.255.255.255", Some(Multicast)),
            ("240.0.0.1", Some(Reserved)),
            ("255.255.255.254", Some(Reserved)),
            ("255.255.255.255", Some(Broadcast)),
            ("8.8.8.8", None),
            ("93.184.215.14", None),
            ("::", Some(Unspecified)),
            ("::1", Some(Loopback)),
            ("::2", Some(ThisNetwork)),
            ("::ffff:127.0.0.1", Some(Loopback)),
            ("::ffff:10.1.2.3", Some(Private)),
            ("::ffff:169.254.169.254", Some(LinkLocal)),
            ("::ffff:8.8.8.8", None),
            ("::ffff:0:127.0.0.1", Some(Loopback)),
            ("::ffff:0:8.8.8.8", None),
            ("::127.0.0.1", Some(Loopback)),
            ("::8.8.8.8", None),
            ("64:ff9b::7f00:1", Some(Loopback)),
            ("64:ff9b::a9fe:a9fe", Some(LinkLocal)),
            ("64:ff9b::c0a8:101", Some(Private)),
            ("64:ff9b::808:808", None),
            ("64:ff9b:1::1", Some(LocalTranslation)),
            ("64:ff9b:1:ffff::1", Some(LocalTranslation)),
            ("2002:7f00:1::1", Some(Loopback)),
            ("2002:c0a8:101::", Some(Private)),
            ("2002:a00:1::", Some(Private)),
            ("2002:808:808::1", None),
            ("100::1", Some(Discard)),
            ("100::ffff:ffff:ffff:ffff", Some(Discard)),
            ("100:0:0:1::", Some(Reserved)),
            ("2001::1", Some(ProtocolAssignments)),
            ("2001:1ff:ffff::1", Some(ProtocolAssignments)),
            ("2001:200::1", None),
            ("2001:db8::1", Some(Documentation)),
            ("2001:db9::1", None),
            ("3fff::1", Some(Documentation)),
            ("3fff:fff::1", Some(Documentation)),
            ("3fff:1000::1", None),
            ("fc00::1", Some(UniqueLocal)),
            ("fdff:ffff::1", Some(UniqueLocal)),
            ("fe80::1", Some(LinkLocal)),
            ("febf::1", Some(LinkLocal)),
            ("fec0::1", Some(Reserved)),
            ("ff02::1", Some(Multicast)),
            ("5f00::1", Some(Reserved)),
            ("4000::1", Some(Reserved)),
            ("1fff:ffff::1", Some(Reserved)),
            ("2606:4700:4700::1111", None),
            ("2001:4860:4860::8888", None),
        ];
        for (text, expected) in table {
            assert_eq!(special_purpose(address(text)), *expected, "{text}");
        }
    }

    #[test]
    fn a_network_holds_what_its_prefix_covers_and_nothing_of_the_other_family() {
        let loopback = IpNetwork::new(address("127.0.0.0"), 8).expect("a prefix that fits");
        assert!(loopback.contains(address("127.1.2.3")));
        assert!(!loopback.contains(address("128.0.0.1")));
        assert!(!loopback.contains(address("::ffff:127.0.0.1")));
        let one = IpNetwork::new(address("10.0.0.7"), 32).expect("a prefix that fits");
        assert!(one.contains(address("10.0.0.7")));
        assert!(!one.contains(address("10.0.0.8")));
        let everything = IpNetwork::new(address("::"), 0).expect("a prefix that fits");
        assert!(everything.contains(address("2606:4700::1")));
        assert!(!everything.contains(address("1.1.1.1")));
        let ula = IpNetwork::new(address("fd00::"), 8).expect("a prefix that fits");
        assert!(ula.contains(address("fd12::1")));
        assert!(!ula.contains(address("fc12::1")));
        assert_eq!(IpNetwork::new(address("10.0.0.0"), 33), None);
        assert_eq!(IpNetwork::new(address("::"), 129), None);
    }

    /// A resolver that answers from a table and counts what it was asked.
    struct Table {
        answers: Vec<(&'static str, Vec<IpAddr>)>,
        asked: Mutex<Vec<String>>,
    }

    impl Resolve for Table {
        fn resolve(&self, host: &str) -> io::Result<Vec<IpAddr>> {
            self.asked
                .lock()
                .expect("the log is not left mid-update")
                .push(host.to_string());
            self.answers
                .iter()
                .find(|(name, _)| *name == host)
                .map(|(_, addresses)| addresses.clone())
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no such name"))
        }
    }

    fn table(answers: &[(&'static str, &[&str])]) -> Arc<Table> {
        Arc::new(Table {
            answers: answers
                .iter()
                .map(|(name, addresses)| (*name, addresses.iter().copied().map(address).collect()))
                .collect(),
            asked: Mutex::new(Vec::new()),
        })
    }

    fn asked(resolver: &Table) -> Vec<String> {
        resolver.asked.lock().expect("not mid-update").clone()
    }

    fn at(addresses: &[&str], port: u16) -> Vec<SocketAddr> {
        addresses
            .iter()
            .copied()
            .map(|text| SocketAddr::new(address(text), port))
            .collect()
    }

    #[test]
    fn a_target_off_the_allow_list_is_refused_before_its_name_is_resolved() {
        let resolver = table(&[("api.example.com", &["93.184.215.14"])]);
        let policy = DestinationPolicy::allow_list(vec![AllowedDestination::new(
            Scheme::Https,
            "api.example.com",
            443,
        )])
        .resolving_with(resolver.clone());
        assert_eq!(
            policy.admit("https", "other.example.com", 443),
            Err(Inadmissible::NotAllowed)
        );
        assert_eq!(
            policy.admit("http", "api.example.com", 443),
            Err(Inadmissible::NotAllowed)
        );
        assert_eq!(
            policy.admit("https", "api.example.com", 8443),
            Err(Inadmissible::NotAllowed)
        );
        assert_eq!(
            policy.admit("ftp", "api.example.com", 443),
            Err(Inadmissible::NotAllowed)
        );
        assert!(asked(&resolver).is_empty());
        assert_eq!(
            policy.admit("https", "API.Example.com.", 443),
            Ok(Destination {
                scheme: Scheme::Https,
                host: "api.example.com".to_string(),
                port: 443,
                addresses: at(&["93.184.215.14"], 443),
            })
        );
        assert_eq!(asked(&resolver), ["api.example.com"]);
    }

    #[test]
    fn a_pattern_below_a_name_admits_its_subdomains_and_not_the_name() {
        let resolver = table(&[
            ("a.example.com", &["93.184.215.14"]),
            ("a.b.example.com", &["93.184.215.14"]),
            ("example.com", &["93.184.215.14"]),
            ("badexample.com", &["93.184.215.14"]),
        ]);
        let policy = DestinationPolicy::allow_list(vec![AllowedDestination::new(
            Scheme::Https,
            "*.example.com",
            443,
        )])
        .resolving_with(resolver);
        assert!(policy.admit("https", "a.example.com", 443).is_ok());
        assert!(policy.admit("https", "a.b.example.com", 443).is_ok());
        assert_eq!(
            policy.admit("https", "example.com", 443),
            Err(Inadmissible::NotAllowed)
        );
        assert_eq!(
            policy.admit("https", "badexample.com", 443),
            Err(Inadmissible::NotAllowed)
        );
    }

    #[test]
    fn an_allow_listed_name_that_resolves_to_a_special_purpose_address_is_refused() {
        let resolver = table(&[
            ("internal.example.com", &["127.0.0.1", "::ffff:10.0.0.1"]),
            ("reached.example.com", &["10.0.3.7"]),
        ]);
        let policy = DestinationPolicy::allow_list(vec![
            AllowedDestination::new(Scheme::Http, "internal.example.com", 80),
            AllowedDestination::new(Scheme::Http, "reached.example.com", 8080)
                .reaching(IpNetwork::new(address("10.0.0.0"), 8).expect("a prefix that fits")),
        ])
        .resolving_with(resolver);
        assert_eq!(
            policy.admit("http", "internal.example.com", 80),
            Err(Inadmissible::AddressesRefused(vec![
                (address("127.0.0.1"), SpecialPurpose::Loopback),
                (address("::ffff:10.0.0.1"), SpecialPurpose::Private),
            ]))
        );
        assert_eq!(
            policy
                .admit("http", "reached.example.com", 8080)
                .map(|destination| destination.addresses),
            Ok(at(&["10.0.3.7"], 8080))
        );
    }

    #[test]
    fn of_a_name_that_resolves_to_both_only_the_public_addresses_are_admitted() {
        let resolver = table(&[("mixed.example.com", &["10.0.0.1", "93.184.215.14", "::1"])]);
        let policy = DestinationPolicy::public_internet().resolving_with(resolver);
        assert_eq!(
            policy
                .admit("https", "mixed.example.com", 443)
                .map(|destination| destination.addresses),
            Ok(at(&["93.184.215.14"], 443))
        );
    }

    #[test]
    fn the_public_internet_is_https_at_a_public_address_only() {
        let resolver = table(&[
            ("public.example.com", &["2606:4700:4700::1111"]),
            ("rebound.example.com", &["64:ff9b::7f00:1"]),
        ]);
        let policy = DestinationPolicy::public_internet().resolving_with(resolver.clone());
        assert_eq!(
            policy.admit("http", "public.example.com", 80),
            Err(Inadmissible::NotAllowed)
        );
        assert!(asked(&resolver).is_empty());
        assert!(policy.admit("https", "public.example.com", 8443).is_ok());
        assert_eq!(
            policy.admit("https", "rebound.example.com", 443),
            Err(Inadmissible::AddressesRefused(vec![(
                address("64:ff9b::7f00:1"),
                SpecialPurpose::Loopback
            )]))
        );
    }

    #[test]
    fn an_address_literal_is_judged_without_being_resolved() {
        let resolver = table(&[]);
        let policy = DestinationPolicy::public_internet().resolving_with(resolver.clone());
        assert_eq!(
            policy.admit("https", "[::ffff:169.254.169.254]", 443),
            Err(Inadmissible::AddressesRefused(vec![(
                address("::ffff:169.254.169.254"),
                SpecialPurpose::LinkLocal
            )]))
        );
        assert_eq!(
            policy
                .admit("https", "[2606:4700:4700::1111]", 443)
                .map(|destination| destination.host),
            Ok("2606:4700:4700::1111".to_string())
        );
        assert_eq!(
            policy.admit("https", "127.0.0.1", 443),
            Err(Inadmissible::AddressesRefused(vec![(
                address("127.0.0.1"),
                SpecialPurpose::Loopback
            )]))
        );
        assert!(asked(&resolver).is_empty());
    }

    #[test]
    fn a_name_that_does_not_resolve_is_a_network_failure_and_a_refusal_is_its_own() {
        let resolver = table(&[("empty.example.com", &[])]);
        let policy = DestinationPolicy::public_internet().resolving_with(resolver);
        let unresolved = policy.admit("https", "missing.example.com", 443);
        assert_eq!(unresolved, Err(Inadmissible::Unresolved));
        assert_eq!(
            policy.admit("https", "empty.example.com", 443),
            Err(Inadmissible::Unresolved)
        );
        assert_eq!(
            Inadmissible::Unresolved.failure(),
            TransportFailure::Network
        );
        assert_eq!(
            Inadmissible::NotAllowed.failure(),
            TransportFailure::DestinationRefused
        );
        assert_eq!(
            Inadmissible::AddressesRefused(Vec::new()).failure(),
            TransportFailure::DestinationRefused
        );
        assert_eq!(
            TransportFailure::DestinationRefused.code(),
            "destination_refused"
        );
    }
}
