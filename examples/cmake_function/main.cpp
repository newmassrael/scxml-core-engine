#include "simple_light_sm.h"
#include "wrappers/AutoProcessStateMachine.h"
#include <iostream>

int main() {
    using namespace SCE::Generated::simple_light;

    std::cout << "=== SCE CMake Integration Example ===" << "\n\n";
    std::cout << "This example demonstrates:\n";
    std::cout << "  - Automatic code generation with sce_add_state_machine()\n";
    std::cout << "  - Dependency tracking (rebuilds when SCXML changes)\n";
    std::cout << "  - Zero-configuration CMake integration\n";
    std::cout << "  - Clean project organization\n\n";

    // Option 1: Easy API - Auto-processing wrapper (recommended for beginners)
    std::cout << "Using easy API (AutoProcessStateMachine):" << "\n";
    {
        SCE::Wrappers::AutoProcessStateMachine<simple_light> light;

        light.initialize();
        std::cout << "  Initial state: " << (light.getCurrentState() == State::Off ? "OFF" : "ON") << "\n";

        light.processEvent(Event::Switch_on);
        std::cout << "  After switch_on: " << (light.getCurrentState() == State::On ? "ON" : "OFF") << "\n";

        light.processEvent(Event::Switch_off);
        std::cout << "  After switch_off: " << (light.getCurrentState() == State::Off ? "OFF" : "ON") << "\n";
    }

    std::cout << "\n";

    // Option 2: Low-level API - Manual control (for advanced users)
    std::cout << "Using low-level API (manual step):" << "\n";
    {
        simple_light light;

        light.initialize();
        std::cout << "  Initial state: " << (light.getCurrentState() == State::Off ? "OFF" : "ON") << "\n";

        light.raiseExternal(Event::Switch_on);
        light.step();
        std::cout << "  After switch_on: " << (light.getCurrentState() == State::On ? "ON" : "OFF") << "\n";

        light.raiseExternal(Event::Switch_off);
        light.step();
        std::cout << "  After switch_off: " << (light.getCurrentState() == State::Off ? "OFF" : "ON") << "\n";
    }

    return 0;
}
