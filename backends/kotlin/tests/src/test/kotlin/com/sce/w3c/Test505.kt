// SCE-GENERATED — DO NOT EDIT
// source-hash: ca3b055deba6b38537c3642140f701b5f98f1b971d59425d3c780765d8a6a19b
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test505.scxml:1
package com.sce.w3c

import com.sce.generated.test505.Test505Event
import com.sce.generated.test505.Test505State
import com.sce.generated.test505.Test505StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: Otherwise, if the transition has 'type' "internal", its source state is a compound state and all its target states are proper descendents of its source state, the target set consists of all active states that are proper descendents of its source state.
@DisplayName("Test 505 -- W3C SCXML 3.13")
class Test505 : W3CTestBase<Test505State, Test505Event>() {
    override fun createStateMachine() = Test505StateMachine(createEngine())
    override val expectedPassState: Test505State = Test505State.Pass
}
