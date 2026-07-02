# ===========================================================================
# SCE Android — ProGuard/R8 rules for JNI and reflection survival
# ===========================================================================

# ---------------------------------------------------------------------------
# JNI native methods
#
# C++ JNI function names are hardcoded as Java_com_sce_scripting_lua_LuaNative_<method>.
# If R8 renames these classes or methods, native method resolution fails with
# UnsatisfiedLinkError at runtime.
# ---------------------------------------------------------------------------
-keepclasseswithmembernames class com.sce.scripting.lua.LuaNative {
    native <methods>;
}
-keepclasseswithmembernames class com.sce.scripting.quickjs.QuickJSNative {
    native <methods>;
}

# Keep the singleton objects that load native libraries
-keep class com.sce.scripting.lua.LuaNative { *; }
-keep class com.sce.scripting.quickjs.QuickJSNative { *; }

# ---------------------------------------------------------------------------
# Engine interface and exception (accessed via polymorphism)
# ---------------------------------------------------------------------------
-keep class com.sce.runtime.ScxmlScriptEngine { *; }
-keep class com.sce.runtime.ScriptEngineException { *; }

# ---------------------------------------------------------------------------
# Rhino — uses reflection extensively for JavaScript object manipulation
# ---------------------------------------------------------------------------
-keep class org.mozilla.javascript.** { *; }
-dontwarn org.mozilla.javascript.**

# Rhino's classfile generation (interpreted mode doesn't need this, but keep for safety)
-dontwarn org.mozilla.classfile.**
