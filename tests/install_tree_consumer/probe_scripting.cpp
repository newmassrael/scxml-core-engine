// Consumer probe for SCE::sce_scripting via find_package(SCE COMPONENTS scripting).
// Exercises the Scripting-tier install surface: scripting flat-path headers
// exposed only when the scripting tier is requested (ScriptResultUtils is
// a tier boundary symbol).

#include <scripting/ScriptResult.h>
#include <scripting/ScriptResultUtils.h>

int main() {
    return 0;
}
