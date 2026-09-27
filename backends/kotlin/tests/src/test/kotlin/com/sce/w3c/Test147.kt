// SCE-GENERATED — DO NOT EDIT
// source-hash: c004aa6d76072d31e9273eed0dc88dc847e351ce7e35f90cc9d56908f01328bd
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
