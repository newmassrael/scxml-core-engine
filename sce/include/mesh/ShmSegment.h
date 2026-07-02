// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh ShmSegment — POSIX shared memory RAII wrapper.
//
// Provides create (producer) and open (consumer) factory functions.
// The creator owns the segment and calls shm_unlink on destruction.
// The opener only munmaps — it does not unlink or destroy contents.

#pragma once

#include <cstddef>
#include <string>

#include <fcntl.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <unistd.h>

namespace SCE::Mesh {

/// RAII wrapper for a POSIX shared memory segment.
///
/// Move-only. The creator (Mode::Create) calls shm_unlink on destruction;
/// the opener (Mode::Open) only munmaps.
class ShmSegment {
    int fd_ = -1;
    void *addr_ = nullptr;
    std::size_t size_ = 0;
    bool owner_ = false;
    std::string name_;

    ShmSegment() = default;

public:
    /// Create a new shared memory segment (producer/sender side).
    /// Truncates to `size` bytes and memory-maps it read/write.
    [[nodiscard]] static ShmSegment create(const char *name, std::size_t size) noexcept {
        ShmSegment seg;
        seg.size_ = size;

        seg.fd_ = ::shm_open(name, O_CREAT | O_RDWR, 0666);
        if (seg.fd_ < 0) {
            // Never created the segment — leave owner_=false so the
            // destructor does not shm_unlink a name we do not own.
            return seg;
        }

        // From here on we own /name. Claim it so destructor cleans up
        // on any subsequent mid-construction failure.
        seg.owner_ = true;
        seg.name_ = name;

        if (::ftruncate(seg.fd_, static_cast<off_t>(size)) != 0) {
            ::close(seg.fd_);
            ::shm_unlink(name);
            seg.fd_ = -1;
            seg.owner_ = false;
            seg.name_.clear();
            return seg;
        }

        seg.addr_ = ::mmap(nullptr, size, PROT_READ | PROT_WRITE, MAP_SHARED, seg.fd_, 0);
        if (seg.addr_ == MAP_FAILED) {
            seg.addr_ = nullptr;
            ::close(seg.fd_);
            ::shm_unlink(name);
            seg.fd_ = -1;
            seg.owner_ = false;
            seg.name_.clear();
        }
        return seg;
    }

    /// Open an existing shared memory segment (consumer/receiver side).
    [[nodiscard]] static ShmSegment open(const char *name, std::size_t size) noexcept {
        ShmSegment seg;
        seg.name_ = name;
        seg.size_ = size;
        seg.owner_ = false;

        seg.fd_ = ::shm_open(name, O_RDWR, 0);
        if (seg.fd_ < 0) {
            return seg;
        }

        seg.addr_ = ::mmap(nullptr, size, PROT_READ | PROT_WRITE, MAP_SHARED, seg.fd_, 0);
        if (seg.addr_ == MAP_FAILED) {
            seg.addr_ = nullptr;
            ::close(seg.fd_);
            seg.fd_ = -1;
        }
        return seg;
    }

    ~ShmSegment() {
        if (addr_) {
            ::munmap(addr_, size_);
        }
        if (fd_ >= 0) {
            ::close(fd_);
        }
        if (owner_ && !name_.empty()) {
            ::shm_unlink(name_.c_str());
        }
    }

    // Move-only
    ShmSegment(ShmSegment &&o) noexcept
        : fd_(o.fd_), addr_(o.addr_), size_(o.size_), owner_(o.owner_), name_(std::move(o.name_)) {
        o.fd_ = -1;
        o.addr_ = nullptr;
        o.size_ = 0;
        o.owner_ = false;
    }

    ShmSegment &operator=(ShmSegment &&o) noexcept {
        if (this != &o) {
            if (addr_) {
                ::munmap(addr_, size_);
            }
            if (fd_ >= 0) {
                ::close(fd_);
            }
            if (owner_ && !name_.empty()) {
                ::shm_unlink(name_.c_str());
            }

            fd_ = o.fd_;
            addr_ = o.addr_;
            size_ = o.size_;
            owner_ = o.owner_;
            name_ = std::move(o.name_);

            o.fd_ = -1;
            o.addr_ = nullptr;
            o.size_ = 0;
            o.owner_ = false;
        }
        return *this;
    }

    ShmSegment(const ShmSegment &) = delete;
    ShmSegment &operator=(const ShmSegment &) = delete;

    [[nodiscard]] void *data() noexcept {
        return addr_;
    }

    [[nodiscard]] const void *data() const noexcept {
        return addr_;
    }

    [[nodiscard]] std::size_t size() const noexcept {
        return size_;
    }

    [[nodiscard]] bool valid() const noexcept {
        return addr_ != nullptr;
    }

    [[nodiscard]] bool is_owner() const noexcept {
        return owner_;
    }
};

}  // namespace SCE::Mesh
