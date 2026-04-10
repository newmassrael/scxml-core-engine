// SCE Forge: Auto-generated from Extended SCXML (sce:kind="timer")
// Do not edit — regenerate from the source SCXML file.

package timer_diag_scheduler

// Timer is the platform timer interface (injected at runtime).
type Timer interface {
	StartPeriodic(intervalMs uint32, callback func())
	StartOneShot(delayMs uint32, callback func())
	Cancel()
}

type TimerDiagScheduler struct {
	TesterPresentTimer Timer
	ResponseTimeoutTimer Timer
	RetryDelayTimer Timer
}

func (s *TimerDiagScheduler) StartTesterPresent() {
	if s.TesterPresentTimer != nil {
		s.TesterPresentTimer.StartPeriodic(2000, func() { s.onTesterPresent() })
	}
}

func (s *TimerDiagScheduler) CancelTesterPresent() {
	if s.TesterPresentTimer != nil {
		s.TesterPresentTimer.Cancel()
	}
}

func (s *TimerDiagScheduler) StartResponseTimeout() {
	if s.ResponseTimeoutTimer != nil {
		s.ResponseTimeoutTimer.StartOneShot(5000, func() { s.onHandleTimeout() })
	}
}

func (s *TimerDiagScheduler) CancelResponseTimeout() {
	if s.ResponseTimeoutTimer != nil {
		s.ResponseTimeoutTimer.Cancel()
	}
}

func (s *TimerDiagScheduler) StartRetryDelay() {
	if s.RetryDelayTimer != nil {
		s.RetryDelayTimer.StartOneShot(10000, func() { s.onRetrySecurityAccess() })
	}
}

func (s *TimerDiagScheduler) CancelRetryDelay() {
	if s.RetryDelayTimer != nil {
		s.RetryDelayTimer.Cancel()
	}
}

func (s *TimerDiagScheduler) onTesterPresent() { /* platform callback */ }

func (s *TimerDiagScheduler) onHandleTimeout() { /* platform callback */ }

func (s *TimerDiagScheduler) onRetrySecurityAccess() { /* platform callback */ }
