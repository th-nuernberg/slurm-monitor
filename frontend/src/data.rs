use anyhow::{Context as _, Result};
use chrono::NaiveDateTime;

pub fn datetime_from_filename(name: &str) -> Result<NaiveDateTime> {
    const FORMAT: &str = "%Y_%m_%d__%H_%M_%S";
    const RESULTING_FORMAT_LEN: usize = "yyyy_mm_dd__hh_mm_ss".len();
    const POSITION: usize = 0; // datetime is at start of filename

    let datetime_part: String = name.chars().skip(POSITION).take(RESULTING_FORMAT_LEN).collect();
    NaiveDateTime::parse_from_str(datetime_part.as_str(), FORMAT).with_context(|| format!("parsing datetime from {name}"))
}
