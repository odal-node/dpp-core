//! Every date a rule selects its answer by is on the obligation calendar, or is
//! listed here with the reason it is not.
//!
//! # The gap this closes
//!
//! `dpp-rules` holds its effective dates as `CalendarDate` constants, and the
//! calendar in `dpp-domain` reads each rule module's `SCHEDULE`. A constant
//! added to a rule and left out of its module's `SCHEDULE`, or a `SCHEDULE` the
//! calendar never reads, is a date the rule applies and nobody is told about.
//! Neither breaks a build.
//!
//! # What this checks
//!
//! - Every `pub const NAME: CalendarDate` in `dpp-rules` is the `date` of a
//!   `SCHEDULE` entry in its own file, or is in [`NOT_SCHEDULED`] with a reason.
//! - Every id in every `SCHEDULE` is returned by the calendar.
//! - Every file that lists a `SCHEDULE` carries a ✅ COMPLIANCE-PIN, since the
//!   calendar reports its dates as sourced.
//! - [`NOT_SCHEDULED`] does not go stale.
//!
//! # What it cannot prove
//!
//! That a date is right. It proves the date is visible, which is what a
//! calendar can promise.

use std::fs;
use std::path::{Path, PathBuf};

use dpp_domain::ObligationCalendar;

/// `CalendarDate` constants that are deliberately not on the calendar, each
/// with the reason.
const NOT_SCHEDULED: &[(&str, &str)] = &[
    (
        "PASSPORT_REQUIRED_FROM",
        "the battery passport start date, which the calendar takes from the battery \
         instrument; a calendar test asserts the two agree",
    ),
    (
        "ART8_DECLARATION_FLOOR_2028",
        "the earliest the Art. 8(1) declaration duty can begin: its start turns on a \
         delegated act not yet adopted, so there is no date to announce",
    ),
    (
        "ART8_DECLARATION_FLOOR_2033",
        "the same floor for LMT batteries",
    ),
];

fn rules_src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../dpp-rules/src")
}

/// Every `.rs` file under `dir` that is not a test module.
fn source_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            source_files(&path, out);
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.ends_with(".rs") && name != "tests.rs" && !name.ends_with("_tests.rs") {
            out.push(path);
        }
    }
}

/// The names of the public `CalendarDate` constants `src` declares.
fn date_constants(src: &str) -> Vec<String> {
    src.lines()
        .filter_map(|line| {
            let rest = line.trim_start().strip_prefix("pub const ")?;
            let (name, ty) = rest.split_once(':')?;
            ty.trim_start()
                .starts_with("CalendarDate =")
                .then(|| name.trim().to_owned())
        })
        .collect()
}

/// Whether `src` uses `name` as the `date` of a schedule entry.
fn is_scheduled(src: &str, name: &str) -> bool {
    let entry = format!("date: {name},");
    src.lines().any(|line| line.trim() == entry)
}

/// The ids of the schedule entries `src` lists, if it lists a `SCHEDULE`.
fn schedule_ids(src: &str) -> Option<Vec<String>> {
    if !src.contains("pub const SCHEDULE: &[ScheduledChange]") {
        return None;
    }
    Some(
        src.lines()
            .filter_map(|line| {
                let rest = line.trim().strip_prefix("id: \"")?;
                Some(rest.split_once('"')?.0.to_owned())
            })
            .collect(),
    )
}

/// Every unscheduled, unlisted constant in `src`, by name.
fn unreachable_constants(src: &str) -> Vec<String> {
    date_constants(src)
        .into_iter()
        .filter(|name| !is_scheduled(src, name))
        .filter(|name| !NOT_SCHEDULED.iter().any(|(n, _)| n == name))
        .collect()
}

fn rule_sources() -> Vec<(PathBuf, String)> {
    let mut files = Vec::new();
    source_files(&rules_src(), &mut files);
    assert!(!files.is_empty(), "no dpp-rules sources found");
    files
        .into_iter()
        .map(|f| {
            let src = fs::read_to_string(&f).unwrap();
            (f, src)
        })
        .collect()
}

#[test]
fn every_rule_date_is_on_the_calendar_or_says_why_not() {
    let mut unreachable = Vec::new();
    for (file, src) in rule_sources() {
        for name in unreachable_constants(&src) {
            unreachable.push(format!("{} ({})", name, file.display()));
        }
    }
    assert!(
        unreachable.is_empty(),
        "these CalendarDate constants are on no SCHEDULE, so the obligation calendar \
         cannot see them. List each in its module's SCHEDULE, or in NOT_SCHEDULED with \
         the reason it has no date to announce:\n{}",
        unreachable.join("\n")
    );
}

#[test]
fn every_schedule_reaches_the_calendar_and_is_pinned() {
    let calendar = ObligationCalendar::new();
    let on_calendar: Vec<&str> = calendar.events().iter().map(|e| e.id.as_str()).collect();
    let mut missing = Vec::new();
    let mut unpinned = Vec::new();
    let mut schedules = 0;
    for (file, src) in rule_sources() {
        let Some(ids) = schedule_ids(&src) else {
            continue;
        };
        schedules += 1;
        if !src.contains("✅ COMPLIANCE-PIN") {
            unpinned.push(file.display().to_string());
        }
        for id in ids {
            if !on_calendar.contains(&id.as_str()) {
                missing.push(format!("{id} ({})", file.display()));
            }
        }
    }
    assert!(
        schedules >= 3,
        "found {schedules} SCHEDULE lists; the scan is broken"
    );
    assert!(
        missing.is_empty(),
        "these scheduled changes are not on the calendar; add their module to \
         RULE_SCHEDULES:\n{}",
        missing.join("\n")
    );
    assert!(
        unpinned.is_empty(),
        "the calendar reports these files' dates as sourced, but they carry no \
         ✅ COMPLIANCE-PIN:\n{}",
        unpinned.join("\n")
    );
}

#[test]
fn the_not_scheduled_list_does_not_go_stale() {
    let sources = rule_sources();
    for (name, reason) in NOT_SCHEDULED {
        assert!(!reason.is_empty(), "{name} needs a reason");
        let declared_in: Vec<&String> = sources
            .iter()
            .filter(|(_, src)| date_constants(src).iter().any(|n| n == name))
            .map(|(_, src)| src)
            .collect();
        assert_eq!(
            declared_in.len(),
            1,
            "{name} is in NOT_SCHEDULED but is not a CalendarDate constant in dpp-rules"
        );
        assert!(
            !is_scheduled(declared_in[0], name),
            "{name} is now scheduled; remove it from NOT_SCHEDULED"
        );
    }
}

/// The detector, on a planted constant, so a gate that reads nothing cannot
/// pass by reading nothing.
#[test]
fn the_detector_catches_a_planted_constant() {
    let planted = "\
        pub const PLANTED_FROM: CalendarDate = CalendarDate::new(2099, 1, 1);\n\
        pub const LISTED_FROM: CalendarDate = CalendarDate::new(2098, 1, 1);\n\
        pub const SCHEDULE: &[ScheduledChange] = &[ScheduledChange {\n\
            id: \"test.listed\",\n\
            date: LISTED_FROM,\n\
        }];\n";
    assert_eq!(unreachable_constants(planted), ["PLANTED_FROM"]);
    assert_eq!(schedule_ids(planted), Some(vec!["test.listed".to_owned()]));
}
