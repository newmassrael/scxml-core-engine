// SCE-GENERATED — DO NOT EDIT
// source-hash: 1e7ac53fad90f351b46524c7a440bbaf037a3968707179a2e6b9c0da26bba9d1
// source: test301.scxml
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
