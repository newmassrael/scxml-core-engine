// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The reference adapter of the metrics port: every series kept in memory and rendered as the
//! OpenMetrics text exposition (OpenMetrics 1.0), which the program serves on a route of its own
//! with [`CONTENT_TYPE`] (docs/SCE_SERVER_DEPLOY.md section 10).
//!
//! Each family of the registry is rendered in [`crate::FAMILIES`] order with its metadata
//! (`# TYPE`, `# UNIT` when it has one, `# HELP`), and then each of its series this adapter was
//! told of, in the order of its label values. A family of no label is rendered at zero before it
//! is told of anything, as a counter or a gauge that exists from the start; one with labels shows
//! only the series that moved, since a series is named by values a client's traffic chose among.
//! The exposition ends with `# EOF`.
//!
//! One lock guards every series: an increment is a few instructions under it, which a server
//! whose exchanges each wait for a durable write does not feel. A program that measures the lock
//! implements [`Metrics`] over a client of its own instead.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::{Counter, Family, Gauge, Histogram, Kind, Labels, Metrics, FAMILIES, REGISTRY};

/// The media type of the exposition (OpenMetrics 1.0, section "Overall Structure").
pub const CONTENT_TYPE: &str = "application/openmetrics-text; version=1.0.0; charset=utf-8";

/// The reference adapter. See the module documentation.
pub struct OpenMetrics {
    series: Mutex<Series>,
}

/// Every series told of, by its key.
#[derive(Default)]
struct Series {
    counters: BTreeMap<Counter, u64>,
    gauges: BTreeMap<Gauge, i64>,
    histograms: BTreeMap<Histogram, Observed>,
}

/// What a histogram series holds: the observations in each bucket (not yet cumulative, the last
/// one past every bound), their count and their sum.
#[derive(Debug, Clone)]
struct Observed {
    buckets: Vec<u64>,
    count: u64,
    sum: f64,
}

impl Observed {
    fn new(family: &Family) -> Self {
        Observed {
            buckets: vec![0; family.buckets.len() + 1],
            count: 0,
            sum: 0.0,
        }
    }
}

impl Default for OpenMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenMetrics {
    /// An adapter told of nothing yet: every family of no label at zero.
    pub fn new() -> Self {
        let mut series = Series::default();
        for counter in Counter::UNLABELLED {
            series.counters.insert(counter, 0);
        }
        for gauge in Gauge::ALL {
            series.gauges.insert(gauge, 0);
        }
        for histogram in Histogram::UNLABELLED {
            series
                .histograms
                .insert(histogram, Observed::new(histogram.family()));
        }
        OpenMetrics {
            series: Mutex::new(series),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Series> {
        // Every update leaves the series whole, so a thread that panicked holding the lock left
        // nothing half written.
        self.series.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The value of `counter`: 0 for one never told of.
    pub fn counter(&self, counter: Counter) -> u64 {
        self.lock().counters.get(&counter).copied().unwrap_or(0)
    }

    /// The level of `gauge`.
    pub fn gauge(&self, gauge: Gauge) -> i64 {
        self.lock().gauges.get(&gauge).copied().unwrap_or(0)
    }

    /// How many observations `histogram` holds, and their sum.
    pub fn histogram(&self, histogram: Histogram) -> (u64, f64) {
        self.lock()
            .histograms
            .get(&histogram)
            .map_or((0, 0.0), |observed| (observed.count, observed.sum))
    }

    /// The OpenMetrics text exposition of every series, ending with `# EOF`.
    pub fn render(&self) -> String {
        let series = self.lock();
        let mut out = String::new();
        for family in FAMILIES.iter() {
            metadata(&mut out, family);
            match family.kind {
                Kind::Info => {
                    sample(
                        &mut out,
                        family.name,
                        "_info",
                        &[("registry", REGISTRY)],
                        None,
                        "1",
                    );
                }
                Kind::Counter => {
                    for (counter, value) in &series.counters {
                        if counter.family().name == family.name {
                            sample(
                                &mut out,
                                family.name,
                                "_total",
                                counter.labels().as_slice(),
                                None,
                                &value.to_string(),
                            );
                        }
                    }
                }
                Kind::Gauge => {
                    for (gauge, value) in &series.gauges {
                        if gauge.family().name == family.name {
                            sample(
                                &mut out,
                                family.name,
                                "",
                                gauge.labels().as_slice(),
                                None,
                                &value.to_string(),
                            );
                        }
                    }
                }
                Kind::Histogram => {
                    for (histogram, observed) in &series.histograms {
                        if histogram.family().name == family.name {
                            histogram_samples(&mut out, family, histogram.labels(), observed);
                        }
                    }
                }
            }
        }
        out.push_str("# EOF\n");
        out
    }
}

impl Metrics for OpenMetrics {
    fn count(&self, counter: Counter, by: u64) {
        let mut series = self.lock();
        let value = series.counters.entry(counter).or_insert(0);
        *value = value.saturating_add(by);
    }

    fn adjust(&self, gauge: Gauge, delta: i64) {
        let mut series = self.lock();
        let value = series.gauges.entry(gauge).or_insert(0);
        *value = value.saturating_add(delta);
    }

    /// A value that is negative or not a number is no observation of any family here, all of
    /// which count durations and sizes, and is let go.
    fn observe(&self, histogram: Histogram, value: f64) {
        if value.is_nan() || value < 0.0 {
            return;
        }
        let family = histogram.family();
        let mut series = self.lock();
        let observed = series
            .histograms
            .entry(histogram)
            .or_insert_with(|| Observed::new(family));
        let bucket = family
            .buckets
            .iter()
            .position(|bound| value <= *bound)
            .unwrap_or(family.buckets.len());
        observed.buckets[bucket] = observed.buckets[bucket].saturating_add(1);
        observed.count = observed.count.saturating_add(1);
        observed.sum += value;
    }
}

/// The metadata lines of `family`.
fn metadata(out: &mut String, family: &Family) {
    let _ = writeln!(out, "# TYPE {} {}", family.name, family.kind.name());
    if !family.unit.is_empty() {
        let _ = writeln!(out, "# UNIT {} {}", family.name, family.unit);
    }
    let _ = writeln!(out, "# HELP {} {}", family.name, escaped(family.help));
}

/// The samples of one histogram series: its buckets, cumulative and ending with `+Inf`, then its
/// count and its sum.
fn histogram_samples(out: &mut String, family: &Family, labels: Labels, observed: &Observed) {
    let mut cumulative: u64 = 0;
    for (index, in_bucket) in observed.buckets.iter().enumerate() {
        cumulative = cumulative.saturating_add(*in_bucket);
        let bound = family
            .buckets
            .get(index)
            .map_or_else(|| "+Inf".to_string(), |bound| number(*bound));
        sample(
            out,
            family.name,
            "_bucket",
            labels.as_slice(),
            Some(bound.as_str()),
            &cumulative.to_string(),
        );
    }
    sample(
        out,
        family.name,
        "_count",
        labels.as_slice(),
        None,
        &observed.count.to_string(),
    );
    sample(
        out,
        family.name,
        "_sum",
        labels.as_slice(),
        None,
        &number(observed.sum),
    );
}

/// One sample line: `name` and `suffix`, the labels with `le` last when given, and `value`.
fn sample(
    out: &mut String,
    name: &str,
    suffix: &str,
    labels: &[(&str, &str)],
    le: Option<&str>,
    value: &str,
) {
    out.push_str(name);
    out.push_str(suffix);
    if !labels.is_empty() || le.is_some() {
        out.push('{');
        let mut separator = "";
        for (label, text) in labels {
            let _ = write!(out, "{separator}{label}=\"{}\"", escaped(text));
            separator = ",";
        }
        if let Some(bound) = le {
            let _ = write!(out, "{separator}le=\"{bound}\"");
        }
        out.push('}');
    }
    out.push(' ');
    out.push_str(value);
    out.push('\n');
}

/// `text` as an escaped string of the exposition: a backslash, a double quote and a line feed
/// each escaped by a backslash.
fn escaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            other => out.push(other),
        }
    }
    out
}

/// A number of the exposition: a whole number with one decimal (`1.0`, the canonical form a
/// bucket's bound takes), any other in its shortest decimal form.
fn number(value: f64) -> String {
    if value.is_finite() && value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{value:.1}")
    } else {
        format!("{value}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_whole_number_keeps_one_decimal_and_a_fraction_its_digits() {
        assert_eq!(number(1.0), "1.0");
        assert_eq!(number(0.0005), "0.0005");
        assert_eq!(number(2.5), "2.5");
        assert_eq!(number(0.0), "0.0");
    }

    #[test]
    fn an_escaped_string_escapes_what_the_grammar_reserves() {
        assert_eq!(escaped("a\\b\"c\nd"), "a\\\\b\\\"c\\nd");
    }
}
