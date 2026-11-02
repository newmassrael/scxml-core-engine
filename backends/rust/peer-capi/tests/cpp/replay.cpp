// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Replays the recorded scenario of the peer core (backends/rust/peer-capi/tests/fixtures) through the
// C ABI, from C++, with no relay: the cores of two devices are made, given the machine's sends, the
// rounds' answers and a save, and every text a core gives (each request, each outcome, the save, the
// reason of a refusal) must be the recorded one.
//
// The scenario is read as the whitespace-separated tokens the C replay reads (the Rust test writes
// them from the JSON file); tests/c/replay.c describes them.

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <fstream>
#include <iostream>
#include <memory>
#include <optional>
#include <sstream>
#include <string>
#include <utility>
#include <vector>

#include "sce_peer.h"

namespace {

int steps = 0;

[[noreturn]] void die(const std::string &what) {
    std::cerr << "replay: step " << steps << ": " << what << " (" << sce_peer_last_error() << ")\n";
    std::exit(1);
}

void check(int32_t status, const char *what) {
    if (status != SCE_PEER_OK) {
        die(what);
    }
}

struct PeerDeleter {
    void operator()(ScePeer *peer) const {
        sce_peer_free(peer);
    }
};

using Peer = std::unique_ptr<ScePeer, PeerDeleter>;

std::vector<uint8_t> unhex(const std::string &hex) {
    auto nibble = [](char c) -> int {
        if (c >= '0' && c <= '9') {
            return c - '0';
        }
        if (c >= 'a' && c <= 'f') {
            return c - 'a' + 10;
        }
        die("a hex digit is expected");
    };
    std::vector<uint8_t> bytes(hex.size() / 2);
    for (size_t i = 0; i < bytes.size(); i++) {
        bytes[i] = static_cast<uint8_t>(nibble(hex[2 * i]) * 16 + nibble(hex[2 * i + 1]));
    }
    return bytes;
}

// A text token: x and the text's UTF-8 bytes in hex.
std::string text_of(const std::string &token) {
    if (token.empty() || token[0] != 'x') {
        die("a text is expected");
    }
    const std::vector<uint8_t> bytes = unhex(token.substr(1));
    return std::string(bytes.begin(), bytes.end());
}

// The text of a buffer the library gave, given back; nothing for an empty buffer.
std::optional<std::string> taken(ScePeerBuf buf) {
    if (buf.ptr == nullptr) {
        return std::nullopt;
    }
    std::string text(reinterpret_cast<const char *>(buf.ptr), buf.len);
    sce_peer_buf_free(buf);
    return text;
}

void expect_text(const std::optional<std::string> &got, const std::string &want, const char *what) {
    const std::string have = got.value_or("");
    if (have != want) {
        std::cerr << "replay: step " << steps << ": " << what << " is not the recorded one\n  got:  " << have
                  << "\n  want: " << want << "\n";
        std::exit(1);
    }
}

int32_t kept_of(const std::string &name) {
    if (name == "taken") {
        return SCE_PEER_KEPT_TAKEN;
    }
    if (name == "held") {
        return SCE_PEER_KEPT_HELD;
    }
    if (name == "not_signed") {
        return SCE_PEER_KEPT_NOT_SIGNED;
    }
    if (name == "refused") {
        return SCE_PEER_KEPT_REFUSED;
    }
    die("an answer to a send the replay does not know");
}

int32_t kind_of(const std::string &name) {
    if (name == "sync") {
        return SCE_PEER_ROUND_SYNC;
    }
    if (name == "resync") {
        return SCE_PEER_ROUND_RESYNC;
    }
    if (name == "reread") {
        return SCE_PEER_ROUND_REREAD;
    }
    die("a kind of round the replay does not know");
}

class Tokens {
public:
    explicit Tokens(std::string text) : in_(std::move(text)) {}

    bool next(std::string &out) {
        return static_cast<bool>(in_ >> out);
    }

    std::string need() {
        std::string token;
        if (!next(token)) {
            die("the scenario ends too soon");
        }
        return token;
    }

    uint64_t number() {
        return std::stoull(need());
    }

    std::string text() {
        return text_of(need());
    }

private:
    std::istringstream in_;
};

struct Device {
    uint64_t node = 0;
    std::vector<uint8_t> seed;
    std::optional<std::vector<uint8_t>> genesis;
    Peer peer;
};

}  // namespace

int main(int argc, char **argv) {
    if (argc != 2) {
        std::cerr << "usage: replay <script>\n";
        return 2;
    }
    if (sce_peer_abi_version() != SCE_PEER_ABI_VERSION) {
        die("the library is another ABI version");
    }
    std::ifstream file(argv[1], std::ios::binary);
    if (!file) {
        die("the script cannot be opened");
    }
    std::stringstream buffer;
    buffer << file.rdbuf();
    Tokens tokens(buffer.str());

    const std::string list = tokens.text();
    std::vector<Device> devices(tokens.number());
    for (Device &device : devices) {
        device.node = tokens.number();
        device.seed = unhex(tokens.need());
        if (device.seed.size() != 32) {
            die("a seed is not 32 bytes");
        }
        const std::string genesis = tokens.need();
        if (genesis != "-") {
            device.genesis = unhex(genesis);
            if (device.genesis->size() != 32) {
                die("a genesis hash is not 32 bytes");
            }
        }
    }

    std::string op;
    while (tokens.next(op)) {
        const uint64_t at = tokens.number();
        if (at >= devices.size()) {
            die("a step names no device");
        }
        Device &device = devices[at];
        if (op == "make") {
            device.peer.reset(device.genesis ? sce_peer_invited(list.c_str(), device.genesis->data())
                                             : sce_peer_open(list.c_str()));
            if (!device.peer) {
                die("the core is not made");
            }
            check(sce_peer_set_identity(device.peer.get(), device.node, device.seed.data()), "set_identity");
            steps++;
            continue;
        }
        ScePeer *peer = device.peer.get();
        if (peer == nullptr) {
            die("the device's core is not made");
        }
        ScePeerBuf out{nullptr, 0};
        if (op == "restore") {
            const std::string saved = tokens.text();
            const uint64_t want = tokens.number();
            uint64_t count = UINT64_MAX;
            check(sce_peer_restore(peer, saved.c_str(), &count), "restore");
            if (count != want) {
                die("the restore kept another number of operations than was recorded");
            }
        } else if (op == "outbound") {
            const std::string event = tokens.text();
            const std::string data = tokens.text();
            const int32_t want = kept_of(tokens.need());
            const std::string reason = tokens.text();
            int32_t kept = -1;
            check(sce_peer_outbound(peer, event.c_str(), data.c_str(), &kept, &out), "outbound");
            if (kept != want) {
                die("the core did not keep the send as recorded");
            }
            expect_text(taken(out), reason, "the reason of the refusal");
        } else if (op == "start") {
            check(sce_peer_start(peer, kind_of(tokens.need())), "start");
        } else if (op == "request") {
            const std::string want = tokens.text();
            check(sce_peer_poll_request(peer, &out), "poll_request");
            const std::optional<std::string> request = taken(out);
            if (!request) {
                die("the core asks nothing where a request was recorded");
            }
            expect_text(request, want, "the request");
        } else if (op == "respond") {
            check(sce_peer_respond(peer, tokens.text().c_str()), "respond");
        } else if (op == "fail") {
            check(sce_peer_fail(peer, tokens.text().c_str()), "fail");
        } else if (op == "outcome") {
            const std::string want = tokens.text();
            check(sce_peer_poll_request(peer, &out), "poll_request");
            if (taken(out)) {
                die("the core still asks where the exchange was over");
            }
            ScePeerBuf outcome{nullptr, 0};
            check(sce_peer_take_outcome(peer, &outcome), "take_outcome");
            const std::optional<std::string> ended = taken(outcome);
            if (!ended) {
                die("no exchange ended where an outcome was recorded");
            }
            expect_text(ended, want, "the outcome");
        } else if (op == "save") {
            const std::string want = tokens.text();
            check(sce_peer_save(peer, &out), "save");
            expect_text(taken(out), want, "the save");
        } else {
            die("a step the replay does not know");
        }
        steps++;
    }
    std::cout << "replayed " << steps << " steps\n";
    return 0;
}
