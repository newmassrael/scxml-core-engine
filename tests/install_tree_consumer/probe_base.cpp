// Consumer probe for SCE::sce_base via find_package(SCE COMPONENTS base).
// Exercises the Core-tier install surface: headers under common/, core/,
// static/, plus the sce_base archive. Keeps dependencies minimal so the
// smoke test also catches unintended header leaks into the Core component.

#include <SCXMLEngine.h>
#include <SCXMLTypes.h>
#include <static/StaticExecutionEngine.h>

int main() {
    return 0;
}
