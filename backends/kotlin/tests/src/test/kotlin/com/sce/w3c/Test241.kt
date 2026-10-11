// SCE-GENERATED — DO NOT EDIT
// source-hash: ca3b055deba6b38537c3642140f701b5f98f1b971d59425d3c780765d8a6a19b
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test241.scxml:1
package com.sce.w3c

import com.sce.generated.test241.Test241Event
import com.sce.generated.test241.Test241State
import com.sce.generated.test241.Test241StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: Invoked services MUST treat values specified by param and namelist identically.
@DisplayName("Test 241 -- W3C SCXML 6.4")
class Test241 : W3CTestBase<Test241State, Test241Event>() {
    override fun createStateMachine() = Test241StateMachine(createEngine())
    override val expectedPassState: Test241State = Test241State.Pass
    override val timeoutMs: Long = 5000L
}
