// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Do not edit — regenerate from the source SCXML file.

package procedure_linear

// ProcedureResult holds the outcome of a procedure execution.
type ProcedureResult struct {
	Completed  bool
	FinalState string
}

var stateNames = [4]string{ "stage_a", "stage_b", "stage_c", "done" }

// Execute runs the procedure to completion and returns the final state reached.
func Execute(value int32) ProcedureResult {
	current := 0
	for iterations := 0; iterations < 4; iterations++ {
		next := -1
		switch current {
		case 0:
			next = 1
		case 1:
			next = 2
		case 2:
			next = 3
		}
		if next < 0 {
			break
		}
		current = next
		if current == 3 {
			break
		}
	}
	completed := current == 3
	return ProcedureResult{Completed: completed, FinalState: stateNames[current]}
}