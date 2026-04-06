/**
 * @file sce_timer.h
 * @brief Pausable timer abstraction for game-managed timers
 *
 * Encapsulates timer state that supports pause/resume semantics.
 * Used by the combo/berserk system where timers must pause when
 * the game menu opens.
 *
 * Design: Pure timing logic - no UI, no state machine coupling.
 */

#ifndef SCE_TIMER_H
#define SCE_TIMER_H

#include <chrono>

class PausableTimer {
public:
    enum class Type { NONE, COMBO, BERSERK };

    void start(Type type, double duration_ms) {
        type_ = type;
        start_ = std::chrono::steady_clock::now();
        duration_ms_ = duration_ms;
        elapsed_before_pause_ms_ = 0;
        paused_ = false;
    }

    void cancel() {
        type_ = Type::NONE;
        duration_ms_ = 0;
        elapsed_before_pause_ms_ = 0;
        paused_ = false;
    }

    void pause() {
        if (type_ == Type::NONE || paused_) return;
        auto now = std::chrono::steady_clock::now();
        elapsed_before_pause_ms_ +=
            std::chrono::duration<double, std::milli>(now - start_).count();
        paused_ = true;
    }

    void resume() {
        if (type_ == Type::NONE || !paused_) return;
        paused_ = false;
        start_ = std::chrono::steady_clock::now();
    }

    bool isActive() const { return type_ != Type::NONE; }
    bool isPaused() const { return paused_; }
    Type getType() const { return type_; }
    double getDurationMs() const { return duration_ms_; }

    double getRemainingMs() const {
        if (type_ == Type::NONE) return 0;
        double remaining = duration_ms_ - getElapsedMs();
        return (remaining > 0) ? remaining : 0;
    }

    bool isExpired() const {
        return isActive() && !isPaused() && (duration_ms_ - getElapsedMs()) <= 0;
    }

private:
    double getElapsedMs() const {
        double elapsed = elapsed_before_pause_ms_;
        if (!paused_) {
            auto now = std::chrono::steady_clock::now();
            elapsed +=
                std::chrono::duration<double, std::milli>(now - start_).count();
        }
        return elapsed;
    }

    Type type_ = Type::NONE;
    std::chrono::steady_clock::time_point start_;
    double duration_ms_ = 0;
    double elapsed_before_pause_ms_ = 0;
    bool paused_ = false;
};

#endif /* SCE_TIMER_H */
