// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Do not edit — regenerate from the source SCXML file.

package procedure_startup_check

// ProcedureResult holds the outcome of a procedure execution.
type ProcedureResult struct {
	Completed  bool
	FinalState string
}

var stateNames = [5]string{ "check_voltage", "check_temp", "success", "fail_voltage", "fail_overtemp" }

// Execute runs the procedure to completion and returns the final state reached.
func Execute(voltage float32, temperature float32) ProcedureResult {
	current := 0
	for iterations := 0; iterations < 5; iterations++ {
		next := -1
		switch current {
		case 0:
			if voltage >= 11.5 && voltage <= 14.5 {
				next = 1
			} else {
				next = 3
			}
		case 1:
			if temperature < 80.0 {
				next = 2
			} else {
				next = 4
			}
		}
		if next < 0 {
			break
		}
		current = next
		if current == 2 || current == 3 || current == 4 {
			break
		}
	}
	completed := current == 2 || current == 3 || current == 4
	return ProcedureResult{Completed: completed, FinalState: stateNames[current]}
}