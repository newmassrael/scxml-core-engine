// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Do not edit — regenerate from the source SCXML file.

package procedure_diamond

// ProcedureResult holds the outcome of a procedure execution.
type ProcedureResult struct {
	Completed  bool
	FinalState string
}

var stateNames = [6]string{ "classify", "high_path", "mid_path", "low_path", "accept", "reject" }

// Execute runs the procedure to completion and returns the final state reached.
func Execute(sensorValue uint16, mode string) ProcedureResult {
	current := 0
	for iterations := 0; iterations < 6; iterations++ {
		next := -1
		switch current {
		case 0:
			if sensorValue > 1000 {
				next = 1
			} else if sensorValue > 500 {
				next = 2
			} else {
				next = 3
			}
		case 1:
			if mode == "strict" {
				next = 5
			} else {
				next = 4
			}
		case 2:
			next = 4
		case 3:
			next = 4
		}
		if next < 0 {
			break
		}
		current = next
		if current == 4 || current == 5 {
			break
		}
	}
	completed := current == 4 || current == 5
	return ProcedureResult{Completed: completed, FinalState: stateNames[current]}
}