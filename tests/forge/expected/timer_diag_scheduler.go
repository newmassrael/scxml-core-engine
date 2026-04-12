// SCE Forge: Auto-generated from Extended SCXML (sce:kind="timer")
// Runtime: sce_forge_runtime::hal
// Do not edit — regenerate from the source SCXML file.

package timer_diag_scheduler

import "github.com/newmassrael/sce-forge-runtime/timer"

// TimerDiagSchedulerHandler is the user-supplied callback interface.
// Missing methods produce a compile error at the constructor call site —
// Go's structural interface satisfaction is checked when NewTimerDiagScheduler
// is invoked, so there is no silent fallback.
type TimerDiagSchedulerHandler interface {
	FireTesterPresent()
	FireHandleTimeout()
	FireRetrySecurityAccess()
}

type TimerDiagScheduler struct {
	handler TimerDiagSchedulerHandler
	testerPresentTimer timer.Timer
	responseTimeoutTimer timer.Timer
	retryDelayTimer timer.Timer
}

func NewTimerDiagScheduler(
	handler TimerDiagSchedulerHandler,
	testerPresentTimer timer.Timer,
	responseTimeoutTimer timer.Timer,
	retryDelayTimer timer.Timer,
) *TimerDiagScheduler {
	return &TimerDiagScheduler{
		handler: handler,
		testerPresentTimer: testerPresentTimer,
		responseTimeoutTimer: responseTimeoutTimer,
		retryDelayTimer: retryDelayTimer,
	}
}

func (s *TimerDiagScheduler) StartTesterPresent() {
	s.testerPresentTimer.StartPeriodic(2000, func() { s.handler.FireTesterPresent() })
}

func (s *TimerDiagScheduler) CancelTesterPresent() {
	s.testerPresentTimer.Cancel()
}

func (s *TimerDiagScheduler) StartResponseTimeout() {
	s.responseTimeoutTimer.StartOneShot(5000, func() { s.handler.FireHandleTimeout() })
}

func (s *TimerDiagScheduler) CancelResponseTimeout() {
	s.responseTimeoutTimer.Cancel()
}

func (s *TimerDiagScheduler) StartRetryDelay() {
	s.retryDelayTimer.StartOneShot(10000, func() { s.handler.FireRetrySecurityAccess() })
}

func (s *TimerDiagScheduler) CancelRetryDelay() {
	s.retryDelayTimer.Cancel()
}
