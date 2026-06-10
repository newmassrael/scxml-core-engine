// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 2c4f76809986b4347703e89a8e901379e8391f815371b53c5a7eecbe187e1cf5
// generated-at: 1781081955
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
