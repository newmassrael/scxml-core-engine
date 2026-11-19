// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The https binding's port on the system's libcurl (docs/adr/0107, D2); the
// contract is in mesh/transports/https/CurlHttpsTransport.h.
//
// SCE_MESH.md §mesh-22.6: the reference adapter on Linux. One easy handle per
// exchange: the port is called from more than one thread at once, and an easy
// handle may be used by one thread at a time. libcurl reads the proxy
// variables itself when no proxy is set on the handle, which is the platform
// policy this adapter owes, so none is set here.

#include "mesh/transports/https/CurlHttpsTransport.h"

#include <curl/curl.h>

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <limits>
#include <memory>
#include <mutex>
#include <string>
#include <utility>
#include <vector>

namespace SCE::Mesh {

namespace {

/// libcurl's global state is set up once per process, before the first
/// handle (curl_global_init is not thread-safe before libcurl 7.84).
bool globalInit() {
    static std::once_flag once;
    static bool initialized = false;
    std::call_once(once, [] { initialized = curl_global_init(CURL_GLOBAL_DEFAULT) == CURLE_OK; });
    return initialized;
}

struct EasyDeleter {
    void operator()(CURL *easy) const {
        curl_easy_cleanup(easy);
    }
};

struct SlistDeleter {
    void operator()(curl_slist *list) const {
        curl_slist_free_all(list);
    }
};

/// What a transfer collects of the response.
struct Collected {
    std::vector<HttpsHeader> headers;
    std::vector<std::uint8_t> body;
};

std::size_t onBody(char *data, std::size_t size, std::size_t count, void *user) {
    auto *collected = static_cast<Collected *>(user);
    const std::size_t bytes = size * count;
    collected->body.insert(collected->body.end(), reinterpret_cast<const std::uint8_t *>(data),
                           reinterpret_cast<const std::uint8_t *>(data) + bytes);
    return bytes;
}

std::string trimmed(std::string text) {
    const auto blank = [](char c) { return c == ' ' || c == '\t' || c == '\r' || c == '\n'; };
    while (!text.empty() && blank(text.back())) {
        text.pop_back();
    }
    std::size_t start = 0;
    while (start < text.size() && blank(text[start])) {
        ++start;
    }
    return text.substr(start);
}

/// One header line. A status line starts a response, so the fields of an
/// interim one (a proxy's answer to CONNECT, a 100 Continue) are dropped and
/// only the final response's are kept.
std::size_t onHeader(char *data, std::size_t size, std::size_t count, void *user) {
    auto *collected = static_cast<Collected *>(user);
    const std::size_t bytes = size * count;
    const std::string line(data, bytes);
    if (line.rfind("HTTP/", 0) == 0) {
        collected->headers.clear();
        return bytes;
    }
    const std::size_t colon = line.find(':');
    if (colon != std::string::npos) {
        collected->headers.emplace_back(trimmed(line.substr(0, colon)), trimmed(line.substr(colon + 1)));
    }
    return bytes;
}

/// Ends the transfer when the caller requested it; libcurl calls this about
/// once a second while it waits, so a cancellation takes effect within one.
int onProgress(void *user, curl_off_t, curl_off_t, curl_off_t, curl_off_t) {
    const auto *cancellation = static_cast<const HttpsCancellation *>(user);
    return (cancellation != nullptr && cancellation->requested()) ? 1 : 0;
}

/// The failure kind of a transfer that produced no response, in the named
/// kinds of the port (SCE_MESH.md §mesh-22.6).
HttpsExchange::Failure classify(CURLcode code, long connectCode) {
    using Failure = HttpsExchange::Failure;
    if (connectCode != 0 && connectCode != 200) {
        // The proxy answered CONNECT with something other than a tunnel.
        return Failure::Proxy;
    }
    if (code == CURLE_OPERATION_TIMEDOUT) {
        return Failure::Timeout;
    }
    if (code == CURLE_COULDNT_RESOLVE_PROXY) {
        return Failure::Proxy;
    }
#if LIBCURL_VERSION_NUM >= 0x074900
    if (code == CURLE_PROXY) {
        return Failure::Proxy;
    }
#endif
    if (code == CURLE_SSL_CONNECT_ERROR || code == CURLE_PEER_FAILED_VERIFICATION || code == CURLE_SSL_CERTPROBLEM ||
        code == CURLE_SSL_CIPHER || code == CURLE_SSL_CACERT_BADFILE || code == CURLE_SSL_ISSUER_ERROR ||
        code == CURLE_SSL_PINNEDPUBKEYNOTMATCH || code == CURLE_SSL_CRL_BADFILE ||
        code == CURLE_SSL_INVALIDCERTSTATUS) {
        return Failure::Tls;
    }
    if (code == CURLE_COULDNT_RESOLVE_HOST || code == CURLE_COULDNT_CONNECT || code == CURLE_GOT_NOTHING ||
        code == CURLE_RECV_ERROR || code == CURLE_SEND_ERROR || code == CURLE_PARTIAL_FILE) {
        return Failure::Connect;
    }
    return Failure::Other;
}

}  // namespace

CurlHttpsTransport::CurlHttpsTransport() : CurlHttpsTransport(Options{}) {}

CurlHttpsTransport::CurlHttpsTransport(Options options) : options_(std::move(options)) {
    (void)globalInit();
}

CurlHttpsTransport::~CurlHttpsTransport() = default;

HttpsExchange CurlHttpsTransport::exchange(const HttpsRequest &request) {
    if (!globalInit()) {
        return HttpsExchange::failed(HttpsExchange::Failure::Other, "curl_global_init failed");
    }
    std::unique_ptr<CURL, EasyDeleter> easy(curl_easy_init());
    if (!easy) {
        return HttpsExchange::failed(HttpsExchange::Failure::Other, "curl_easy_init failed");
    }

    std::unique_ptr<curl_slist, SlistDeleter> headers;
    for (const auto &header : request.headers) {
        const std::string line = header.first + ": " + header.second;
        curl_slist *grown = curl_slist_append(headers.get(), line.c_str());
        if (grown == nullptr) {
            return HttpsExchange::failed(HttpsExchange::Failure::Other, "curl_slist_append failed");
        }
        (void)headers.release();
        headers.reset(grown);
    }
    // No `Expect: 100-continue` round trip before the body: the binding's
    // bodies are small and a held exchange should not wait twice.
    if (curl_slist *grown = curl_slist_append(headers.get(), "Expect:")) {
        (void)headers.release();
        headers.reset(grown);
    }

    Collected collected;
    char errors[CURL_ERROR_SIZE] = {0};
    static const char kEmpty[] = "";
    const char *body = request.body.empty() ? kEmpty : reinterpret_cast<const char *>(request.body.data());
    const long timeoutMs = static_cast<long>(
        std::min<long long>(request.timeout.count(), static_cast<long long>(std::numeric_limits<long>::max())));

    CURL *handle = easy.get();
    curl_easy_setopt(handle, CURLOPT_URL, request.url.c_str());
    curl_easy_setopt(handle, CURLOPT_POST, 1L);
    curl_easy_setopt(handle, CURLOPT_POSTFIELDS, body);
    curl_easy_setopt(handle, CURLOPT_POSTFIELDSIZE_LARGE, static_cast<curl_off_t>(request.body.size()));
    curl_easy_setopt(handle, CURLOPT_HTTPHEADER, headers.get());
    curl_easy_setopt(handle, CURLOPT_NOSIGNAL, 1L);
    if (timeoutMs > 0) {
        curl_easy_setopt(handle, CURLOPT_TIMEOUT_MS, timeoutMs);
    }
#if LIBCURL_VERSION_NUM >= 0x072F00
    // HTTP/2 when the server offers it over TLS, HTTP/1.1 otherwise.
    curl_easy_setopt(handle, CURLOPT_HTTP_VERSION, static_cast<long>(CURL_HTTP_VERSION_2TLS));
#endif
    if (!options_.ca_bundle.empty()) {
        curl_easy_setopt(handle, CURLOPT_CAINFO, options_.ca_bundle.c_str());
    }
    if (!options_.client_certificate.empty()) {
        curl_easy_setopt(handle, CURLOPT_SSLCERT, options_.client_certificate.c_str());
    }
    if (!options_.client_key.empty()) {
        curl_easy_setopt(handle, CURLOPT_SSLKEY, options_.client_key.c_str());
    }
    curl_easy_setopt(handle, CURLOPT_WRITEFUNCTION, &onBody);
    curl_easy_setopt(handle, CURLOPT_WRITEDATA, static_cast<void *>(&collected));
    curl_easy_setopt(handle, CURLOPT_HEADERFUNCTION, &onHeader);
    curl_easy_setopt(handle, CURLOPT_HEADERDATA, static_cast<void *>(&collected));
    curl_easy_setopt(handle, CURLOPT_NOPROGRESS, 0L);
    curl_easy_setopt(handle, CURLOPT_XFERINFOFUNCTION, &onProgress);
    curl_easy_setopt(handle, CURLOPT_XFERINFODATA, static_cast<void *>(request.cancellation));
    curl_easy_setopt(handle, CURLOPT_ERRORBUFFER, errors);

    const CURLcode code = curl_easy_perform(handle);
    long connectCode = 0;
    curl_easy_getinfo(handle, CURLINFO_HTTP_CONNECTCODE, &connectCode);
    if (code != CURLE_OK) {
        if (code == CURLE_ABORTED_BY_CALLBACK) {
            return HttpsExchange::failed(HttpsExchange::Failure::Other, "cancelled");
        }
        std::string detail = curl_easy_strerror(code);
        if (errors[0] != '\0') {
            detail += ": ";
            detail += errors;
        }
        return HttpsExchange::failed(classify(code, connectCode), std::move(detail));
    }

    long status = 0;
    curl_easy_getinfo(handle, CURLINFO_RESPONSE_CODE, &status);
    if (status < 100 || status > 999) {
        return HttpsExchange::failed(HttpsExchange::Failure::Other,
                                     "libcurl reported no status code for a completed transfer");
    }
    return HttpsExchange::response(static_cast<std::uint16_t>(status), std::move(collected.headers),
                                   std::move(collected.body));
}

}  // namespace SCE::Mesh
