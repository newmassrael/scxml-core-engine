// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: e58c03089e515b4f87df3e09e89234f06d61979361ed8fef1646aeb0069c2169
// generated-at: 1779596481
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test147.scxml:1
package com.sce.w3c

import com.sce.generated.test147.Test147Event
import com.sce.generated.test147.Test147State
import com.sce.generated.test147.Test147StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 4.3: When the if element is executed, the SCXML processor MUST execute the first partition in document order that is defined by a tag whose 'cond' attribute evaluates to true, if there is one.
@DisplayName("Test 147 -- W3C SCXML 4.3")
class Test147 : W3CTestBase<Test147State, Test147Event>() {
    override fun createStateMachine() = Test147StateMachine(createEngine())
    override val expectedPassState: Test147State = Test147State.Pass
}
