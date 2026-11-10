// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The runner program of a generated suite (docs/SCE_TTCN3_LANE.md §4, §10.5):
//! it runs every test case a world selects against its recorded input trace
//! and writes each one's canonical test log.
//!
//! The unit of a suite generates the program's `main` (`<suite>::main`), which
//! hands [`main`] the suite's tables and the reader of its module parameters
//! (`<suite>::module_parameter`); a crate makes it a program with a one-line
//! binary target. The command line is the same on every backend:
//!
//! ```text
//! <runner> --trace-dir <dir> --log-dir <dir> [--world <feature>,…] [--module-parameter <id>=<value>]…
//! ```
//!
//! - `--trace-dir`: the directory of the traces, `<test case>.json` each
//!   (`schemas/sce-test-trace.v1.schema.json`);
//! - `--log-dir`: the directory the logs are written to, `<test case>.log.json`
//!   each, the canonical test log's text with nothing after it
//!   (`schemas/sce-test-log.v1.schema.json`), made where it is missing;
//! - `--world`: the features the world enables, by name, separated by commas;
//!   none where it is not given (ADR 0057 §2);
//! - `--module-parameter`: a module parameter's value in place of its default,
//!   written as `<sce:module-parameter value>` writes a literal of its type
//!   (docs/SCE_TEST_PROFILE.md §11.4, §11.8); repeated, one per parameter.
//!
//! Each test case run prints `<test case> <verdict>` on its own line. A test
//! case whose trace is missing or refused ends with `error` and its log is
//! written, the reason on the standard error (P13). The exit status is 0 when
//! every selected test case ran and its log was written, whatever its
//! verdict: the verdict is the log's. It is 2 when the command line is not
//! one of the above, a module parameter's value is refused, or a log cannot
//! be written.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use super::from_canonical;
use super::module_parameters::{ModuleParameterValue, ModuleParameters};
use super::run::LogKind;
use super::suite::{run_suite_with_traces, Features, OverrideRefusal, Suite};
use super::trace::{Trace, TraceRefusal};
use crate::json::Value;
use crate::saved_state::{bytes_to_saved, SavedValue};

/// The reader of a suite's module parameters, generated as
/// `<suite>::module_parameter`: the saved value of the module parameter `id`
/// of its declared type, read from `literal`, the value the command line gives
/// it, or why it is not one.
pub type ParameterReader = fn(&str, &str) -> Result<ModuleParameterValue, OverrideRefusal>;

/// The program's usage line.
pub const USAGE: &str = "usage: <runner> --trace-dir <dir> --log-dir <dir> \
                         [--world <feature>,...] [--module-parameter <id>=<value>]...";

/// What the command line asks of the runner.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RunnerOptions {
    /// The directory of the traces, `<test case>.json` each.
    pub trace_dir: PathBuf,
    /// The directory the logs are written to, `<test case>.log.json` each.
    pub log_dir: PathBuf,
    /// The features the world enables.
    pub world: Features,
    /// Each `--module-parameter`, its id and the literal it gives, in the order
    /// given.
    pub module_parameters: Vec<(String, String)>,
}

impl RunnerOptions {
    /// The options `args` give, the program's name not among them; refused,
    /// naming why, where they are not the program's command line.
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut options = RunnerOptions::default();
        let (mut trace_dir, mut log_dir, mut world) = (None, None, None);
        // Every option takes one value: the line is read a pair at a time.
        let args: Vec<String> = args.into_iter().collect();
        for pair in args.chunks(2) {
            let [option, value] = pair else {
                return Err(format!("{} is given no value", pair[0]));
            };
            match option.as_str() {
                "--trace-dir" if trace_dir.is_none() => trace_dir = Some(PathBuf::from(value)),
                "--log-dir" if log_dir.is_none() => log_dir = Some(PathBuf::from(value)),
                "--world" if world.is_none() => world = Some(value.clone()),
                "--module-parameter" => {
                    let (id, literal) = value
                        .split_once('=')
                        .ok_or_else(|| format!("--module-parameter {value} is not <id>=<value>"))?;
                    if options
                        .module_parameters
                        .iter()
                        .any(|(given, _)| given == id)
                    {
                        return Err(format!("the module parameter '{id}' is given twice"));
                    }
                    options
                        .module_parameters
                        .push((id.to_string(), literal.to_string()));
                }
                "--trace-dir" | "--log-dir" | "--world" => {
                    return Err(format!("{option} is given twice"));
                }
                _ => return Err(format!("{option} is no option of the runner")),
            }
        }
        options.trace_dir = trace_dir.ok_or("--trace-dir is not given")?;
        options.log_dir = log_dir.ok_or("--log-dir is not given")?;
        options.world = world
            .iter()
            .flat_map(|features| features.split(','))
            .filter(|feature| !feature.is_empty())
            .map(str::to_string)
            .collect();
        Ok(options)
    }
}

/// The trace of the test case `test_case`, `<dir>/<test case>.json`.
pub fn read_trace(dir: &Path, test_case: &str) -> Result<Trace, TraceRefusal> {
    let path = dir.join(format!("{test_case}.json"));
    let text = std::fs::read_to_string(&path)
        .map_err(|error| TraceRefusal::new(format!("{} is not read: {error}", path.display())))?;
    Trace::parse(&text)
}

/// The runner program of `suite`, its module parameters read by `parameter`,
/// on the command line `args`, the program's name not among them
/// (docs/SCE_TTCN3_LANE.md §4): see the module's documentation.
pub fn main(
    suite: &Suite,
    parameter: ParameterReader,
    args: impl IntoIterator<Item = String>,
) -> ExitCode {
    let usage = |why: &str| {
        eprintln!("{why}\n{USAGE}");
        ExitCode::from(2)
    };
    let options = match RunnerOptions::parse(args) {
        Ok(options) => options,
        Err(why) => return usage(&why),
    };
    let mut overrides = ModuleParameters::default();
    for (id, literal) in &options.module_parameters {
        match parameter(id, literal) {
            Ok(value) => overrides.set(id.clone(), value),
            Err(refused) => return usage(&format!("--module-parameter {id}={literal}: {refused}")),
        }
    }
    let results = match run_suite_with_traces(
        suite,
        &options.world,
        |test_case| read_trace(&options.trace_dir, test_case),
        &overrides,
    ) {
        Ok(results) => results,
        Err(refused) => return usage(refused.reason()),
    };
    if let Err(error) = std::fs::create_dir_all(&options.log_dir) {
        eprintln!("{} is not made: {error}", options.log_dir.display());
        return ExitCode::from(2);
    }
    for result in &results {
        for line in &result.result.log {
            if line.kind == LogKind::Error {
                eprintln!("{}: {}: {}", result.test_case, line.component, line.detail);
            }
        }
        let path = options
            .log_dir
            .join(format!("{}.log.json", result.test_case));
        if let Err(error) = std::fs::write(&path, result.canonical_log(&suite.name)) {
            eprintln!("{} is not written: {error}", path.display());
            return ExitCode::from(2);
        }
        println!("{} {}", result.test_case, result.result.verdict.name());
    }
    ExitCode::SUCCESS
}

/// The refusal of a value for the module parameter `id`, which the suite does
/// not declare.
pub fn undeclared_parameter(id: &str) -> OverrideRefusal {
    OverrideRefusal::new(format!("the suite declares no module parameter '{id}'"))
}

/// A value of the integer type `T` of the module parameter `id`: the text of
/// its decimal digits, as the canonical form writes one.
pub fn integer_parameter<T: TryFrom<i128> + SavedValue>(
    id: &str,
    literal: &str,
) -> Result<ModuleParameterValue, OverrideRefusal> {
    from_canonical::integer::<T>(&Value::Text(literal.to_string()), id)
        .map(|value| value.to_saved())
        .map_err(|refused| OverrideRefusal::new(refused.reason()))
}

/// A value of the `bool` module parameter `id`: `true` or `false`.
pub fn bool_parameter(id: &str, literal: &str) -> Result<ModuleParameterValue, OverrideRefusal> {
    match literal {
        "true" => Ok(Value::Bool(true)),
        "false" => Ok(Value::Bool(false)),
        _ => Err(OverrideRefusal::new(format!(
            "{id}: '{literal}' is neither true nor false"
        ))),
    }
}

/// A value of a text module parameter: the text itself. Its bound, like every
/// value's type, is held where the component reads it, as for a default.
pub fn text_parameter(literal: &str) -> ModuleParameterValue {
    Value::Text(literal.to_string())
}

/// A value of an enum module parameter: the name its document declares for the
/// variant, which is how a value of an enum is saved. A name its enum does not
/// declare is refused where the component reads it, as for a default.
pub fn enum_parameter(literal: &str) -> ModuleParameterValue {
    Value::Text(literal.to_string())
}

/// A value of the byte string module parameter `id`: its octets in hex, two
/// digits each, as a literal of docs/SCE_TEST_PROFILE.md §11.4 writes them.
pub fn bytes_parameter(id: &str, literal: &str) -> Result<ModuleParameterValue, OverrideRefusal> {
    let refused = || OverrideRefusal::new(format!("{id}: '{literal}' is not octets in hex"));
    let pairs = literal.as_bytes().chunks_exact(2);
    if !pairs.remainder().is_empty() {
        return Err(refused());
    }
    let octets: Option<Vec<u8>> = pairs
        .map(|pair| {
            // Digits alone: `from_str_radix` would also read a sign.
            if !pair.iter().all(u8::is_ascii_hexdigit) {
                return None;
            }
            u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()
        })
        .collect();
    octets
        .map(|octets| bytes_to_saved(&octets))
        .ok_or_else(refused)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| item.to_string()).collect()
    }

    /// The command line every backend's runner takes.
    #[test]
    fn the_command_line_is_read_as_every_runner_reads_it() {
        let options = RunnerOptions::parse(args(&[
            "--trace-dir",
            "traces",
            "--log-dir",
            "logs",
            "--world",
            "ptcs,guard",
            "--module-parameter",
            "job=9",
            "--module-parameter",
            "label=a=b",
        ]))
        .expect("the program's command line");
        assert_eq!(options.trace_dir, PathBuf::from("traces"));
        assert_eq!(options.log_dir, PathBuf::from("logs"));
        assert_eq!(
            options.world,
            Features::from(["ptcs".to_string(), "guard".to_string()])
        );
        assert_eq!(
            options.module_parameters,
            [
                ("job".to_string(), "9".to_string()),
                ("label".to_string(), "a=b".to_string())
            ]
        );
        let bare = RunnerOptions::parse(args(&["--log-dir", "l", "--trace-dir", "t"]))
            .expect("no world and no parameter");
        assert!(bare.world.is_empty());
        for (line, why) in [
            (args(&["--log-dir", "l"]), "--trace-dir is not given"),
            (args(&["--trace-dir", "t"]), "--log-dir is not given"),
            (
                args(&["--trace-dir", "t", "--log-dir", "l", "--trace-dir", "u"]),
                "--trace-dir is given twice",
            ),
            (
                args(&["--trace-dir", "t", "--log-dir", "l", "--seed", "1"]),
                "--seed is no option",
            ),
            (args(&["--trace-dir"]), "--trace-dir is given no value"),
            (
                args(&[
                    "--trace-dir",
                    "t",
                    "--log-dir",
                    "l",
                    "--module-parameter",
                    "job",
                ]),
                "is not <id>=<value>",
            ),
            (
                args(&[
                    "--trace-dir",
                    "t",
                    "--log-dir",
                    "l",
                    "--module-parameter",
                    "job=1",
                    "--module-parameter",
                    "job=2",
                ]),
                "'job' is given twice",
            ),
        ] {
            let refused = RunnerOptions::parse(line).expect_err(why);
            assert!(refused.contains(why), "{why}: {refused}");
        }
    }

    /// A module parameter's literal is read as the saved value of its type, and
    /// one that is no value of it is refused, naming the parameter.
    #[test]
    fn a_module_parameter_is_read_as_the_saved_value_of_its_type() {
        assert_eq!(integer_parameter::<u32>("job", "9"), Ok(9_u32.to_saved()));
        assert_eq!(
            integer_parameter::<u64>("wide", "18446744073709551615"),
            Ok(u64::MAX.to_saved())
        );
        assert!(integer_parameter::<u8>("job", "256")
            .expect_err("past u8")
            .reason()
            .starts_with("job: "));
        assert!(integer_parameter::<u32>("job", "09").is_err());
        assert_eq!(bool_parameter("armed", "true"), Ok(Value::Bool(true)));
        assert!(bool_parameter("armed", "yes").is_err());
        assert_eq!(
            text_parameter("nightly"),
            Value::Text("nightly".to_string())
        );
        assert_eq!(enum_parameter("green"), Value::Text("green".to_string()));
        assert_eq!(
            bytes_parameter("key", "0aFF"),
            Ok(bytes_to_saved(&[0x0a, 0xff]))
        );
        assert!(bytes_parameter("key", "0a0").is_err());
        assert!(bytes_parameter("key", "0g").is_err());
        assert!(bytes_parameter("key", "+a").is_err());
        assert_eq!(
            undeclared_parameter("limit").reason(),
            "the suite declares no module parameter 'limit'"
        );
    }
}
