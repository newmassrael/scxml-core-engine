// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Message signatures of the Mesh binding over HTTP (docs/adr/0109, D9). The
// profile is stated in mesh/transports/HttpsSignature.h; this file is its one
// C++ implementation, which the client (HttpsClient.cpp) calls.

#include "mesh/transports/HttpsSignature.h"

#include "host/Cose.h"
#include "mesh/transports/HttpsClient.h"

#include <algorithm>

namespace SCE::Mesh::Https::Signatures {

namespace {

char lowerAscii(char c) {
    return (c >= 'A' && c <= 'Z') ? static_cast<char>(c - 'A' + 'a') : c;
}

std::string lowered(std::string_view text) {
    std::string out(text);
    std::transform(out.begin(), out.end(), out.begin(), lowerAscii);
    return out;
}

bool isLowerAlpha(char c) {
    return c >= 'a' && c <= 'z';
}

bool isDigit(char c) {
    return c >= '0' && c <= '9';
}

bool isAlpha(char c) {
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z');
}

constexpr char kAlphabet[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// The value of one base64 digit, or -1.
int base64Digit(char c) {
    if (c >= 'A' && c <= 'Z') {
        return c - 'A';
    }
    if (c >= 'a' && c <= 'z') {
        return c - 'a' + 26;
    }
    if (c >= '0' && c <= '9') {
        return c - '0' + 52;
    }
    if (c == '+') {
        return 62;
    }
    if (c == '/') {
        return 63;
    }
    return -1;
}

/// A cursor over a structured field's text.
class Reader {
public:
    explicit Reader(std::string_view text) : text_(text) {}

    bool atEnd() const {
        return at_ >= text_.size();
    }

    char peek() const {
        return atEnd() ? '\0' : text_[at_];
    }

    void advance() {
        ++at_;
    }

    void skipSpaces() {
        while (!atEnd() && text_[at_] == ' ') {
            ++at_;
        }
    }

    void skipWhitespace() {
        while (!atEnd() && (text_[at_] == ' ' || text_[at_] == '\t')) {
            ++at_;
        }
    }

    std::optional<std::string> key() {
        if (atEnd() || !(isLowerAlpha(peek()) || peek() == '*')) {
            return std::nullopt;
        }
        const std::size_t start = at_;
        while (!atEnd()) {
            const char c = peek();
            if (isLowerAlpha(c) || isDigit(c) || c == '_' || c == '-' || c == '.' || c == '*') {
                ++at_;
            } else {
                break;
            }
        }
        return std::string(text_.substr(start, at_ - start));
    }

    std::optional<BareItem> bareItem() {
        if (atEnd()) {
            return std::nullopt;
        }
        const char c = peek();
        if (c == '-' || isDigit(c)) {
            return integer();
        }
        if (c == '"') {
            return string();
        }
        if (c == ':') {
            return byteSequence();
        }
        if (c == '?') {
            return boolean();
        }
        if (isAlpha(c) || c == '*') {
            return token();
        }
        return std::nullopt;
    }

    std::optional<Parameters> parameters() {
        Parameters out;
        while (!atEnd() && peek() == ';') {
            ++at_;
            skipSpaces();
            auto name = key();
            if (!name) {
                return std::nullopt;
            }
            BareItem value;
            if (!atEnd() && peek() == '=') {
                ++at_;
                auto read = bareItem();
                if (!read) {
                    return std::nullopt;
                }
                value = std::move(*read);
            }
            auto held = std::find_if(out.begin(), out.end(), [&](const auto &entry) { return entry.first == *name; });
            if (held != out.end()) {
                held->second = std::move(value);
            } else {
                out.emplace_back(std::move(*name), std::move(value));
            }
        }
        return out;
    }

    std::optional<Item> item() {
        auto bare = bareItem();
        if (!bare) {
            return std::nullopt;
        }
        auto params = parameters();
        if (!params) {
            return std::nullopt;
        }
        return Item{std::move(*bare), std::move(*params)};
    }

    std::optional<Member> innerList() {
        ++at_;
        Member member;
        member.innerList = true;
        for (;;) {
            skipSpaces();
            if (atEnd()) {
                return std::nullopt;
            }
            if (peek() == ')') {
                ++at_;
                auto params = parameters();
                if (!params) {
                    return std::nullopt;
                }
                member.parameters = std::move(*params);
                return member;
            }
            auto read = item();
            if (!read) {
                return std::nullopt;
            }
            member.items.push_back(std::move(*read));
            if (atEnd() || (peek() != ' ' && peek() != ')')) {
                return std::nullopt;
            }
        }
    }

private:
    std::optional<BareItem> integer() {
        const bool negative = peek() == '-';
        if (negative) {
            ++at_;
        }
        const std::size_t start = at_;
        while (!atEnd() && isDigit(peek())) {
            ++at_;
        }
        const std::size_t digits = at_ - start;
        // RFC 8941 section 3.3.1: at most fifteen digits; a `.` would make it a decimal.
        if (digits == 0 || digits > 15 || (!atEnd() && peek() == '.')) {
            return std::nullopt;
        }
        std::int64_t magnitude = 0;
        for (std::size_t i = start; i < at_; ++i) {
            magnitude = magnitude * 10 + (text_[i] - '0');
        }
        return BareItem::ofInteger(negative ? -magnitude : magnitude);
    }

    std::optional<BareItem> string() {
        ++at_;
        std::string out;
        while (!atEnd()) {
            const char c = text_[at_++];
            if (c == '\\') {
                if (atEnd()) {
                    return std::nullopt;
                }
                const char escaped = text_[at_++];
                if (escaped != '"' && escaped != '\\') {
                    return std::nullopt;
                }
                out += escaped;
            } else if (c == '"') {
                return BareItem::ofString(std::move(out));
            } else if (c >= 0x20 && c <= 0x7e) {
                out += c;
            } else {
                return std::nullopt;
            }
        }
        return std::nullopt;
    }

    std::optional<BareItem> token() {
        const std::size_t start = at_;
        ++at_;
        static constexpr std::string_view kTokenPunctuation = "!#$%&'*+-.^_`|~:/";
        while (!atEnd()) {
            const char c = peek();
            if (isAlpha(c) || isDigit(c) || kTokenPunctuation.find(c) != std::string_view::npos) {
                ++at_;
            } else {
                break;
            }
        }
        BareItem item;
        item.kind = BareItem::Kind::Token;
        item.text = std::string(text_.substr(start, at_ - start));
        return item;
    }

    std::optional<BareItem> byteSequence() {
        ++at_;
        const std::size_t start = at_;
        while (!atEnd() && peek() != ':') {
            const char c = peek();
            if (base64Digit(c) < 0 && c != '=') {
                return std::nullopt;
            }
            ++at_;
        }
        if (atEnd()) {
            return std::nullopt;
        }
        const std::string_view encoded = text_.substr(start, at_ - start);
        ++at_;
        auto decoded = fromBase64(encoded);
        if (!decoded) {
            return std::nullopt;
        }
        return BareItem::ofBytes(std::move(*decoded));
    }

    std::optional<BareItem> boolean() {
        ++at_;
        if (atEnd()) {
            return std::nullopt;
        }
        const char c = text_[at_++];
        if (c != '0' && c != '1') {
            return std::nullopt;
        }
        BareItem item;
        item.kind = BareItem::Kind::Boolean;
        item.boolean = c == '1';
        return item;
    }

    std::string_view text_;
    std::size_t at_ = 0;
};

void serializeBareItem(const BareItem &item, std::string &out) {
    switch (item.kind) {
    case BareItem::Kind::Integer:
        out += std::to_string(item.integer);
        return;
    case BareItem::Kind::String:
        out += '"';
        for (const char c : item.text) {
            if (c == '"' || c == '\\') {
                out += '\\';
            }
            out += c;
        }
        out += '"';
        return;
    case BareItem::Kind::Token:
        out += item.text;
        return;
    case BareItem::Kind::ByteSequence:
        out += ':';
        out += base64(item.bytes);
        out += ':';
        return;
    case BareItem::Kind::Boolean:
        out += item.boolean ? "?1" : "?0";
        return;
    }
}

void serializeParameters(const Parameters &parameters, std::string &out) {
    for (const auto &[name, value] : parameters) {
        out += ';';
        out += name;
        if (!(value.kind == BareItem::Kind::Boolean && value.boolean)) {
            out += '=';
            serializeBareItem(value, out);
        }
    }
}

/// The components a response signature covers, given whether the response
/// carries Sce-Mesh-First.
std::vector<Component> responseComponents(bool hasFirst) {
    std::vector<Component> covered = {
        {"@status", false}, {lowered(kContentDigestHeader), false}, {lowered(kEpochHeader), false}};
    if (hasFirst) {
        covered.push_back({lowered(kFirstHeader), false});
    }
    covered.push_back({"@method", true});
    covered.push_back({"@target-uri", true});
    covered.push_back({lowered(kAckHeader), true});
    covered.push_back({lowered(kNonceHeader), true});
    return covered;
}

/// The components a request signature covers, given whether the request
/// carries Sce-Mesh-Epoch.
std::vector<Component> requestComponents(bool hasEpoch) {
    std::vector<Component> covered = {{"@method", false},
                                      {"@target-uri", false},
                                      {lowered(kContentDigestHeader), false},
                                      {lowered(kAckHeader), false}};
    if (hasEpoch) {
        covered.push_back({lowered(kEpochHeader), false});
    }
    covered.push_back({lowered(kHoldHeader), false});
    covered.push_back({lowered(kNonceHeader), false});
    return covered;
}

/// The parameter `name` of `parameters`, if it is there.
const BareItem *parameter(const Parameters &parameters, std::string_view name) {
    for (const auto &[key, value] : parameters) {
        if (key == name) {
            return &value;
        }
    }
    return nullptr;
}

/// Whether `parameters` are exactly `names`, in any order, with `alg` and `tag`
/// the profile's.
bool parametersAre(const Parameters &parameters, const std::vector<std::string_view> &names) {
    if (parameters.size() != names.size()) {
        return false;
    }
    for (const auto name : names) {
        if (parameter(parameters, name) == nullptr) {
            return false;
        }
    }
    const BareItem *alg = parameter(parameters, "alg");
    const BareItem *tag = parameter(parameters, "tag");
    return alg != nullptr && *alg == BareItem::ofString(kAlgorithm) && tag != nullptr &&
           *tag == BareItem::ofString(kTag);
}

}  // namespace

// ── Structured field values ──

BareItem BareItem::ofInteger(std::int64_t value) {
    BareItem item;
    item.kind = Kind::Integer;
    item.integer = value;
    return item;
}

BareItem BareItem::ofString(std::string value) {
    BareItem item;
    item.kind = Kind::String;
    item.text = std::move(value);
    return item;
}

BareItem BareItem::ofBytes(Bytes value) {
    BareItem item;
    item.kind = Kind::ByteSequence;
    item.bytes = std::move(value);
    return item;
}

bool BareItem::operator==(const BareItem &other) const {
    if (kind != other.kind) {
        return false;
    }
    switch (kind) {
    case Kind::Integer:
        return integer == other.integer;
    case Kind::String:
    case Kind::Token:
        return text == other.text;
    case Kind::ByteSequence:
        return bytes == other.bytes;
    case Kind::Boolean:
        return boolean == other.boolean;
    }
    return false;
}

std::optional<Dictionary> parseDictionary(std::string_view text) {
    Reader reader(text);
    reader.skipSpaces();
    Dictionary members;
    while (!reader.atEnd()) {
        auto name = reader.key();
        if (!name) {
            return std::nullopt;
        }
        Member member;
        if (!reader.atEnd() && reader.peek() == '=') {
            reader.advance();
            if (!reader.atEnd() && reader.peek() == '(') {
                auto list = reader.innerList();
                if (!list) {
                    return std::nullopt;
                }
                member = std::move(*list);
            } else {
                auto read = reader.item();
                if (!read) {
                    return std::nullopt;
                }
                member.item = std::move(*read);
            }
        } else {
            auto params = reader.parameters();
            if (!params) {
                return std::nullopt;
            }
            member.item = Item{BareItem{}, std::move(*params)};
        }
        auto held =
            std::find_if(members.begin(), members.end(), [&](const auto &entry) { return entry.first == *name; });
        if (held != members.end()) {
            held->second = std::move(member);
        } else {
            members.emplace_back(std::move(*name), std::move(member));
        }
        reader.skipWhitespace();
        if (reader.atEnd()) {
            break;
        }
        if (reader.peek() != ',') {
            return std::nullopt;
        }
        reader.advance();
        reader.skipWhitespace();
        // A trailing comma ends no member.
        if (reader.atEnd()) {
            return std::nullopt;
        }
    }
    return members;
}

std::string serializeInnerList(const std::vector<Item> &items, const Parameters &parameters) {
    std::string out = "(";
    for (std::size_t i = 0; i < items.size(); ++i) {
        if (i > 0) {
            out += ' ';
        }
        serializeBareItem(items[i].bare, out);
        serializeParameters(items[i].parameters, out);
    }
    out += ')';
    serializeParameters(parameters, out);
    return out;
}

// ── The signature base ──

Item identifier(const Component &component) {
    Item item{BareItem::ofString(component.name), {}};
    if (component.req) {
        item.parameters.emplace_back("req", BareItem{});
    }
    return item;
}

std::string signatureBase(const std::vector<std::pair<Component, std::string>> &covered,
                          const std::string &signatureParams) {
    std::string base;
    for (const auto &[component, value] : covered) {
        const Item id = identifier(component);
        serializeBareItem(id.bare, base);
        serializeParameters(id.parameters, base);
        base += ": ";
        base += value;
        base += '\n';
    }
    base += "\"@signature-params\": ";
    base += signatureParams;
    return base;
}

// ── What the profile reads and writes ──

std::string base64(const Bytes &bytes) {
    std::string out;
    out.reserve((bytes.size() + 2) / 3 * 4);
    std::size_t i = 0;
    for (; i + 3 <= bytes.size(); i += 3) {
        const std::uint32_t triple = (static_cast<std::uint32_t>(bytes[i]) << 16) |
                                     (static_cast<std::uint32_t>(bytes[i + 1]) << 8) | bytes[i + 2];
        out += kAlphabet[(triple >> 18) & 0x3f];
        out += kAlphabet[(triple >> 12) & 0x3f];
        out += kAlphabet[(triple >> 6) & 0x3f];
        out += kAlphabet[triple & 0x3f];
    }
    const std::size_t rest = bytes.size() - i;
    if (rest == 1) {
        const std::uint32_t one = static_cast<std::uint32_t>(bytes[i]) << 16;
        out += kAlphabet[(one >> 18) & 0x3f];
        out += kAlphabet[(one >> 12) & 0x3f];
        out += "==";
    } else if (rest == 2) {
        const std::uint32_t two =
            (static_cast<std::uint32_t>(bytes[i]) << 16) | (static_cast<std::uint32_t>(bytes[i + 1]) << 8);
        out += kAlphabet[(two >> 18) & 0x3f];
        out += kAlphabet[(two >> 12) & 0x3f];
        out += kAlphabet[(two >> 6) & 0x3f];
        out += '=';
    }
    return out;
}

std::optional<Bytes> fromBase64(std::string_view text) {
    // RFC 8941 section 4.2.7: a parser does not fail on absent `=` padding.
    while (!text.empty() && text.back() == '=') {
        text.remove_suffix(1);
    }
    if (text.size() % 4 == 1) {
        return std::nullopt;
    }
    Bytes out;
    out.reserve(text.size() * 3 / 4);
    std::uint32_t buffer = 0;
    int bits = 0;
    for (const char c : text) {
        const int digit = base64Digit(c);
        if (digit < 0) {
            return std::nullopt;
        }
        buffer = (buffer << 6) | static_cast<std::uint32_t>(digit);
        bits += 6;
        if (bits >= 8) {
            bits -= 8;
            out.push_back(static_cast<std::uint8_t>((buffer >> bits) & 0xff));
        }
    }
    // The bits left over are padding, which a canonical encoding leaves zero.
    if (bits > 0 && (buffer & ((1u << bits) - 1)) != 0) {
        return std::nullopt;
    }
    return out;
}

std::optional<std::string> fieldValue(const std::vector<HttpsHeader> &headers, std::string_view name) {
    std::optional<std::string> value;
    const std::string wanted = lowered(name);
    for (const auto &[key, line] : headers) {
        if (lowered(key) != wanted) {
            continue;
        }
        std::string_view trimmed = line;
        while (!trimmed.empty() && (trimmed.front() == ' ' || trimmed.front() == '\t')) {
            trimmed.remove_prefix(1);
        }
        while (!trimmed.empty() && (trimmed.back() == ' ' || trimmed.back() == '\t')) {
            trimmed.remove_suffix(1);
        }
        if (value) {
            *value += ", ";
            *value += trimmed;
        } else {
            value = std::string(trimmed);
        }
    }
    return value;
}

std::optional<std::string> normalizeTargetUri(std::string_view url) {
    const std::size_t schemeEnd = url.find("://");
    if (schemeEnd == std::string_view::npos) {
        return std::nullopt;
    }
    const std::string scheme = lowered(url.substr(0, schemeEnd));
    std::string_view defaultPort;
    if (scheme == "https") {
        defaultPort = "443";
    } else if (scheme == "http") {
        defaultPort = "80";
    } else {
        return std::nullopt;
    }
    std::string_view rest = url.substr(schemeEnd + 3);
    const std::size_t split = std::min(rest.find('/'), rest.find('?'));
    std::string_view authority = rest.substr(0, std::min(split, rest.size()));
    const std::string_view path = split == std::string_view::npos ? std::string_view() : rest.substr(split);
    if (const std::size_t at = authority.rfind('@'); at != std::string_view::npos) {
        authority.remove_prefix(at + 1);
    }
    std::string host;
    std::optional<std::string_view> port;
    if (!authority.empty() && authority.front() == '[') {
        const std::size_t close = authority.find(']');
        if (close == std::string_view::npos) {
            return std::nullopt;
        }
        host = "[" + lowered(authority.substr(1, close - 1)) + "]";
        const std::string_view after = authority.substr(close + 1);
        if (!after.empty()) {
            if (after.front() != ':') {
                return std::nullopt;
            }
            port = after.substr(1);
        }
    } else if (const std::size_t colon = authority.rfind(':'); colon != std::string_view::npos) {
        host = lowered(authority.substr(0, colon));
        port = authority.substr(colon + 1);
    } else {
        host = lowered(authority);
    }
    if (host.empty() || (port && !std::all_of(port->begin(), port->end(), isDigit))) {
        return std::nullopt;
    }
    std::string out = scheme + "://" + host;
    if (port && !port->empty() && *port != defaultPort) {
        out += ':';
        out += *port;
    }
    out += path.empty() ? std::string_view("/") : path;
    return out;
}

std::optional<std::string> contentDigest(StandardHost::CoseCrypto &crypto, const Bytes &body) {
    auto digest = crypto.sha256(body);
    if (!digest) {
        return std::nullopt;
    }
    return "sha-256=:" + base64(*digest) + ":";
}

bool digestMatches(StandardHost::CoseCrypto &crypto, std::string_view field, const Bytes &body) {
    const auto members = parseDictionary(field);
    const auto digest = crypto.sha256(body);
    if (!members || !digest) {
        return false;
    }
    for (const auto &[name, member] : *members) {
        if (name == "sha-256" && !member.innerList && member.item.bare.kind == BareItem::Kind::ByteSequence &&
            member.item.bare.bytes == *digest) {
            return true;
        }
    }
    return false;
}

const char *refusalText(Refusal refusal) {
    switch (refusal) {
    case Refusal::None:
        return "the message verified";
    case Refusal::Digest:
        return "the Content-Digest is absent or not the body's";
    case Refusal::Unsigned:
        return "no signature of the binding's profile";
    case Refusal::Profile:
        return "a signature that covers other than the profile names";
    case Refusal::UnknownKey:
        return "a signature by no key the verifier holds";
    case Refusal::Signature:
        return "a signature that does not verify";
    }
    return "a signature that does not verify";
}

Refusal verifyResponse(StandardHost::CoseCrypto &crypto, const std::vector<TrustedKey> &trusted, std::uint16_t status,
                       const std::vector<HttpsHeader> &headers, const Bytes &body, const Answered &answered) {
    // SCE_MESH.md §mesh-22.9: the list a response's signature covers is the
    // profile's, built here from what the response carries and never read
    // from it; a signature of any other list is refused.
    if (status != 200) {
        return Refusal::Profile;
    }
    const auto digest = fieldValue(headers, kContentDigestHeader);
    if (!digest || !digestMatches(crypto, *digest, body)) {
        return Refusal::Digest;
    }
    const auto inputText = fieldValue(headers, kSignatureInputHeader);
    const auto signatureText = fieldValue(headers, kSignatureHeader);
    const auto inputs = inputText ? parseDictionary(*inputText) : std::nullopt;
    const auto signatures = signatureText ? parseDictionary(*signatureText) : std::nullopt;
    if (!inputs || !signatures) {
        return Refusal::Unsigned;
    }
    const auto input = std::find_if(inputs->begin(), inputs->end(), [](const auto &m) { return m.first == kLabel; });
    const auto signature =
        std::find_if(signatures->begin(), signatures->end(), [](const auto &m) { return m.first == kLabel; });
    if (input == inputs->end() || !input->second.innerList || signature == signatures->end() ||
        signature->second.innerList || signature->second.item.bare.kind != BareItem::Kind::ByteSequence) {
        return Refusal::Unsigned;
    }
    const Member &carried = input->second;
    const std::vector<Component> expected = responseComponents(fieldValue(headers, kFirstHeader).has_value());
    if (carried.items.size() != expected.size() || !parametersAre(carried.parameters, {"keyid", "alg", "tag"})) {
        return Refusal::Profile;
    }
    for (std::size_t i = 0; i < expected.size(); ++i) {
        if (!(carried.items[i] == identifier(expected[i]))) {
            return Refusal::Profile;
        }
    }
    const BareItem *keyId = parameter(carried.parameters, "keyid");
    if (keyId == nullptr || keyId->kind != BareItem::Kind::String) {
        return Refusal::Profile;
    }
    const auto signer =
        std::find_if(trusted.begin(), trusted.end(), [&](const TrustedKey &key) { return key.id == keyId->text; });
    if (signer == trusted.end()) {
        return Refusal::UnknownKey;
    }
    std::vector<std::pair<Component, std::string>> covered;
    for (const auto &component : expected) {
        std::optional<std::string> value;
        if (component.name == "@status") {
            value = std::to_string(status);
        } else if (component.name == "@method") {
            value = "POST";
        } else if (component.name == "@target-uri") {
            value = answered.targetUri;
        } else if (component.req && component.name == lowered(kAckHeader)) {
            value = answered.ack;
        } else if (component.req && component.name == lowered(kNonceHeader)) {
            value = answered.nonce;
        } else {
            value = fieldValue(headers, component.name);
        }
        if (!value) {
            return Refusal::Profile;
        }
        covered.emplace_back(component, std::move(*value));
    }
    const std::string base = signatureBase(covered, serializeInnerList(carried.items, carried.parameters));
    if (!crypto.ed25519Verify(signer->publicKey, Bytes(base.begin(), base.end()), signature->second.item.bare.bytes)) {
        return Refusal::Signature;
    }
    return Refusal::None;
}

// ── The request signer ──

std::optional<RequestSigner> RequestSigner::make(StandardHost::CoseCrypto &crypto, Bytes seed) {
    if (seed.size() != 32) {
        crypto.wipe(seed);
        return std::nullopt;
    }
    auto publicKey = crypto.ed25519PublicKey(seed);
    if (!publicKey) {
        crypto.wipe(seed);
        return std::nullopt;
    }
    // RFC 9679: the thumbprint is the SHA-256 of the COSE_Key of the members
    // the key type requires, which is also what offers the key.
    const Bytes coseKey = StandardHost::Cose::coseKeyEd25519Required(*publicKey);
    auto thumbprint = crypto.sha256(coseKey);
    auto keyId = thumbprint ? crypto.base64Url(*thumbprint) : std::nullopt;
    auto offer = crypto.base64Url(coseKey);
    if (!keyId || !offer) {
        crypto.wipe(seed);
        return std::nullopt;
    }
    return RequestSigner(crypto, std::move(seed), std::move(*publicKey), std::move(*keyId), std::move(*offer));
}

RequestSigner::RequestSigner(StandardHost::CoseCrypto &crypto, Bytes seed, Bytes publicKey, std::string keyId,
                             std::string offer)
    : crypto_(&crypto), seed_(std::move(seed)), publicKey_(std::move(publicKey)), keyId_(std::move(keyId)),
      offer_(std::move(offer)) {}

RequestSigner::RequestSigner(RequestSigner &&other) noexcept
    : crypto_(other.crypto_), seed_(std::move(other.seed_)), publicKey_(std::move(other.publicKey_)),
      keyId_(std::move(other.keyId_)), offer_(std::move(other.offer_)) {
    other.seed_.clear();
}

RequestSigner &RequestSigner::operator=(RequestSigner &&other) noexcept {
    if (this != &other) {
        crypto_->wipe(seed_);
        crypto_ = other.crypto_;
        seed_ = std::move(other.seed_);
        other.seed_.clear();
        publicKey_ = std::move(other.publicKey_);
        keyId_ = std::move(other.keyId_);
        offer_ = std::move(other.offer_);
    }
    return *this;
}

RequestSigner::~RequestSigner() {
    if (!seed_.empty()) {
        crypto_->wipe(seed_);
    }
}

std::optional<std::vector<HttpsHeader>>
RequestSigner::sign(const std::string &targetUri, const std::vector<HttpsHeader> &headers, std::int64_t created) const {
    // SCE_MESH.md §mesh-22.9: the `key` authenticator's request signature,
    // over the request's method, target, body digest, numbering, hold and
    // nonce, made at `created`.
    const std::vector<Component> expected = requestComponents(fieldValue(headers, kEpochHeader).has_value());
    std::vector<std::pair<Component, std::string>> covered;
    std::vector<Item> items;
    for (const auto &component : expected) {
        std::optional<std::string> value;
        if (component.name == "@method") {
            value = "POST";
        } else if (component.name == "@target-uri") {
            value = targetUri;
        } else {
            value = fieldValue(headers, component.name);
        }
        if (!value) {
            return std::nullopt;
        }
        items.push_back(identifier(component));
        covered.emplace_back(component, std::move(*value));
    }
    const Parameters parameters = {{"created", BareItem::ofInteger(created)},
                                   {"keyid", BareItem::ofString(keyId_)},
                                   {"alg", BareItem::ofString(kAlgorithm)},
                                   {"tag", BareItem::ofString(kTag)}};
    const std::string params = serializeInnerList(items, parameters);
    const std::string base = signatureBase(covered, params);
    auto signature = crypto_->ed25519Sign(seed_, Bytes(base.begin(), base.end()));
    if (!signature) {
        return std::nullopt;
    }
    std::string value = std::string(kLabel) + "=";
    serializeBareItem(BareItem::ofBytes(std::move(*signature)), value);
    return std::vector<HttpsHeader>{{kKeyHeader, offer_},
                                    {kSignatureInputHeader, std::string(kLabel) + "=" + params},
                                    {kSignatureHeader, std::move(value)}};
}

}  // namespace SCE::Mesh::Https::Signatures
