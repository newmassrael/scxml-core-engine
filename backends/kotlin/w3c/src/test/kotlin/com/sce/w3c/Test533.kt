// SCE-GENERATED — DO NOT EDIT
// source-hash: 797ed37cc08347dc2e5e34e9d6899fee0ba6634a915dce6de9d8ce52cfd2cf6b
// source: test533.scxml
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test533.scxml:1
package com.sce.w3c

import com.sce.generated.test533.Test533Event
import com.sce.generated.test533.Test533State
import com.sce.generated.test533.Test533StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: If a transition has 'type' of "internal", but its source state is not a compound state, its exit set is defined as if it had 'type' of "external".
@DisplayName("Test 533 -- W3C SCXML 3.13")
class Test533 : W3CTestBase<Test533State, Test533Event>() {
    override fun createStateMachine() = Test533StateMachine(createEngine())
    override val expectedPassState: Test533State = Test533State.Pass
}
