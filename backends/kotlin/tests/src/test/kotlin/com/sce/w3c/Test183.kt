// SCE-GENERATED — DO NOT EDIT
// source-hash: c004aa6d76072d31e9273eed0dc88dc847e351ce7e35f90cc9d56908f01328bd
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test183.scxml:1
package com.sce.w3c

import com.sce.generated.test183.Test183Event
import com.sce.generated.test183.Test183State
import com.sce.generated.test183.Test183StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: If 'idlocation' is present, the SCXML Processor MUST generate an id when the parent send element is evaluated and store it in this location
@DisplayName("Test 183 -- W3C SCXML 6.2")
class Test183 : W3CTestBase<Test183State, Test183Event>() {
    override fun createStateMachine() = Test183StateMachine(createEngine())
    override val expectedPassState: Test183State = Test183State.Pass
}
