// ECMA-262 through a Lua-LOWERED Kotlin artifact, compiled and run.
//
// The twin of `tests/CMakeLists.txt`'s `lowered_ecma262_test`, and it exists
// for the reason that one does: until a generated artifact is COMPILED with
// the build-time lowering in it and RUN, "the frontend answers the shared
// table" is a statement about `sce-build` and somebody else's Lua.
//
// A MODULE OF ITS OWN rather than a source set inside `:sce-kotlin-tests`.
// The subject here is a pair of artifacts generated from one document with
// two different `--script-engine` selections, and both have to be in the
// compile unit at once. Adding them to the conformance suite would put 98
// cases of expression fixture into a module whose 373 cases are about
// something else, and would make an edit to this fixture invalidate that
// suite. The C++ side draws the same line with a separate target.
// Imported rather than spelled inline: inside a Gradle Kotlin DSL script
// `java` resolves to the JavaPluginExtension, so `java.io.FileOutputStream` is
// an unresolved reference rather than a fully-qualified class name.
import java.io.FileOutputStream
import javax.inject.Inject
import org.gradle.process.ExecOperations

plugins {
    alias(libs.plugins.kotlin.jvm)
}

/**
 * One `sce-codegen generate` run, with its MANIFEST kept.
 *
 * A task class rather than `Exec`, for two reasons that turned out to be the
 * same reason. `Exec` can only send stdout to a stream held on the task, and
 * the configuration cache refuses to serialise one — `ExecOperations`,
 * injected, is the supported way to run a process from a task action. And the
 * thing on stdout is the generate MANIFEST, which is the generator's own
 * statement about what it just emitted (`script_engine_language`,
 * `needs_script_engine`). Discarding it would leave the gate below asking a
 * SECOND generation what this one did; kept as a file, the gate reads the
 * artifact's own manifest. Ask the product, not a re-run of the entry.
 */
abstract class GenerateLoweredArtifact : DefaultTask() {
    @get:Inject
    abstract val execOps: ExecOperations

    /** The generator binary, resolved by the root build. */
    @get:Input
    abstract val codegen: Property<String>

    /**
     * The generator binary's CONTENT, so rebuilding it regenerates the artifact.
     *
     * ⚠ Measured 2026-08-30, and it is the same failure this lane exists to
     * refuse, one level up. With only [codegen] declared — a String, the PATH —
     * a `cargo build` that changed what the generator emits left this task
     * UP-TO-DATE, so the suite compiled machines from the previous generator
     * and the run said nothing about the change. It surfaced only because the
     * runtime signature had moved underneath and the stale Kotlin no longer
     * compiled; a change that merely altered emitted BEHAVIOUR would have
     * passed, green, over the old artifact.
     *
     * `NAME_ONLY` is wrong here and `@InputFile` alone is right: what matters
     * is the bytes, not where they sit, because the build machine and CI put
     * the binary at different absolute paths.
     */
    @get:InputFile
    abstract val codegenBinary: RegularFileProperty

    /** The SCXML this artifact is generated from. */
    @get:InputFile
    abstract val document: RegularFileProperty

    /** The `--script-engine` selection, which is the whole subject here. */
    @get:Input
    abstract val scriptEngine: Property<String>

    /**
     * Where the machine, its sourcemap and the manifest land.
     *
     * ONE DIRECTORY PER ARTIFACT, and that is not tidiness: `sce-codegen`
     * writes `sce_sourcemap.json` beside the machine under a name that does
     * not vary with the document, so two generations sharing an output
     * directory would each overwrite the other's map. Gradle would also see
     * two tasks declaring one output and quietly stop caching both.
     */
    @get:OutputDirectory
    abstract val outputDir: DirectoryProperty

    /**
     * The generator's own statement about what it emitted, kept beside the
     * artifact. `@Internal` because it lives inside [outputDir], which is
     * already declared.
     */
    @get:Internal
    val manifest: File get() = File(outputDir.get().asFile, "manifest.json")

    @TaskAction
    fun generate() {
        val target = outputDir.get().asFile
        target.mkdirs()
        FileOutputStream(manifest).use { sink ->
            execOps.exec {
                commandLine(
                    codegen.get(), "generate",
                    document.get().asFile.absolutePath,
                    "-o", target.absolutePath,
                    "-l", "kotlin",
                    "--script-engine", scriptEngine.get(),
                )
                standardOutput = sink
                // A determinism pin, for the same reason `:sce-kotlin-tests`
                // carries one: the emitted header stamps a generation time,
                // and an unpinned one makes every build produce new bytes.
                environment("SOURCE_DATE_EPOCH", System.getenv("SOURCE_DATE_EPOCH") ?: "0")
            }
        }
    }
}

group = "com.sce"
version = "1.0.0"

dependencies {
    // The Lua engine and nothing else. This lane is about ONE engine reading
    // an artifact emitted for it; Rhino and QuickJS refuse the language under
    // test, so a dependency on them would only widen the classpath.
    implementation(project(":sce-kotlin-runtime"))
    implementation(project(":sce-kotlin-lua"))

    testImplementation(kotlin("test"))
    testImplementation(libs.junit.jupiter)
    // The shared ECMA-262 table is read with a parser that is not one of the
    // engines under test — an engine that answers the language wrong must not
    // also be the thing that reads what the language is supposed to answer.
    testImplementation(libs.kotlinx.serialization.json)
}

apply(from = rootProject.file("gradle/sce-codegen.gradle.kts"))
val repoRoot: String = rootProject.projectDir.absolutePath
val sceCodegenAbsoluteOrNull: String? by rootProject.extra
val sceCodegenBinary: String? = sceCodegenAbsoluteOrNull

// ---------------------------------------------------------------------------
// The fixture, and the two artifacts it becomes
//
// ONE document, two selections. `ecma262_lowered` crosses the seam at BUILD
// time — its expressions arrive at the engine as Lua the frontend produced,
// wrapped in `ScriptSource.lua(lowered, source)`, and `LuaScriptEngine` passes
// that text through untouched. `ecma262_source` is what this backend emits by
// default — `ScriptSource.ecmascript(...)` — which the same engine offers to
// the same frontend at RUN time. Two routes to one answer, and the whole point
// of the lane is that they are separately reachable and separately measured.
//
// The document is GENERATED, never committed: the population is the shared
// table in full and `tools/generate_lowered_ecma262_fixture.py` is the only
// place it is expanded. A committed copy would be a second population, free to
// fall behind the table it claims to ask.
// ---------------------------------------------------------------------------

/** Where the expanded SCXML and the machines generated from it land. */
val fixtureDir: Provider<Directory> = layout.buildDirectory.dir("lowered-ecma262")

/**
 * `<stem>` to the `--script-engine` selection its artifact is generated with.
 *
 * Named on BOTH rows, including the one whose value is this backend's current
 * default. An omitted flag on the control would make it indistinguishable from
 * a call that had simply forgotten one — and on the day
 * `Language::Kotlin.default_script_engine_target()` flips, that omission would
 * silently make the control a second copy of the subject and the comparison
 * below would be the lowered machine against itself. The C++ twin names it for
 * exactly this reason, and its gate asserts the pair count to prove it.
 */
val artifacts: Map<String, String> = mapOf(
    "ecma262_lowered" to "lua",
    "ecma262_source" to "ecmascript",
)

val generateLoweredFixture by tasks.registering(Exec::class) {
    group = "code generation"
    description = "Expand the shared ECMA-262 table into the SCXML both artifacts are generated from"

    workingDir = File(repoRoot)

    // One document, copied to each stem. The machine name, the file name and
    // the Kotlin package all come from the stem, so generating the same bytes
    // under two names is what lets both artifacts live in one compile unit.
    val cases = File(repoRoot, "tests/ecmascript/ecma262_semantics.json")
    val generator = File(repoRoot, "tools/generate_lowered_ecma262_fixture.py")
    val primary = fixtureDir.map { it.file("${artifacts.keys.first()}.scxml") }

    commandLine(
        "python3", generator.absolutePath,
        "--cases", cases.absolutePath,
        "-o", primary.get().asFile.absolutePath,
    )

    inputs.file(cases).withPropertyName("sharedEcma262Table")
    inputs.file(generator).withPropertyName("fixtureGenerator")
    outputs.dir(fixtureDir).withPropertyName("fixtureDir")

    val stems = artifacts.keys.toList()
    val outDir = fixtureDir.get().asFile
    doLast {
        val first = File(outDir, "${stems.first()}.scxml")
        for (stem in stems.drop(1)) {
            first.copyTo(File(outDir, "$stem.scxml"), overwrite = true)
        }
    }

    isIgnoreExitValue = false
}

/**
 * One generation per artifact, as its own task.
 *
 * `Exec` rather than a `doLast` shelling out, for the reason the conformance
 * module uses one: the command, its inputs and its outputs are then Gradle's
 * to track, and the configuration cache does not have to serialise a closure
 * holding a `Project`.
 *
 * `enabled` rather than a skip when the generator is absent: without it there
 * is no artifact, and a lane whose subject cannot be produced has nothing to
 * say. The gate refuses on a missing binary before it ever gets here; this
 * only keeps a bare `./gradlew build` from failing on a tree that has never
 * built the generator.
 */
val generateLoweredMachines = artifacts.map { (stem, language) ->
    val suffix = stem.split('_').joinToString("") { it.replaceFirstChar(Char::uppercase) }
    tasks.register<GenerateLoweredArtifact>("generate$suffix") {
        group = "code generation"
        description = "Generate the $stem Kotlin artifact with --script-engine $language"

        dependsOn(generateLoweredFixture)
        enabled = sceCodegenBinary != null

        codegen.set(sceCodegenBinary ?: "sce-codegen")
        // Only settable when the binary was resolved to a path; the task is
        // `enabled = false` in that case anyway, and Gradle does not check the
        // inputs of a disabled task.
        sceCodegenBinary?.let { codegenBinary.set(File(it)) }
        document.set(fixtureDir.map { it.file("$stem.scxml") })
        scriptEngine.set(language)
        outputDir.set(fixtureDir.map { it.dir("generated/$stem") })
    }
}

for (stem in artifacts.keys) {
    sourceSets["main"].kotlin.srcDir(fixtureDir.map { it.dir("generated/$stem") })
}

tasks.named("compileKotlin") {
    dependsOn(generateLoweredMachines)
}

tasks.test {
    useJUnitPlatform()

    // The shared table is resolved relative to the repository root, the same
    // anchor every other reader of `tests/ecmascript` uses.
    workingDir = File(repoRoot)

    // Read at RUN time and not on the compile classpath, so without this the
    // suite stays UP-TO-DATE across an edit to the very tables it holds. The
    // conformance module carries the same declaration after the same failure.
    inputs.dir(File(repoRoot, "tests/ecmascript"))
        .withPropertyName("sharedEcmaScriptTables")
        .withPathSensitivity(PathSensitivity.RELATIVE)

    val luaLibDir = project(":sce-kotlin-lua").layout.buildDirectory.dir("native/lib")
    systemProperty("java.library.path", luaLibDir.get().asFile.absolutePath)

    systemProperty("junit.jupiter.execution.timeout.default", "120s")

    testLogging {
        events("passed", "skipped", "failed")
        showStandardStreams = true
        exceptionFormat = org.gradle.api.tasks.testing.logging.TestExceptionFormat.FULL
    }
}

kotlin {
    jvmToolchain(17)
    compilerOptions {
        allWarningsAsErrors.set(true)
    }
}
