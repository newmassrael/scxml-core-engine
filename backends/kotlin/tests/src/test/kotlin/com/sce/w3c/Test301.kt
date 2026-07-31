// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 7aab3b29aa8f5ef17f1c8730c3954aecc89c78aabf4a2226d70ddd8c24038efe
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
