use serde::{Serialize, Deserialize};
use sysinfo::{System, SystemExt, ProcessExt, Pid, PidExt};

use super::job::Job;

/// Contains static information on node
/// The id gets changed by the server to match its configuration for this client
#[derive(Debug, Serialize, Deserialize)]
pub struct NodeInfo {
    pub id: u32,
    pub mem_total: u64,
}

impl NodeInfo {
   pub fn get_static_info(sys: &System) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(NodeInfo{
            id: 1,
            // sys.total_memory return size in byte, need to divide it
            // TODO: currently in MB due to size restriction
            mem_total: sys.total_memory() / 1000000,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct NodeUsage {
    pub timestamp: String,
    // Node Id gets changed at the server according to configs
    pub node_id: u32,
    pub mem_alloc: u64,
    pub job_id: Option<String>,
}

impl NodeUsage {
    pub fn get_usage_per_job(sys: &System, job: &Job) -> Result<Self, Box<dyn std::error::Error>> {
        let mut mem_alloc_per_job = 0;

        for pid in &job.processes {
            if let Some(process) = sys.process(Pid::from_u32(*pid)) {
                mem_alloc_per_job += process.memory();
            }
        }

        Ok(NodeUsage {
            timestamp: chrono::offset::Local::now().format("%F %T").to_string(),
            node_id: 1,
            mem_alloc: mem_alloc_per_job/1000000,
            job_id: Some(job.id.clone()),
        })
    }

    pub fn get_non_job_usage(sys: &System, jobs: &[Job]) -> Result<Self, Box<dyn std::error::Error>> {
        let mut job_processes: Vec<u32> = Vec::new();
        jobs.iter().for_each(|job| job.processes.iter().for_each(|process| job_processes.push(*process)));
        
        let processes_wo_job: Vec<u32> = sys.processes().iter().filter(|process| !job_processes.contains(&process.0.as_u32())).map(|process| process.0.as_u32()).collect();
        let mut mem_alloc = 0;

        for pid in processes_wo_job {
            if let Some(process) = sys.process(Pid::from_u32(pid)) {
                mem_alloc += process.memory();
            }
        }

        Ok(NodeUsage {
            timestamp: chrono::offset::Local::now().format("%F %T").to_string(),
            node_id: 1,
            // process.memory() returns size in byte, need to divide it first to get kB
            // TODO: currently in MB, due to size restriction in mysql
            mem_alloc: mem_alloc/1000000,
            job_id: None,
        })
    }
}
