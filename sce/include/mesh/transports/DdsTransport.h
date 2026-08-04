// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh DDS transport: Eclipse Cyclone DDS carrying CBOR MeshEnvelopes as
// an opaque octet sequence.
//
// Wire format: every pattern rides ONE IDL type — `sce_mesh::Envelope`, a
// bare `sequence<octet>` holding the CBOR MeshEnvelope (SCE_MESH.md 7.5). This is
// the deliberate divergence from OMG DDS-RPC, which defines an IDL interface
// per service and regenerates code whenever a service changes: here the
// service vocabulary lives in the envelope's `type` field, so adding an
// event or a whole machine changes no IDL and triggers no regeneration.
//
// Topic triple, derived from the binding's single `topic:` name so an author
// cannot pair a request topic with an unrelated reply topic:
//   <topic>          request leg  (client → server)
//   <topic>_Reply    reply leg    (server → client)
//   <topic>_Event    notification (server → subscribers)
//
// QoS is derived from what each leg means rather than left to the deployment
// (SCE_MESH.md 8.1):
//   * request / reply — RELIABLE + KEEP_ALL + VOLATILE. A late-joining
//     server must NOT be handed a backlog of stale requests, so the
//     durability that helps notifications would be wrong here.
//   * notification   — RELIABLE + KEEP_LAST(1) + TRANSIENT_LOCAL. A field
//     notification is a current-value semantic: a subscriber that joins
//     after the last publish still needs that value, which is exactly what
//     transient-local depth 1 delivers. This is the DDS-native expression of
//     the same "latest value" contract SOME/IP fields carry.
//   * every reader   — IGNORE_LOCAL(participant). Without it a device reads
//     its own writes: one participant hosts every binding on the device, so
//     a client and a server on the same device would each receive the
//     other's traffic AND their own. This is a correctness gate, not a
//     tuning knob.
//
// Conformance (SCE Mesh SCE_MESH.md 10.4):
//   * Per-sender FIFO     — RELIABLE writer preserves per-writer order;
//                           `supplies_ordering` stays false because DDS
//                           orders per writer, not per logical sender across
//                           reconnects, so ordered bindings still layer the
//                           runtime buffer.
//   * At-least-once       — RELIABLE reliability
//   * Duplicate tolerance — `supplies_dedup` is false; the SCE_MESH.md 10.5
//                           DedupRouter admits inbound envelopes.
//   * Fault signal        — decode failures reach `DecodeErrorCallback`;
//                           participant creation failure is visible through
//                           `valid()`.
//
// Threading model: one drain thread per reader, mirroring custom_tcp's
// one-read-thread-per-connection. The receive callback runs on that thread;
// dispatch to the engine is the caller's concern, exactly as for the other
// transports.

#pragma once

#include "mesh/MeshEnvelope.h"
#include "mesh/MeshEnvelopeCodec.h"

// idlcxx-generated from sce/idl/sce_mesh.idl. The generated header is a build
// artifact; CMake puts its directory on the include path for any target that
// links the generated types library.
#include "sce_mesh.hpp"

#include <dds/dds.hpp>

#include <atomic>
#include <chrono>
#include <cstdint>
#include <functional>
#include <memory>
#include <mutex>
#include <optional>
#include <string>
#include <thread>
#include <utility>
#include <vector>

namespace SCE::Mesh::Dds {

/// Decode-error callback: invoked on a reader's drain thread when an inbound
/// sample's CBOR decode fails. Codegen wires this to
/// `raiseCommunicationError(ENVELOPE_CORRUPT, transport="dds")` so the
/// SCE_MESH.md 16.7 row 4 catalog row fires here the same way it does at codegen
/// `decodeEnvelope` sites. A single bad envelope does not tear down the
/// reader — the drain thread continues with the next sample.
using DecodeErrorCallback = std::function<void()>;

/// Receive callback: invoked once per successfully decoded envelope.
///
/// Unlike custom_tcp there is no peer-link parameter, because a DDS reply
/// does not travel back up the path the request arrived on — it is published
/// on the reply topic and correlated by `correlation_id` (SCE_MESH.md 8.3). There
/// is therefore nothing per-arrival for the callback to hold onto.
using ReceiveCallback = std::function<void(const SCE::Mesh::MeshEnvelope &)>;

namespace detail {

/// Reply-leg topic for a binding's base topic name.
inline std::string replyTopicName(const std::string &base) {
    return base + "_Reply";
}

/// Notification-leg topic for a binding's base topic name.
inline std::string eventTopicName(const std::string &base) {
    return base + "_Event";
}

/// QoS for the request and reply legs. KEEP_ALL because dropping a request
/// under load would surface as a lost RPC rather than as backpressure.
inline dds::pub::qos::DataWriterQos requestWriterQos(const dds::pub::Publisher &pub) {
    auto qos = pub.default_datawriter_qos();
    qos << dds::core::policy::Reliability::Reliable() << dds::core::policy::History::KeepAll();
    return qos;
}

/// QoS for the notification leg — see the header comment on why this one
/// differs from the request/reply legs.
inline dds::pub::qos::DataWriterQos notifyWriterQos(const dds::pub::Publisher &pub) {
    auto qos = pub.default_datawriter_qos();
    qos << dds::core::policy::Reliability::Reliable() << dds::core::policy::History::KeepLast(1)
        << dds::core::policy::Durability::TransientLocal();
    return qos;
}

inline dds::sub::qos::DataReaderQos requestReaderQos(const dds::sub::Subscriber &sub) {
    auto qos = sub.default_datareader_qos();
    qos << dds::core::policy::Reliability::Reliable() << dds::core::policy::History::KeepAll()
        << dds::core::policy::IgnoreLocal::Participant();
    return qos;
}

inline dds::sub::qos::DataReaderQos notifyReaderQos(const dds::sub::Subscriber &sub) {
    auto qos = sub.default_datareader_qos();
    qos << dds::core::policy::Reliability::Reliable() << dds::core::policy::History::KeepLast(1)
        << dds::core::policy::Durability::TransientLocal() << dds::core::policy::IgnoreLocal::Participant();
    return qos;
}

/// One reader plus the thread that drains it.
///
/// A waitset blocks the thread until data arrives or the poll interval
/// elapses; the interval bounds shutdown latency, so a stopping pump never
/// waits on a topic that may never see another sample again.
class ReaderPump {
public:
    ReaderPump(dds::sub::DataReader<sce_mesh::Envelope> reader, ReceiveCallback on_receive,
               std::shared_ptr<std::atomic<bool>> decode_error_flag)
        : reader_(std::move(reader)), on_receive_(std::move(on_receive)), decode_error_(std::move(decode_error_flag)) {
        thread_ = std::thread([this] { drain(); });
    }

    ReaderPump(const ReaderPump &) = delete;
    ReaderPump &operator=(const ReaderPump &) = delete;

    ~ReaderPump() {
        stop();
    }

    void stop() {
        bool expected = true;
        if (!running_.compare_exchange_strong(expected, false)) {
            return;
        }
        if (thread_.joinable()) {
            thread_.join();
        }
    }

    void setDecodeErrorHandler(DecodeErrorCallback handler) {
        std::lock_guard<std::mutex> lock(handler_mutex_);
        on_decode_error_ = std::move(handler);
    }

    /// True once at least one remote writer has matched this reader.
    [[nodiscard]] bool matched() {
        return reader_.subscription_matched_status().current_count() > 0;
    }

private:
    static constexpr auto kPollInterval = std::chrono::milliseconds(20);

    void drain() {
        dds::core::cond::WaitSet waitset;
        dds::sub::cond::ReadCondition condition(reader_, dds::sub::status::DataState::new_data());
        waitset.attach_condition(condition);

        while (running_.load(std::memory_order_acquire)) {
            try {
                waitset.wait(dds::core::Duration::from_millisecs(kPollInterval.count()));
            } catch (const dds::core::TimeoutError &) {
                // No data this interval — re-check `running_` and wait again.
                continue;
            } catch (const dds::core::Exception &) {
                // The participant is going away underneath us (shutdown
                // races the drain thread). Stop rather than spin.
                return;
            }
            deliverAvailable();
        }
    }

    void deliverAvailable() {
        dds::sub::LoanedSamples<sce_mesh::Envelope> samples;
        try {
            samples = reader_.take();
        } catch (const dds::core::Exception &) {
            return;
        }
        for (const auto &sample : samples) {
            if (!sample.info().valid()) {
                continue;  // dispose / unregister notification, not payload
            }
            const auto &bytes = sample.data().payload();
            SCE::Mesh::MeshEnvelope env;
            if (!SCE::Mesh::decodeEnvelope(bytes.data(), bytes.size(), env)) {
                raiseDecodeError();
                continue;
            }
            if (on_receive_) {
                on_receive_(env);
            }
        }
    }

    void raiseDecodeError() {
        DecodeErrorCallback handler;
        {
            std::lock_guard<std::mutex> lock(handler_mutex_);
            handler = on_decode_error_;
        }
        if (handler) {
            handler();
        }
        if (decode_error_) {
            decode_error_->store(true, std::memory_order_release);
        }
    }

    dds::sub::DataReader<sce_mesh::Envelope> reader_;
    ReceiveCallback on_receive_;
    std::shared_ptr<std::atomic<bool>> decode_error_;
    DecodeErrorCallback on_decode_error_;
    std::mutex handler_mutex_;
    std::atomic<bool> running_{true};
    std::thread thread_;
};

/// Write one envelope through a writer, encoding it as the opaque payload.
inline bool writeEnvelope(dds::pub::DataWriter<sce_mesh::Envelope> &writer, const SCE::Mesh::MeshEnvelope &env) {
    try {
        auto bytes = SCE::Mesh::encodeEnvelope(env);
        writer.write(sce_mesh::Envelope(std::vector<uint8_t>(bytes.begin(), bytes.end())));
        return true;
    } catch (const dds::core::Exception &) {
        return false;
    }
}

/// Poll a writer's publication-matched status until a reader appears.
inline bool waitForWriterMatch(dds::pub::DataWriter<sce_mesh::Envelope> &writer, std::chrono::milliseconds timeout) {
    const auto deadline = std::chrono::steady_clock::now() + timeout;
    while (std::chrono::steady_clock::now() < deadline) {
        try {
            if (writer.publication_matched_status().current_count() > 0) {
                return true;
            }
        } catch (const dds::core::Exception &) {
            return false;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(2));
    }
    return false;
}

}  // namespace detail

/// Device-shared DDS participant.
///
/// One per device, matching the deploy.yaml `transports.dds:` block. Every
/// binding on the device publishes and subscribes through this participant,
/// which is what makes `IGNORE_LOCAL(participant)` the right granularity:
/// "local" means "this device", which is exactly the traffic that must not
/// come back in.
///
/// `config` is the deploy.yaml `transports.dds.config:` value, forwarded to
/// the participant constructor that CycloneDDS-CXX provides for exactly this
/// purpose: Cyclone reads it as a file name, or as XML text when it starts
/// with '<'. Passing it here rather than exporting `CYCLONEDDS_URI` keeps the
/// deployment description in one file, and is what lets two processes on one
/// host run with different Cyclone configurations. `domain_id` still wins over
/// any domain id inside the file — Cyclone's rule, which is why the two are
/// not alternatives.
///
/// A default-constructed `config` selects Cyclone's own resolution order
/// (`CYCLONEDDS_URI`, then built-in defaults), which is the pre-existing
/// behaviour and the one every deployment that declares no config keeps.
class Participant {
public:
    explicit Participant(std::uint32_t domain_id, const char *config = nullptr) {
        try {
            if (config != nullptr && *config != '\0') {
                // The 5-argument form is the 1-argument constructor's own
                // body plus the config string (TDomainParticipantImpl.hpp),
                // so the defaults below are not choices — they are what the
                // short form already passes.
                participant_.emplace(
                    domain_id, org::eclipse::cyclonedds::domain::DomainParticipantDelegate::default_participant_qos(),
                    nullptr, dds::core::status::StatusMask::none(), std::string(config));
            } else {
                participant_.emplace(domain_id);
            }
            publisher_.emplace(*participant_);
            subscriber_.emplace(*participant_);
            valid_ = true;
        } catch (const dds::core::Exception &) {
            valid_ = false;
        }
    }

    Participant(const Participant &) = delete;
    Participant &operator=(const Participant &) = delete;

    /// False when the domain could not be joined. Callers must check before
    /// constructing endpoints on it.
    [[nodiscard]] bool valid() const noexcept {
        return valid_;
    }

    [[nodiscard]] dds::domain::DomainParticipant &raw() {
        return *participant_;
    }

    [[nodiscard]] dds::pub::Publisher &publisher() {
        return *publisher_;
    }

    [[nodiscard]] dds::sub::Subscriber &subscriber() {
        return *subscriber_;
    }

private:
    std::optional<dds::domain::DomainParticipant> participant_;
    std::optional<dds::pub::Publisher> publisher_;
    std::optional<dds::sub::Subscriber> subscriber_;
    bool valid_ = false;
};

/// Client role for one binding: writes requests, reads replies, and — once
/// subscribed — reads notifications.
///
/// The notification reader is created on `subscribe()` and destroyed on
/// `unsubscribe()`, so an unsubscribed client holds no notification-side
/// resources and a publisher sees the subscription disappear through DDS
/// discovery rather than through an application-level unsubscribe message.
class Client {
public:
    Client(Participant &participant, const std::string &topic, ReceiveCallback on_receive)
        : participant_(participant), topic_name_(topic), on_receive_(std::move(on_receive)) {
        if (!participant.valid()) {
            return;
        }
        try {
            request_topic_.emplace(participant.raw(), topic_name_);
            reply_topic_.emplace(participant.raw(), detail::replyTopicName(topic_name_));
            request_writer_.emplace(participant.publisher(), *request_topic_,
                                    detail::requestWriterQos(participant.publisher()));
            reply_pump_ = std::make_unique<detail::ReaderPump>(
                dds::sub::DataReader<sce_mesh::Envelope>(participant.subscriber(), *reply_topic_,
                                                         detail::requestReaderQos(participant.subscriber())),
                on_receive_, decode_error_);
            valid_ = true;
        } catch (const dds::core::Exception &) {
            valid_ = false;
        }
    }

    Client(const Client &) = delete;
    Client &operator=(const Client &) = delete;

    ~Client() {
        shutdown();
    }

    [[nodiscard]] bool valid() const noexcept {
        return valid_;
    }

    /// Publish one envelope on the request leg.
    [[nodiscard]] bool send(const SCE::Mesh::MeshEnvelope &env) {
        if (!valid_) {
            return false;
        }
        return detail::writeEnvelope(*request_writer_, env);
    }

    /// Block until a server's request reader has matched, bounded by
    /// `timeout`. DDS discovery is asynchronous: a write issued before the
    /// match completes is dropped by a VOLATILE writer with no error, so a
    /// caller that needs its first request delivered has to gate on this.
    [[nodiscard]] bool waitForServer(std::chrono::milliseconds timeout) {
        return valid_ && detail::waitForWriterMatch(*request_writer_, timeout);
    }

    /// Start reading the notification leg. Idempotent — a second call while
    /// already subscribed is a no-op, matching the two-level subscription
    /// gating in generated code (SCE_MESH.md 10.9 invariant 6).
    [[nodiscard]] bool subscribe() {
        if (!valid_) {
            return false;
        }
        std::lock_guard<std::mutex> lock(notify_mutex_);
        if (notify_pump_) {
            return true;
        }
        try {
            notify_topic_.emplace(participant_.raw(), detail::eventTopicName(topic_name_));
            notify_pump_ = std::make_unique<detail::ReaderPump>(
                dds::sub::DataReader<sce_mesh::Envelope>(participant_.subscriber(), *notify_topic_,
                                                         detail::notifyReaderQos(participant_.subscriber())),
                on_receive_, decode_error_);
            return true;
        } catch (const dds::core::Exception &) {
            notify_pump_.reset();
            return false;
        }
    }

    /// Stop reading the notification leg and release its reader. The
    /// publisher observes the unsubscribe as a DDS discovery event; there is
    /// no unsubscribe message on the wire to lose.
    void unsubscribe() {
        std::unique_ptr<detail::ReaderPump> pump;
        {
            std::lock_guard<std::mutex> lock(notify_mutex_);
            pump = std::move(notify_pump_);
        }
        // Joined outside the lock so a drain thread that is mid-callback
        // cannot deadlock against a subscribe() on another thread.
        pump.reset();
    }

    void setDecodeErrorHandler(DecodeErrorCallback handler) {
        if (reply_pump_) {
            reply_pump_->setDecodeErrorHandler(handler);
        }
        std::lock_guard<std::mutex> lock(notify_mutex_);
        if (notify_pump_) {
            notify_pump_->setDecodeErrorHandler(std::move(handler));
        }
    }

    void shutdown() {
        unsubscribe();
        if (reply_pump_) {
            reply_pump_->stop();
            reply_pump_.reset();
        }
        valid_ = false;
    }

private:
    Participant &participant_;
    std::string topic_name_;
    ReceiveCallback on_receive_;
    std::shared_ptr<std::atomic<bool>> decode_error_ = std::make_shared<std::atomic<bool>>(false);
    std::optional<dds::topic::Topic<sce_mesh::Envelope>> request_topic_;
    std::optional<dds::topic::Topic<sce_mesh::Envelope>> reply_topic_;
    std::optional<dds::topic::Topic<sce_mesh::Envelope>> notify_topic_;
    std::optional<dds::pub::DataWriter<sce_mesh::Envelope>> request_writer_;
    std::unique_ptr<detail::ReaderPump> reply_pump_;
    std::unique_ptr<detail::ReaderPump> notify_pump_;
    std::mutex notify_mutex_;
    bool valid_ = false;
};

/// Server role for one binding: reads requests, writes replies, publishes
/// notifications.
///
/// There is no per-request state to hold: a reply is addressed by the
/// correlation the server established (SCE_MESH.md 8.3) and travels on the reply
/// topic, so unlike custom_tcp there is no arrival-path handle to stash.
class Server {
public:
    Server(Participant &participant, const std::string &topic, ReceiveCallback on_receive) {
        if (!participant.valid()) {
            return;
        }
        try {
            request_topic_.emplace(participant.raw(), topic);
            reply_topic_.emplace(participant.raw(), detail::replyTopicName(topic));
            notify_topic_.emplace(participant.raw(), detail::eventTopicName(topic));
            reply_writer_.emplace(participant.publisher(), *reply_topic_,
                                  detail::requestWriterQos(participant.publisher()));
            notify_writer_.emplace(participant.publisher(), *notify_topic_,
                                   detail::notifyWriterQos(participant.publisher()));
            request_pump_ = std::make_unique<detail::ReaderPump>(
                dds::sub::DataReader<sce_mesh::Envelope>(participant.subscriber(), *request_topic_,
                                                         detail::requestReaderQos(participant.subscriber())),
                std::move(on_receive), decode_error_);
            valid_ = true;
        } catch (const dds::core::Exception &) {
            valid_ = false;
        }
    }

    Server(const Server &) = delete;
    Server &operator=(const Server &) = delete;

    ~Server() {
        shutdown();
    }

    [[nodiscard]] bool valid() const noexcept {
        return valid_;
    }

    /// Publish a correlated reply on the reply leg.
    [[nodiscard]] bool reply(const SCE::Mesh::MeshEnvelope &env) {
        if (!valid_) {
            return false;
        }
        return detail::writeEnvelope(*reply_writer_, env);
    }

    /// Publish a notification on the event leg. Transient-local depth 1
    /// means a subscriber that joins later still receives this value.
    [[nodiscard]] bool publish(const SCE::Mesh::MeshEnvelope &env) {
        if (!valid_) {
            return false;
        }
        return detail::writeEnvelope(*notify_writer_, env);
    }

    /// True once at least one client's reply reader has matched, so a caller
    /// can gate a first reply the same way `Client::waitForServer` gates a
    /// first request.
    [[nodiscard]] bool waitForClient(std::chrono::milliseconds timeout) {
        return valid_ && detail::waitForWriterMatch(*reply_writer_, timeout);
    }

    void setDecodeErrorHandler(DecodeErrorCallback handler) {
        if (request_pump_) {
            request_pump_->setDecodeErrorHandler(std::move(handler));
        }
    }

    void shutdown() {
        if (request_pump_) {
            request_pump_->stop();
            request_pump_.reset();
        }
        valid_ = false;
    }

private:
    std::shared_ptr<std::atomic<bool>> decode_error_ = std::make_shared<std::atomic<bool>>(false);
    std::optional<dds::topic::Topic<sce_mesh::Envelope>> request_topic_;
    std::optional<dds::topic::Topic<sce_mesh::Envelope>> reply_topic_;
    std::optional<dds::topic::Topic<sce_mesh::Envelope>> notify_topic_;
    std::optional<dds::pub::DataWriter<sce_mesh::Envelope>> reply_writer_;
    std::optional<dds::pub::DataWriter<sce_mesh::Envelope>> notify_writer_;
    std::unique_ptr<detail::ReaderPump> request_pump_;
    bool valid_ = false;
};

}  // namespace SCE::Mesh::Dds
