use std::process::Command;
use std::io::{BufReader, BufRead};
use std::fs::File;
use std::collections::HashMap;

use color_eyre::Result;
use serde::{Serialize, Deserialize};
use sysinfo::{System, SystemExt,ProcessExt, Pid, PidExt};
use derive_builder::Builder;

use super::job::Job;


///
/// Static information on CPU
///
#[derive(Default, Builder, Clone, Debug, Serialize, Deserialize)]
#[builder(setter(into))]
pub struct CpuNode {
    pub id: String,
    pub core_count: u32,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Cpu {
    nodes: Vec<CpuNode>
}

impl Cpu {

    fn fetch_from_system() -> Result<Self> {
        let file = File::open("/proc/cpuinfo")?;
        let buf_reader = BufReader::new(file);
        let mut builder = &mut CpuNodeBuilder::default();
        let mut cpu_infos = Vec::<CpuNode>::new();

        for line in buf_reader.lines() {
            let line = line.unwrap();
            let kv: Vec<_> = line.splitn(2, ':').map(|s| s.trim()).collect();
            builder = match kv.as_slice() {
                ["processor", v] => builder.id(v.to_string()),
                ["cpu cores", v] => builder.core_count(v.parse::<u32>().unwrap()),
                // if line is empty a new processor follows
                [""] => {
                    let result = builder.build().expect("Ain't working");
                    cpu_infos.push(result);
                    continue;
                },
                [_, _] => builder,
                [_] => builder,
                _ => unreachable!(),
            };
       }

       Ok(Cpu{nodes: cpu_infos })
   }
}

/// 
/// Information on current CPU usage
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuUsage {
    pub timestamp: DateTime,
    pub node_id: String,
    pub usage: f32,
    pub job_id: Option<String>,
}

impl CpuUsage {
   pub fn get_usage_per_job(sys: &System, job: &Job) -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        // get cpu id
        // for each process the cpu id (psr)
        let cpu_id_per_pid: HashMap<u32, u32> = Self::get_pid_psr()?;
       
        // HashMap with CpuID as Key
        let mut cpu_usages = HashMap::<u32, CpuUsage>::new();

        for pid in &job.processes {
            if let Some(process) = sys.process(Pid::from(*pid as usize).into()) {
                let cpu_id = match cpu_id_per_pid.get(&pid) {
                    Some(cpu_id) => cpu_id,
                    None => continue,
                };

                if !cpu_usages.contains_key(&cpu_id) {
                    cpu_usages.insert(*cpu_id, CpuUsage{
                        timestamp: chrono::offset::Utc::now().into(),
                        id: cpu_id.to_string(),
                        usage: 0.0,
                        job_id: Some(job.id.clone())
                    });
                }

                cpu_usages.get_mut(&cpu_id).unwrap().usage += process.cpu_usage();
            } 
        }

        Ok(cpu_usages.values().cloned().collect())
   }

    pub fn get_non_job_usage(sys: &System, jobs: &[Job]) -> Result<Vec<Self>, Box <dyn std::error::Error>> {
        let mut job_processes: Vec<u32> = Vec::new();
        jobs.iter().for_each(|job| job.processes.iter().for_each(|process| job_processes.push(*process)));
        
        let processes_wo_job: Vec<u32> = sys.processes()
            .iter()
            .map(|process| process.0.as_u32())
            .filter(|process| !job_processes.contains(&process))
            .collect();

        // get cpu id
        // for each process the cpu id (psr)
        let cpu_id_per_pid: HashMap<u32, u32> = Self::get_pid_psr()?;
       
        let mut cpu_usages = HashMap::<u32, CpuUsage>::new();

        for pid in processes_wo_job {
            if let Some(process) = sys.process(Pid::from_u32(pid)) {
                let cpu_id = match cpu_id_per_pid.get(&pid) {
                    Some(cpu_id) => cpu_id,
                    None => continue,
                };

                if !cpu_usages.contains_key(&cpu_id) {
                    cpu_usages.insert(*cpu_id, CpuUsage{
                        timestamp: chrono::offset::Utc::now().into(),
                        id: cpu_id.to_string(),
                        usage: 0.0,
                        job_id: None,
                    });
                }

                cpu_usages.get_mut(&cpu_id).unwrap().usage += process.cpu_usage();
            }
        }

        Ok(cpu_usages.values().cloned().collect())
    }

    /// Help function to get information on pid and psr
    fn get_pid_psr() -> Result<HashMap<u32, u32>, Box<dyn std::error::Error>> {
        let output_ps = Command::new("ps").arg("-eo").arg("pid,psr").output()?;
        let output_ps_str = String::from_utf8_lossy(&output_ps.stdout);
        let cpu_id_per_pid: HashMap<u32, u32> = output_ps_str
            .lines()
            .skip(1)
            .map(|line| 
                 {
                     let elements: Vec<&str> = line.split_whitespace().collect();
                     (
                         elements[0].parse::<u32>().unwrap(),
                         elements[1].parse::<u32>().unwrap()
                         )
                 }).collect();

        Ok(cpu_id_per_pid)
    }
}
