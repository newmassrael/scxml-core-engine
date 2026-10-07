// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A [`Generator`] that runs Codex, headless, to write the model of a work.
//!
//! The client is started once per draft with `codex exec`, in a folder of its own that is removed
//! when it is done, and is given:
//!
//! - **the authoring server and nothing else to reach**, and of its tools only [`AUTHOR_TOOLS`]:
//!   it can read the work it was started for and check what it writes, and it cannot save to any
//!   work, take or finish a request, or record an acceptance. The application saves what it
//!   answers;
//! - **a sandbox that does not write and reads only the minimum and its own folder**
//!   ([`permission_settings`]), no question to a person who is not there
//!   (`approval_policy="never"`), no web search, nothing kept (`--ephemeral`) and nothing read
//!   from the person's own settings (`--ignore-user-config`);
//! - **the built-in features switched off that the support list says to switch off** (`--disable`),
//!   which is a list and not a guarantee: Codex has no switch that turns its tools off, and gains
//!   tools from version to version. That is why a version is run only when a person verified it
//!   against a specification written to attack it, and a feature that is on and that nobody
//!   switched off or reviewed is a refusal ([`crate::codex_support`]);
//! - **one credential**, the one the connection chose, and none of the others
//!   ([`crate::codex_environment`]);
//! - **an answer it must give in one form** (`--output-schema`), the same as the other client's:
//!   the documents of the model and the requirement list, which is what [`Draft`] is.
//!
//! What it is told goes in on standard input and not on the command line; the specification itself
//! is not in it at all, because the client reads the work through the authoring server. What it
//! answers is read from the file `-o` names, the last message it wrote, and nothing else it prints.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use serde_json::Value;

use crate::auth_policy::Observed;
use crate::claude_code::AuthorServer;
use crate::client_find::{find_programs, Candidate, Search};
use crate::client_run::{
    blank_job, capture, draft_from, prompt, schema, scoped_to, span_words, supervise, tail, Ended,
    Scratch, SERVER, SYSTEM_PROMPT,
};
use crate::codex_environment::environment_for;
use crate::codex_support::{enabled_features, Support};
use crate::connection::AuthSource;
use crate::revision::Revision;
use crate::runner::{Cancel, Draft, GenerateError, Generator, Job};

// The authoring server's tools a client may use, said once for every client that has them
// (`client_run::AUTHOR_TOOLS`), and named here as it always has been.
pub use crate::client_run::AUTHOR_TOOLS;

/// How long a program that may be Codex is given to say its version or its features.
const SAY: Duration = Duration::from_secs(15);

/// The folder, in the settings folder, that is the application's own home for Codex: where a
/// connection that chose the application's stored login keeps it, apart from the person's. Said
/// once, because the host that runs Codex and the screen that asks who is signed in must name the
/// same folder.
pub const HOME_DIR: &str = "codex-home";

/// How a run of the client is bounded.
#[derive(Debug, Clone)]
pub struct CodexConfig {
    /// A model name; the client's own default when absent.
    pub model: Option<String>,
    /// How long a run may take before it is killed.
    pub timeout: Duration,
}

impl Default for CodexConfig {
    fn default() -> Self {
        CodexConfig {
            model: None,
            timeout: Duration::from_secs(30 * 60),
        }
    }
}

/// Codex, as a generator.
#[derive(Debug, Clone)]
pub struct Codex {
    binary: PathBuf,
    author: AuthorServer,
    config: CodexConfig,
    auth: AuthSource,
    app_home: PathBuf,
    support: Support,
    version: Option<String>,
    environment: Vec<(String, String)>,
}

impl Codex {
    /// A generator that runs `binary` with the credential `auth` names, `app_home` being the
    /// application's own home for the client. Its version is asked once, now.
    pub fn new(
        binary: PathBuf,
        author: AuthorServer,
        config: CodexConfig,
        auth: AuthSource,
        app_home: PathBuf,
        support: Support,
    ) -> Self {
        let version = version_of(&binary);
        Codex {
            binary,
            author,
            config,
            auth,
            app_home,
            support,
            version,
            environment: std::env::vars().collect(),
        }
    }

    /// The same, started with `environment` as the process's own: what a run is given is worked
    /// out from it, and it is all the client sees of it.
    pub fn with_environment(mut self, environment: Vec<(String, String)>) -> Self {
        self.environment = environment;
        self
    }

    /// The same, bounded as `config` says: what a request pinned of the model and the time, put on
    /// a client that was already asked what it is.
    pub fn with_config(mut self, config: CodexConfig) -> Self {
        self.config = config;
        self
    }

    /// The program this runs.
    pub fn binary(&self) -> &Path {
        &self.binary
    }

    /// The arguments of a run, which are what a person verifies. `scratch` is the folder the run
    /// works in, and `last` the file it writes its last message to.
    fn arguments(
        &self,
        scratch: &Path,
        schema_file: &Path,
        last: &Path,
        work: &str,
    ) -> Vec<String> {
        exec_arguments(
            &scoped_to(&self.author, work),
            self.support.disabled_features(),
            self.config.model.as_deref(),
            scratch,
            schema_file,
            last,
        )
    }

    /// Everything that must be true before the client is started, asked in one place so that a run
    /// and the reason a request waits are the same judgement: it is Codex, a version a person
    /// verified, with nothing switched on that nobody looked at, and a credential to give it.
    /// A refusal is a sentence that names no path: it is said where the works folder is.
    fn preflight(&self) -> Result<Run, String> {
        let version = self
            .version
            .as_deref()
            .ok_or("the program did not say it is Codex, so it is not run")?;
        support_verdict(&self.binary, version, &self.support).map_err(|e| e.to_string())?;
        run_environment(self.auth, &self.app_home, &self.environment)
    }

    /// Why this Codex cannot be run now, or `None` when it can: for the request that waits.
    pub fn why_not(&self) -> Option<String> {
        self.preflight().err()
    }

    /// Who is signed in to Codex for this connection, asked of the client the way a run
    /// is started (the same credentials, the same home), or why it could not be asked. `None` is
    /// nobody.
    pub fn observe_login(&self) -> Result<Option<Observed>, String> {
        login_of(&self.binary, self.auth, &self.app_home, &self.environment)
    }
}

/// The arguments of `codex exec` for a run. A free function and not a method so that what a person
/// verifies can be named from the arguments themselves ([`instructions_of`]): `author` says how the
/// client reaches the authoring server, `disabled` which of its features are switched off, `scratch`
/// is the folder the run works in, `schema_file` the form its answer must take and `last` the file
/// it writes its last message to.
fn exec_arguments(
    author: &AuthorServer,
    disabled: &[String],
    model: Option<&str>,
    scratch: &Path,
    schema_file: &Path,
    last: &Path,
) -> Vec<String> {
    let mut args: Vec<String> = vec!["exec".into(), "--json".into()];
    args.extend(["--output-schema".into(), schema_file.display().to_string()]);
    args.extend(["-o".into(), last.display().to_string()]);
    args.extend([
        "--ephemeral".into(),
        "--ignore-user-config".into(),
        "--ignore-rules".into(),
        "--strict-config".into(),
        "--skip-git-repo-check".into(),
        "-C".into(),
        scratch.display().to_string(),
    ]);
    if let Some(model) = model {
        args.extend(["-m".into(), model.to_string()]);
    }
    for feature in disabled {
        args.extend(["--disable".into(), feature.clone()]);
    }
    for setting in run_settings(author) {
        args.extend(["-c".into(), setting]);
    }
    // The prompt is standard input.
    args.push("-".into());
    args
}

/// The name of the permission profile a run is given.
const PERMISSIONS: &str = "sce_run";

/// What a command the client runs may reach, as settings (`-c`): the minimum a program needs to
/// start and the folder the run works in (empty: the client reads the work through the authoring
/// server), read and never written, and no network.
///
/// The mode `-s read-only` is a profile that reads the whole disk (`:root`), so a command the
/// client was asked to run by a specification could read whatever the person's account can. The
/// client cannot switch off the tool that runs commands (`unified_exec`), and this is what its
/// sandbox is told instead. Codex refuses a sandbox mode beside a default profile, so the run is
/// given no `-s`.
///
/// What a person can read back is what Codex derived from these, held by
/// `app-core/tests/codex_live.rs`: the file system from `codex debug prompt-input`, and the
/// network setting from the answer to a session's start (`app-server`). That a connection is
/// refused is not among what has been tried. The sandbox starts only where the operating system
/// lets the user create namespaces (`bwrap`): where it cannot, no command runs at all.
pub fn permission_settings() -> Vec<String> {
    vec![
        format!("default_permissions={}", toml_string(PERMISSIONS)),
        format!(
            "permissions.{PERMISSIONS}.filesystem={{\":minimal\"=\"read\",\":project_roots\"={{\".\"=\"read\"}}}}"
        ),
        format!("permissions.{PERMISSIONS}.network={{enabled=false}}"),
    ]
}

/// The configuration a run is given on the command line, as `key=value` with a TOML value.
fn run_settings(author: &AuthorServer) -> Vec<String> {
    let key = |name: &str| format!("mcp_servers.{SERVER}.{name}");
    let env: Vec<String> = author
        .env
        .iter()
        .map(|(name, value)| format!("{}={}", toml_string(name), toml_string(value)))
        .collect();
    let mut settings = permission_settings();
    settings.extend([
        "approval_policy=\"never\"".to_string(),
        "web_search=\"disabled\"".to_string(),
        "agents.enabled=false".to_string(),
        "project_doc_max_bytes=0".to_string(),
        format!("developer_instructions={}", toml_string(SYSTEM_PROMPT)),
        auth_store_override(),
        format!(
            "{}={}",
            key("command"),
            toml_string(&author.command.display().to_string())
        ),
        format!(
            "{}=[{}]",
            key("args"),
            author
                .args
                .iter()
                .map(|a| toml_string(a))
                .collect::<Vec<_>>()
                .join(",")
        ),
        format!("{}={{{}}}", key("env"), env.join(",")),
        format!("{}=true", key("required")),
        format!(
            "{}=[{}]",
            key("enabled_tools"),
            AUTHOR_TOOLS
                .iter()
                .map(|t| toml_string(t))
                .collect::<Vec<_>>()
                .join(",")
        ),
    ]);
    // Only these read/check tools may run unattended. Unknown or newly added tools never get
    // this override, and the scoped authoring server also refuses them at dispatch.
    settings.extend(AUTHOR_TOOLS.iter().map(|name| {
        format!(
            "{}=\"approve\"",
            key(&format!("tools.{name}.approval_mode"))
        )
    }));
    settings
}

/// What decides where the client keeps a login, and the one place a login is kept for this build.
/// A run ignores the person's settings file (`--ignore-user-config`), and the client's
/// `login status` has no such flag and reads it, so a file that says the login is in the system's
/// keychain made the check say somebody is signed in that a run could not see. Both are told the
/// same, and the sign-in commands the person is given say it too, so that a login made by the
/// person is made where a run looks. What else a settings file can say about a login
/// (`forced_login_method`, `forced_chatgpt_workspace_id`, a provider) cannot be unsaid on the
/// command line: it is part of what a person verifies a version against.
const AUTH_STORE_KEY: &str = "cli_auth_credentials_store";
const AUTH_STORE_VALUE: &str = "file";

/// The setting that says where a login is kept, as the client is told it on the command line
/// (`-c`): a TOML value, which is how a run and the sign-in check are given it.
pub fn auth_store_override() -> String {
    format!("{AUTH_STORE_KEY}=\"{AUTH_STORE_VALUE}\"")
}

/// The same setting as a person types it after `codex login -c`. No quotes: a value that is not
/// TOML is taken as the word it is, and the quotes of one shell are not those of another.
pub fn auth_store_setting() -> String {
    format!("{AUTH_STORE_KEY}={AUTH_STORE_VALUE}")
}

/// What a run is started with.
struct Run {
    /// The process's own environment less the credentials that are not the one chosen.
    kept: Vec<(String, String)>,
    /// The home to give the client, when it is not the one it would use itself.
    home: Option<PathBuf>,
}

/// The environment a client is started in for a connection that chose `auth`: `environment` (the
/// process's own) less the credentials that are not the one chosen, with the home set when the
/// connection has one of its own.
fn run_environment(
    auth: AuthSource,
    app_home: &Path,
    environment: &[(String, String)],
) -> Result<Run, String> {
    let policy = environment_for(auth, app_home, environment).map_err(|e| e.to_string())?;
    let kept = environment
        .iter()
        .filter(|(name, _)| !policy.remove.contains(name))
        .cloned()
        .collect();
    Ok(Run {
        kept,
        home: policy.home,
    })
}

/// Who is signed in to `binary` for a connection that chose `auth`, asked the way a run is
/// started (the same credentials, the same home), or why it could not be asked. `None` is nobody.
/// Needs no authoring server: a screen asks it too.
pub fn login_of(
    binary: &Path,
    auth: AuthSource,
    app_home: &Path,
    environment: &[(String, String)],
) -> Result<Option<Observed>, String> {
    let run = run_environment(auth, app_home, environment)?;
    let mut command = Command::new(binary);
    command
        .args(["login", "status", "-c"])
        .arg(auth_store_override())
        .env_clear()
        .envs(run.kept);
    give_home(&mut command, run.home.as_deref())?;
    let said = capture(command, SAY)
        .map_err(|_| "Codex could not be asked who is signed in".to_string())?;
    // It says so on either stream, and fails when nobody is: what it printed is read, and how it
    // ended says whether what it printed is an answer.
    observed_from_login_status(
        &format!("{}\n{}", said.stdout, said.stderr),
        said.status.success(),
    )
}

/// Give the client the home a connection chose, when it chose one.
///
/// The home is the application's own folder, and the client does not start in a `CODEX_HOME` that
/// is not there, so it is made before the client is given it: by whichever asks first, the check of
/// who is signed in (which the commands that sign in are shown after) or a run. Only the person
/// who runs the application needs to read what is kept in it, and a login is.
fn give_home(command: &mut Command, home: Option<&Path>) -> Result<(), String> {
    let Some(home) = home else {
        return Ok(());
    };
    make_private_folder(home)
        .map_err(|e| format!("the application's folder for Codex could not be made: {e}"))?;
    command.env("CODEX_HOME", home);
    Ok(())
}

/// A folder, and the ones above it that are not there, that only its owner may enter. One that is
/// there is left as it is.
fn make_private_folder(path: &Path) -> std::io::Result<()> {
    let mut folder = fs::DirBuilder::new();
    folder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        folder.mode(0o700);
    }
    folder.create(path)
}

/// Default features, without the personal configuration a generation ignores. A private empty
/// home is enough for this offline question; credentials stay in the home used for generation.
pub fn features_on(binary: &Path) -> Result<Vec<String>, String> {
    let scratch = Scratch::new("sce-codex-features")
        .map_err(|e| format!("Codex feature check could not be prepared: {e}"))?;
    let mut command = Command::new(binary);
    command
        .args(["features", "list"])
        .current_dir(scratch.path())
        .env("CODEX_HOME", scratch.path());
    let said =
        capture(command, SAY).map_err(|e| format!("Codex could not list its features: {e}"))?;
    if !said.status.success() {
        return Err(format!(
            "Codex could not list its features: {}",
            tail(said.stderr.trim(), 500)
        ));
    }
    Ok(enabled_features(&said.stdout))
}

/// Why this build may not run a Codex: either it was asked and the answer is no, or it could not
/// be asked. A screen says the two apart (the first is a fact about the version, the second is
/// "ask again"), and a request that waits says either as the sentence it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotRunnable {
    /// The version is not one a person verified, or something is on that nobody looked at.
    Unverified(String),
    /// What is on could not be listed, so nothing is known of the version.
    Unasked(String),
}

impl std::fmt::Display for NotRunnable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotRunnable::Unverified(why) | NotRunnable::Unasked(why) => f.write_str(why),
        }
    }
}

/// Whether this build may run `binary`, which says it is Codex `version`: a version a person
/// verified against the instructions this build gives it, with nothing switched on that nobody
/// looked at. The reason is a sentence that names no path. Needs no authoring server: a screen
/// asks it too.
pub fn support_verdict(binary: &Path, version: &str, support: &Support) -> Result<(), NotRunnable> {
    let enabled = features_on(binary).map_err(NotRunnable::Unasked)?;
    support
        .check(
            std::env::consts::OS,
            version,
            &instructions_of(support),
            &enabled,
        )
        .map_err(|e| NotRunnable::Unverified(e.to_string()))
}

/// What the client's program is called on this system. An installer that leaves a shim of another
/// name (a command script) is named to the application with `SCE_CODEX`, and is not guessed at.
const PROGRAM: &str = if cfg!(windows) { "codex.exe" } else { "codex" };

/// The programs in `search` that say they are Codex, in the order they were found
/// ([`find_programs`]): the search path's, then the known locations'.
pub fn candidates(search: &Search) -> Vec<Candidate> {
    find_programs(search, PROGRAM, |binary, timeout| {
        version_within(binary, timeout)
    })
}

/// The program that is Codex on this computer: the one that was named (by the environment or by a
/// connection), whatever it is, so that a wrong one is said to be wrong and not quietly replaced;
/// or else the first `search` finds. `None` is no program to ask.
pub fn locate(named: Option<&Path>, search: &Search) -> Option<PathBuf> {
    match named {
        Some(path) => Some(path.to_path_buf()),
        None => candidates(search)
            .into_iter()
            .next()
            .map(|found| found.path),
    }
}

/// The version of the working instructions this build gives Codex (`codex/<digest>`): named by
/// what the client is told, what it must answer in, which tools it may use, what is switched off
/// and reviewed, and the arguments it is started with. A change to any of them is another version,
/// and another entry on the list of what a person verified, with no one to remember to say so.
pub fn instructions_of(support: &Support) -> String {
    let digest = Revision::of(instruction_material(support).as_bytes()).to_string();
    format!("codex/{}", &digest[..12])
}

/// What [`instructions_of`] names. The arguments are in it as they are made, with the parts that
/// belong to one run (the folders, the authoring server's launcher) as names: what a person
/// verified is the sandbox, the settings and the flags, and those are the same for every run.
fn instruction_material(support: &Support) -> String {
    let author = AuthorServer {
        command: PathBuf::from("<launcher>"),
        args: vec!["<argument>".to_string()],
        env: vec![("<name>".to_string(), "<value>".to_string())],
    };
    let arguments = exec_arguments(
        &scoped_to(&author, "<work>"),
        support.disabled_features(),
        None,
        Path::new("<scratch>"),
        Path::new("<schema>"),
        Path::new("<last>"),
    );
    format!(
        "{SYSTEM_PROMPT}\n{}\n{}\n{}\n{}\n{}\n{}",
        prompt(&blank_job()),
        schema(),
        AUTHOR_TOOLS.join(","),
        support.disabled_features().join(","),
        support.reviewed_features().join(","),
        arguments.join("\n"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn support() -> Support {
        Support::from_json(
            r#"{"verified":[],"disabled_features":["shell_tool"],"reviewed_features":["fast_mode"]}"#,
        )
        .unwrap()
    }

    #[test]
    fn what_a_person_verified_is_named_by_the_arguments_of_a_run() {
        let material = instruction_material(&support());

        // The profile, the settings, the flags and the one place a login is kept.
        for part in [
            "--ignore-user-config",
            "default_permissions=",
            "approval_policy=\"never\"",
            "web_search=\"disabled\"",
            "--disable\nshell_tool",
            &auth_store_override(),
            "mcp_servers.sce-author.enabled_tools=",
        ] {
            assert!(material.contains(part), "{part} is not named: {material}");
        }
    }

    #[test]
    fn a_command_the_client_runs_may_read_the_minimum_and_the_work_folder_and_nothing_else() {
        let settings = permission_settings();
        let text = settings.join("\n");

        // The profile that is chosen is the profile that is defined.
        assert!(
            text.contains(&format!("default_permissions=\"{PERMISSIONS}\"")),
            "{text}"
        );
        let filesystem = settings
            .iter()
            .find(|s| s.starts_with(&format!("permissions.{PERMISSIONS}.filesystem=")))
            .unwrap_or_else(|| panic!("no file system is defined: {text}"));
        assert!(filesystem.contains("\":minimal\"=\"read\""), "{filesystem}");
        assert!(filesystem.contains("\":project_roots\""), "{filesystem}");
        // Not the whole disk, not a place to write, and not the person's home.
        for forbidden in [":root", "\"write\"", "\"none\"", "home", "~", "/"] {
            assert!(
                !filesystem.contains(forbidden),
                "{forbidden:?} is in {filesystem}"
            );
        }
        // No `-s`: Codex refuses a sandbox mode beside a default profile.
        assert!(!text.contains("sandbox_mode"), "{text}");
        assert!(
            text.contains(&format!(
                "permissions.{PERMISSIONS}.network={{enabled=false}}"
            )),
            "{text}"
        );
    }

    #[test]
    fn the_name_does_not_depend_on_what_belongs_to_one_run() {
        // The folders and the launcher change from run to run and from computer to computer.
        let material = instruction_material(&support());

        assert!(!material.contains("/home"), "{material}");
        assert_eq!(instructions_of(&support()), instructions_of(&support()));
    }
}

/// What `codex login status` said, as the kind of credential in use, `None` for nobody, or that it
/// said neither. `succeeded` is whether the client ended well.
///
/// Somebody is signed in when the client says so and does not fail in saying it. Anything else it
/// prints is not a login: an error (a home that is not there, a settings file it cannot read) is
/// the client not saying who is signed in, and read as one it showed a login that did not exist.
/// The kind is read from the line that says it, because the words of a login's name are in a path
/// as well. A way it does not name is [`Observed::Other`] and is not guessed at.
pub fn observed_from_login_status(text: &str, succeeded: bool) -> Result<Option<Observed>, String> {
    let said = text.to_ascii_lowercase();
    if said.contains("not logged in") {
        return Ok(None);
    }
    if succeeded {
        if let Some(line) = said.lines().find(|line| line.contains("logged in")) {
            return Ok(Some(if line.contains("chatgpt") {
                Observed::Subscription
            } else if line.contains("api key") {
                Observed::ApiKey
            } else {
                Observed::Other
            }));
        }
    }
    let printed = text.trim();
    Err(if printed.is_empty() {
        "Codex did not say who is signed in".to_string()
    } else {
        format!("Codex did not say who is signed in: {}", tail(printed, 300))
    })
}

/// The Codex a host found, and how a run of it is given what it needs: what a directory needs to
/// make a generator for a request that names a connection to Codex.
#[derive(Debug, Clone)]
pub struct CodexLaunch {
    pub binary: PathBuf,
    pub author: AuthorServer,
    /// The application's own home for the client, which a connection that chose the application's
    /// stored login uses.
    pub app_home: PathBuf,
    /// Which versions were verified, and what is switched off.
    pub support: Support,
    /// The environment a run is worked out from: the process's own.
    pub environment: Vec<(String, String)>,
}

/// A string as a TOML value: a basic string, whose escapes are JSON's.
fn toml_string(text: &str) -> String {
    serde_json::to_string(text).expect("a string is always JSON")
}

/// What `binary --version` says when it says it is Codex (`codex-cli 0.159.0`): the version.
pub fn version_of(binary: &Path) -> Option<String> {
    version_within(binary, SAY)
}

/// The same, given `timeout` to answer.
pub(crate) fn version_within(binary: &Path, timeout: Duration) -> Option<String> {
    let mut command = Command::new(binary);
    command.arg("--version");
    let said = capture(command, timeout).ok()?;
    if !said.status.success() {
        return None;
    }
    let first = said.stdout.lines().next()?.trim();
    if !first.to_ascii_lowercase().contains("codex") {
        return None;
    }
    first.split_whitespace().last().map(str::to_string)
}

impl Generator for Codex {
    fn kind(&self) -> &str {
        "codex"
    }

    fn version(&self) -> Option<String> {
        self.version.clone()
    }

    fn instructions(&self) -> Option<String> {
        Some(instructions_of(&self.support))
    }

    fn generate(&self, job: &Job, cancel: &Cancel) -> Result<Draft, GenerateError> {
        // Nothing is started before it is known to be a Codex this build has verified, with
        // nothing on that nobody looked at, and a credential to give it.
        let Run { kept, home } = self.preflight().map_err(GenerateError::Failed)?;

        let scratch = Scratch::new("sce-codex")
            .map_err(|e| GenerateError::Failed(format!("a folder for the client: {e}")))?;
        let schema_file = scratch.path().join("schema.json");
        let last = scratch.path().join("last.json");
        fs::write(&schema_file, schema().to_string())
            .map_err(|e| GenerateError::Failed(format!("the form of the answer: {e}")))?;

        let mut command = Command::new(&self.binary);
        command
            .current_dir(scratch.path())
            .args(self.arguments(scratch.path(), &schema_file, &last, job.work.as_str()))
            .env_clear()
            .envs(kept)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // The home the connection gave the client, when it gave one.
        give_home(&mut command, home.as_deref()).map_err(GenerateError::Failed)?;
        let child = command.spawn().map_err(|e| {
            GenerateError::Failed(format!(
                "{} could not be started: {e}",
                self.binary.display()
            ))
        })?;
        let ended = supervise(
            child,
            Some(prompt(job).into_bytes()),
            cancel,
            self.config.timeout,
        )
        .map_err(|e| GenerateError::Failed(format!("the client could not be read: {e}")))?;

        match ended {
            Ended::Cancelled => Err(GenerateError::Cancelled),
            Ended::TimedOut => Err(GenerateError::Failed(format!(
                "the client took longer than {} and was stopped",
                span_words(self.config.timeout)
            ))),
            Ended::Exited {
                status,
                stdout,
                stderr,
            } => {
                if !status.success() {
                    let said = if stderr.trim().is_empty() {
                        &stdout
                    } else {
                        &stderr
                    };
                    return Err(GenerateError::Failed(format!(
                        "the client stopped with {status}: {}",
                        tail(said.trim(), 2000)
                    )));
                }
                read_answer(&last)
            }
        }
    }
}

/// The last message the client wrote, as a draft or as the reason there is none.
fn read_answer(last: &Path) -> Result<Draft, GenerateError> {
    let text = fs::read_to_string(last).map_err(|_| {
        GenerateError::Unusable(
            "the client did not write a last message, so there is no draft".to_string(),
        )
    })?;
    let answer: Value = serde_json::from_str(text.trim()).map_err(|_| {
        GenerateError::Unusable(format!(
            "the last message is not the draft asked for: it is not JSON: {}",
            tail(text.trim(), 2000)
        ))
    })?;
    draft_from(&answer).map_err(GenerateError::Unusable)
}
