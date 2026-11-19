// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

/**
 * @file DestinationPolicy.h
 * @brief Where a transport of the HTTP operation may connect for a server (docs/SCE_HOST_FEATURES.md,
 *        section 5, "Destination policy"; docs/adr/0109, D4).
 *
 * A DestinationPolicy judges a target before anything is sent: whether its scheme, host and port are
 * ones the policy admits, then every address its name resolves to, against the special-purpose ranges
 * of RFC 6890 and the IANA registries it set up (specialPurpose). The policy resolves the name itself,
 * through the Resolver it is built with, and answers the addresses it admitted; a transport connects
 * to those and to no other, so a name that resolves to another address between a check and a connect
 * is never resolved a second time.
 *
 * The policy is data and a resolver and opens no socket, so this header needs no network library; the
 * system's resolver is in host/CurlTransport.h, beside the transport that uses it.
 */

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <memory>
#include <optional>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

#include "common/StringUtils.h"
#include "host/StandardHost.h"

namespace SCE::StandardHost {

/// An IPv4 or IPv6 address: an IPv4 one in the first four bytes.
struct IpAddress {
    bool v6 = false;
    std::array<uint8_t, 16> bytes{};

    static IpAddress fromV4(const std::array<uint8_t, 4> &octets) {
        IpAddress address;
        for (size_t i = 0; i < 4; ++i) {
            address.bytes[i] = octets[i];
        }
        return address;
    }

    static IpAddress fromV6(const std::array<uint8_t, 16> &octets) {
        IpAddress address;
        address.v6 = true;
        address.bytes = octets;
        return address;
    }

    /// The address a literal writes: four decimal octets without a leading zero, or the text of RFC 4291
    /// section 2.2 (one `::`, a final dotted IPv4 address), without a zone; nothing for another text.
    static std::optional<IpAddress> parse(std::string_view text) {
        if (text.find(':') == std::string_view::npos) {
            const auto octets = parseV4(text);
            if (!octets) {
                return std::nullopt;
            }
            return fromV4(*octets);
        }
        return parseV6(text);
    }

    /// The canonical text: dotted decimal, or RFC 5952's (lower case, the longest run of two or more zero
    /// groups written `::`).
    std::string text() const {
        if (!v6) {
            return std::to_string(bytes[0]) + "." + std::to_string(bytes[1]) + "." + std::to_string(bytes[2]) + "." +
                   std::to_string(bytes[3]);
        }
        std::array<unsigned, 8> groups{};
        for (size_t i = 0; i < 8; ++i) {
            groups[i] = (static_cast<unsigned>(bytes[2 * i]) << 8) | bytes[2 * i + 1];
        }
        size_t runStart = 8;
        size_t runLength = 0;
        for (size_t i = 0; i < 8;) {
            size_t j = i;
            while (j < 8 && groups[j] == 0) {
                ++j;
            }
            if (j - i > runLength && j - i >= 2) {
                runStart = i;
                runLength = j - i;
            }
            i = j == i ? i + 1 : j;
        }
        static const char *const digits = "0123456789abcdef";
        const auto hex = [](unsigned group) {
            std::string out;
            bool started = false;
            for (int shift = 12; shift >= 0; shift -= 4) {
                const unsigned nibble = (group >> shift) & 0xFu;
                if (nibble != 0 || started || shift == 0) {
                    out += digits[nibble];
                    started = true;
                }
            }
            return out;
        };
        std::string out;
        for (size_t i = 0; i < 8; ++i) {
            if (i == runStart) {
                out += "::";
                i += runLength - 1;
                continue;
            }
            if (!out.empty() && out.back() != ':') {
                out += ':';
            }
            out += hex(groups[i]);
        }
        return out;
    }

    bool operator==(const IpAddress &other) const {
        return v6 == other.v6 && bytes == other.bytes;
    }

    bool operator!=(const IpAddress &other) const {
        return !(*this == other);
    }

private:
    static std::optional<std::array<uint8_t, 4>> parseV4(std::string_view text) {
        std::array<uint8_t, 4> octets{};
        size_t at = 0;
        for (size_t part = 0; part < 4; ++part) {
            if (part > 0) {
                if (at >= text.size() || text[at] != '.') {
                    return std::nullopt;
                }
                ++at;
            }
            const size_t begin = at;
            unsigned value = 0;
            while (at < text.size() && text[at] >= '0' && text[at] <= '9' && at - begin < 3) {
                value = value * 10 + static_cast<unsigned>(text[at] - '0');
                ++at;
            }
            const size_t length = at - begin;
            if (length == 0 || value > 255 || (length > 1 && text[begin] == '0')) {
                return std::nullopt;
            }
            octets[part] = static_cast<uint8_t>(value);
        }
        if (at != text.size()) {
            return std::nullopt;
        }
        return octets;
    }

    /// The groups of one side of `::`, a final dotted IPv4 address counting as two when `lastMayBeV4`.
    static bool parseGroups(std::string_view side, bool lastMayBeV4, std::vector<uint16_t> &groups) {
        if (side.empty()) {
            return true;
        }
        size_t at = 0;
        for (;;) {
            const size_t colon = side.find(':', at);
            const std::string_view piece = side.substr(at, colon == std::string_view::npos ? side.npos : colon - at);
            const bool last = colon == std::string_view::npos;
            if (last && lastMayBeV4 && piece.find('.') != std::string_view::npos) {
                const auto octets = parseV4(piece);
                if (!octets) {
                    return false;
                }
                groups.push_back(static_cast<uint16_t>(((*octets)[0] << 8) | (*octets)[1]));
                groups.push_back(static_cast<uint16_t>(((*octets)[2] << 8) | (*octets)[3]));
                return true;
            }
            if (piece.empty() || piece.size() > 4) {
                return false;
            }
            unsigned value = 0;
            for (const char c : piece) {
                unsigned nibble = 0;
                if (c >= '0' && c <= '9') {
                    nibble = static_cast<unsigned>(c - '0');
                } else if (c >= 'a' && c <= 'f') {
                    nibble = static_cast<unsigned>(c - 'a' + 10);
                } else if (c >= 'A' && c <= 'F') {
                    nibble = static_cast<unsigned>(c - 'A' + 10);
                } else {
                    return false;
                }
                value = (value << 4) | nibble;
            }
            groups.push_back(static_cast<uint16_t>(value));
            if (last) {
                return true;
            }
            at = colon + 1;
        }
    }

    static std::optional<IpAddress> parseV6(std::string_view text) {
        const size_t gap = text.find("::");
        std::vector<uint16_t> head;
        std::vector<uint16_t> tail;
        if (gap == std::string_view::npos) {
            if (!parseGroups(text, true, head) || head.size() != 8) {
                return std::nullopt;
            }
        } else {
            const std::string_view left = text.substr(0, gap);
            const std::string_view right = text.substr(gap + 2);
            if (!parseGroups(left, false, head) || !parseGroups(right, true, tail) || head.size() + tail.size() > 7) {
                return std::nullopt;
            }
        }
        std::array<uint8_t, 16> octets{};
        const auto put = [&octets](size_t group, uint16_t value) {
            octets[2 * group] = static_cast<uint8_t>(value >> 8);
            octets[2 * group + 1] = static_cast<uint8_t>(value & 0xFFu);
        };
        for (size_t i = 0; i < head.size(); ++i) {
            put(i, head[i]);
        }
        for (size_t i = 0; i < tail.size(); ++i) {
            put(8 - tail.size() + i, tail[i]);
        }
        return fromV6(octets);
    }
};

/// A special-purpose range an address lies in: an address a server's outbound request is not sent to
/// unless the deployment names the range for the destination (AllowedDestination::reaching).
enum class SpecialPurpose {
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
    /// The IETF protocol assignments: `192.0.0.0/24`, `2001::/23` (Teredo, the benchmarking and ORCHID
    /// blocks among them).
    ProtocolAssignments,
    /// `192.0.2.0/24`, `198.51.100.0/24`, `203.0.113.0/24`, `2001:db8::/32`, `3fff::/20`.
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
    /// Reserved, deprecated, or not allocated for global unicast: `240.0.0.0/4`, the 6to4 relay anycast
    /// `192.88.99.0/24`, and every IPv6 address outside `2000::/3` no other range names (site-local
    /// `fec0::/10` among them).
    Reserved,
};

namespace detail {

/// Whether the first `prefix` bits of `address` are those of `network`.
inline bool withinPrefix(const uint8_t *address, const uint8_t *network, size_t prefix) {
    const size_t whole = prefix / 8;
    for (size_t i = 0; i < whole; ++i) {
        if (address[i] != network[i]) {
            return false;
        }
    }
    const size_t rest = prefix % 8;
    if (rest == 0) {
        return true;
    }
    const unsigned mask = (0xFFu << (8 - rest)) & 0xFFu;
    return (address[whole] & mask) == (network[whole] & mask);
}

struct V4Range {
    std::array<uint8_t, 4> network;
    size_t prefix;
    SpecialPurpose range;
};

struct V6Range {
    std::array<uint8_t, 16> network;
    size_t prefix;
    SpecialPurpose range;
};

/// The IPv4 special-purpose ranges; the first that holds an address names it, so a range inside another
/// comes first.
inline const std::array<V4Range, 17> &v4Ranges() {
    static const std::array<V4Range, 17> ranges = {{
        {{{0, 0, 0, 0}}, 32, SpecialPurpose::Unspecified},
        {{{0, 0, 0, 0}}, 8, SpecialPurpose::ThisNetwork},
        {{{10, 0, 0, 0}}, 8, SpecialPurpose::Private},
        {{{100, 64, 0, 0}}, 10, SpecialPurpose::Shared},
        {{{127, 0, 0, 0}}, 8, SpecialPurpose::Loopback},
        {{{169, 254, 0, 0}}, 16, SpecialPurpose::LinkLocal},
        {{{172, 16, 0, 0}}, 12, SpecialPurpose::Private},
        {{{192, 0, 0, 0}}, 24, SpecialPurpose::ProtocolAssignments},
        {{{192, 0, 2, 0}}, 24, SpecialPurpose::Documentation},
        {{{192, 88, 99, 0}}, 24, SpecialPurpose::Reserved},
        {{{192, 168, 0, 0}}, 16, SpecialPurpose::Private},
        {{{198, 18, 0, 0}}, 15, SpecialPurpose::Benchmarking},
        {{{198, 51, 100, 0}}, 24, SpecialPurpose::Documentation},
        {{{203, 0, 113, 0}}, 24, SpecialPurpose::Documentation},
        {{{224, 0, 0, 0}}, 4, SpecialPurpose::Multicast},
        {{{255, 255, 255, 255}}, 32, SpecialPurpose::Broadcast},
        {{{240, 0, 0, 0}}, 4, SpecialPurpose::Reserved},
    }};
    return ranges;
}

/// The IPv6 special-purpose ranges that embed no IPv4 address, as v4Ranges lists them.
inline const std::array<V6Range, 8> &v6Ranges() {
    static const std::array<V6Range, 8> ranges = {{
        {{{0x00, 0x64, 0xff, 0x9b, 0x00, 0x01}}, 48, SpecialPurpose::LocalTranslation},
        {{{0x01, 0x00}}, 64, SpecialPurpose::Discard},
        {{{0x20, 0x01, 0x0d, 0xb8}}, 32, SpecialPurpose::Documentation},
        {{{0x20, 0x01}}, 23, SpecialPurpose::ProtocolAssignments},
        {{{0x3f, 0xff}}, 20, SpecialPurpose::Documentation},
        {{{0xfc}}, 7, SpecialPurpose::UniqueLocal},
        {{{0xfe, 0x80}}, 10, SpecialPurpose::LinkLocal},
        {{{0xff}}, 8, SpecialPurpose::Multicast},
    }};
    return ranges;
}

/// The IPv6 forms that carry an IPv4 address in their last 32 bits, each a /96: IPv4-mapped
/// (`::ffff:0:0/96`), IPv4-translated (`::ffff:0:0:0/96`), the well-known NAT64 prefix (`64:ff9b::/96`)
/// and IPv4-compatible (`::/96`).
inline const std::array<std::array<uint8_t, 16>, 4> &v6EmbeddingLow() {
    static const std::array<std::array<uint8_t, 16>, 4> prefixes = {{
        {{0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff}},
        {{0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff, 0, 0}},
        {{0x00, 0x64, 0xff, 0x9b}},
        {{}},
    }};
    return prefixes;
}

/// A host as it is compared: without the brackets of an IPv6 literal and a final dot, in ASCII lower
/// case, and an address literal in its canonical text.
inline std::string normalizedHost(std::string_view host) {
    if (host.size() >= 2 && host.front() == '[' && host.back() == ']') {
        host = host.substr(1, host.size() - 2);
    }
    if (const auto literal = IpAddress::parse(host)) {
        return literal->text();
    }
    if (!host.empty() && host.back() == '.') {
        host.remove_suffix(1);
    }
    std::string name(host);
    ::SCE::lowerAsciiInPlace(name);
    return name;
}

}  // namespace detail

/// The special-purpose range `address` lies in, or nothing for an address of the public internet.
///
/// An IPv6 address that carries an IPv4 address (IPv4-mapped, IPv4-translated, IPv4-compatible, 6to4, the
/// well-known NAT64 prefix) is judged by the address it carries, so `::ffff:127.0.0.1`, `64:ff9b::7f00:1`
/// and `2002:7f00:1::` are loopback, and `::ffff:8.8.8.8` is not special.
inline std::optional<SpecialPurpose> specialPurpose(const IpAddress &address) {
    const uint8_t *bytes = address.bytes.data();
    if (!address.v6) {
        for (const auto &entry : detail::v4Ranges()) {
            if (detail::withinPrefix(bytes, entry.network.data(), entry.prefix)) {
                return entry.range;
            }
        }
        return std::nullopt;
    }
    static const std::array<uint8_t, 16> zero{};
    if (detail::withinPrefix(bytes, zero.data(), 120)) {
        if (bytes[15] == 0) {
            return SpecialPurpose::Unspecified;
        }
        if (bytes[15] == 1) {
            return SpecialPurpose::Loopback;
        }
    }
    for (const auto &prefix : detail::v6EmbeddingLow()) {
        if (detail::withinPrefix(bytes, prefix.data(), 96)) {
            return specialPurpose(
                IpAddress::fromV4(std::array<uint8_t, 4>{{bytes[12], bytes[13], bytes[14], bytes[15]}}));
        }
    }
    static const std::array<uint8_t, 16> sixToFour{{0x20, 0x02}};
    if (detail::withinPrefix(bytes, sixToFour.data(), 16)) {
        return specialPurpose(IpAddress::fromV4(std::array<uint8_t, 4>{{bytes[2], bytes[3], bytes[4], bytes[5]}}));
    }
    for (const auto &entry : detail::v6Ranges()) {
        if (detail::withinPrefix(bytes, entry.network.data(), entry.prefix)) {
            return entry.range;
        }
    }
    static const std::array<uint8_t, 16> globalUnicast{{0x20}};
    if (!detail::withinPrefix(bytes, globalUnicast.data(), 3)) {
        return SpecialPurpose::Reserved;
    }
    return std::nullopt;
}

/// A network: an address and the length of its prefix.
class IpNetwork {
public:
    /// The network of `address` with a prefix of `prefix` bits, or nothing for a prefix longer than the
    /// address.
    static std::optional<IpNetwork> make(const IpAddress &address, size_t prefix) {
        if (prefix > (address.v6 ? 128u : 32u)) {
            return std::nullopt;
        }
        return IpNetwork(address, prefix);
    }

    /// Whether `address` lies in the network. An address of the other family lies in none, an
    /// IPv4-mapped one included.
    bool contains(const IpAddress &address) const {
        return address.v6 == address_.v6 && detail::withinPrefix(address.bytes.data(), address_.bytes.data(), prefix_);
    }

private:
    IpNetwork(const IpAddress &address, size_t prefix) : address_(address), prefix_(prefix) {}

    IpAddress address_;
    size_t prefix_;
};

/// The schemes the HTTP operation carries.
enum class Scheme { Http, Https };

/// One destination an allow-list admits: a scheme, a host pattern and a port, and the special-purpose
/// networks it may reach.
class AllowedDestination {
public:
    /// `scheme://host:port`. `host` is a name, `*.` before a name for every name below it (not the name
    /// itself), or an address literal; names are compared without case and without a final dot. The
    /// addresses it resolves to are still judged: one in a special-purpose range is refused unless
    /// reaching() names a network that holds it.
    AllowedDestination(Scheme scheme, std::string_view host, uint16_t port) : scheme_(scheme), port_(port) {
        if (host.size() > 2 && host.substr(0, 2) == "*.") {
            below_ = true;
            host.remove_prefix(2);
        }
        host_ = detail::normalizedHost(host);
    }

    /// The same destination, which may also reach `network`, special-purpose or not: an internal service
    /// the deployment names, and no other.
    AllowedDestination reaching(const IpNetwork &network) const {
        AllowedDestination wider = *this;
        wider.reaches_.push_back(network);
        return wider;
    }

    bool matches(Scheme scheme, const std::string &host, uint16_t port) const {
        if (scheme != scheme_ || port != port_) {
            return false;
        }
        if (!below_) {
            return host == host_;
        }
        return host.size() > host_.size() + 1 && host.compare(host.size() - host_.size(), host_.size(), host_) == 0 &&
               host[host.size() - host_.size() - 1] == '.';
    }

    const std::vector<IpNetwork> &reaches() const {
        return reaches_;
    }

private:
    Scheme scheme_;
    std::string host_;
    bool below_ = false;
    uint16_t port_;
    std::vector<IpNetwork> reaches_;
};

/// What resolves a name for a policy.
class Resolver {
public:
    virtual ~Resolver() = default;
    /// The addresses `host` resolves to; nothing, or no address, for a name that did not resolve.
    virtual std::optional<std::vector<IpAddress>> resolve(const std::string &host) = 0;
};

/// A target a policy admitted: the host as it was compared, which a transport names in `Host` and
/// verifies TLS against, its port, and the addresses it may connect to, in the order resolved.
struct Destination {
    Scheme scheme = Scheme::Https;
    std::string host;
    uint16_t port = 0;
    std::vector<IpAddress> addresses;
};

/// A policy's answer to a target.
struct Admission {
    enum class Verdict {
        /// `destination` is where the transport may connect.
        Admitted,
        /// The scheme, host or port is on no entry of the allow-list, or the scheme is not `https` under
        /// publicInternet(). The name was not resolved.
        NotAllowed,
        /// Every address the name resolved to is refused: `refused` holds each, with its range.
        AddressesRefused,
        /// The name resolved to no address.
        Unresolved,
    };
    Verdict verdict = Verdict::NotAllowed;
    Destination destination;
    std::vector<std::pair<IpAddress, SpecialPurpose>> refused;

    /// What a transport answers a target that was not admitted with: `destination_refused`, or `network`
    /// for a name that did not resolve.
    Exchange::Kind failure() const {
        return verdict == Verdict::Unresolved ? Exchange::Kind::Failure : Exchange::Kind::DestinationRefused;
    }
};

/// Where a transport may connect: an allow-list, or any public address over `https`, with every address a
/// name resolves to judged.
class DestinationPolicy {
public:
    /// A policy that admits the destinations `entries` name, and no other, resolving names with `resolver`.
    static DestinationPolicy allowList(std::vector<AllowedDestination> entries, std::shared_ptr<Resolver> resolver) {
        return DestinationPolicy(false, std::move(entries), std::move(resolver));
    }

    /// A policy that admits any host on any port over `https`, at an address of the public internet only,
    /// resolving names with `resolver`.
    static DestinationPolicy publicInternet(std::shared_ptr<Resolver> resolver) {
        return DestinationPolicy(true, {}, std::move(resolver));
    }

    /// Judge `scheme://host:port`, resolving `host` once when it is a name.
    ///
    /// A target the scope does not admit is refused before its name is resolved. An address is admitted
    /// when it lies in no special-purpose range, or in a network the matching entry reaches; the others
    /// are dropped, and a name none of whose addresses is admitted is refused.
    Admission admit(std::string_view schemeText, std::string_view hostText, uint16_t port) const {
        Admission answer;
        if (schemeText != "http" && schemeText != "https") {
            return answer;
        }
        const Scheme scheme = schemeText == "http" ? Scheme::Http : Scheme::Https;
        const std::string host = detail::normalizedHost(hostText);
        static const std::vector<IpNetwork> none;
        const std::vector<IpNetwork> *reaches = &none;
        if (publicInternet_) {
            if (scheme != Scheme::Https) {
                return answer;
            }
        } else {
            const auto entry = std::find_if(entries_.begin(), entries_.end(), [&](const AllowedDestination &allowed) {
                return allowed.matches(scheme, host, port);
            });
            if (entry == entries_.end()) {
                return answer;
            }
            reaches = &entry->reaches();
        }
        std::vector<IpAddress> resolved;
        if (const auto literal = IpAddress::parse(host)) {
            resolved.push_back(*literal);
        } else if (resolver_) {
            if (auto addresses = resolver_->resolve(host)) {
                resolved = std::move(*addresses);
            }
        }
        if (resolved.empty()) {
            answer.verdict = Admission::Verdict::Unresolved;
            return answer;
        }
        for (const auto &address : resolved) {
            const auto range = specialPurpose(address);
            const bool reached = std::any_of(reaches->begin(), reaches->end(), [&address](const IpNetwork &network) {
                return network.contains(address);
            });
            if (range && !reached) {
                answer.refused.emplace_back(address, *range);
            } else {
                answer.destination.addresses.push_back(address);
            }
        }
        if (answer.destination.addresses.empty()) {
            answer.verdict = Admission::Verdict::AddressesRefused;
            return answer;
        }
        answer.verdict = Admission::Verdict::Admitted;
        answer.destination.scheme = scheme;
        answer.destination.host = host;
        answer.destination.port = port;
        return answer;
    }

private:
    DestinationPolicy(bool publicInternet, std::vector<AllowedDestination> entries, std::shared_ptr<Resolver> resolver)
        : publicInternet_(publicInternet), entries_(std::move(entries)), resolver_(std::move(resolver)) {}

    bool publicInternet_;
    std::vector<AllowedDestination> entries_;
    std::shared_ptr<Resolver> resolver_;
};

}  // namespace SCE::StandardHost
