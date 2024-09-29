// could look like this:
//
// struct SAcct {
//     jobid: String,
//     jobidraw: String,
//     jobname: String,
//     user: String,

//     elapsed: String, // TODO parse time
//     state: String,   // TODO maybe parse enum?

//     partition: String,
//     ntasks: u32,
//     alloccpus: u32,

//     maxrss: String, // TODO parse units
//     averss: String, // TODO parse units
//     avecpu: String, // TODO parse time

//     consumedenergy: f64,
// }
pub mod gpu_dep;
pub mod job;
pub mod cpu;
pub mod node;
pub mod gpu;
pub mod monitoring_info;

use std::time::Duration;

pub const DEFAULT_INTERVAL: Duration = Duration::from_secs(30);
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);