// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §16.8.1 — distributed conformance partition runner registry.
//
// Mirrors the shape of `tests/w3c/aot_tests/AotTestRegistry.h` (keyed by
// test-id string) but widens the key to (test_id, partition_name) so a
// single process binary can dispatch to the right partition's entry
// point. In practice, each per-partition binary compiles the runner
// source with exactly one `Test<seed>_<partition>.h` header included,
// so the registry ends up holding one entry per process — the map
// form exists so adding a new partition is "drop a header and link"
// rather than "edit the dispatcher".
//
// Namespace scope: the codegen baked into `testXXX_sm.h` emits
// `namespace SCE::Generated::testXXX::testXXX`. Two partitions of the
// same test generate headers with the SAME namespace but DIFFERENT
// class bodies (Root vs NonRoot branches). Including both headers in
// one TU is an ODR violation — so the seed harness follows the
// rule-12 fork+exec precedent (§16.8 arch-debt #4 remains gated on
// per-partition namespace baking). The runtime dispatch through this
// registry therefore targets one entry per binary; runtime selection
// across partitions requires a second resolved via fork+exec.

#pragma once

#include <functional>
#include <map>
#include <memory>
#include <string>
#include <utility>

namespace SCE::W3C::Dist {

/// Per-partition runnable unit. `run(deploy_dir)` returns the process
/// exit code: 0 on success, non-zero on a partition-local assertion
/// failure or transport error. `deploy_dir` is the build-tree
/// directory that holds the generated `testXXX.scxml` + deploy.yaml
/// + partition-specific generated headers; partition implementations
/// pass it through to transport ctor arguments that expect to find
/// peer shm segments in a predictable location.
class PartitionBase {
public:
    virtual ~PartitionBase() = default;
    virtual int run(const std::string &deploy_dir) = 0;
};

/// Singleton registry of (test_id, partition_name) → factory. Static
/// registration is the pattern `AotTestRegistrar` uses for the W3C
/// AOT suite; the key widening to a pair is the only semantic change
/// because one physical binary may carry many partitions of
/// different tests (future S5+ once textbook debt #4 closes) or
/// today's one-entry-per-binary fork+exec shape.
class PartitionRegistry {
public:
    using Key = std::pair<std::string, std::string>;
    using Factory = std::function<std::unique_ptr<PartitionBase>()>;

    static PartitionRegistry &instance() {
        static PartitionRegistry r;
        return r;
    }

    void registerPartition(std::string test_id, std::string partition, Factory factory) {
        entries_[Key{std::move(test_id), std::move(partition)}] = std::move(factory);
    }

    std::unique_ptr<PartitionBase> create(const std::string &test_id,
                                          const std::string &partition) const {
        auto it = entries_.find(Key{test_id, partition});
        return it != entries_.end() ? it->second() : nullptr;
    }

private:
    PartitionRegistry() = default;
    std::map<Key, Factory> entries_;
};

/// Static-init CRTP registrar. A `Test<seed>_<partition>.h` header
/// drops one `inline static PartitionRegistrar<T> registrar_T;` line
/// at namespace scope, and inclusion of the header from the runner
/// TU registers the entry before `main()`. The `T::TEST_ID` and
/// `T::PARTITION` constants supply the registry key — matching the
/// manifest's partition names avoids a second mapping layer.
template <typename T> struct PartitionRegistrar {
    PartitionRegistrar() {
        PartitionRegistry::instance().registerPartition(
            T::TEST_ID, T::PARTITION, []() { return std::make_unique<T>(); });
    }
};

}  // namespace SCE::W3C::Dist
