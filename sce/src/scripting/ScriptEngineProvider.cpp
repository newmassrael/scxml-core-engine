#include "scripting/ScriptEngineProvider.h"

#ifdef SCE_SCRIPT_ENGINE_LUA
#include "scripting/LuaEngine.h"
#else
#include "scripting/JSEngine.h"
#endif

namespace SCE {

IScriptEngine &ScriptEngineProvider::getScriptEngine() {
#ifdef SCE_SCRIPT_ENGINE_LUA
    return LuaEngine::instance();
#else
    return JSEngine::instance();
#endif
}

ISessionManager &ScriptEngineProvider::getSessionManager() {
#ifdef SCE_SCRIPT_ENGINE_LUA
    return LuaEngine::instance();
#else
    return JSEngine::instance();
#endif
}

}  // namespace SCE
