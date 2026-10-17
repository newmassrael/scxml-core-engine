// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Replays the recorded scenario of the group key agreement (backends/rust/mls/tests/fixtures) through
// the C ABI, from C++: a device is loaded as it was before it joined, and is given the Welcome and the
// commits with the membership log as data. The keys it derives must be the recorded ones, until the
// commit that removes it, after which it derives none.
//
// The scenario is read as the whitespace-separated tokens the C replay reads (the Rust test writes
// them from the JSON file): list NODE SEED STATE then one line per step, each ending in its log:
//   join <welcome> <commit> <epoch> <epoch_key> <section> <section_key> LOG
//   apply <commit> <epoch> <epoch_key> LOG
//   removed <commit> LOG
// where LOG is: <epoch> <n> (<node> <key>)xn <m> (<epoch> <hash>)xm.

#include <array>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <fstream>
#include <iostream>
#include <memory>
#include <sstream>
#include <stdexcept>
#include <string>
#include <vector>

#include "sce_mls.h"

namespace {

[[noreturn]] void die(const std::string &what) {
    std::cerr << "replay: " << what << " (" << sce_mls_last_error() << ")\n";
    std::exit(1);
}

struct LogDeleter {
    void operator()(SceMlsLog *log) const {
        sce_mls_log_free(log);
    }
};

struct DeviceDeleter {
    void operator()(SceMlsDevice *device) const {
        sce_mls_device_free(device);
    }
};

using Log = std::unique_ptr<SceMlsLog, LogDeleter>;
using Device = std::unique_ptr<SceMlsDevice, DeviceDeleter>;

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

void expect_key(const std::array<uint8_t, 32> &got, const std::string &want_hex, const char *what) {
    const std::vector<uint8_t> want = unhex(want_hex);
    if (want.size() != 32 || !std::equal(want.begin(), want.end(), got.begin())) {
        std::cerr << "replay: " << what << " is not the recorded key\n";
        std::exit(1);
    }
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

private:
    std::istringstream in_;
};

Log read_log(Tokens &tokens) {
    Log log(sce_mls_log_new(static_cast<uint32_t>(tokens.number())));
    const uint64_t members = tokens.number();
    for (uint64_t i = 0; i < members; i++) {
        const uint64_t node = tokens.number();
        const std::vector<uint8_t> key = unhex(tokens.need());
        if (key.size() != 32 || sce_mls_log_add_member(log.get(), node, key.data()) != SCE_MLS_OK) {
            die("add_member");
        }
    }
    const uint64_t commits = tokens.number();
    for (uint64_t i = 0; i < commits; i++) {
        const auto epoch = static_cast<uint32_t>(tokens.number());
        const std::vector<uint8_t> hash = unhex(tokens.need());
        if (hash.size() != 32 || sce_mls_log_name_commit(log.get(), epoch, hash.data()) != SCE_MLS_OK) {
            die("name_commit");
        }
    }
    return log;
}

}  // namespace

int main(int argc, char **argv) {
    if (argc != 2) {
        std::cerr << "usage: replay <script>\n";
        return 2;
    }
    if (sce_mls_abi_version() != SCE_MLS_ABI_VERSION) {
        die("the library is another ABI version");
    }
    std::ifstream file(argv[1], std::ios::binary);
    if (!file) {
        die("the script cannot be opened");
    }
    std::stringstream buffer;
    buffer << file.rdbuf();
    Tokens tokens(buffer.str());

    const std::string list = tokens.need();
    const uint64_t node = tokens.number();
    const std::vector<uint8_t> seed = unhex(tokens.need());
    const std::vector<uint8_t> state = unhex(tokens.need());
    Device device(sce_mls_device_from_bytes(list.c_str(), node, seed.data(), state.data(), state.size()));
    if (!device) {
        die("the device does not load");
    }

    int steps = 0;
    std::string op;
    while (tokens.next(op)) {
        std::array<uint8_t, 32> key{};
        uint64_t epoch = 0;
        if (op == "join") {
            const std::vector<uint8_t> welcome = unhex(tokens.need());
            const std::vector<uint8_t> commit = unhex(tokens.need());
            const uint64_t want_epoch = tokens.number();
            const std::string want_key = tokens.need();
            const std::string section = tokens.need();
            const std::string want_section = tokens.need();
            Log log = read_log(tokens);
            if (sce_mls_join(device.get(), welcome.data(), welcome.size(), commit.data(), commit.size(), log.get(),
                             &epoch) != SCE_MLS_OK) {
                die("join");
            }
            if (epoch != want_epoch) {
                die("the epoch joined is not the recorded one");
            }
            if (sce_mls_epoch_key(device.get(), key.data()) != SCE_MLS_OK) {
                die("epoch_key");
            }
            expect_key(key, want_key, "the key of the epoch joined");
            if (sce_mls_section_key(device.get(), section.c_str(), key.data()) != SCE_MLS_OK) {
                die("section_key");
            }
            expect_key(key, want_section, "the key of the section");
        } else if (op == "apply") {
            const std::vector<uint8_t> commit = unhex(tokens.need());
            const uint64_t want_epoch = tokens.number();
            const std::string want_key = tokens.need();
            Log log = read_log(tokens);
            int32_t removed = -1;
            if (sce_mls_apply_commit(device.get(), commit.data(), commit.size(), log.get(), &removed, &epoch) !=
                SCE_MLS_OK) {
                die("apply_commit");
            }
            if (removed != 0 || epoch != want_epoch) {
                die("the commit is not the recorded epoch");
            }
            if (sce_mls_epoch_key(device.get(), key.data()) != SCE_MLS_OK) {
                die("epoch_key");
            }
            expect_key(key, want_key, "the key of the epoch applied");
        } else if (op == "removed") {
            const std::vector<uint8_t> commit = unhex(tokens.need());
            Log log = read_log(tokens);
            int32_t removed = -1;
            if (sce_mls_apply_commit(device.get(), commit.data(), commit.size(), log.get(), &removed, &epoch) !=
                SCE_MLS_OK) {
                die("apply_commit of the removal");
            }
            if (removed != 1) {
                die("the commit did not remove the device");
            }
            if (sce_mls_is_active(device.get()) != 0) {
                die("a removed device is still active");
            }
            if (sce_mls_epoch_key(device.get(), key.data()) != SCE_MLS_ERR_REMOVED) {
                die("a removed device derived a key");
            }
        } else {
            die("a step the replay does not know");
        }
        steps++;
    }
    std::cout << "replayed " << steps << " steps\n";
    return steps == 3 ? 0 : 1;
}
