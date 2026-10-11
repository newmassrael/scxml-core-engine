// SCE-GENERATED — DO NOT EDIT
// source-hash: ca3b055deba6b38537c3642140f701b5f98f1b971d59425d3c780765d8a6a19b
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test148.scxml:1
package com.sce.w3c

import com.sce.generated.test148.Test148Event
import com.sce.generated.test148.Test148State
import com.sce.generated.test148.Test148StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 4.3: When the if element is executed, if no 'cond'attribute evaluates to true, the SCXML Processor must execute the partition defined by the else tag, if there is one.
@DisplayName("Test 148 -- W3C SCXML 4.3")
class Test148 : W3CTestBase<Test148State, Test148Event>() {
    override fun createStateMachine() = Test148StateMachine(createEngine())
    override val expectedPassState: Test148State = Test148State.Pass
}
