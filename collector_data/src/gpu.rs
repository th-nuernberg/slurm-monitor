use std::collections::HashMap;

use super::job::Job;
use chrono;
use nvml_wrapper::{
    enums::device::UsedGpuMemory::{Unavailable, Used},
    Device, Nvml,
};
use serde::{Deserialize, Serialize};
use sysinfo::{PidExt, System, SystemExt};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub id: String,
    pub mem_total: u32,
}

impl GpuInfo {
    // TODO: (for usage too) set nvml or device as a parameter instead of creating a intance here
    pub fn get_static_info(nvml: &Nvml) -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        let device_count = nvml.device_count()?;
        let mut devices = Vec::<Device>::new();
        let mut static_info = Vec::<Self>::new();

        for i in 0..device_count - 1 {
            devices.push(nvml.device_by_index(i)?);
        }

        let mut mem_total;
        let mut uuid;
        for device in devices.iter() {
            mem_total = device.memory_info()?.total;
            uuid = device.uuid()?;
            static_info.push(Self {
                id: uuid,
                mem_total: (mem_total / 1000000) as u32,
            });
        }

        Ok(static_info)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuUsage {
    pub timestamp: String,
    pub gpu_id: String,
    pub gpu_mem_alloc: u32,
    pub gpu_usage: f32,
    pub job_id: Option<String>,
}

impl GpuUsage {
    pub fn get_usage_per_job(job: &Job, nvml: &Nvml) -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        let mut gpu_usages = HashMap::<&str, GpuUsage>::new();

        let gpu_usage_per_pid = Self::get_gpu_usage_per_pid(&nvml)?;

        for pid in &job.processes {
            if let Some(gpu_usage) = gpu_usage_per_pid.get(pid) {
                let gpu_id = &gpu_usage.0[..];
                if !gpu_usages.contains_key(&gpu_id) {
                    gpu_usages.insert(
                        &gpu_id,
                        GpuUsage {
                            timestamp: chrono::offset::Local::now().format("%F %T").to_string(),
                            gpu_id: gpu_id.to_string(),
                            gpu_mem_alloc: gpu_usage.1,
                            gpu_usage: gpu_usage.2,
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

    pub fn get_non_job_usage(sys: &System, jobs: &[Job], nvml: &Nvml) -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        let mut job_processes: Vec<u32> = Vec::new();
        jobs.iter()
            .for_each(|job| job.processes.iter().for_each(|process| job_processes.push(*process)));

        let processes_wo_job: Vec<u32> = sys
            .processes()
            .iter()
            .map(|process| process.0.as_u32())
            .filter(|process| !job_processes.contains(&process))
            .collect();

        let gpu_usage_per_pid = Self::get_gpu_usage_per_pid(&nvml)?;

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
                            gpu_mem_alloc: gpu_usage.1,
                            gpu_usage: gpu_usage.2,
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

    fn get_gpu_usage_per_pid(nvml: &Nvml) -> Result<HashMap<u32, (String, u32, f32)>, Box<dyn std::error::Error>> {
        let mut gpu_usage_per_pid: HashMap<u32, (String, u32, f32)> = HashMap::<u32, (String, u32, f32)>::new();

        let device_count = nvml.device_count()?;

        // Vec<(pid, gpu_uuid, used_gpu_mem, utilization in u32 per device)
        let mut process_util_sample: Vec<(u32, String, u32, f32)> = vec![];

        for i in 0..(device_count - 1) {
            let device = nvml.device_by_index(i)?;
            let processes_per_device = device.running_compute_processes()?;
            let mut process_util: Vec<(u32, String, u32, f32)> = processes_per_device
                .iter()
                .map(|process| {
                    let used_gpu_mem = match &process.used_gpu_memory {
                        Used(gpu_mem) => gpu_mem,
                        Unavailable => &0,
                    };
                    let gpu_utilization = device.utilization_rates().unwrap().gpu;
                    (
                        process.pid,
                        device.uuid().unwrap(),
                        (used_gpu_mem / 1000000) as u32,
                        (gpu_utilization as f32) / 100.0,
                    )
                })
                .collect();
            process_util_sample.append(&mut process_util);
        }

        process_util_sample.iter().for_each(|process| {
            gpu_usage_per_pid.insert(process.0, (process.1.clone(), process.2, process.3));
        });

        Ok(gpu_usage_per_pid)
    }
}
