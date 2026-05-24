// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 2e32d261d6350eb3a25f2f20128ae90019b36b8835127308d167f05b44688be3
// generated-at: 1779589482
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test307.scxml:1
package com.sce.w3c

import com.sce.generated.test307.Test307Event
import com.sce.generated.test307.Test307State
import com.sce.generated.test307.Test307StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.9: When "late" data binding is used, accessing data substructure in expressions before the corresponding data element is loaded MUST yield the same execution-time behavior as accessing non-existent data substructure in a loaded data instance.
@DisplayName("Test 307 -- W3C SCXML 5.9")
class Test307 : W3CTestBase<Test307State, Test307Event>() {
    override fun createStateMachine() = Test307StateMachine(createEngine())
    override val expectedPassState: Test307State = Test307State.Final
}
