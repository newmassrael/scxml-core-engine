// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The edge's rate limits (docs/adr/0109, D2): how fast a remote address, an authenticated
//! client, a run of failed authentications and the names an address pins for the first time may
//! cost the server, each a limiter of the generic cell rate algorithm.
//!
//! Every decision is a call of `sce:std/rate/gcra_admit`, generated into this crate
//! ([`crate::generated_rate`]) as the section 10.5 rules are, so a document's rate limit and the
//! edge's are one algorithm with one definition. What this module adds is the state the rule is
//! applied to, one theoretical arrival time per key, bounded: a limiter tracks at most
//! `max_tracked` keys, and when it must make room it drops the key whose arrival time is earliest,
//! which is a fresh limiter (its arrival time at or before now) whenever one is held, and the
//! oldest otherwise. A restart starts every limiter fresh.
//!
//! The remote address of a connection from a trusted proxy is read from its `Forwarded` header
//! (RFC 7239): the rightmost address it names that is not itself a trusted proxy.

use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;
use core::hash::Hash;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::Duration;

use crate::generated_rate::gcra_admit::gcra_admit;
use crate::peers::{EdgeRate, EdgeRates};

/// What a limiter answers a request with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Within the rate.
    Admitted,
    /// Past it: the same request would be admitted after `wait`.
    Refused { wait: Duration },
}

impl Verdict {
    /// Whether the request was refused.
    pub fn refused(self) -> bool {
        matches!(self, Verdict::Refused { .. })
    }
}

/// One limiter: each key's theoretical arrival time, in milliseconds on the server's monotonic
/// clock, under one rate.
#[derive(Debug)]
pub struct Limiter<K> {
    rate: EdgeRate,
    max_tracked: usize,
    arrivals: HashMap<K, i64>,
    /// The same keys by their arrival time, earliest first: the one dropped to make room.
    by_arrival: BTreeSet<(i64, K)>,
}

impl<K: Eq + Hash + Ord + Clone> Limiter<K> {
    /// A limiter of `rate` that tracks at most `max_tracked` keys (at least one).
    pub fn new(rate: EdgeRate, max_tracked: u32) -> Self {
        Limiter {
            rate,
            max_tracked: usize::try_from(max_tracked).unwrap_or(usize::MAX).max(1),
            arrivals: HashMap::new(),
            by_arrival: BTreeSet::new(),
        }
    }

    /// How many keys the limiter tracks.
    pub fn tracked(&self) -> usize {
        self.arrivals.len()
    }

    /// Spend one permit of `key` at `now_ms`: admitted, and the key's arrival time moves on, or
    /// refused with the wait, and nothing moves.
    pub fn spend(&mut self, key: &K, now_ms: i64) -> Verdict {
        let (verdict, arrival) = self.judge(key, now_ms);
        if let Some(arrival) = arrival {
            self.set(key, arrival);
        }
        verdict
    }

    /// Whether `key` would be admitted at `now_ms`, spending nothing.
    pub fn ask(&self, key: &K, now_ms: i64) -> Verdict {
        self.judge(key, now_ms).0
    }

    /// The rule's answer for `key` at `now_ms`, with the arrival time to keep when it admits. A
    /// key the limiter does not track is a fresh limiter, whose arrival time is 0, at or before
    /// any reading of a clock that starts at 0. The build bounds a limiter's span far below what
    /// the rule's 64-bit arithmetic holds, so the rule refuses nothing it is given; were it to,
    /// the request is refused for one interval rather than admitted unjudged.
    fn judge(&self, key: &K, now_ms: i64) -> (Verdict, Option<i64>) {
        let tat = self.arrivals.get(key).copied().unwrap_or(0);
        let interval = i64::try_from(self.rate.interval_ms).unwrap_or(i64::MAX);
        match gcra_admit(tat, now_ms, 1, interval, self.rate.burst) {
            Ok(decision) if decision.admitted => (Verdict::Admitted, Some(decision.tat)),
            Ok(decision) => (
                Verdict::Refused {
                    wait: Duration::from_millis(
                        u64::try_from(decision.retry_after).unwrap_or(u64::MAX),
                    ),
                },
                None,
            ),
            Err(_) => (
                Verdict::Refused {
                    wait: Duration::from_millis(self.rate.interval_ms),
                },
                None,
            ),
        }
    }

    /// Keep `arrival` for `key`, dropping the key of the earliest arrival first when the limiter
    /// tracks `max_tracked` keys and `key` is not one of them.
    fn set(&mut self, key: &K, arrival: i64) {
        match self.arrivals.get(key).copied() {
            Some(held) => {
                self.by_arrival.remove(&(held, key.clone()));
            }
            None => {
                if self.arrivals.len() >= self.max_tracked {
                    if let Some((_, dropped)) = self.by_arrival.pop_first() {
                        self.arrivals.remove(&dropped);
                    }
                }
            }
        }
        self.arrivals.insert(key.clone(), arrival);
        self.by_arrival.insert((arrival, key.clone()));
    }
}

/// The limiters of one server's edge (docs/adr/0109, D2), each absent when its rate is off.
#[derive(Debug)]
pub struct Edge {
    address: Option<Limiter<IpAddr>>,
    client: Option<Limiter<String>>,
    failed_by_name: Option<Limiter<String>>,
    failed_by_address: Option<Limiter<IpAddr>>,
    new_names: Option<Limiter<IpAddr>>,
}

impl Edge {
    /// The limiters `rates` declares.
    pub fn new(rates: &EdgeRates) -> Self {
        let tracked = rates.max_tracked;
        Edge {
            address: rates.address.map(|rate| Limiter::new(rate, tracked)),
            client: rates.client.map(|rate| Limiter::new(rate, tracked)),
            failed_by_name: rates.failed_auth.map(|rate| Limiter::new(rate, tracked)),
            failed_by_address: rates.failed_auth.map(|rate| Limiter::new(rate, tracked)),
            new_names: rates.new_names.map(|rate| Limiter::new(rate, tracked)),
        }
    }

    /// Every exchange, from `address` (an [`super::server::address_key`]), before it is
    /// authenticated.
    pub fn exchange(&mut self, address: Option<IpAddr>, now_ms: i64) -> Verdict {
        spend_on(&mut self.address, address.as_ref(), now_ms)
    }

    /// Whether an exchange of `name` from `address` may reach the authenticator: refused while
    /// the failed authentications of the name or of the address are past their rate, so that a
    /// guess costs the guesser the rate and the server no check. Spends nothing.
    pub fn may_authenticate(&self, name: &str, address: Option<IpAddr>, now_ms: i64) -> Verdict {
        let by_name = self
            .failed_by_name
            .as_ref()
            .map_or(Verdict::Admitted, |limiter| {
                limiter.ask(&String::from(name), now_ms)
            });
        let by_address = match (&self.failed_by_address, address) {
            (Some(limiter), Some(address)) => limiter.ask(&address, now_ms),
            _ => Verdict::Admitted,
        };
        longer(by_name, by_address)
    }

    /// An exchange of `name` from `address` failed authentication (a `401`): it spends a permit
    /// of both, so that a run of failures is refused before the authenticator.
    pub fn failed(&mut self, name: &str, address: Option<IpAddr>, now_ms: i64) {
        spend_on(&mut self.failed_by_name, Some(&String::from(name)), now_ms);
        spend_on(&mut self.failed_by_address, address.as_ref(), now_ms);
    }

    /// An exchange `name` authenticated.
    pub fn client(&mut self, name: &str, now_ms: i64) -> Verdict {
        spend_on(&mut self.client, Some(&String::from(name)), now_ms)
    }

    /// `address` is about to pin a name for the first time.
    pub fn new_name(&mut self, address: Option<IpAddr>, now_ms: i64) -> Verdict {
        spend_on(&mut self.new_names, address.as_ref(), now_ms)
    }
}

/// Spend a permit of `key` on `limiter`; admitted when the limiter is off or there is no key.
fn spend_on<K: Eq + Hash + Ord + Clone>(
    limiter: &mut Option<Limiter<K>>,
    key: Option<&K>,
    now_ms: i64,
) -> Verdict {
    match (limiter, key) {
        (Some(limiter), Some(key)) => limiter.spend(key, now_ms),
        _ => Verdict::Admitted,
    }
}

/// The verdict that asks the longer wait.
fn longer(a: Verdict, b: Verdict) -> Verdict {
    match (a, b) {
        (Verdict::Refused { wait: x }, Verdict::Refused { wait: y }) => {
            Verdict::Refused { wait: x.max(y) }
        }
        (refused @ Verdict::Refused { .. }, _) | (_, refused @ Verdict::Refused { .. }) => refused,
        _ => Verdict::Admitted,
    }
}

/// A block of addresses, `<address>/<prefix>` (deploy.yaml's `trusted_proxies`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cidr {
    network: IpAddr,
    prefix: u8,
}

impl Cidr {
    /// The block `text` names: `<address>/<prefix>`, or a bare address, which is the block of
    /// that address alone. `None` for anything else.
    pub fn parse(text: &str) -> Option<Cidr> {
        let (address, prefix) = match text.trim().split_once('/') {
            Some((address, prefix)) => (address, Some(prefix)),
            None => (text.trim(), None),
        };
        let network = canonical(address.parse().ok()?);
        let width = if network.is_ipv4() { 32 } else { 128 };
        let prefix = match prefix {
            Some(prefix) => prefix.parse::<u8>().ok().filter(|n| *n <= width)?,
            None => width,
        };
        Some(Cidr { network, prefix })
    }

    /// Whether `address` is in the block.
    pub fn contains(&self, address: IpAddr) -> bool {
        match (self.network, canonical(address)) {
            (IpAddr::V4(network), IpAddr::V4(address)) => {
                let mask = u32::MAX
                    .checked_shl(32 - u32::from(self.prefix))
                    .unwrap_or(0);
                u32::from(network) & mask == u32::from(address) & mask
            }
            (IpAddr::V6(network), IpAddr::V6(address)) => {
                let mask = u128::MAX
                    .checked_shl(128 - u32::from(self.prefix))
                    .unwrap_or(0);
                u128::from(network) & mask == u128::from(address) & mask
            }
            _ => false,
        }
    }
}

/// An IPv4 address mapped into IPv6 as the IPv4 address it maps; any other as it is.
fn canonical(address: IpAddr) -> IpAddr {
    match address {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(address, IpAddr::V4),
        IpAddr::V4(_) => address,
    }
}

/// Whether `address` is in one of `trusted`.
pub fn is_trusted(trusted: &[Cidr], address: IpAddr) -> bool {
    trusted.iter().any(|block| block.contains(address))
}

/// The address a client connected from: `peer`, the address the connection came from, unless it
/// is a trusted proxy, in which case the rightmost address the `forwarded` headers name (RFC 7239,
/// in the order they arrived) that is not itself a trusted proxy. A name that is no address (an
/// obfuscated or `unknown` node) ends the walk at the trusted hop that named it; when every
/// address named is trusted, the leftmost is the client.
pub fn client_address<'a>(
    peer: IpAddr,
    forwarded: impl IntoIterator<Item = &'a str>,
    trusted: &[Cidr],
) -> IpAddr {
    let peer = canonical(peer);
    if !is_trusted(trusted, peer) {
        return peer;
    }
    let named: Vec<Option<IpAddr>> = forwarded
        .into_iter()
        .flat_map(|header| header.split(','))
        .filter_map(forwarded_for)
        .map(node_address)
        .collect();
    let mut client = peer;
    for named in named.into_iter().rev() {
        let Some(address) = named else {
            return client;
        };
        client = canonical(address);
        if !is_trusted(trusted, client) {
            return client;
        }
    }
    client
}

/// The `for` parameter of one element of a `Forwarded` header, its quotes removed; `None` for an
/// element that names none.
fn forwarded_for(element: &str) -> Option<&str> {
    element.split(';').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        name.trim()
            .eq_ignore_ascii_case("for")
            .then(|| value.trim().trim_matches('"'))
    })
}

/// The address a `for` node names (RFC 7239, section 6): an IPv4 address, or an IPv6 address in
/// brackets, either with a port or without; `None` for `unknown` or an obfuscated name.
fn node_address(node: &str) -> Option<IpAddr> {
    if let Some(rest) = node.strip_prefix('[') {
        let (address, _) = rest.split_once(']')?;
        return address.parse::<Ipv6Addr>().ok().map(IpAddr::V6);
    }
    let address = node.split_once(':').map_or(node, |(address, _)| address);
    address.parse::<Ipv4Addr>().ok().map(IpAddr::V4)
}

/// The seconds a `Retry-After` header asks for a wait: rounded up, and at least one.
pub fn retry_seconds(wait: Duration) -> u64 {
    let seconds = wait.as_millis().div_ceil(1000).max(1);
    u64::try_from(seconds).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: EdgeRate = EdgeRate {
        interval_ms: 100,
        burst: 3,
    };

    /// The cases `sce:std/rate/gcra_admit` states of itself, through a limiter: at 1000 ms with
    /// an interval of 100 and a burst of 3, three permits are admitted, each moving the arrival
    /// time on by the interval, and the fourth is refused with the wait until one regenerates.
    /// Red if the limiter spends a refused request, or keeps the arrival time of none.
    #[test]
    fn a_limiter_admits_its_burst_and_refuses_with_the_exact_wait() {
        let mut limiter = Limiter::new(RATE, 16);
        let key = String::from("phone");
        for _ in 0..3 {
            assert_eq!(limiter.spend(&key, 1000), Verdict::Admitted);
        }
        assert_eq!(
            limiter.spend(&key, 1000),
            Verdict::Refused {
                wait: Duration::from_millis(100)
            }
        );
        assert_eq!(
            limiter.spend(&key, 1050),
            Verdict::Refused {
                wait: Duration::from_millis(50)
            },
            "a refusal spends nothing: the wait shrinks with the clock"
        );
        assert_eq!(limiter.spend(&key, 1100), Verdict::Admitted);
        assert_eq!(
            limiter.ask(&String::from("tablet"), 1100),
            Verdict::Admitted,
            "another key is its own"
        );
        assert_eq!(limiter.tracked(), 1, "asking tracks nothing");
    }

    /// docs/adr/0109, D2: a limiter tracks at most its bound, and makes room by dropping the key
    /// of the earliest arrival, a fresh one first. Red if the table grows past its bound.
    #[test]
    fn a_limiter_at_its_bound_drops_the_earliest_arrival() {
        let mut limiter = Limiter::new(RATE, 2);
        let (a, b, c) = (String::from("a"), String::from("b"), String::from("c"));
        limiter.spend(&a, 0);
        limiter.spend(&b, 0);
        limiter.spend(&b, 0);
        limiter.spend(&b, 0);
        assert_eq!(limiter.tracked(), 2);
        // a's arrival time (100) is the earliest: it is dropped for c.
        limiter.spend(&c, 0);
        assert_eq!(limiter.tracked(), 2);
        assert!(limiter.ask(&b, 0).refused(), "b is still tracked and spent");
        for n in 0..1_000u32 {
            limiter.spend(&alloc::format!("k{n}"), 0);
        }
        assert_eq!(limiter.tracked(), 2, "a thousand keys later, still two");
    }

    /// docs/adr/0109, D2: a run of failed authentications from one address is refused before the
    /// authenticator once it is past its rate, for any name from that address and for that name
    /// from any address, while an established client from another address is served. Red if
    /// `may_authenticate` reads neither failure limiter.
    #[test]
    fn failed_authentications_stop_reaching_the_authenticator_and_spare_others() {
        let rates = EdgeRates {
            failed_auth: Some(EdgeRate {
                interval_ms: 60_000,
                burst: 3,
            }),
            ..EdgeRates::NONE
        };
        let mut edge = Edge::new(&rates);
        let guesser: IpAddr = "198.51.100.7".parse().unwrap();
        let elsewhere: IpAddr = "203.0.113.9".parse().unwrap();
        for n in 0..3 {
            assert_eq!(
                edge.may_authenticate("victim", Some(guesser), n),
                Verdict::Admitted
            );
            edge.failed("victim", Some(guesser), n);
        }
        assert!(edge.may_authenticate("victim", Some(guesser), 10).refused());
        assert!(
            edge.may_authenticate("other", Some(guesser), 10).refused(),
            "the address is past its rate"
        );
        assert!(
            edge.may_authenticate("victim", Some(elsewhere), 10)
                .refused(),
            "the name is past its rate"
        );
        assert_eq!(
            edge.may_authenticate("phone", Some(elsewhere), 10),
            Verdict::Admitted,
            "an established client elsewhere is served"
        );
        assert_eq!(
            edge.client("phone", 10),
            Verdict::Admitted,
            "its rate is off"
        );
    }

    /// docs/adr/0109, D2: a flood of new names from one address is refused past its rate, while
    /// an established client, which pins nothing, is served, and another address pins its own.
    /// Red if `new_name` spends nothing.
    #[test]
    fn a_flood_of_new_names_is_throttled_while_an_established_client_is_served() {
        let rates = EdgeRates {
            new_names: Some(EdgeRate {
                interval_ms: 180_000,
                burst: 20,
            }),
            client: Some(EdgeRate {
                interval_ms: 50,
                burst: 40,
            }),
            ..EdgeRates::NONE
        };
        let mut edge = Edge::new(&rates);
        let flooder: IpAddr = "198.51.100.7".parse().unwrap();
        let mut pinned = 0;
        for n in 0..1_000 {
            if edge.new_name(Some(flooder), n) == Verdict::Admitted {
                pinned += 1;
            }
            assert_eq!(edge.client("established", n * 50), Verdict::Admitted);
        }
        assert_eq!(pinned, 20, "the burst, and no more within the hour");
        assert_eq!(
            edge.new_name(Some("203.0.113.9".parse().unwrap()), 1_000),
            Verdict::Admitted
        );
        let Verdict::Refused { wait } = edge.new_name(Some(flooder), 1_000) else {
            panic!("the flooder is past its rate");
        };
        assert_eq!(
            retry_seconds(wait),
            180 - 1,
            "the wait, in whole seconds rounded up"
        );
    }

    #[test]
    fn a_wait_is_asked_in_whole_seconds_rounded_up() {
        assert_eq!(retry_seconds(Duration::ZERO), 1);
        assert_eq!(retry_seconds(Duration::from_millis(1)), 1);
        assert_eq!(retry_seconds(Duration::from_millis(1000)), 1);
        assert_eq!(retry_seconds(Duration::from_millis(1001)), 2);
    }

    #[test]
    fn a_block_holds_the_addresses_its_prefix_covers() {
        let block = Cidr::parse("10.0.0.0/8").expect("a block");
        assert!(block.contains("10.200.3.4".parse().unwrap()));
        assert!(block.contains("::ffff:10.1.1.1".parse().unwrap()));
        assert!(!block.contains("11.0.0.1".parse().unwrap()));
        let one = Cidr::parse("2001:db8::1").expect("an address");
        assert!(one.contains("2001:db8::1".parse().unwrap()));
        assert!(!one.contains("2001:db8::2".parse().unwrap()));
        assert!(Cidr::parse("0.0.0.0/0")
            .unwrap()
            .contains("192.0.2.1".parse().unwrap()));
        assert_eq!(Cidr::parse("10.0.0.0/33"), None);
        assert_eq!(Cidr::parse("proxy.example"), None);
    }

    /// docs/adr/0109, D2: `Forwarded` is read only from a trusted proxy, and names the client as
    /// the rightmost address that is not itself a trusted proxy. Red if the header is read from
    /// any connection: a client then names whatever address it likes.
    #[test]
    fn forwarded_is_read_only_from_a_trusted_proxy() {
        let trusted = [Cidr::parse("10.0.0.0/8").unwrap()];
        let proxy: IpAddr = "10.0.0.2".parse().unwrap();
        let stranger: IpAddr = "198.51.100.7".parse().unwrap();
        let header = ["for=192.0.2.60;proto=https, for=\"[2001:db8::1]:4711\", for=10.0.0.9"];
        assert_eq!(
            client_address(stranger, header, &trusted),
            stranger,
            "a stranger's header is not read"
        );
        assert_eq!(
            client_address(proxy, header, &trusted),
            "2001:db8::1".parse::<IpAddr>().unwrap(),
            "the rightmost untrusted hop"
        );
        assert_eq!(
            client_address(proxy, ["for=192.0.2.60", "for=10.0.0.9"], &trusted),
            "192.0.2.60".parse::<IpAddr>().unwrap(),
            "headers are read in the order they arrived"
        );
        assert_eq!(
            client_address(proxy, ["for=unknown, for=10.0.0.9"], &trusted),
            "10.0.0.9".parse::<IpAddr>().unwrap(),
            "an unknown node ends the walk at the trusted hop that named it"
        );
        assert_eq!(
            client_address(proxy, core::iter::empty(), &trusted),
            proxy,
            "a proxy that names nobody is its own client"
        );
        assert_eq!(
            client_address(proxy, ["for=192.0.2.60:9443;by=10.0.0.2"], &[]),
            proxy,
            "with no trusted proxy, nothing is read"
        );
    }
}
