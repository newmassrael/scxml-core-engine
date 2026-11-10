// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Replays the recorded scenario of the peer core (backends/rust/peer-capi/tests/fixtures) through the
// C ABI, from C++, with no relay: the cores of two devices are made, given the administrator's acts,
// the machine's sends and the rounds' answers, and one is sealed and made again from its sealed state
// with no key given again; every text a core gives (each request, each outcome, each standing, the
// genesis, the sealed state, the reason of a refusal) must be the recorded one.
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
    if (name == "waiting") {
        return SCE_PEER_KEPT_WAITING;
    }
    die("an answer to a send the replay does not know");
}

int32_t profile_of(const std::string &name) {
    if (name == "untrusted") {
        return SCE_PEER_PROFILE_UNTRUSTED;
    }
    if (name == "trusted_hub") {
        return SCE_PEER_PROFILE_TRUSTED_HUB;
    }
    die("a profile the replay does not know");
}

// The bytes as lowercase hex.
std::string hex_of(const uint8_t *bytes, size_t n) {
    static const char digits[] = "0123456789abcdef";
    std::string text;
    for (size_t i = 0; i < n; i++) {
        text += digits[bytes[i] >> 4];
        text += digits[bytes[i] & 0x0f];
    }
    return text;
}

// A data key token: 32 bytes in hex.
std::vector<uint8_t> data_key_of(const std::string &token) {
    std::vector<uint8_t> key = unhex(token);
    if (key.size() != 32) {
        die("a data key is not 32 bytes");
    }
    return key;
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
    std::optional<std::vector<uint8_t>> administrator;
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
        const std::string administrator = tokens.need();
        if (administrator != "-") {
            device.administrator = unhex(administrator);
            if (device.administrator->size() != 32) {
                die("an administrator's seed is not 32 bytes");
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
            if (device.administrator) {
                check(sce_peer_set_administrator(device.peer.get(), device.administrator->data()), "set_administrator");
            }
            steps++;
            continue;
        }
        if (op == "make_invited") {
            // The device started from the text of its invitation alone.
            const std::string invitation = tokens.text();
            device.peer.reset(
                sce_peer_invited_by(reinterpret_cast<const uint8_t *>(invitation.data()), invitation.size()));
            if (!device.peer) {
                die("the core is not made from the invitation");
            }
            check(sce_peer_set_identity(device.peer.get(), device.node, device.seed.data()), "set_identity");
            if (device.administrator) {
                check(sce_peer_set_administrator(device.peer.get(), device.administrator->data()), "set_administrator");
            }
            steps++;
            continue;
        }
        if (op == "restore") {
            // The device started again: a core given nothing but the list and the genesis.
            const std::vector<uint8_t> key = data_key_of(tokens.need());
            const std::vector<uint8_t> state = unhex(tokens.need());
            const uint64_t want = tokens.number();
            device.peer.reset(device.genesis ? sce_peer_invited(list.c_str(), device.genesis->data())
                                             : sce_peer_open(list.c_str()));
            if (!device.peer) {
                die("the core is not made");
            }
            uint64_t count = UINT64_MAX;
            check(
                sce_peer_restore_sealed(device.peer.get(), device.node, state.data(), state.size(), key.data(), &count),
                "restore_sealed");
            if (count != want) {
                die("the restore kept another number of operations than was recorded");
            }
            steps++;
            continue;
        }
        ScePeer *peer = device.peer.get();
        if (peer == nullptr) {
            die("the device's core is not made");
        }
        ScePeerBuf out{nullptr, 0};
        if (op == "open") {
            const int32_t profile = profile_of(tokens.need());
            const std::string want = tokens.need();
            uint8_t genesis[32] = {0};
            check(sce_peer_open_list(peer, profile, genesis), "open_list");
            expect_text(hex_of(genesis, sizeof genesis), want, "the genesis");
        } else if (op == "grant") {
            const uint64_t node = tokens.number();
            const uint32_t role = static_cast<uint32_t>(tokens.number());
            const std::vector<uint8_t> key = unhex(tokens.need());
            if (key.size() != 32) {
                die("a key is not 32 bytes");
            }
            check(sce_peer_grant(peer, node, role, key.data()), "grant");
        } else if (op == "remove") {
            check(sce_peer_remove_member(peer, tokens.number()), "remove_member");
        } else if (op == "standing") {
            const std::string want = tokens.text();
            check(sce_peer_standing(peer, &out), "standing");
            expect_text(taken(out), want, "the standing");
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
        } else if (op == "login") {
            check(sce_peer_login(peer, tokens.text().c_str()), "login");
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
            const std::vector<uint8_t> key = data_key_of(tokens.need());
            const std::string want = tokens.need();
            check(sce_peer_save_sealed_under(peer, key.data(), &out), "save_sealed_under");
            if (out.ptr == nullptr) {
                die("the sealed state is empty");
            }
            const std::string got = hex_of(out.ptr, out.len);
            sce_peer_buf_free(out);
            expect_text(got, want, "the sealed state");
        } else if (op == "invite") {
            const std::string relay = tokens.text();
            const std::string audience = tokens.text();
            const uint64_t node = tokens.number();
            const uint64_t now_ms = tokens.number();
            const uint64_t valid_ms = tokens.number();
            const std::vector<uint8_t> secret = data_key_of(tokens.need());
            const std::string want = tokens.text();
            check(sce_peer_invite_under(peer, relay.c_str(), audience.c_str(), node, now_ms, valid_ms, secret.data(),
                                        &out),
                  "invite_under");
            expect_text(taken(out), want, "the invitation");
        } else if (op == "invitation") {
            const std::string invitation = tokens.text();
            const std::string want = tokens.text();
            check(
                sce_peer_invitation_read(reinterpret_cast<const uint8_t *>(invitation.data()), invitation.size(), &out),
                "invitation_read");
            expect_text(taken(out), want, "what the invitation says");
        } else if (op == "join") {
            const std::string invitation = tokens.text();
            const std::string want = tokens.text();
            check(sce_peer_join(peer, reinterpret_cast<const uint8_t *>(invitation.data()), invitation.size(), &out),
                  "join");
            expect_text(taken(out), want, "the fingerprint");
        } else if (op == "joined") {
            const uint64_t node = tokens.number();
            const std::string want = tokens.text();
            check(sce_peer_joined(peer, node, &out), "joined");
            expect_text(taken(out), want, "the devices that joined");
        } else if (op == "made") {
            const std::string data = tokens.text();
            const int32_t want = kept_of(tokens.need());
            const std::string reason = tokens.text();
            int32_t kept = -1;
            check(sce_peer_made(peer, data.c_str(), &kept, &out), "made");
            if (kept != want) {
                die("the core did not give the verdict recorded");
            }
            expect_text(taken(out), reason, "the reason of the refusal or of the wait");
        } else if (op == "records") {
            const std::string want = tokens.text();
            check(sce_peer_take_records(peer, &out), "take_records");
            expect_text(taken(out), want, "the records");
        } else if (op == "recorded") {
            check(sce_peer_recorded(peer, tokens.number()), "recorded");
        } else if (op == "restore_records") {
            const std::string records = tokens.text();
            const uint64_t want = tokens.number();
            uint64_t count = UINT64_MAX;
            check(sce_peer_restore_records(peer, records.c_str(), &count), "restore_records");
            if (count != want) {
                die("the records given back held another number of new ones than was recorded");
            }
        } else if (op == "history") {
            const std::string want = tokens.text();
            check(sce_peer_history(peer, nullptr, &out), "history");
            expect_text(taken(out), want, "the history");
        } else if (op == "stable_cut") {
            const std::string want = tokens.text();
            check(sce_peer_stable_cut(peer, &out), "stable_cut");
            expect_text(taken(out), want, "the stable cut");
        } else if (op == "mark") {
            const uint64_t want = tokens.number();
            uint64_t seq = UINT64_MAX;
            check(sce_peer_mark(peer, &seq), "mark");
            if (seq != want) {
                die("the core signed another mark than was recorded");
            }
        } else if (op == "own_stamp") {
            const int64_t want_wall = std::stoll(tokens.need());
            const auto want_counter = static_cast<uint32_t>(tokens.number());
            const auto want_read_back = static_cast<int32_t>(std::stol(tokens.need()));
            int64_t wall_time = -1;
            uint32_t counter = UINT32_MAX;
            int32_t read_back = -1;
            check(sce_peer_own_stamp(peer, &wall_time, &counter, &read_back), "own_stamp");
            if (wall_time != want_wall || counter != want_counter || read_back != want_read_back) {
                die("the core's own stamp is not the recorded one");
            }
        } else {
            die("a step the replay does not know");
        }
        steps++;
    }
    std::cout << "replayed " << steps << " steps\n";
    return 0;
}
