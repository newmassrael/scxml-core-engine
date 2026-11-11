// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The test binary of this package as the runner program of the generated
// suite `runtime_suite` (docs/SCE_TTCN3_LANE.md §4), so a test runs the program
// as a process of its own without building one: started with
// suiteRunnerProgramEnv set, the binary is runtime_suite.Main, which reads the
// command line it is given, the runner's, and exits; started without it, it
// runs this package's tests. runtime_suite.Main is the function the program
// backends/go/tests/cmd/runtime_suite_runner calls, so the process the test
// reads is that program's.

package static_datamodel

import (
	"os"
	"testing"

	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/runtime_suite"
)

// suiteRunnerProgramEnv is the variable of the environment that makes this
// test binary the suite's runner program.
const suiteRunnerProgramEnv = "SCE_GO_RUNTIME_SUITE_RUNNER_PROGRAM"

func TestMain(m *testing.M) {
	if os.Getenv(suiteRunnerProgramEnv) == "1" {
		// Main exits the process with the runner's status.
		runtime_suite.Main()
	}
	os.Exit(m.Run())
}
