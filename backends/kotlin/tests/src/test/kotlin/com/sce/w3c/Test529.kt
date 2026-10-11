// SCE-GENERATED — DO NOT EDIT
// source-hash: ca3b055deba6b38537c3642140f701b5f98f1b971d59425d3c780765d8a6a19b
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test529.scxml:1
package com.sce.w3c

import com.sce.generated.test529.Test529Event
import com.sce.generated.test529.Test529State
import com.sce.generated.test529.Test529StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.6: If the 'expr' attribute is not present, the Processor MUST use the children of content as the output.
@DisplayName("Test 529 -- W3C SCXML 5.6")
class Test529 : W3CTestBase<Test529State, Test529Event>() {
    override fun createStateMachine() = Test529StateMachine(createEngine())
    override val expectedPassState: Test529State = Test529State.Pass
}
