// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The reference adapter's exposition, read back by a parser of the OpenMetrics 1.0 text format
//! (docs/adr/0109, D16: "the exposition parsed by an OpenMetrics parser").
//!
//! No OpenMetrics parser is in the workspace's lock, and the exposition's grammar is small, so the
//! parser is written here from the specification's ABNF and its rules for each type, strictly: a
//! line, a name, a label, a number or a family the format does not take is refused with where. Its
//! own refusals are tested below, so that a parser that took everything would fail them.

use std::collections::BTreeMap;

use sce_rust_metrics::openmetrics::OpenMetrics;
use sce_rust_metrics::{
    AuthenticatorKind, Counter, Eviction, ExchangeOutcome, Gauge, Histogram, HostFailure,
    HostOperation, Kind, Metrics, PoolShed, ResponseStatus, RestoredFrom, ServerShed, FAMILIES,
    REGISTRY,
};

/// The labels of a sample, each its name and its value, in the order the line gives them.
type LabelPairs = Vec<(String, String)>;

/// One sample line: its name, its labels in order, and its value.
#[derive(Debug, Clone, PartialEq)]
struct Sample {
    name: String,
    labels: LabelPairs,
    value: f64,
}

/// One family: its name, its metadata, and its samples in order.
#[derive(Debug, Clone, PartialEq)]
struct Parsed {
    name: String,
    kind: String,
    unit: Option<String>,
    help: Option<String>,
    samples: Vec<Sample>,
}

const TYPES: [&str; 8] = [
    "counter",
    "gauge",
    "histogram",
    "gaugehistogram",
    "stateset",
    "info",
    "summary",
    "unknown",
];

/// The suffixes a sample of a family of `kind` may add to the family's name.
fn suffixes(kind: &str) -> &'static [&'static str] {
    match kind {
        "counter" => &["_total", "_created"],
        "gauge" | "unknown" | "stateset" => &[""],
        "histogram" => &["_bucket", "_count", "_sum", "_created"],
        "gaugehistogram" => &["_bucket", "_gcount", "_gsum"],
        "summary" => &["", "_count", "_sum", "_created"],
        "info" => &["_info"],
        _ => &[],
    }
}

/// `metricname = metricname-initial-char 0*metricname-char`, `[a-zA-Z_:][a-zA-Z0-9_:]*`.
fn is_metric_name(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == ':')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':')
}

/// `label-name = label-name-initial-char *label-name-char`, `[a-zA-Z_][a-zA-Z0-9_]*`.
fn is_label_name(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// `number = realnumber / ("+" / "-") ("inf" / "infinity") / "nan"`, the words in any case;
/// `realnumber = [sign] ((1*DIGIT ["." *DIGIT]) / ("." 1*DIGIT)) [("e" / "E") [sign] 1*DIGIT]`.
fn number(text: &str) -> Option<f64> {
    let lower = text.to_ascii_lowercase();
    match lower.as_str() {
        "+inf" | "+infinity" => return Some(f64::INFINITY),
        "-inf" | "-infinity" => return Some(f64::NEG_INFINITY),
        "nan" => return Some(f64::NAN),
        _ => {}
    }
    let bytes = text.as_bytes();
    let mut at = 0;
    if at < bytes.len() && (bytes[at] == b'+' || bytes[at] == b'-') {
        at += 1;
    }
    let whole = bytes[at..]
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .count();
    at += whole;
    let mut fraction = 0;
    if at < bytes.len() && bytes[at] == b'.' {
        at += 1;
        fraction = bytes[at..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count();
        at += fraction;
    }
    if whole == 0 && fraction == 0 {
        return None;
    }
    if at < bytes.len() && (bytes[at] == b'e' || bytes[at] == b'E') {
        at += 1;
        if at < bytes.len() && (bytes[at] == b'+' || bytes[at] == b'-') {
            at += 1;
        }
        let exponent = bytes[at..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count();
        if exponent == 0 {
            return None;
        }
        at += exponent;
    }
    if at != bytes.len() {
        return None;
    }
    text.parse().ok()
}

/// `escaped-string`: every character but a line feed, a backslash and a double quote, or one of
/// `\\`, `\"` and `\n`; read from `text` up to the double quote that ends it, which is consumed.
/// Answers the string and what follows the quote.
fn escaped_until_quote(text: &str) -> Result<(String, &str), String> {
    let mut out = String::new();
    let mut chars = text.char_indices();
    while let Some((at, c)) = chars.next() {
        match c {
            '"' => return Ok((out, &text[at + 1..])),
            '\\' => match chars.next() {
                Some((_, '\\')) => out.push('\\'),
                Some((_, '"')) => out.push('"'),
                Some((_, 'n')) => out.push('\n'),
                other => return Err(format!("an escape the grammar does not take: {other:?}")),
            },
            '\n' => return Err("a line feed in a string".to_string()),
            other => out.push(other),
        }
    }
    Err("a string that is not closed".to_string())
}

/// The labels of a sample read from its line: the pairs in order, and what follows them.
struct LabelsRead<'a> {
    pairs: LabelPairs,
    rest: &'a str,
}

/// `labels = "{" [label *(COMMA label)] "}"`, `label = label-name EQ DQUOTE escaped-string
/// DQUOTE`. Answers the labels and what follows the closing brace.
fn labels(text: &str) -> Result<LabelsRead<'_>, String> {
    let mut rest = text
        .strip_prefix('{')
        .ok_or("labels that do not open with a brace")?;
    let mut read: LabelPairs = Vec::new();
    if let Some(after) = rest.strip_prefix('}') {
        return Ok(LabelsRead {
            pairs: read,
            rest: after,
        });
    }
    loop {
        let equals = rest.find('=').ok_or("a label with no value")?;
        let name = &rest[..equals];
        if !is_label_name(name) {
            return Err(format!("a label name the grammar does not take: {name:?}"));
        }
        if read.iter().any(|(held, _)| held == name) {
            return Err(format!("the label {name} twice"));
        }
        let after = rest[equals + 1..]
            .strip_prefix('"')
            .ok_or("a label value that is not quoted")?;
        let (value, after) = escaped_until_quote(after)?;
        read.push((name.to_string(), value));
        if let Some(after) = after.strip_prefix(',') {
            if after.starts_with('}') {
                return Err("a comma before the closing brace".to_string());
            }
            rest = after;
            continue;
        }
        if let Some(after) = after.strip_prefix('}') {
            return Ok(LabelsRead {
                pairs: read,
                rest: after,
            });
        }
        return Err("labels that do not close".to_string());
    }
}

/// One sample line: `metric = metricname [labels] SP number [SP timestamp] LF`.
fn sample(line: &str) -> Result<Sample, String> {
    let end = line.find(['{', ' ']).ok_or("a sample with no value")?;
    let name = &line[..end];
    if !is_metric_name(name) {
        return Err(format!("a metric name the grammar does not take: {name:?}"));
    }
    let read = if line[end..].starts_with('{') {
        labels(&line[end..])?
    } else {
        LabelsRead {
            pairs: Vec::new(),
            rest: &line[end..],
        }
    };
    let rest = read
        .rest
        .strip_prefix(' ')
        .ok_or("a value not after one space")?;
    let mut tokens = rest.split(' ');
    let value = tokens
        .next()
        .and_then(number)
        .ok_or("a value that is no number")?;
    if let Some(timestamp) = tokens.next() {
        number(timestamp).ok_or("a timestamp that is no number")?;
    }
    if tokens.next().is_some() {
        return Err("more after the value than a timestamp".to_string());
    }
    Ok(Sample {
        name: name.to_string(),
        labels: read.pairs,
        value,
    })
}

/// The labels of a histogram sample but `le`, which group its buckets into one series.
fn series_of(sample: &Sample) -> LabelPairs {
    sample
        .labels
        .iter()
        .filter(|(name, _)| name != "le")
        .cloned()
        .collect()
}

/// What each type asks of its samples beyond the grammar.
fn judge(family: &Parsed) -> Result<(), String> {
    let name = &family.name;
    for sample in &family.samples {
        let suffix = sample
            .name
            .strip_prefix(name.as_str())
            .ok_or_else(|| format!("{} outside its family {name}", sample.name))?;
        if !suffixes(&family.kind).contains(&suffix) {
            return Err(format!("{} in a {} family", sample.name, family.kind));
        }
        let le = sample.labels.iter().any(|(label, _)| label == "le");
        if le != (suffix == "_bucket") {
            return Err(format!(
                "{}: le belongs to a bucket and only to it",
                sample.name
            ));
        }
    }
    let mut seen: Vec<(String, LabelPairs)> = Vec::new();
    for sample in &family.samples {
        let key = (sample.name.clone(), sample.labels.clone());
        if seen.contains(&key) {
            return Err(format!("{} with one label set twice", sample.name));
        }
        seen.push(key);
    }
    match family.kind.as_str() {
        "counter" => {
            for sample in &family.samples {
                if sample.value.is_nan() || sample.value < 0.0 {
                    return Err(format!("{} is a counter below zero", sample.name));
                }
            }
        }
        "info" => {
            for sample in &family.samples {
                if sample.value != 1.0 {
                    return Err(format!("{} is an info whose value is not 1", sample.name));
                }
            }
        }
        "histogram" => {
            let mut groups: BTreeMap<LabelPairs, Vec<&Sample>> = BTreeMap::new();
            for sample in &family.samples {
                groups.entry(series_of(sample)).or_default().push(sample);
            }
            for (labels, samples) in groups {
                let mut last_bound = f64::NEG_INFINITY;
                let mut last_count = 0.0;
                let mut infinite = None;
                let mut count = None;
                for sample in &samples {
                    match sample.name.strip_prefix(name.as_str()) {
                        Some("_bucket") => {
                            let bound = sample
                                .labels
                                .iter()
                                .find(|(label, _)| label == "le")
                                .and_then(|(_, value)| number(value))
                                .ok_or("a bucket whose le is no number")?;
                            if bound <= last_bound {
                                return Err(format!("{labels:?}: buckets that do not rise"));
                            }
                            if sample.value < last_count {
                                return Err(format!("{labels:?}: buckets that are not cumulative"));
                            }
                            last_bound = bound;
                            last_count = sample.value;
                            if bound == f64::INFINITY {
                                infinite = Some(sample.value);
                            }
                        }
                        Some("_count") => count = Some(sample.value),
                        _ => {}
                    }
                }
                let infinite = infinite.ok_or_else(|| format!("{labels:?}: no +Inf bucket"))?;
                if count.is_some_and(|count| count != infinite) {
                    return Err(format!("{labels:?}: a count that is not the +Inf bucket's"));
                }
            }
        }
        _ => {}
    }
    Ok(())
}

/// `exposition = metricset HASH SP eof [LF]`: the families in order, each its metadata and then
/// its samples, every family name once.
fn parse(text: &str) -> Result<Vec<Parsed>, String> {
    let body = text
        .strip_suffix("# EOF\n")
        .or_else(|| text.strip_suffix("# EOF"))
        .ok_or("no # EOF at the end")?;
    let mut families: Vec<Parsed> = Vec::new();
    if body.is_empty() {
        return Ok(families);
    }
    let lines = body
        .strip_suffix('\n')
        .ok_or("a line before # EOF that does not end")?;
    for (number_of, line) in lines.split('\n').enumerate() {
        let at = |why: String| format!("line {}: {why}: {line:?}", number_of + 1);
        if let Some(meta) = line.strip_prefix("# ") {
            let mut parts = meta.splitn(3, ' ');
            let (Some(keyword), Some(name), Some(value)) =
                (parts.next(), parts.next(), parts.next())
            else {
                return Err(at("metadata of fewer than three parts".to_string()));
            };
            if !is_metric_name(name) {
                return Err(at("a family name the grammar does not take".to_string()));
            }
            let opens = match families.last() {
                Some(family) => family.name != name || !family.samples.is_empty(),
                None => true,
            };
            if opens {
                if families.iter().any(|family| family.name == name) {
                    return Err(at(format!("the family {name} again")));
                }
                families.push(Parsed {
                    name: name.to_string(),
                    kind: "unknown".to_string(),
                    unit: None,
                    help: None,
                    samples: Vec::new(),
                });
            }
            let family = families.last_mut().expect("a family was opened");
            match keyword {
                "TYPE" => {
                    if !TYPES.contains(&value) {
                        return Err(at(format!("a type the format does not name: {value}")));
                    }
                    if !opens && family.kind != "unknown" {
                        return Err(at("a second TYPE".to_string()));
                    }
                    family.kind = value.to_string();
                }
                "UNIT" => {
                    if family.unit.is_some() {
                        return Err(at("a second UNIT".to_string()));
                    }
                    if !name.ends_with(&format!("_{value}")) {
                        return Err(at("a family that does not end with its unit".to_string()));
                    }
                    family.unit = Some(value.to_string());
                }
                "HELP" => {
                    if family.help.is_some() {
                        return Err(at("a second HELP".to_string()));
                    }
                    let mut quoted = value.to_string();
                    quoted.push('"');
                    let (help, rest) = escaped_until_quote(&quoted).map_err(&at)?;
                    if !rest.is_empty() {
                        return Err(at("a help with a double quote".to_string()));
                    }
                    family.help = Some(help);
                }
                other => {
                    return Err(at(format!(
                        "a metadata keyword the format does not name: {other}"
                    )))
                }
            }
            continue;
        }
        if line.starts_with('#') {
            return Err(at("a comment the format does not take".to_string()));
        }
        let read = sample(line).map_err(&at)?;
        let family = families
            .last_mut()
            .filter(|family| read.name.starts_with(family.name.as_str()))
            .ok_or_else(|| at("a sample of no family open here".to_string()))?;
        family.samples.push(read);
    }
    for family in &families {
        if family.kind == "unknown" {
            return Err(format!("{} has no TYPE", family.name));
        }
        judge(family)?;
    }
    Ok(families)
}

/// The value of the sample `name` with exactly `labels` in `families`.
fn value_of(families: &[Parsed], name: &str, labels: &[(&str, &str)]) -> Option<f64> {
    families
        .iter()
        .flat_map(|family| family.samples.iter())
        .find(|sample| {
            sample.name == name
                && sample.labels.len() == labels.len()
                && sample.labels.iter().zip(labels).all(
                    |((name, value), (want_name, want_value))| {
                        name == want_name && value == want_value
                    },
                )
        })
        .map(|sample| sample.value)
}

/// Every counter series of the registry, each label set taken whole.
fn every_counter() -> Vec<Counter> {
    let mut all = Vec::new();
    for status in ResponseStatus::ALL {
        all.push(Counter::ServerExchanges { status: *status });
    }
    for status in ExchangeOutcome::ALL {
        all.push(Counter::ClientExchanges { status: *status });
    }
    for reason in ServerShed::ALL {
        all.push(Counter::ServerSheds { reason: *reason });
    }
    for authenticator in AuthenticatorKind::ALL {
        all.push(Counter::AuthenticationFailures {
            authenticator: *authenticator,
        });
    }
    for checkpoint in RestoredFrom::ALL {
        all.push(Counter::Restores {
            checkpoint: *checkpoint,
        });
    }
    for reason in Eviction::ALL {
        all.push(Counter::Evictions { reason: *reason });
    }
    for reason in PoolShed::ALL {
        all.push(Counter::PoolSheds { reason: *reason });
    }
    for operation in HostOperation::ALL {
        for code in HostFailure::ALL {
            all.push(Counter::HostOperationFailures {
                operation: *operation,
                code: *code,
            });
        }
    }
    all.extend(Counter::UNLABELLED);
    all
}

/// An adapter told of nothing renders every family of the registry, in order, with its type and
/// unit, the families of no label at zero and the registry's name. Red if a family's metadata
/// is left out or a series of no label is not rendered before it moves.
#[test]
fn an_adapter_told_of_nothing_renders_every_family_the_parser_takes() {
    let text = OpenMetrics::new().render();
    let families = parse(&text).unwrap_or_else(|why| panic!("{why}\n{text}"));
    let names: Vec<&str> = families.iter().map(|family| family.name.as_str()).collect();
    let registry: Vec<&str> = FAMILIES.iter().map(|family| family.name).collect();
    assert_eq!(names, registry);
    for (parsed, family) in families.iter().zip(FAMILIES.iter()) {
        assert_eq!(parsed.kind, family.kind.name(), "{}", family.name);
        assert_eq!(
            parsed.unit.as_deref().unwrap_or(""),
            family.unit,
            "{}",
            family.name
        );
        assert_eq!(parsed.help.as_deref(), Some(family.help), "{}", family.name);
    }
    assert_eq!(
        value_of(&families, "sce_metrics_info", &[("registry", REGISTRY)]),
        Some(1.0)
    );
    for counter in Counter::UNLABELLED {
        let name = format!("{}_total", counter.family().name);
        assert_eq!(value_of(&families, &name, &[]), Some(0.0), "{name}");
    }
    for gauge in Gauge::ALL {
        assert_eq!(
            value_of(&families, gauge.family().name, &[]),
            Some(0.0),
            "{gauge:?}"
        );
    }
    assert_eq!(
        value_of(&families, "sce_store_commit_batch_size_count", &[]),
        Some(0.0)
    );
    assert!(
        families
            .iter()
            .find(|family| family.name == "sce_server_exchanges")
            .is_some_and(|family| family.samples.is_empty()),
        "a labelled family shows no series before one moves"
    );
}

/// Every counter series of the registry, each moved by a different amount, every gauge and a
/// histogram of each family, rendered and read back: each value is the one the adapter was told.
/// Red if a series is rendered under another name or label, or a histogram's buckets are not
/// cumulative.
#[test]
fn every_series_is_rendered_as_it_was_told() {
    let metrics = OpenMetrics::new();
    let counters = every_counter();
    for (index, counter) in counters.iter().enumerate() {
        metrics.count(*counter, index as u64 + 1);
    }
    metrics.adjust(Gauge::HeldExchanges, 3);
    metrics.adjust(Gauge::HeldExchanges, -1);
    metrics.adjust(Gauge::LiveSessions, 7);
    metrics.adjust(Gauge::OutboxDepth, 11);
    for value in [0.0004, 0.001, 0.003, 100.0] {
        metrics.observe(Histogram::CommitDuration, value);
    }
    metrics.observe(Histogram::CommitBatchSize, 3.0);
    metrics.observe(
        Histogram::HostOperationDuration {
            operation: HostOperation::KvGet,
        },
        0.02,
    );
    metrics.observe(Histogram::CommitBatchSize, f64::NAN);

    let text = metrics.render();
    let families = parse(&text).unwrap_or_else(|why| panic!("{why}\n{text}"));
    for (index, counter) in counters.iter().enumerate() {
        let name = format!("{}_total", counter.family().name);
        assert_eq!(
            value_of(&families, &name, counter.labels().as_slice()),
            Some(index as f64 + 1.0),
            "{counter:?}"
        );
    }
    assert_eq!(
        value_of(&families, "sce_server_held_exchanges", &[]),
        Some(2.0)
    );
    assert_eq!(
        value_of(&families, "sce_pool_live_sessions", &[]),
        Some(7.0)
    );
    assert_eq!(
        value_of(&families, "sce_pool_outbox_depth", &[]),
        Some(11.0)
    );

    let bucket = |le: &str| {
        value_of(
            &families,
            "sce_store_commit_duration_seconds_bucket",
            &[("le", le)],
        )
    };
    assert_eq!(bucket("0.0005"), Some(1.0));
    assert_eq!(bucket("0.001"), Some(2.0), "a bound holds what equals it");
    assert_eq!(bucket("0.0025"), Some(2.0));
    assert_eq!(bucket("0.005"), Some(3.0));
    assert_eq!(bucket("30.0"), Some(3.0));
    assert_eq!(bucket("+Inf"), Some(4.0));
    assert_eq!(
        value_of(&families, "sce_store_commit_duration_seconds_count", &[]),
        Some(4.0)
    );
    let sum = value_of(&families, "sce_store_commit_duration_seconds_sum", &[]).expect("a sum");
    assert!((sum - 100.0044).abs() < 1e-9, "{sum}");
    assert_eq!(
        value_of(&families, "sce_store_commit_batch_size_count", &[]),
        Some(1.0),
        "a value that is not a number is no observation"
    );
    assert_eq!(
        value_of(
            &families,
            "sce_host_operation_duration_seconds_bucket",
            &[("type", "sce-host-kv-get"), ("le", "0.025")]
        ),
        Some(1.0)
    );
    assert_eq!(metrics.counter(Counter::Loads), counters.len() as u64 - 3);
    assert_eq!(metrics.gauge(Gauge::HeldExchanges), 2);
    assert_eq!(metrics.histogram(Histogram::CommitDuration).0, 4);
    assert_eq!(Kind::Info.name(), "info");
}

/// The parser refuses what the format does not take, each from an exposition it takes changed
/// in one place: so its acceptance of the adapter's output means something. Red if the parser
/// is loosened to take any of them.
#[test]
fn the_parser_refuses_what_the_format_does_not_take() {
    let good = "# TYPE a counter\n# HELP a A.\na_total{x=\"1\"} 1\n\
                # TYPE h histogram\nh_bucket{le=\"1.0\"} 1\nh_bucket{le=\"+Inf\"} 2\nh_count 2\nh_sum 3.0\n\
                # EOF\n";
    parse(good).expect("the control parses");
    let refused = [
        ("no end", good.replace("# EOF\n", "")),
        (
            "a sample of no family",
            good.replace("a_total{", "b_total{"),
        ),
        (
            "a gauge suffix on a counter",
            good.replace("a_total{", "a{"),
        ),
        ("a trailing comma", good.replace("x=\"1\"}", "x=\"1\",}")),
        ("an unquoted value", good.replace("x=\"1\"", "x=1")),
        ("an unknown escape", good.replace("x=\"1\"", "x=\"\\t\"")),
        (
            "a label name with a dash",
            good.replace("x=\"1\"", "x-y=\"1\""),
        ),
        (
            "a counter below zero",
            good.replace("a_total{x=\"1\"} 1", "a_total{x=\"1\"} -1"),
        ),
        (
            "a value that is no number",
            good.replace("a_total{x=\"1\"} 1", "a_total{x=\"1\"} one"),
        ),
        (
            "buckets that fall",
            good.replace("h_bucket{le=\"+Inf\"} 2", "h_bucket{le=\"+Inf\"} 0"),
        ),
        (
            "no +Inf bucket",
            good.replace("h_bucket{le=\"+Inf\"} 2\n", ""),
        ),
        (
            "a count that is not +Inf's",
            good.replace("h_count 2", "h_count 5"),
        ),
        (
            "a family twice",
            good.replace("# TYPE h histogram", "# TYPE a histogram"),
        ),
        (
            "an unknown type",
            good.replace("# TYPE a counter", "# TYPE a tally"),
        ),
        ("a plain comment", good.replace("# HELP a A.", "# a remark")),
        (
            "a unit the name does not end with",
            good.replace("# HELP a A.", "# UNIT a seconds"),
        ),
    ];
    for (why, text) in refused {
        assert!(parse(&text).is_err(), "{why} was taken:\n{text}");
    }
}
