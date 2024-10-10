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
pub mod cpu;
pub mod gpu;
pub mod gpu_dep;
pub mod job;
pub mod misc;
pub mod monitoring_info;
pub mod node;

use chrono::Duration;

pub const DEFAULT_INTERVAL: Duration = Duration::seconds(30);
pub const DEFAULT_TIMEOUT: Duration = Duration::seconds(30);
