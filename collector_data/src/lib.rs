pub mod cpu;
pub mod gpu;
pub mod gpu_dep;
pub mod job;
pub mod misc;
pub mod monitoring_info;
pub mod node;
pub mod slurm;

use std::path::{Path, PathBuf};

use chrono::{Duration, NaiveDate};
use color_eyre::{
    eyre::{ensure, eyre},
    Result,
};
use tracing::trace;

pub const DEFAULT_INTERVAL: Duration = Duration::seconds(30);
pub const DEFAULT_TIMEOUT: Duration = Duration::seconds(30);

pub const FILENAME_DATE_FMT: &str = "%Y-%m-%d";
pub const FILENAME_SUFFIX: &str = "json.br";

#[tracing::instrument(skip(path), fields(path=format!("{:?}", path.as_ref())))]
pub fn parse_filename(path: impl AsRef<Path>) -> Result<NaiveDate> {
    let path = path.as_ref();
    let filename = path.file_name().ok_or_else(|| eyre!("File has no file stem: {path:?}"))?;
    let filename = filename.to_str().ok_or_else(|| eyre!("File name contains non-unicode: {filename:?}"))?;

    let (date, suffix) = NaiveDate::parse_and_remainder(filename, FILENAME_DATE_FMT)?;

    trace!(filename, ?date, suffix);
    ensure!(suffix == format!(".{FILENAME_SUFFIX}"));

    Ok(date)
}

#[tracing::instrument]
pub fn generate_filename(date: NaiveDate) -> PathBuf {
    format!("{date}.{suffix}", date = date.format(FILENAME_DATE_FMT), suffix = FILENAME_SUFFIX).into()
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use chrono::NaiveDate;
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn generate_filename_() {
        let date = NaiveDate::from_ymd(2024, 11, 15);
        let expected_filename = "2024-11-15.json.br";
        let generated_filename = generate_filename(date);
        assert_eq!(generated_filename, PathBuf::from(expected_filename));
    }

    #[test]
    fn parse_filename_valid() {
        let path = PathBuf::from("2024-11-15.json.br");
        let expected_date = NaiveDate::from_ymd(2024, 11, 15);
        let parsed_date = parse_filename(path).expect("Failed to parse valid filename");
        assert_eq!(parsed_date, expected_date);
    }

    #[test]
    fn parse_and_generate_roundtrip() {
        let original_date = NaiveDate::from_ymd(2024, 11, 15);
        let filename = generate_filename(original_date);
        let parsed_date = parse_filename(filename).expect("Failed to parse generated filename");
        assert_eq!(parsed_date, original_date);
    }

    #[test]
    fn parse_filename_invalid_format() {
        let path = PathBuf::from("15-11-2024.json.br");
        let result = parse_filename(path);
        assert!(result.is_err(), "Expected error for invalid date format");
    }

    #[test]
    fn parse_filename_invalid_suffix() {
        let path = PathBuf::from("2024-11-15.txt");
        let result = parse_filename(path);
        assert!(result.is_err(), "Expected error for invalid file suffix");
    }

    #[test]
    fn parse_filename_missing_stem() {
        let path = PathBuf::from(".json.br");
        let result = parse_filename(path);
        assert!(result.is_err(), "Expected error for missing file stem");
    }

    #[test]
    fn parse_filename_non_unicode() {
        let path: PathBuf = vec![0x80 as char, 0x81 as char, 0x82 as char].into_iter().collect::<String>().into(); // Non-unicode bytes
        let result = parse_filename(path);
        assert!(result.is_err(), "Expected error for non-unicode filename");
    }

    #[test]
    fn parse_filename_empty() {
        let path = PathBuf::from("");
        let result = parse_filename(path);
        assert!(result.is_err(), "Expected error for empty filename");
    }

    #[test]
    fn generate_filename_edge_case() {
        let date = NaiveDate::from_ymd(1900, 1, 1);
        let expected_filename = "1900-01-01.json.br";
        let generated_filename = generate_filename(date);
        assert_eq!(generated_filename, PathBuf::from(expected_filename));
    }

    #[test]
    fn parse_and_generate_roundtrip_edge_case() {
        let original_date = NaiveDate::from_ymd(1900, 1, 1);
        let filename = generate_filename(original_date);
        let parsed_date = parse_filename(filename).expect("Failed to parse generated filename");
        assert_eq!(parsed_date, original_date);
    }
}
