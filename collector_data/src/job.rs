use std::process::Command;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Job {
    pub id: String,
    pub processes: Vec<u32>,
    pub name: String,
    pub user: String,
    pub start_time: String,
    pub end_time: String,
}

impl Job {
    /// Get job information via command line tools scontrol and nvidia-smi
    ///
    /// # Steps:
    /// + Get active jobids (squeue)
    /// + Get pids per job (scontrol listpids)
    /// + Get pid info from other tools (nvidia-smi)
    pub fn get_jobs() -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        // Format: JobId, JobName, User, StartTime, EndTime, JobPartition, JobState, Time used by job, Number of Nodes
        // allocated for job, 
        // TODO: Use JobState to only get running jobs
        let output = Command::new("squeue").arg("-o %.18i %.8j %.8u %S %e %.8P %.2t %.10M %.6D").arg("--state=R").output().expect("Failed to execute process");

        let output_str = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = output_str.lines().skip(1).collect();

        // Tuple structur (job_id, name, user, start_time, end_time)
        let job_raws: Vec<(String, String, String, String, String)> = lines.iter().map(|line| {
            let elements: Vec<&str> = line.split_whitespace().collect();
            let start_time = match elements[2] {
                "N/A" => "1970-01-01 00:00:00".to_string(),
                start_time => String::from(start_time).replace("T", " "),
            };
            let end_time = match elements[3] {
                "N/A" => "1970-01-01 00:00:00".to_string(),
                end_time => String::from(end_time).replace("T", " "),
            };
            (
                String::from(elements[0]),
                String::from(elements[1]),
                // slurm time format is "yyyy-mm-ddThh:mm:ss", replace T with whitespace
                start_time,
                end_time,
                String::from(elements[4])
             )
        }).collect();

        // Get pids per job
        let mut jobs: Vec<Job> = Vec::new();

        for job in job_raws {
            let job_id_clone = job.0.clone();
            jobs.push(Job{ 
                id: job_id_clone, 
                processes: Self::get_process_per_job_id(&job.0),
                name: job.1,
                user: job.2,
                start_time: job.3,
                end_time: job.4,
            } );
        }

        Ok(jobs)
    }

    fn get_process_per_job_id(job_id: &str) -> Vec<u32> {
        let output = Command::new("scontrol").arg("listpids").arg(&job_id).output().expect("Failed to execute scontrol");

        let output_str = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = output_str.lines().skip(1).collect();

        let processes: Vec<u32> = lines.iter().map(|line| {
            let elements: Vec<&str> = line.split_whitespace().collect();
            elements[0].parse().unwrap()
        }).collect();

        processes
    }
}
