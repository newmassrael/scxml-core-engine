// SCE-GENERATED — DO NOT EDIT
// source-hash: ca3b055deba6b38537c3642140f701b5f98f1b971d59425d3c780765d8a6a19b
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test343.scxml:1
package com.sce.w3c

import com.sce.generated.test343.Test343Event
import com.sce.generated.test343.Test343State
import com.sce.generated.test343.Test343StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.7: If the 'location' attribute on a param element does not refer to a valid location in the data model, or if the evaluation of the 'expr' produces an error, the processor MUST ignore the name and value.
@DisplayName("Test 343 -- W3C SCXML 5.7")
class Test343 : W3CTestBase<Test343State, Test343Event>() {
    override fun createStateMachine() = Test343StateMachine(createEngine())
    override val expectedPassState: Test343State = Test343State.Pass
}
