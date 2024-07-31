use anyhow::{Context as _, Result};
use chrono::NaiveDateTime;

pub fn datetime_from_filename(name: &str) -> Result<NaiveDateTime> {
    let format = "%Y_%m_%d__%H_%M_%S";
    //                        yyyy  _  mm   _  dd   __   hh   _  mm   _  ss
    const resulting_format_len: usize = (4 + 1 + 2 + 1 + 2) + 2 + (2 + 1 + 2 + 1 + 2);
    const position: usize = 0; // datetime is at start of filename

    let datetime_part: String = name
        .chars()
        .skip(position)
        .take(resulting_format_len)
        .collect();
    Ok(
        NaiveDateTime::parse_from_str(&datetime_part.as_str(), &format)
            .with_context(|| format!("parsing datetime from "))?,
    )
}
