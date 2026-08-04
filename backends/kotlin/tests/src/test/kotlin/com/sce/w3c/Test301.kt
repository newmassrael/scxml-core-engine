// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 15d6029f4f85b34e5f54293320d31d5c234aec7e25e520553a3723febef59859
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test301.scxml:1
package com.sce.w3c

import com.sce.generated.test301.Test301Event
import com.sce.generated.test301.Test301State
import com.sce.generated.test301.Test301StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.8: If the script specified by the 'src' attribute of a script element cannot be downloaded within a platform-specific timeout interval, the document is considered non-conformant, and the platform MUST reject it. N.B. This test is valid only for datamodels that support scripting.
@DisplayName("Test 301 -- W3C SCXML 5.8")
class Test301 : W3CTestBase<Test301State, Test301Event>() {
    override fun createStateMachine() = Test301StateMachine()
    override val expectedPassState: Test301State = Test301State.Pass
}
