// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//! The runtime's diagnostics are its `log` feature (docs/adr/0053, amendment of
//! 2026-11-25).
//!
//! With the feature each `sce_log_*!` call is one record of the `log` facade at
//! its level, under the target `sce`, reached through the runtime's re-export.
//! Without it the crate has no facade, and a call evaluates none of its
//! arguments and writes nothing.
//!
//! The default build runs the first module; `scripts/gates/nostd-mcu.sh` runs
//! this crate's tests with `--no-default-features --features std`, which runs
//! the second.

#[cfg(feature = "log")]
mod with_the_feature {
    use std::sync::Mutex;

    use sce_rust_runtime::log::{self, Level, LevelFilter, Log, Metadata, Record};

    /// Each record the facade was handed: its level, its target and its text.
    static RECORDS: Mutex<Vec<(Level, String, String)>> = Mutex::new(Vec::new());

    struct Capture;

    impl Log for Capture {
        fn enabled(&self, _: &Metadata<'_>) -> bool {
            true
        }

        fn log(&self, record: &Record<'_>) {
            RECORDS
                .lock()
                .expect("no test panics holding the records")
                .push((
                    record.level(),
                    record.target().to_owned(),
                    record.args().to_string(),
                ));
        }

        fn flush(&self) {}
    }

    static CAPTURE: Capture = Capture;

    #[test]
    fn each_call_is_one_record_at_its_level_under_the_target_sce() {
        log::set_logger(&CAPTURE).expect("no other test of this binary installs a logger");
        log::set_max_level(LevelFilter::Trace);

        let count = 3;
        sce_rust_runtime::sce_log_debug!("debug {count}");
        sce_rust_runtime::sce_log_info!("info {}", count);
        sce_rust_runtime::sce_log_warn!("warn {}", count + 1);
        let why = "text";
        sce_rust_runtime::sce_log_error!("error {why}");

        let records = RECORDS.lock().expect("no test panics holding the records");
        assert_eq!(
            *records,
            [
                (Level::Debug, "sce".to_owned(), "debug 3".to_owned()),
                (Level::Info, "sce".to_owned(), "info 3".to_owned()),
                (Level::Warn, "sce".to_owned(), "warn 4".to_owned()),
                (Level::Error, "sce".to_owned(), "error text".to_owned()),
            ]
        );
    }
}

#[cfg(not(feature = "log"))]
mod without_the_feature {
    use std::cell::Cell;

    #[test]
    fn a_call_evaluates_none_of_its_arguments() {
        let evaluated = Cell::new(0u32);
        let note = || {
            evaluated.set(evaluated.get() + 1);
            evaluated.get()
        };

        sce_rust_runtime::sce_log_debug!("debug {}", note());
        sce_rust_runtime::sce_log_info!("info {}", note());
        sce_rust_runtime::sce_log_warn!("warn {}", note());
        sce_rust_runtime::sce_log_error!("error {}", note());

        assert_eq!(evaluated.get(), 0);
    }
}
