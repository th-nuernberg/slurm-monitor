use std::process::Command;

use chrono::{Duration, Utc};
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
    let gpu_time_table = collector_data::gpu_dep::AllGpuTimesReportedBySlurm::query(Utc::now() - Duration::days(365)..Utc::now())?;
    println!("{gpu_time_table:?}");
    Ok(())
}
