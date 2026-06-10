// SCE-MAP: timer_diag_scheduler:1

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="timer")
// Shape: watching-zenoh RFC §synth-5-D line 880-886 — single timer per
// doc with event-driven reset / state-exit cancel / fire event.
// Runtime: sce_forge_runtime::hal
// Do not edit — regenerate from the source SCXML file.

package timer_diag_scheduler

import "github.com/newmassrael/sce-forge-runtime/timer"

// Period configured at compile time from `<sce:period>`. Microseconds
// (uint64) cover MCU microsecond ticks through minute-scale watchdogs
// in one type.
const (
	PeriodUs uint64 = 2000000
	PeriodMs uint32 = 2000
	ResetOnEvent string = "diag.heartbeat"
	CancelOnStateExit string = "diag.idle"
)

// TimerDiagSchedulerHandler is the user-supplied fire callback interface.
type TimerDiagSchedulerHandler interface {
	FireDiagTick()
}

type TimerDiagScheduler struct {
	handler TimerDiagSchedulerHandler
	timer   timer.Timer
}

func NewTimerDiagScheduler(handler TimerDiagSchedulerHandler, t timer.Timer) *TimerDiagScheduler {
	return &TimerDiagScheduler{handler: handler, timer: t}
}

// Start the periodic timer at compile-time `PeriodMs`.
func (s *TimerDiagScheduler) Start() {
	s.timer.StartPeriodic(PeriodMs, func() { s.handler.FireDiagTick() })
}

// Cancel the timer. Idempotent per the runtime contract.
func (s *TimerDiagScheduler) Cancel() {
	s.timer.Cancel()
}

// `<sce:reset-on event="diag.heartbeat"/>` consumer hook.
func (s *TimerDiagScheduler) OnResetDiagHeartbeat() {
	s.Cancel()
	s.Start()
}

// `<sce:cancel-on state-exit="diag.idle"/>` consumer hook.
func (s *TimerDiagScheduler) OnCancelDiagIdleExit() {
	s.Cancel()
}
