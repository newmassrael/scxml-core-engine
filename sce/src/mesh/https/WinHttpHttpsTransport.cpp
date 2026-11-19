// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The https binding's port on WinHTTP (docs/adr/0107, D2); the contract is in
// mesh/transports/https/WinHttpHttpsTransport.h.
//
// One session per adapter, one connection and request handle per exchange,
// in WinHTTP's synchronous mode: the port is called from more than one thread
// at once and each call owns its request handle. A cancellation closes that
// handle from the cancelling thread, which is how WinHTTP ends a synchronous
// call that is waiting.
//
// SCE_MESH.md §mesh-22.6: the reference adapter on Windows, under the
// platform policy that section sets. The proxy is the current user's
// configuration, read once when the adapter is made: a named proxy with its
// bypass list, or an automatic configuration script and WPAD, which
// WinHttpGetProxyForUrl resolves for each URL. The server's chain is validated
// by WinHTTP against the Windows certificate store, which is the platform
// trust the port requires; no flag weakens it.

#include "mesh/transports/https/WinHttpHttpsTransport.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>

#include <wincrypt.h>
#include <winhttp.h>

#include <algorithm>
#include <climits>
#include <cstdint>
#include <mutex>
#include <string>
#include <utility>
#include <vector>

namespace SCE::Mesh {

namespace {

std::wstring widen(const std::string &text) {
    if (text.empty()) {
        return std::wstring();
    }
    const int length = MultiByteToWideChar(CP_UTF8, 0, text.data(), static_cast<int>(text.size()), nullptr, 0);
    if (length <= 0) {
        return std::wstring();
    }
    std::wstring wide(static_cast<std::size_t>(length), L'\0');
    MultiByteToWideChar(CP_UTF8, 0, text.data(), static_cast<int>(text.size()), &wide[0], length);
    return wide;
}

std::string narrow(const wchar_t *text, std::size_t count) {
    if (count == 0) {
        return std::string();
    }
    const int length = WideCharToMultiByte(CP_UTF8, 0, text, static_cast<int>(count), nullptr, 0, nullptr, nullptr);
    if (length <= 0) {
        return std::string();
    }
    std::string out(static_cast<std::size_t>(length), '\0');
    WideCharToMultiByte(CP_UTF8, 0, text, static_cast<int>(count), &out[0], length, nullptr, nullptr);
    return out;
}

/// Takes a string WinHTTP allocated with GlobalAlloc and frees it.
std::wstring takeGlobal(LPWSTR text) {
    std::wstring out;
    if (text != nullptr) {
        out = text;
        GlobalFree(text);
    }
    return out;
}

/// A WinHTTP handle closed when it goes out of scope.
class Handle {
public:
    explicit Handle(HINTERNET handle) : handle_(handle) {}

    Handle(const Handle &) = delete;
    Handle &operator=(const Handle &) = delete;

    ~Handle() {
        if (handle_ != nullptr) {
            WinHttpCloseHandle(handle_);
        }
    }

    HINTERNET get() const {
        return handle_;
    }

private:
    HINTERNET handle_;
};

/// A request handle another thread may close to end a waiting call. Closed
/// once, by whichever comes first: the cancellation or the end of the
/// exchange.
class RequestHandle {
public:
    explicit RequestHandle(HINTERNET handle) : handle_(handle) {}

    RequestHandle(const RequestHandle &) = delete;
    RequestHandle &operator=(const RequestHandle &) = delete;

    ~RequestHandle() {
        close();
    }

    HINTERNET get() const {
        return handle_;
    }

    void close() {
        std::lock_guard<std::mutex> lock(mutex_);
        if (handle_ != nullptr && !closed_) {
            WinHttpCloseHandle(handle_);
            closed_ = true;
        }
    }

private:
    std::mutex mutex_;
    HINTERNET handle_;
    bool closed_ = false;
};

/// The failure kind of a WinHTTP error, in the named kinds of the port
/// (SCE_MESH.md §mesh-22.6). A name that does not resolve and a
/// connection refused are the proxy's when the exchange went through one,
/// since the client resolves and dials only the proxy then.
HttpsExchange::Failure classify(DWORD error, bool viaProxy) {
    using Failure = HttpsExchange::Failure;
    switch (error) {
    case ERROR_WINHTTP_TIMEOUT:
        return Failure::Timeout;
    case ERROR_WINHTTP_SECURE_FAILURE:
    case ERROR_WINHTTP_SECURE_CERT_DATE_INVALID:
    case ERROR_WINHTTP_SECURE_CERT_CN_INVALID:
    case ERROR_WINHTTP_SECURE_INVALID_CA:
    case ERROR_WINHTTP_SECURE_CERT_REV_FAILED:
    case ERROR_WINHTTP_SECURE_CHANNEL_ERROR:
    case ERROR_WINHTTP_SECURE_INVALID_CERT:
    case ERROR_WINHTTP_SECURE_CERT_REVOKED:
    case ERROR_WINHTTP_SECURE_CERT_WRONG_USAGE:
    case ERROR_WINHTTP_CLIENT_AUTH_CERT_NEEDED:
        return Failure::Tls;
    case ERROR_WINHTTP_NAME_NOT_RESOLVED:
    case ERROR_WINHTTP_CANNOT_CONNECT:
        return viaProxy ? Failure::Proxy : Failure::Connect;
    case ERROR_WINHTTP_CONNECTION_ERROR:
    case ERROR_WINHTTP_INVALID_SERVER_RESPONSE:
        return Failure::Connect;
    default:
        return Failure::Other;
    }
}

/// `Name: value` lines of a raw header block, the status line skipped.
std::vector<HttpsHeader> parseHeaders(const std::string &raw) {
    std::vector<HttpsHeader> headers;
    std::size_t start = 0;
    bool statusLine = true;
    while (start < raw.size()) {
        std::size_t end = raw.find("\r\n", start);
        if (end == std::string::npos) {
            end = raw.size();
        }
        const std::string line = raw.substr(start, end - start);
        start = end + 2;
        if (statusLine) {
            statusLine = false;
            continue;
        }
        const std::size_t colon = line.find(':');
        if (colon == std::string::npos) {
            continue;
        }
        std::size_t valueStart = colon + 1;
        while (valueStart < line.size() && (line[valueStart] == ' ' || line[valueStart] == '\t')) {
            ++valueStart;
        }
        std::size_t valueEnd = line.size();
        while (valueEnd > valueStart && (line[valueEnd - 1] == ' ' || line[valueEnd - 1] == '\t')) {
            --valueEnd;
        }
        headers.emplace_back(line.substr(0, colon), line.substr(valueStart, valueEnd - valueStart));
    }
    return headers;
}

/// The bytes a hexadecimal thumbprint names; empty when it names none.
std::vector<BYTE> thumbprintBytes(const std::string &hex) {
    std::vector<BYTE> bytes;
    const auto nibble = [](char c) -> int {
        if (c >= '0' && c <= '9') {
            return c - '0';
        }
        if (c >= 'a' && c <= 'f') {
            return c - 'a' + 10;
        }
        if (c >= 'A' && c <= 'F') {
            return c - 'A' + 10;
        }
        return -1;
    };
    std::string digits;
    for (const char c : hex) {
        if (c != ' ' && c != ':') {
            digits += c;
        }
    }
    if (digits.empty() || digits.size() % 2 != 0) {
        return {};
    }
    for (std::size_t i = 0; i < digits.size(); i += 2) {
        const int high = nibble(digits[i]);
        const int low = nibble(digits[i + 1]);
        if (high < 0 || low < 0) {
            return {};
        }
        bytes.push_back(static_cast<BYTE>((high << 4) | low));
    }
    return bytes;
}

HttpsExchange failedWith(HttpsExchange::Failure failure, const char *step, DWORD error) {
    return HttpsExchange::failed(failure, std::string(step) + " failed with WinHTTP error " + std::to_string(error));
}

}  // namespace

struct WinHttpHttpsTransport::Impl {
    Options options;
    HINTERNET session = nullptr;
    /// The proxy configuration read when the adapter was made.
    bool explicitProxy = false;
    bool autoDetect = false;
    std::wstring autoConfigUrl;
    std::wstring proxy;
    std::wstring bypass;
    /// The `mtls` client certificate, and the store it was found in.
    HCERTSTORE store = nullptr;
    PCCERT_CONTEXT clientCertificate = nullptr;
    bool clientCertificateMissing = false;

    explicit Impl(Options opts) : options(std::move(opts)) {
        const std::wstring agent = widen(options.user_agent);
        session =
            WinHttpOpen(agent.c_str(), WINHTTP_ACCESS_TYPE_NO_PROXY, WINHTTP_NO_PROXY_NAME, WINHTTP_NO_PROXY_BYPASS, 0);
        readProxyConfiguration();
        findClientCertificate();
    }

    ~Impl() {
        if (clientCertificate != nullptr) {
            CertFreeCertificateContext(clientCertificate);
        }
        if (store != nullptr) {
            CertCloseStore(store, 0);
        }
        if (session != nullptr) {
            WinHttpCloseHandle(session);
        }
    }

    Impl(const Impl &) = delete;
    Impl &operator=(const Impl &) = delete;

    void readProxyConfiguration() {
        if (!options.proxy.empty()) {
            explicitProxy = true;
            proxy = widen(options.proxy);
            return;
        }
        WINHTTP_CURRENT_USER_IE_PROXY_CONFIG config{};
        if (!WinHttpGetIEProxyConfigForCurrentUser(&config)) {
            // No configuration to read (a service account has none): WPAD is
            // what the platform falls back to.
            autoDetect = true;
            return;
        }
        autoDetect = config.fAutoDetect != FALSE;
        autoConfigUrl = takeGlobal(config.lpszAutoConfigUrl);
        proxy = takeGlobal(config.lpszProxy);
        bypass = takeGlobal(config.lpszProxyBypass);
    }

    void findClientCertificate() {
        if (options.client_certificate_thumbprint.empty()) {
            return;
        }
        std::vector<BYTE> hash = thumbprintBytes(options.client_certificate_thumbprint);
        store = CertOpenStore(CERT_STORE_PROV_SYSTEM_W, 0, 0, CERT_SYSTEM_STORE_CURRENT_USER | CERT_STORE_READONLY_FLAG,
                              L"MY");
        if (store != nullptr && !hash.empty()) {
            CRYPT_HASH_BLOB blob{};
            blob.cbData = static_cast<DWORD>(hash.size());
            blob.pbData = hash.data();
            clientCertificate = CertFindCertificateInStore(store, X509_ASN_ENCODING | PKCS_7_ASN_ENCODING, 0,
                                                           CERT_FIND_SHA1_HASH, &blob, nullptr);
        }
        clientCertificateMissing = clientCertificate == nullptr;
    }

    /// Set the proxy `url` takes on `request`: the host's own when it names
    /// one, otherwise the one the platform's settings name (SCE_MESH.md
    /// §mesh-22.6). Whether one is used.
    bool applyProxy(HINTERNET request, const std::wstring &url) {
        if (explicitProxy) {
            WINHTTP_PROXY_INFO info{};
            info.dwAccessType = WINHTTP_ACCESS_TYPE_NAMED_PROXY;
            info.lpszProxy = &proxy[0];
            WinHttpSetOption(request, WINHTTP_OPTION_PROXY, &info, sizeof(info));
            return true;
        }
        if (autoDetect || !autoConfigUrl.empty()) {
            WINHTTP_AUTOPROXY_OPTIONS automatic{};
            if (autoDetect) {
                automatic.dwFlags |= WINHTTP_AUTOPROXY_AUTO_DETECT;
                automatic.dwAutoDetectFlags = WINHTTP_AUTO_DETECT_TYPE_DHCP | WINHTTP_AUTO_DETECT_TYPE_DNS_A;
            }
            if (!autoConfigUrl.empty()) {
                automatic.dwFlags |= WINHTTP_AUTOPROXY_CONFIG_URL;
                automatic.lpszAutoConfigUrl = autoConfigUrl.c_str();
            }
            automatic.fAutoLogonIfChallenged = TRUE;
            WINHTTP_PROXY_INFO found{};
            if (WinHttpGetProxyForUrl(session, url.c_str(), &automatic, &found)) {
                WinHttpSetOption(request, WINHTTP_OPTION_PROXY, &found, sizeof(found));
                const bool named = found.dwAccessType == WINHTTP_ACCESS_TYPE_NAMED_PROXY && found.lpszProxy != nullptr;
                (void)takeGlobal(found.lpszProxy);
                (void)takeGlobal(found.lpszProxyBypass);
                return named;
            }
            // The script and WPAD named nothing: the static setting stands.
        }
        if (!proxy.empty()) {
            WINHTTP_PROXY_INFO info{};
            info.dwAccessType = WINHTTP_ACCESS_TYPE_NAMED_PROXY;
            info.lpszProxy = &proxy[0];
            info.lpszProxyBypass = bypass.empty() ? nullptr : &bypass[0];
            WinHttpSetOption(request, WINHTTP_OPTION_PROXY, &info, sizeof(info));
            return true;
        }
        return false;
    }
};

WinHttpHttpsTransport::WinHttpHttpsTransport() : WinHttpHttpsTransport(Options{}) {}

WinHttpHttpsTransport::WinHttpHttpsTransport(Options options) : impl_(std::make_unique<Impl>(std::move(options))) {}

WinHttpHttpsTransport::~WinHttpHttpsTransport() = default;

bool WinHttpHttpsTransport::valid() const noexcept {
    return impl_->session != nullptr;
}

HttpsExchange WinHttpHttpsTransport::exchange(const HttpsRequest &request) {
    using Failure = HttpsExchange::Failure;
    if (impl_->session == nullptr) {
        return HttpsExchange::failed(Failure::Other, "the WinHTTP session did not open");
    }
    if (impl_->clientCertificateMissing) {
        return HttpsExchange::failed(Failure::Tls, "the client certificate is not in the current user's store");
    }

    const std::wstring url = widen(request.url);
    URL_COMPONENTS parts{};
    parts.dwStructSize = sizeof(parts);
    parts.dwSchemeLength = static_cast<DWORD>(-1);
    parts.dwHostNameLength = static_cast<DWORD>(-1);
    parts.dwUrlPathLength = static_cast<DWORD>(-1);
    parts.dwExtraInfoLength = static_cast<DWORD>(-1);
    if (!WinHttpCrackUrl(url.c_str(), static_cast<DWORD>(url.size()), 0, &parts)) {
        return failedWith(Failure::Other, "WinHttpCrackUrl", GetLastError());
    }
    const std::wstring host(parts.lpszHostName, parts.dwHostNameLength);
    std::wstring path(parts.lpszUrlPath, parts.dwUrlPathLength);
    if (parts.lpszExtraInfo != nullptr) {
        path.append(parts.lpszExtraInfo, parts.dwExtraInfoLength);
    }
    const bool secure = parts.nScheme == INTERNET_SCHEME_HTTPS;

    Handle connection(WinHttpConnect(impl_->session, host.c_str(), parts.nPort, 0));
    if (connection.get() == nullptr) {
        return failedWith(Failure::Other, "WinHttpConnect", GetLastError());
    }
    RequestHandle handle(WinHttpOpenRequest(connection.get(), L"POST", path.c_str(), nullptr, WINHTTP_NO_REFERER,
                                            WINHTTP_DEFAULT_ACCEPT_TYPES, secure ? WINHTTP_FLAG_SECURE : 0));
    if (handle.get() == nullptr) {
        return failedWith(Failure::Other, "WinHttpOpenRequest", GetLastError());
    }
    // Declared after the handle so it ends first: a cancellation that comes
    // after the exchange finds no handle to close.
    HttpsCancellation::Subscription cancelled;
    if (request.cancellation != nullptr) {
        cancelled = request.cancellation->subscribe([&handle] { handle.close(); });
    }
    const auto cancelledFailure = [&request]() {
        return request.cancellation != nullptr && request.cancellation->requested();
    };

    const int timeout = static_cast<int>(std::max<long long>(1, std::min<long long>(request.timeout.count(), INT_MAX)));
    WinHttpSetTimeouts(handle.get(), timeout, timeout, timeout, timeout);
#ifdef WINHTTP_OPTION_ENABLE_HTTP_PROTOCOL
    // HTTP/2 where this WinHTTP has it; an older one refuses the option and
    // speaks HTTP/1.1.
    DWORD protocols = WINHTTP_PROTOCOL_FLAG_HTTP2;
    WinHttpSetOption(handle.get(), WINHTTP_OPTION_ENABLE_HTTP_PROTOCOL, &protocols, sizeof(protocols));
#endif
    if (impl_->clientCertificate != nullptr) {
        WinHttpSetOption(handle.get(), WINHTTP_OPTION_CLIENT_CERT_CONTEXT,
                         const_cast<CERT_CONTEXT *>(impl_->clientCertificate), sizeof(CERT_CONTEXT));
    } else {
        WinHttpSetOption(handle.get(), WINHTTP_OPTION_CLIENT_CERT_CONTEXT, WINHTTP_NO_CLIENT_CERT_CONTEXT, 0);
    }
    const bool viaProxy = impl_->applyProxy(handle.get(), url);

    std::wstring headerBlock;
    for (const auto &header : request.headers) {
        headerBlock += widen(header.first);
        headerBlock += L": ";
        headerBlock += widen(header.second);
        headerBlock += L"\r\n";
    }
    if (!headerBlock.empty() &&
        !WinHttpAddRequestHeaders(handle.get(), headerBlock.c_str(), static_cast<DWORD>(headerBlock.size()),
                                  WINHTTP_ADDREQ_FLAG_ADD | WINHTTP_ADDREQ_FLAG_REPLACE)) {
        return failedWith(Failure::Other, "WinHttpAddRequestHeaders", GetLastError());
    }

    const DWORD bodySize = static_cast<DWORD>(request.body.size());
    LPVOID body = request.body.empty() ? WINHTTP_NO_REQUEST_DATA
                                       : const_cast<LPVOID>(static_cast<const void *>(request.body.data()));
    if (!WinHttpSendRequest(handle.get(), WINHTTP_NO_ADDITIONAL_HEADERS, 0, body, bodySize, bodySize, 0)) {
        const DWORD error = GetLastError();
        if (cancelledFailure()) {
            return HttpsExchange::failed(Failure::Other, "cancelled");
        }
        return failedWith(classify(error, viaProxy), "WinHttpSendRequest", error);
    }
    if (!WinHttpReceiveResponse(handle.get(), nullptr)) {
        const DWORD error = GetLastError();
        if (cancelledFailure()) {
            return HttpsExchange::failed(Failure::Other, "cancelled");
        }
        return failedWith(classify(error, viaProxy), "WinHttpReceiveResponse", error);
    }

    DWORD status = 0;
    DWORD statusSize = sizeof(status);
    if (!WinHttpQueryHeaders(handle.get(), WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                             WINHTTP_HEADER_NAME_BY_INDEX, &status, &statusSize, WINHTTP_NO_HEADER_INDEX)) {
        return failedWith(Failure::Other, "WinHttpQueryHeaders", GetLastError());
    }
    if (status == 407) {
        // The proxy asked for credentials this adapter does not hold: the
        // exchange never reached the server.
        return HttpsExchange::failed(Failure::Proxy, "the proxy requires authentication (407)");
    }

    std::vector<HttpsHeader> headers;
    DWORD rawSize = 0;
    WinHttpQueryHeaders(handle.get(), WINHTTP_QUERY_RAW_HEADERS_CRLF, WINHTTP_HEADER_NAME_BY_INDEX,
                        WINHTTP_NO_OUTPUT_BUFFER, &rawSize, WINHTTP_NO_HEADER_INDEX);
    if (GetLastError() == ERROR_INSUFFICIENT_BUFFER && rawSize > 0) {
        std::wstring raw(rawSize / sizeof(wchar_t), L'\0');
        if (WinHttpQueryHeaders(handle.get(), WINHTTP_QUERY_RAW_HEADERS_CRLF, WINHTTP_HEADER_NAME_BY_INDEX, &raw[0],
                                &rawSize, WINHTTP_NO_HEADER_INDEX)) {
            headers = parseHeaders(narrow(raw.c_str(), rawSize / sizeof(wchar_t)));
        }
    }

    std::vector<std::uint8_t> received;
    for (;;) {
        DWORD available = 0;
        if (!WinHttpQueryDataAvailable(handle.get(), &available)) {
            const DWORD error = GetLastError();
            if (cancelledFailure()) {
                return HttpsExchange::failed(Failure::Other, "cancelled");
            }
            return failedWith(classify(error, viaProxy), "WinHttpQueryDataAvailable", error);
        }
        if (available == 0) {
            break;
        }
        const std::size_t before = received.size();
        received.resize(before + available);
        DWORD read = 0;
        if (!WinHttpReadData(handle.get(), received.data() + before, available, &read)) {
            const DWORD error = GetLastError();
            if (cancelledFailure()) {
                return HttpsExchange::failed(Failure::Other, "cancelled");
            }
            return failedWith(classify(error, viaProxy), "WinHttpReadData", error);
        }
        received.resize(before + read);
        if (read == 0) {
            break;
        }
    }
    return HttpsExchange::response(static_cast<std::uint16_t>(status), std::move(headers), std::move(received));
}

}  // namespace SCE::Mesh
