use std::collections::HashMap;
use std::process::Command;

use chrono;
use serde::{Deserialize, Serialize};
use sysinfo::{PidExt, System, SystemExt};

use super::job::Job;

#[derive(Debug, Serialize, Deserialize)]
pub struct GpuInfo {
    pub id: String,
    pub mem_total: u32,
}

impl GpuInfo {
    pub fn get_static_info() -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        let output = Command::new("nvidia-smi")
            .arg("--query-gpu=gpu_uuid,memory.total")
            .arg("--format=csv,noheader")
            .output()?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = output_str.lines().collect();

        // Tuple structur (gpu_uuid, total_mem)
        let gpus: Vec<(String, u32)> = lines
            .iter()
            .map(|line| {
                let elements: Vec<&str> = line.split_whitespace().collect();
                (
                    String::from(elements[0]).replace(",", ""),
                    elements[1].parse::<u32>().unwrap(),
                )
            })
            .collect();

        let gpu_info = gpus
            .iter()
            .map(|gpu| GpuInfo {
                id: gpu.0.clone(),
                mem_total: gpu.1,
            })
            .collect();

        Ok(gpu_info)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct GpuUsage {
    pub timestamp: String,
    pub gpu_id: String,
    pub gpu_mem_alloc: u32,
    pub gpu_usage: f32,
    pub job_id: Option<String>,
}

impl GpuUsage {
    pub fn get_usage_per_job(job: &Job) -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        let mut gpu_usages = HashMap::<&str, GpuUsage>::new();

        let gpu_usage_per_pid = Self::get_gpu_usage_per_pid()?;

        for pid in &job.processes {
            if let Some(gpu_usage) = gpu_usage_per_pid.get(pid) {
                let gpu_id = &gpu_usage.0[..];
                if !gpu_usages.contains_key(&gpu_id) {
                    gpu_usages.insert(
                        &gpu_id,
                        GpuUsage {
                            timestamp: chrono::offset::Local::now().format("%F %T").to_string(),
                            gpu_id: gpu_id.to_string(),
                            gpu_mem_alloc: 0,
                            gpu_usage: 0.0,
                            job_id: Some(job.id.clone()),
                        },
                    );
                }

                match gpu_usages.get_mut(&gpu_id) {
                    Some(gpu) => {
                        gpu.gpu_mem_alloc += gpu_usage.1;
                        gpu.gpu_usage += gpu_usage.2;
                    }
                    // TODO: Error handle that one here better
                    None => {}
                }
            }
        }

        Ok(gpu_usages.values().cloned().collect())
    }

    pub fn get_non_job_usage(
        sys: &System,
        jobs: &[Job],
    ) -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        let mut job_processes: Vec<u32> = Vec::new();
        jobs.iter().for_each(|job| {
            job.processes
                .iter()
                .for_each(|process| job_processes.push(*process))
        });

        let processes_wo_job: Vec<u32> = sys
            .processes()
            .iter()
            .map(|process| process.0.as_u32())
            .filter(|process| !job_processes.contains(&process))
            .collect();

        let gpu_usage_per_pid = Self::get_gpu_usage_per_pid()?;

        let mut gpu_usages = HashMap::<&str, GpuUsage>::new();

        for pid in processes_wo_job {
            if let Some(gpu_usage) = gpu_usage_per_pid.get(&pid) {
                let gpu_id = &gpu_usage.0[..];
                if !gpu_usages.contains_key(&gpu_id) {
                    gpu_usages.insert(
                        &gpu_id,
                        GpuUsage {
                            timestamp: chrono::offset::Local::now().format("%F %T").to_string(),
                            gpu_id: gpu_id.to_string(),
                            gpu_mem_alloc: 0,
                            gpu_usage: 0.0,
                            job_id: None,
                        },
                    );
                }

                match gpu_usages.get_mut(&gpu_id) {
                    Some(gpu) => {
                        gpu.gpu_mem_alloc += gpu_usage.1;
                        gpu.gpu_usage += gpu_usage.2;
                    }
                    // TODO: Error handle that one here better
                    None => {}
                }
            }
        }

        Ok(gpu_usages.values().cloned().collect())
    }

    fn get_gpu_usage_per_pid(
    ) -> Result<HashMap<u32, (String, u32, f32)>, Box<dyn std::error::Error>> {
        let output_per_pid = Command::new("nvidia-smi")
            .arg("--query-compute-apps=pid,gpu_uuid,used_gpu_memory") // for computing
            //process
            //            .arg("--query-accounted-apps=pid,gpu_uuid,mem_util,gpu_util")
            .arg("--format=csv,noheader")
            .output()?;
        //let output_whole = Command::new("nvidia-smi").arg("--query-gpu=gpu_uuid,utilization.gpu,memory.used").arg("--format=csv,noheader").output().expect("nvidia-smi cannot execute");

        // Get GPU Usage per job
        let output_per_pid_str = String::from_utf8_lossy(&output_per_pid.stdout);
        //let lines_per_pid: Vec<&str> = output_per_pid_str.lines().collect();

        // Tuple structur (pid, gpu_id, mem_util, gpu_util)
        let gpu_usage_per_pid: HashMap<u32, (String, u32, f32)> = output_per_pid_str
            .lines()
            .map(|line| {
                let elements: Vec<&str> = line.split_whitespace().collect();
                (
                    elements[0].replace(",", "").parse::<u32>().unwrap(),
                    (
                        elements[1].replace(",", "").to_string(),
                        elements[2].replace(",", "").parse::<u32>().unwrap(), //elements[2].parse::<u32>().unwrap(),
                        0.0, //elements[2].parse::<f32>().unwrap()
                    ),
                )
            })
            .collect();

        // TODO: ooff
        Ok(gpu_usage_per_pid)
    }
}
