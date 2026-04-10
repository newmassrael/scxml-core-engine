// sce_forge_runtime — observer
// Building blocks for the observer kind. See SCE_FORGE.md §4.11.
//
//   ThresholdState         — single-bit hysteresis bookkeeping
//   Event<Domain>          — domain-tagged event value (cross-file safety)
//   EventQueue<Domain, N>  — fixed-capacity FIFO returned from update()

#pragma once

#include <array>
#include <cstddef>

namespace sce::forge {

/// Models a 1-bit hysteresis state machine. The generated update() loop calls
/// `enterIf(highCondition)` and `leaveIf(lowCondition)`; both methods return
/// `true` exactly when a transition actually occurred, so the generated code
/// can push the corresponding event without re-checking state.
class ThresholdState {
public:
    bool enterIf(bool condition) noexcept {
        if (!active_ && condition) {
            active_ = true;
            return true;
        }
        return false;
    }

    bool leaveIf(bool condition) noexcept {
        if (active_ && condition) {
            active_ = false;
            return true;
        }
        return false;
    }

    bool active() const noexcept { return active_; }
    void reset() noexcept { active_ = false; }

private:
    bool active_ = false;
};

/// Domain-tagged event value. `Domain` must be a user-defined struct that
/// declares a nested `enum Tag { ... };` listing all event names valid in
/// the domain. Different domains produce incompatible Event<> types — this
/// is the static type-safety mechanism for cross-file event composition.
template <typename Domain>
struct Event {
    typename Domain::Tag tag;

    constexpr Event() noexcept : tag{} {}
    constexpr Event(typename Domain::Tag t) noexcept : tag(t) {}
    constexpr operator typename Domain::Tag() const noexcept { return tag; }
};

/// Fixed-capacity FIFO of Event<Domain>. Returned by value from observer
/// update() methods. Default capacity (8) covers one push per monitor for the
/// monitor counts seen in current generated code; generators may instantiate
/// with a larger capacity if needed.
template <typename Domain, std::size_t Capacity = 8>
class EventQueue {
public:
    using value_type = Event<Domain>;

    bool push(typename Domain::Tag tag) noexcept {
        if (size_ >= Capacity) return false;
        buffer_[size_++] = Event<Domain>(tag);
        return true;
    }

    std::size_t size() const noexcept { return size_; }
    bool empty() const noexcept { return size_ == 0; }
    constexpr std::size_t capacity() const noexcept { return Capacity; }

    const value_type &operator[](std::size_t i) const noexcept { return buffer_[i]; }
    value_type &operator[](std::size_t i) noexcept { return buffer_[i]; }

    const value_type *begin() const noexcept { return buffer_.data(); }
    const value_type *end() const noexcept { return buffer_.data() + size_; }

    void clear() noexcept { size_ = 0; }

private:
    std::array<value_type, Capacity> buffer_{};
    std::size_t size_ = 0;
};

} // namespace sce::forge
