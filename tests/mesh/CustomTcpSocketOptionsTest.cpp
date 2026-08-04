// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-16.8.3
//
// custom_tcp `SocketOptions` — SCE_MESH.md §16.8.3.
//
// The declaration end of this axis (a deploy.yaml value reaching the
// emitted `SocketOptions`) is `sce-build/tests/mesh_custom_tcp_socket_options.rs`.
// This file covers the other end: that the runtime applies what it is
// handed. The two can fail independently — a codegen that drops a field
// and a runtime that ignores one look identical from either side alone.
//
// Where an option is observable through `getsockopt` the assertion reads
// it back off the real descriptor rather than trusting the setter's
// return, because `setsockopt` failures are deliberately ignored (a
// kernel that declines `SO_RCVBUF` should still yield a working
// connection). The dial retry is observed by timing instead: it has no
// socket-level readback, but a bounded attempt count is exactly a
// bounded wall-clock, which is the property a deployment cares about.

#include "mesh/transports/CustomTcpTransport.h"

#include <gtest/gtest.h>

#include <netinet/in.h>
#include <netinet/tcp.h>
#include <sys/socket.h>
#include <unistd.h>

#include <chrono>
#include <string>

using SCE::Mesh::MeshEnvelope;
using SCE::Mesh::CustomTcp::Client;
using SCE::Mesh::CustomTcp::PeerLink;
using SCE::Mesh::CustomTcp::Server;
using SCE::Mesh::CustomTcp::SocketOptions;

namespace {

int getsockopt_int(int fd, int level, int name) {
    int value = 0;
    socklen_t len = sizeof(value);
    if (::getsockopt(fd, level, name, &value, &len) != 0) {
        return -1;
    }
    return value;
}

/// Server on a kernel-assigned ephemeral port, so concurrent ctest jobs
/// cannot collide on a fixed one.
std::string ephemeral_server_endpoint(Server &server) {
    auto endpoint = server.local_endpoint();
    EXPECT_TRUE(endpoint.has_value()) << "a bound server must report its assigned port";
    return endpoint.value_or(std::string{});
}

/// `Client` dials lazily on first use; `link()` is the cheapest way to
/// force it without pushing an envelope a bare `Server` would then have
/// to decode.
int dial_and_get_fd(Client &client) {
    const auto link = client.link();
    EXPECT_TRUE(link.valid()) << "dial must succeed for the readback to mean anything";
    return client.connected_fd();
}

}  // namespace

TEST(CustomTcpSocketOptionsTest, DefaultsMatchHistory) {
    // These defaults are a contract with every deployment that declares
    // no socket options: they are the literals `CustomTcpTransport.h`
    // carried before the fields existed, and `deploy.rs` mirrors them as
    // `DEFAULT_CUSTOM_TCP_*`. Changing one silently changes the behaviour
    // of every such deployment, so the values are pinned here rather than
    // left to whatever the struct happens to declare.
    SocketOptions defaults;
    EXPECT_EQ(defaults.backlog, 16);
    EXPECT_TRUE(defaults.reuse_addr);
    EXPECT_TRUE(defaults.nodelay);
    EXPECT_EQ(defaults.connect_max_attempts, 20);
    EXPECT_EQ(defaults.connect_retry_interval_ms, 50);
    EXPECT_EQ(defaults.recv_buffer_bytes, 0) << "zero means 'leave the kernel default'";
    EXPECT_EQ(defaults.send_buffer_bytes, 0);
}

TEST(CustomTcpSocketOptionsTest, NodelayFalseReachesTheDialedSocket) {
    // `nodelay` is the option whose default the header justified with
    // "keep harness latencies deterministic" — a test-convenience choice
    // that a bandwidth-sensitive deployment must be able to decline. The
    // readback is on the client's own descriptor, which is the side a
    // deploy.yaml `nodelay: false` is meant to change.
    SocketOptions opts;
    opts.nodelay = false;

    Server server("127.0.0.1:0", [](const MeshEnvelope &, const PeerLink &) {});
    ASSERT_TRUE(server.valid());

    Client client(ephemeral_server_endpoint(server), [](const MeshEnvelope &, const PeerLink &) {}, opts);
    const int fd = dial_and_get_fd(client);
    ASSERT_GE(fd, 0);

    EXPECT_EQ(getsockopt_int(fd, IPPROTO_TCP, TCP_NODELAY), 0)
        << "`nodelay: false` must leave Nagle enabled on the dialed socket";
}

TEST(CustomTcpSocketOptionsTest, NodelayTrueReachesTheDialedSocket) {
    // The other direction, so the test above cannot pass by the option
    // being ignored in both cases.
    Server server("127.0.0.1:0", [](const MeshEnvelope &, const PeerLink &) {});
    ASSERT_TRUE(server.valid());

    Client client(ephemeral_server_endpoint(server), [](const MeshEnvelope &, const PeerLink &) {}, SocketOptions{});
    const int fd = dial_and_get_fd(client);
    ASSERT_GE(fd, 0);

    EXPECT_NE(getsockopt_int(fd, IPPROTO_TCP, TCP_NODELAY), 0) << "the default `nodelay: true` must disable Nagle";
}

TEST(CustomTcpSocketOptionsTest, ReceiveBufferSizeReachesTheDialedSocket) {
    // The Cyclone-parity option (`SocketReceiveBufferSize`). Linux
    // reports back roughly twice the requested size — it reserves the
    // other half for bookkeeping — so the assertion is "grew towards what
    // was asked", not an equality that would encode one kernel's
    // doubling rule.
    SocketOptions opts;
    opts.recv_buffer_bytes = 256 * 1024;

    Server server("127.0.0.1:0", [](const MeshEnvelope &, const PeerLink &) {});
    ASSERT_TRUE(server.valid());

    Client baseline(ephemeral_server_endpoint(server), [](const MeshEnvelope &, const PeerLink &) {});
    const int baseline_fd = dial_and_get_fd(baseline);
    ASSERT_GE(baseline_fd, 0);
    const int default_size = getsockopt_int(baseline_fd, SOL_SOCKET, SO_RCVBUF);

    Client tuned(ephemeral_server_endpoint(server), [](const MeshEnvelope &, const PeerLink &) {}, opts);
    const int tuned_fd = dial_and_get_fd(tuned);
    ASSERT_GE(tuned_fd, 0);
    const int tuned_size = getsockopt_int(tuned_fd, SOL_SOCKET, SO_RCVBUF);

    ASSERT_GT(default_size, 0) << "getsockopt(SO_RCVBUF) must succeed for the comparison to mean anything";
    EXPECT_GT(tuned_size, default_size) << "a declared recv_buffer_bytes must move the socket off the kernel default";
}

TEST(CustomTcpSocketOptionsTest, BoundedRetryFailsFastOnARefusedPort) {
    // The dial retry's default is 20 x 50 ms — the ~1 s the header
    // justified as "covers ctest startup jitter". A deployment whose
    // peers either exist or do not should not pay a second per dial, so
    // the bound has to be real: 2 attempts at 10 ms must return in far
    // under the default's second rather than after it.
    SocketOptions opts;
    opts.connect_max_attempts = 2;
    opts.connect_retry_interval_ms = 10;

    // Port 1 on loopback: privileged and unbound in any sane CI
    // environment, so connect() refuses immediately and the retry loop
    // is what governs how long dial() takes.
    Client client("127.0.0.1:1", [](const MeshEnvelope &, const PeerLink &) {}, opts);

    const auto started = std::chrono::steady_clock::now();
    const auto link = client.link();
    const auto elapsed = std::chrono::steady_clock::now() - started;

    EXPECT_FALSE(link.valid()) << "a refused endpoint must not yield a usable link";
    EXPECT_LT(std::chrono::duration_cast<std::chrono::milliseconds>(elapsed).count(), 500)
        << "a 2 x 10 ms bound must not spend the default's ~1 s";
}

TEST(CustomTcpSocketOptionsTest, AcceptSideSharesTheSameApplier) {
    // The accept loop and the dial path both route through
    // `detail::apply_connection_options`, which is what keeps the two
    // ends of a link from disagreeing about Nagle. The wiring is a
    // one-line call on each side; this pins the helper's behaviour so a
    // change there cannot silently alter what an accepted connection
    // gets.
    int fds[2] = {-1, -1};
    ASSERT_EQ(::socketpair(AF_UNIX, SOCK_STREAM, 0, fds), 0);

    SocketOptions opts;
    opts.nodelay = false;
    opts.recv_buffer_bytes = 128 * 1024;
    const int before = getsockopt_int(fds[0], SOL_SOCKET, SO_RCVBUF);
    SCE::Mesh::CustomTcp::detail::apply_connection_options(fds[0], opts);
    const int after = getsockopt_int(fds[0], SOL_SOCKET, SO_RCVBUF);

    ASSERT_GT(before, 0);
    EXPECT_GT(after, before) << "the shared applier must honour recv_buffer_bytes";

    ::close(fds[0]);
    ::close(fds[1]);
}
