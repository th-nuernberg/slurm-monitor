use std::process::Command;

use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, Utc};
use color_eyre::Result;

fn are_we_on_slurm_machine() -> bool {
    let success = Command::new("which")
        .arg("sacct")
        .output()
        .expect("error while executing `which`")
        .status
        .success();
    if !success {
        eprintln!("No slurm found, SKIPPING");
    }
    success
}

#[test]
fn sreport_gpu_time_per_user() -> Result<()> {
    if !are_we_on_slurm_machine() {
        return Ok(());
    }
    // look if querying and parsing can happen without an error
    // if so, just print out the result, since we have no way to actually validate the data
    // TODO maybe limit time span to time with known data, and actually test the results
    // limit to one week, fetching _everything_ takes about 9secs in the unit test
    let start = NaiveDate::from_ymd_opt(2024, 10, 3).unwrap().and_hms_opt(0, 0, 0).unwrap().and_utc();
    let end = NaiveDate::from_ymd_opt(2024, 10, 3).unwrap().and_hms_opt(0, 0, 0).unwrap().and_utc();
    let gpu_time_table = collector_data::gpu_dep::AllGpuTimesReportedBySlurm::query(start..end)?;
    println!("{gpu_time_table:?}");
    Ok(())
}
