use std::fmt::Debug;

use chrono::{DateTime, Local};
use derive_more::derive::{Deref, Into};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deref, Into, Serialize)]
pub struct SlurmUser(pub String);

/// Use `%+` format specifier (ISO 8601 / RFC 3339,`2001-07-08T00:34:60.026490+09:30`) then cut off
/// the `+xx:yy` end (slurm don't like).
///
/// working theory: slurm takes _local_ time, not UTC (sadly, the `+xx:yy` timestamp format doesn't
/// seem to work, and neither I nor ChatJibbidy were able to find any evidence supporting either
/// hypothesis)
// TODO maybe check that out ^^^
pub fn format_datetime_for_slurm(date: DateTime<Local>) -> String {
    const FMT: &str = "%Y-%m-%dT%H:%M:%S";
    // ISO 8601 / RFC 3339
    date.format(FMT).to_string()
}
