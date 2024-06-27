use std::{collections::HashMap, num::ParseIntError};

use anyhow::{anyhow, bail, Result};
use itertools::Itertools;
use thiserror::Error;

/// Given output from `sacct -P`, parses it into a line vector consisting of HashMaps. This works by taking the first line as header.
///
/// Returns a (header, data) tuple
pub fn sacct_csvlike(
    input: impl AsRef<str>,
) -> Result<(Vec<String>, Vec<Result<HashMap<String, String>>>)> {
    let input = input.as_ref();
    let mut lines = input.lines();
    let Some(header) = lines.next() else {
        bail!("data seems to be empty ({input})")
    };
    let header = header.split('|').map(String::from).collect_vec();

    let data = lines
        .enumerate()
        .map(|(line_number, line)| {
            line.split('|')
                .enumerate()
                .map(|(i, field)| match header.get(i) {
                    Some(key) => Ok((String::from(key), String::from(field))),
                    None => Err(anyhow!(
                        "Parsing error at line {line_number}: too many fields"
                    )),
                })
                .process_results(|iter| iter.collect::<HashMap<_, _>>())
        })
        .collect_vec();

    Ok((header, data))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileSize(pub usize);

impl FileSize {
    pub fn as_bytes(&self) -> usize {
        self.0
    }

    pub fn as_kib(&self) -> f64 {
        self.0 as f64 / 1024f64
    }

    pub fn as_mib(&self) -> f64 {
        self.0 as f64 / 1024f64.powi(2)
    }

    pub fn as_gib(&self) -> f64 {
        self.0 as f64 / 1024f64.powi(3)
    }

    pub fn as_tib(&self) -> f64 {
        self.0 as f64 / 1024f64.powi(4)
    }

    pub fn as_pib(&self) -> f64 {
        self.0 as f64 / 1024f64.powi(5)
    }

    pub fn from_bytes(val: usize) -> Self {
        Self(val)
    }

    pub fn from_kib(val: usize) -> Self {
        Self(val * 1024usize)
    }

    pub fn from_mib(val: usize) -> Self {
        Self(val * 1024usize.pow(2))
    }

    pub fn from_gib(val: usize) -> Self {
        Self(val * 1024usize.pow(3))
    }

    pub fn from_tib(val: usize) -> Self {
        Self(val * 1024usize.pow(4))
    }

    pub fn from_pib(val: usize) -> Self {
        Self(val * 1024usize.pow(5))
    }

    /// empty means empty after trimming
    pub fn parse(input: &str) -> Result<FileSize, FileSizeParseError> {
        use FileSizeParseError::*;
        let input = input.trim();

        fn strip_suffix(s: &str) -> String {
            s.chars().dropping_back(1).collect()
        }

        match input.chars().last() {
            Some('K') => Ok(FileSize::from_kib(strip_suffix(input).parse()?)),
            Some('M') => Ok(FileSize::from_mib(strip_suffix(input).parse()?)),
            Some('G') => Ok(FileSize::from_gib(strip_suffix(input).parse()?)),
            Some('T') => Ok(FileSize::from_tib(strip_suffix(input).parse()?)),
            Some('P') => Ok(FileSize::from_pib(strip_suffix(input).parse()?)),
            None => Err(Empty),
            _ => Ok(FileSize::from_bytes(input.parse()?)),
        }
    }
}

/// empty means empty after trimming
#[derive(Debug, Clone, Error)]
pub enum FileSizeParseError {
    #[error("trying to parse an empty string")]
    Empty,
    #[error("scalar part is not an usize")]
    InvalidInt(#[from] ParseIntError),
}

#[cfg(test)]
mod tests {
    use super::*;
    const ε: f64 = 0.0000000001;

    #[test]
    fn test_parse_bytes() {
        let size = FileSize::parse("10").unwrap();
        assert_eq!(size.as_bytes(), 10);
    }

    #[test]
    fn test_parse_kb() {
        let size = FileSize::parse("10K").unwrap();
        assert!((size.as_kib() - 10f64).abs() < ε);
    }

    #[test]
    fn test_parse_megabytes() {
        let size = FileSize::parse("10M").unwrap();
        assert!((size.as_mib() - 10f64).abs() < ε);
    }

    #[test]
    fn test_parse_gigabytes() {
        let size = FileSize::parse("10G").unwrap();
        assert!((size.as_gib() - 10f64).abs() < ε);
    }

    #[test]
    fn test_parse_terabytes() {
        let size = FileSize::parse("10T").unwrap();
        assert!((size.as_tib() - 10f64).abs() < ε);
    }

    #[test]
    fn test_parse_petabytes() {
        let size = FileSize::parse("10P").unwrap();
        assert!((size.as_pib() - 10f64).abs() < ε);
    }

    #[test]
    fn test_parse_invalid_suffix() {
        assert!(FileSize::parse("10X").is_err());
    }

    #[test]
    fn test_parse_negative() {
        assert!(FileSize::parse("-321").is_err());
    }

    #[test]
    fn test_parse_float() {
        assert!(FileSize::parse("42.1337").is_err());
    }
}
