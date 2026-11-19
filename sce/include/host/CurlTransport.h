// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

/**
 * @file CurlTransport.h
 * @brief The network transport of the standard host features' HTTP operation (docs/SCE_HOST_FEATURES.md,
 *        section 5) for a server, on the system's libcurl, judged first by a DestinationPolicy
 *        (docs/adr/0109, D4).
 *
 * - Where it connects. The policy admits the target's scheme, host and port, resolves the name once, and
 *   answers the addresses it admitted. The transport pins the name to those addresses (CURLOPT_RESOLVE),
 *   so libcurl does not resolve it again, and refuses at the socket any address it was not given
 *   (CURLOPT_OPENSOCKETFUNCTION). The URL is read once, by libcurl's own parser, and the host the policy
 *   judges is the one libcurl connects to; `Host` and the TLS server name stay the URL's.
 * - What it follows. Nothing: a `3xx` is answered as it came.
 * - Through a proxy. None is read from the environment (the proxy variables included). A proxy the
 *   deployment configures (CurlTransportSettings::proxy) is reached as itself: its own address is not
 *   judged, the target still is, every exchange is tunnelled with `CONNECT` (CURLOPT_HTTPPROXYTUNNEL),
 *   for `http` as for `https`, to the address the policy admitted (CURLOPT_CONNECT_TO), so the proxy
 *   never resolves the name, and the socket guard admits the proxy's addresses alone. The proxy's
 *   credentials travel in the `CONNECT` alone (CURLOPT_PROXYHEADER) and never reach the target.
 * - How long it waits. The connection and its TLS handshake, the response's head from the request's
 *   sending on, and the whole transfer each have a bound (CurlTimeouts); one that passes fails `network`.
 *   libcurl tells the head's wait about once a second, so that bound is kept to about a second.
 * - How much it reads. A body past maxResponseBytes or a head past maxHeaderBytes fails
 *   `response_too_large`, and nothing of it is kept.
 * - What it writes. The request's own header lines, and `host` and `content-length`, which it writes
 *   itself; none of libcurl's defaults (`accept`, a form's `content-type`, `expect`). A request that
 *   names a field that frames or routes the message (CURL_TRANSPORT_FIELDS) is not sent and fails
 *   `network`.
 *
 * The response's fields are given in the order received. libcurl's global state is set up on the first
 * exchange (curl_global_init, which on Windows also starts Winsock for the system resolver below).
 *
 * The target `sce_host_http_curl` (cmake/SCEHostHttp.cmake) links the libcurl found; this header is
 * compiled only where it is.
 */

#include <curl/curl.h>

#if defined(_WIN32)
#include <winsock2.h>
#include <ws2tcpip.h>
#else
#include <netdb.h>
#include <netinet/in.h>
#include <sys/socket.h>
#include <sys/types.h>
#endif

#include <algorithm>
#include <array>
#include <chrono>
#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <memory>
#include <optional>
#include <string>
#include <utility>
#include <vector>

#include "common/StringUtils.h"
#include "host/DestinationPolicy.h"
#include "host/StandardHost.h"

namespace SCE::StandardHost {

/// The fields that frame or route a message, which the transport writes or leaves out itself and a request
/// may not name (compared in lower case).
inline constexpr std::array<const char *, 7> CURL_TRANSPORT_FIELDS = {
    "host", "content-length", "transfer-encoding", "connection", "upgrade", "expect", "te",
};

namespace detail {

/// The address a socket address holds, or nothing for another family.
inline std::optional<IpAddress> addressOf(const struct sockaddr *raw) {
    if (raw->sa_family == AF_INET) {
        struct sockaddr_in in4;
        std::memcpy(&in4, raw, sizeof in4);
        std::array<uint8_t, 4> octets{};
        std::memcpy(octets.data(), &in4.sin_addr, 4);
        return IpAddress::fromV4(octets);
    }
    if (raw->sa_family == AF_INET6) {
        struct sockaddr_in6 in6;
        std::memcpy(&in6, raw, sizeof in6);
        std::array<uint8_t, 16> octets{};
        std::memcpy(octets.data(), &in6.sin6_addr, 16);
        return IpAddress::fromV6(octets);
    }
    return std::nullopt;
}

}  // namespace detail

/// The operating system's resolver (getaddrinfo). On Windows it needs Winsock started, which the first
/// CurlTransport exchange does.
class SystemResolver : public Resolver {
public:
    std::optional<std::vector<IpAddress>> resolve(const std::string &host) override {
        struct addrinfo hints;
        std::memset(&hints, 0, sizeof hints);
        hints.ai_family = AF_UNSPEC;
        hints.ai_socktype = SOCK_STREAM;
        struct addrinfo *found = nullptr;
        if (getaddrinfo(host.c_str(), nullptr, &hints, &found) != 0) {
            return std::nullopt;
        }
        std::vector<IpAddress> addresses;
        for (const struct addrinfo *entry = found; entry != nullptr; entry = entry->ai_next) {
            if (const auto address = detail::addressOf(entry->ai_addr)) {
                addresses.push_back(*address);
            }
        }
        freeaddrinfo(found);
        return addresses;
    }
};

/// How long each part of an exchange may take.
struct CurlTimeouts {
    /// Opening a connection to one address, with its TLS handshake for `https` (libcurl's connect phase
    /// holds both).
    std::chrono::milliseconds connect{10000};
    /// The TLS handshake: added to `connect`, since libcurl bounds the two together.
    std::chrono::milliseconds tls{10000};
    /// From the request being sent to the response's head being read, kept to within about a second.
    std::chrono::milliseconds header{30000};
    /// The whole transfer from its first connection to the end of the body. The policy resolves a name
    /// before it, on the resolver's own bound.
    std::chrono::milliseconds total{60000};
};

/// An HTTP proxy every exchange is tunnelled through with `CONNECT` (docs/adr/0109, D4). It is the
/// deployment's, reached as itself: the policy judges the target and not the proxy.
struct CurlProxy {
    /// The proxy's host: a name, which the system's resolver answers, or an address literal without brackets.
    std::string host;
    uint16_t port = 0;
    /// The value of `Proxy-Authorization` each `CONNECT` carries (`Basic dXNlcjpwYXNz`), sent to the proxy and
    /// to nothing else; empty for none.
    std::string authorization;
};

/// What a transport is built with besides its policy.
struct CurlTransportSettings {
    CurlTimeouts timeouts;
    /// The most bytes of a response's body read; the default is the bound of the result record's `body`.
    size_t maxResponseBytes = 16384;
    /// The most bytes of a response's head read, the status line and every field line counted.
    size_t maxHeaderBytes = 65536;
    /// A file of authorities believed in place of the CA bundle libcurl was built with; empty for that.
    std::string caBundle;
    /// The proxy every exchange is tunnelled through; nothing to connect directly.
    std::optional<CurlProxy> proxy;
};

namespace detail {

/// libcurl's global state, set up once per process before the first handle.
inline bool curlReady() {
    static const bool ready = curl_global_init(CURL_GLOBAL_DEFAULT) == CURLE_OK;
    return ready;
}

struct CurlEasyDeleter {
    void operator()(CURL *easy) const {
        curl_easy_cleanup(easy);
    }
};

struct CurlUrlDeleter {
    void operator()(CURLU *url) const {
        curl_url_cleanup(url);
    }
};

struct CurlSlistDeleter {
    void operator()(curl_slist *list) const {
        curl_slist_free_all(list);
    }
};

struct CurlTextDeleter {
    void operator()(char *text) const {
        curl_free(text);
    }
};

using CurlText = std::unique_ptr<char, CurlTextDeleter>;
using CurlSlist = std::unique_ptr<curl_slist, CurlSlistDeleter>;

/// A part of a parsed URL, or nothing where it has none.
inline std::optional<std::string> urlPart(CURLU *url, CURLUPart part, unsigned int flags) {
    char *text = nullptr;
    if (curl_url_get(url, part, &text, flags) != CURLUE_OK || text == nullptr) {
        return std::nullopt;
    }
    const CurlText owned(text);
    return std::string(owned.get());
}

/// Append `line` to `list`; false when libcurl could not.
inline bool appendLine(CurlSlist &list, const std::string &line) {
    curl_slist *grown = curl_slist_append(list.get(), line.c_str());
    if (grown == nullptr) {
        return false;
    }
    (void)list.release();
    list.reset(grown);
    return true;
}

/// What one transfer collects, and how it ended.
struct CurlTransfer {
    CURL *easy = nullptr;
    const std::vector<IpAddress> *admitted = nullptr;
    size_t maxBody = 0;
    size_t maxHead = 0;
    std::chrono::milliseconds headerBound{0};
    std::vector<HttpHeader> headers;
    std::string body;
    size_t headBytes = 0;
    long lastStatus = 0;
    bool headed = false;
    std::chrono::steady_clock::time_point startedAt;
    bool refusedAddress = false;
    bool tooLarge = false;
};

inline size_t onCurlBody(char *data, size_t size, size_t count, void *user) {
    auto *transfer = static_cast<CurlTransfer *>(user);
    const size_t bytes = size * count;
    if (transfer->body.size() + bytes > transfer->maxBody) {
        transfer->tooLarge = true;
        return 0;
    }
    transfer->body.append(data, bytes);
    return bytes;
}

/// One line of a response's head. A status line starts a response, so the fields of an interim one (a
/// `100 Continue`) are dropped and only the final response's are kept; the blank line that ends a final
/// response's head marks the head read.
inline size_t onCurlHeader(char *data, size_t size, size_t count, void *user) {
    auto *transfer = static_cast<CurlTransfer *>(user);
    const size_t bytes = size * count;
    transfer->headBytes += bytes;
    if (transfer->headBytes > transfer->maxHead) {
        transfer->tooLarge = true;
        return 0;
    }
    std::string line(data, bytes);
    while (!line.empty() && (line.back() == '\r' || line.back() == '\n')) {
        line.pop_back();
    }
    if (line.rfind("HTTP/", 0) == 0) {
        transfer->headers.clear();
        const size_t space = line.find(' ');
        transfer->lastStatus = space == std::string::npos ? 0 : std::strtol(line.c_str() + space + 1, nullptr, 10);
        return bytes;
    }
    if (line.empty()) {
        if (transfer->lastStatus >= 200) {
            transfer->headed = true;
        }
        return bytes;
    }
    const size_t colon = line.find(':');
    if (colon != std::string::npos) {
        transfer->headers.push_back({line.substr(0, colon), trimBlanks(line.substr(colon + 1))});
    }
    return bytes;
}

/// Ends the transfer when the response's head has not come within its bound of the request being sent.
/// libcurl calls this about once a second while it waits.
inline int onCurlProgress(void *user, curl_off_t, curl_off_t, curl_off_t, curl_off_t) {
    auto *transfer = static_cast<CurlTransfer *>(user);
    if (transfer->headed) {
        return 0;
    }
    // The time from the transfer's start until the request began to be sent, in microseconds; zero until
    // it has.
    curl_off_t pretransfer = 0;
    if (curl_easy_getinfo(transfer->easy, CURLINFO_PRETRANSFER_TIME_T, &pretransfer) != CURLE_OK || pretransfer == 0) {
        return 0;
    }
    const auto sentAt = transfer->startedAt + std::chrono::microseconds(static_cast<long long>(pretransfer));
    return std::chrono::steady_clock::now() - sentAt > transfer->headerBound ? 1 : 0;
}

/// Whether a socket libcurl is about to open reaches an address in `admitted`.
inline bool socketAdmitted(const struct sockaddr *target, const std::vector<IpAddress> &admitted) {
    const auto peer = addressOf(target);
    return peer && std::find(admitted.begin(), admitted.end(), *peer) != admitted.end();
}

/// Opens a socket only to an address the policy admitted; any other is refused before it is reached.
inline curl_socket_t onCurlOpenSocket(void *user, curlsocktype, struct curl_sockaddr *address) {
    auto *transfer = static_cast<CurlTransfer *>(user);
    if (!socketAdmitted(&address->addr, *transfer->admitted)) {
        transfer->refusedAddress = true;
        return CURL_SOCKET_BAD;
    }
    return ::socket(address->family, address->socktype, address->protocol);
}

}  // namespace detail

/// The network transport of section 5 on libcurl, built with the policy every exchange is judged by; there
/// is no transport without one. A proxy named by its host name is answered by the system's resolver
/// (SystemResolver).
class CurlTransport : public Transport {
public:
    explicit CurlTransport(DestinationPolicy policy, CurlTransportSettings settings = {})
        : policy_(std::move(policy)), settings_(std::move(settings)) {}

    Exchange exchange(const HttpRequest &request) override {
        const Exchange failed{Exchange::Kind::Failure, {}};
        if (!detail::curlReady()) {
            return failed;
        }
        for (const auto &header : request.headers) {
            std::string name = header.name;
            ::SCE::lowerAsciiInPlace(name);
            if (std::find_if(CURL_TRANSPORT_FIELDS.begin(), CURL_TRANSPORT_FIELDS.end(),
                             [&name](const char *field) { return name == field; }) != CURL_TRANSPORT_FIELDS.end()) {
                return failed;
            }
        }
        // The URL as libcurl reads it, the same handle it is then carried by: the host judged is the host
        // connected to.
        std::unique_ptr<CURLU, detail::CurlUrlDeleter> url(curl_url());
        if (!url || curl_url_set(url.get(), CURLUPART_URL, request.url.c_str(), 0) != CURLUE_OK) {
            return failed;
        }
        const auto scheme = detail::urlPart(url.get(), CURLUPART_SCHEME, 0);
        const auto host = detail::urlPart(url.get(), CURLUPART_HOST, 0);
        const auto portText = detail::urlPart(url.get(), CURLUPART_PORT, CURLU_DEFAULT_PORT);
        if (!scheme || !host || !portText) {
            return failed;
        }
        const long port = std::strtol(portText->c_str(), nullptr, 10);
        if (port <= 0 || port > 65535) {
            return failed;
        }
        const Admission admission = policy_.admit(*scheme, *host, static_cast<uint16_t>(port));
        if (admission.verdict != Admission::Verdict::Admitted) {
            return {admission.failure(), {}};
        }
        return carry(request, url.get(), *host, admission.destination);
    }

private:
    Exchange carry(const HttpRequest &request, CURLU *url, const std::string &host, const Destination &destination) {
        const Exchange failed{Exchange::Kind::Failure, {}};
        std::unique_ptr<CURL, detail::CurlEasyDeleter> easy(curl_easy_init());
        if (!easy) {
            return failed;
        }
        CURL *handle = easy.get();

        const auto written = [](const IpAddress &address) {
            return address.v6 ? "[" + address.text() + "]" : address.text();
        };
        const auto pinned = [&written](const std::string &name, uint16_t port,
                                       const std::vector<IpAddress> &addresses) {
            std::string entry = name + ":" + std::to_string(port) + ":";
            for (size_t i = 0; i < addresses.size(); ++i) {
                entry += (i == 0 ? "" : ",") + written(addresses[i]);
            }
            return entry;
        };
        const bool literalTarget = IpAddress::parse(detail::normalizedHost(host)).has_value();

        // Where a socket may be opened to: the addresses the policy admitted, or through a proxy the proxy's
        // own. A name is pinned to them (CURLOPT_RESOLVE), so libcurl resolves nothing itself; an address
        // literal is not resolved.
        std::vector<IpAddress> reachable = destination.addresses;
        detail::CurlSlist resolve;
        detail::CurlSlist connectTo;
        detail::CurlSlist proxyHeaders;
        std::string proxyUrl;
        if (!settings_.proxy) {
            if (!literalTarget && !detail::appendLine(resolve, pinned(host, destination.port, reachable))) {
                return failed;
            }
        } else {
            const CurlProxy &proxy = *settings_.proxy;
            const auto proxyLiteral = IpAddress::parse(proxy.host);
            reachable.clear();
            if (proxyLiteral) {
                reachable.push_back(*proxyLiteral);
            } else if (auto found = proxyResolver_->resolve(proxy.host)) {
                reachable = std::move(*found);
            }
            if (reachable.empty()) {
                return failed;
            }
            if (!proxyLiteral && !detail::appendLine(resolve, pinned(proxy.host, proxy.port, reachable))) {
                return failed;
            }
            proxyUrl =
                "http://" + (proxyLiteral ? written(*proxyLiteral) : proxy.host) + ":" + std::to_string(proxy.port);
            // The tunnel is asked for to the address the policy checked, never to the name.
            if (!literalTarget && !detail::appendLine(connectTo, host + ":" + std::to_string(destination.port) + ":" +
                                                                     written(destination.addresses.front()) + ":" +
                                                                     std::to_string(destination.port))) {
                return failed;
            }
            if (!proxy.authorization.empty() &&
                !detail::appendLine(proxyHeaders, "Proxy-Authorization: " + proxy.authorization)) {
                return failed;
            }
        }

        const bool sendsBody =
            request.method == "POST" || request.method == "PUT" || request.method == "PATCH" || !request.body.empty();
        bool wroteAccept = false;
        bool wroteContentType = false;
        detail::CurlSlist headers;
        for (const auto &header : request.headers) {
            std::string name = header.name;
            ::SCE::lowerAsciiInPlace(name);
            wroteAccept = wroteAccept || name == "accept";
            wroteContentType = wroteContentType || name == "content-type";
            // `Name:` with nothing after it is libcurl's spelling of "leave this field out"; an empty value
            // is written `Name;`.
            const std::string line = header.value.empty() ? header.name + ";" : header.name + ": " + header.value;
            if (!detail::appendLine(headers, line)) {
                return failed;
            }
        }
        // None of libcurl's own fields: what the document did not write is not sent.
        if ((!wroteAccept && !detail::appendLine(headers, "Accept:")) || !detail::appendLine(headers, "Expect:") ||
            (sendsBody && !wroteContentType && !detail::appendLine(headers, "Content-Type:"))) {
            return failed;
        }

        detail::CurlTransfer transfer;
        transfer.easy = handle;
        transfer.admitted = &reachable;
        transfer.maxBody = settings_.maxResponseBytes;
        transfer.maxHead = settings_.maxHeaderBytes;
        transfer.headerBound = settings_.timeouts.header;

        curl_easy_setopt(handle, CURLOPT_CURLU, url);
        if (request.method == "HEAD") {
            curl_easy_setopt(handle, CURLOPT_NOBODY, 1L);
        } else if (request.method == "GET") {
            curl_easy_setopt(handle, CURLOPT_HTTPGET, 1L);
        } else {
            if (sendsBody) {
                curl_easy_setopt(handle, CURLOPT_POSTFIELDSIZE_LARGE, static_cast<curl_off_t>(request.body.size()));
                curl_easy_setopt(handle, CURLOPT_POSTFIELDS, request.body.c_str());
            }
            if (request.method != "POST") {
                curl_easy_setopt(handle, CURLOPT_CUSTOMREQUEST, request.method.c_str());
            }
        }
        curl_easy_setopt(handle, CURLOPT_HTTPHEADER, headers.get());
        if (resolve) {
            curl_easy_setopt(handle, CURLOPT_RESOLVE, resolve.get());
        }
        curl_easy_setopt(handle, CURLOPT_OPENSOCKETFUNCTION, &detail::onCurlOpenSocket);
        curl_easy_setopt(handle, CURLOPT_OPENSOCKETDATA, static_cast<void *>(&transfer));
        // The configured proxy, or an empty one, which is no proxy whatever the environment says. The
        // request's own fields go to the target alone and the proxy's to the proxy alone.
        curl_easy_setopt(handle, CURLOPT_PROXY, proxyUrl.c_str());
        curl_easy_setopt(handle, CURLOPT_HEADEROPT, static_cast<long>(CURLHEADER_SEPARATE));
        if (settings_.proxy) {
            curl_easy_setopt(handle, CURLOPT_HTTPPROXYTUNNEL, 1L);
            if (connectTo) {
                curl_easy_setopt(handle, CURLOPT_CONNECT_TO, connectTo.get());
            }
            if (proxyHeaders) {
                curl_easy_setopt(handle, CURLOPT_PROXYHEADER, proxyHeaders.get());
            }
        }
        curl_easy_setopt(handle, CURLOPT_FOLLOWLOCATION, 0L);
#if LIBCURL_VERSION_NUM >= 0x075500
        curl_easy_setopt(handle, CURLOPT_PROTOCOLS_STR, "http,https");
#else
        curl_easy_setopt(handle, CURLOPT_PROTOCOLS, static_cast<long>(CURLPROTO_HTTP | CURLPROTO_HTTPS));
#endif
        curl_easy_setopt(handle, CURLOPT_HTTP_VERSION, static_cast<long>(CURL_HTTP_VERSION_1_1));
        curl_easy_setopt(handle, CURLOPT_NOSIGNAL, 1L);
        curl_easy_setopt(handle, CURLOPT_CONNECTTIMEOUT_MS,
                         static_cast<long>((settings_.timeouts.connect + settings_.timeouts.tls).count()));
        curl_easy_setopt(handle, CURLOPT_TIMEOUT_MS, static_cast<long>(settings_.timeouts.total.count()));
        curl_easy_setopt(handle, CURLOPT_MAXFILESIZE_LARGE, static_cast<curl_off_t>(settings_.maxResponseBytes));
        if (!settings_.caBundle.empty()) {
            curl_easy_setopt(handle, CURLOPT_CAINFO, settings_.caBundle.c_str());
        }
        curl_easy_setopt(handle, CURLOPT_WRITEFUNCTION, &detail::onCurlBody);
        curl_easy_setopt(handle, CURLOPT_WRITEDATA, static_cast<void *>(&transfer));
        curl_easy_setopt(handle, CURLOPT_HEADERFUNCTION, &detail::onCurlHeader);
        curl_easy_setopt(handle, CURLOPT_HEADERDATA, static_cast<void *>(&transfer));
        curl_easy_setopt(handle, CURLOPT_NOPROGRESS, 0L);
        curl_easy_setopt(handle, CURLOPT_XFERINFOFUNCTION, &detail::onCurlProgress);
        curl_easy_setopt(handle, CURLOPT_XFERINFODATA, static_cast<void *>(&transfer));

        transfer.startedAt = std::chrono::steady_clock::now();
        const CURLcode code = curl_easy_perform(handle);
        if (transfer.refusedAddress) {
            return {Exchange::Kind::DestinationRefused, {}};
        }
        if (transfer.tooLarge || code == CURLE_FILESIZE_EXCEEDED) {
            return {Exchange::Kind::ResponseTooLarge, {}};
        }
        if (code != CURLE_OK) {
            return failed;
        }
        long status = 0;
        curl_easy_getinfo(handle, CURLINFO_RESPONSE_CODE, &status);
        if (status < 100 || status > 999) {
            return failed;
        }
        HttpResponse response;
        response.status = static_cast<int>(status);
        response.headers = std::move(transfer.headers);
        response.body = std::move(transfer.body);
        return {Exchange::Kind::Response, std::move(response)};
    }

    DestinationPolicy policy_;
    CurlTransportSettings settings_;
    /// What answers a proxy named by its host name: the system's resolver, since the proxy is the
    /// deployment's and not a target the policy judges.
    std::shared_ptr<Resolver> proxyResolver_ = std::make_shared<SystemResolver>();
};

}  // namespace SCE::StandardHost
