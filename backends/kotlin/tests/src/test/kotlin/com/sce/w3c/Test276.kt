// SCE-GENERATED — DO NOT EDIT
// source-hash: ca3b055deba6b38537c3642140f701b5f98f1b971d59425d3c780765d8a6a19b
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test276.scxml:1
package com.sce.w3c

import com.sce.generated.test276.Test276Event
import com.sce.generated.test276.Test276State
import com.sce.generated.test276.Test276StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.3: The SCXML Processor MUST allow the environment to provide values for top-level data elements at instantiation time. (Top-level data elements are those that are children of the datamodel element that is a child of scxml). Specifically, the Processor MUST use the values provided at instantiation time instead of those contained in these data elements.
@DisplayName("Test 276 -- W3C SCXML 5.3")
class Test276 : W3CTestBase<Test276State, Test276Event>() {
    override fun createStateMachine() = Test276StateMachine(createEngine())
    override val expectedPassState: Test276State = Test276State.Pass
    override val timeoutMs: Long = 5000L
}
