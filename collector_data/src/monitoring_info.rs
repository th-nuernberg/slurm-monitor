use chrono::{DateTime, Utc};
use color_eyre::Result;
use nvml_wrapper::Nvml;
use serde::{Deserialize, Serialize};
use sysinfo::{System, SystemExt};

use crate::cpu::Cpu;

use super::{
    cpu::CpuUsage,
    gpu::{GpuInfo, GpuUsage},
    job::Job,
    node::{NodeInfo, NodeUsage},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum State {
    Initial(StaticInfo),
    Current(MonitorInfo),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Measurement {
    pub time: DateTime<Utc>,
    pub state: State,
}

impl State {
    pub fn get_starting_info(sys: &mut System, nvml: &Nvml) -> Result<String> {
        sys.refresh_all();
        let static_info = StaticInfo::get_static_info(sys, nvml)?;
        Ok(serde_json::to_string(&Measurement {
            time: chrono::offset::Utc::now(),
            state: State::Initial(static_info),
        })?)
    }

    pub fn get_monitoring_data(sys: &mut System, nvml: &Nvml) -> Result<String> {
        sys.refresh_all();
        let monitor_info = MonitorInfo::get_monitoring_info(sys, nvml)?;
        Ok(serde_json::to_string(&Measurement {
            time: chrono::offset::Utc::now(),
            state: State::Current(monitor_info),
        })?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticInfo {
    pub node_info: NodeInfo,
    pub cpu_info: Cpu,
    pub gpu_info: Vec<GpuInfo>,
}

impl StaticInfo {
    fn get_static_info(sys: &System, nvml: &Nvml) -> Result<Self> {
        let node_info = NodeInfo::get_static_info(sys)?;
        let cpu_info = Cpu::get_static_info()?;
        let gpu_info = GpuInfo::get_static_info(nvml)?;

        Ok(StaticInfo {
            node_info,
            cpu_info,
            gpu_info,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorInfo {
    pub jobs: Vec<Job>,
    pub node_usages: Vec<NodeUsage>,
    pub cpu_usages: Vec<CpuUsage>,
    pub gpu_usages: Vec<GpuUsage>, // TODO most important
}

impl MonitorInfo {
    fn get_monitoring_info(sys: &System, nvml: &Nvml) -> Result<Self> {
        let jobs = Job::get_jobs()?;
        let mut node_usages = Vec::<NodeUsage>::new();
        let mut cpu_usages = Vec::<CpuUsage>::new();
        let mut gpu_usages = Vec::<GpuUsage>::new();

        for job in &jobs {
            node_usages.push(NodeUsage::get_usage_per_job(sys, job)?);
            cpu_usages.append(&mut CpuUsage::get_usage_per_job(sys, job)?);
            gpu_usages.append(&mut GpuUsage::get_usage_per_job(job, nvml)?);
        }

        // non job usage
        node_usages.push(NodeUsage::get_non_job_usage(sys, &jobs)?);
        cpu_usages.append(&mut CpuUsage::get_non_job_usage(sys, &jobs)?);
        gpu_usages.append(&mut GpuUsage::get_non_job_usage(sys, &jobs, nvml)?);

        Ok(MonitorInfo {
            jobs,
            node_usages,
            cpu_usages,
            gpu_usages,
        })
    }
}
