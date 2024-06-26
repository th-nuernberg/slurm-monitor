use std::process::Command;

use anyhow::{ensure, Result};

// All keys from `s<x> --json`: `sacct -a --json | jq -r 'paths(scalars | true) as $p  | [ ( [ $p[] | tostring ] | join(".") ), ( getpath($p) | tojson )] | join(": ")' | grep -v '\\.[1-9]\d*\\.'`
pub fn collect_sacct_json() -> Result<String> {
    let output = Command::new("sacct").args(["-a", "--json"]).output()?;
    ensure!(output.status.success());

    let result = String::from_utf8(output.stdout)?;
    Ok(result)
}

pub fn sacct_csvlike() -> Result<String> {
    
}