#![deny(warnings)]
use serde::{Deserialize, Serialize};
use stat_common::server_status::{IpInfo, SysInfo};
use std::time::{SystemTime, UNIX_EPOCH};

fn default_as_true() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HostStat {
    pub name: String,
    #[serde(default = "Default::default")]
    pub alias: String,
    #[serde(rename = "type", default = "Default::default")]
    pub host_type: String,
    #[serde(default = "Default::default")]
    pub location: String,
    #[serde(default = "default_as_true")]
    pub notify: bool,
    #[serde(default = "bool::default")]
    pub vnstat: bool,

    #[serde(default = "default_as_true")]
    pub online4: bool,
    #[serde(default = "default_as_true")]
    pub online6: bool,

    #[serde(rename(deserialize = "uptime"), skip_serializing)]
    pub uptime: u64,
    #[serde(rename(serialize = "uptime"), skip_deserializing)]
    pub uptime_str: String,

    pub load_1: f64,
    pub load_5: f64,
    pub load_15: f64,

    pub ping_10010: f64,
    pub ping_189: f64,
    pub ping_10086: f64,
    pub time_10010: f64,
    pub time_189: f64,
    pub time_10086: f64,

    #[serde(rename(deserialize = "tcp"))]
    pub tcp_count: u32,
    #[serde(rename(deserialize = "udp"))]
    pub udp_count: u32,
    #[serde(rename(deserialize = "process"))]
    pub process_count: u32,
    #[serde(rename(deserialize = "thread"))]
    pub thread_count: u32,

    pub network_rx: u64,
    pub network_tx: u64,
    pub network_in: u64,
    pub network_out: u64,

    #[serde(default)]
    pub last_network_in: u64,
    #[serde(default)]
    pub last_network_out: u64,

    pub cpu: f32,
    pub memory_total: u64,
    pub memory_used: u64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub hdd_total: u64,
    pub hdd_used: u64,

    #[serde(skip_deserializing)]
    pub labels: String,
    #[serde(skip_deserializing)]
    pub custom: String,

    #[serde(skip_serializing)]
    pub ip_info: Option<IpInfo>,
    #[serde(skip_serializing)]
    pub sys_info: Option<SysInfo>,

    // group
    #[serde(default = "Default::default")]
    pub gid: String,
    #[serde(default = "Default::default")]
    pub weight: u64,

    // user data
    #[serde(skip_deserializing)]
    pub latest_ts: u64,

    #[serde(skip_serializing, skip_deserializing)]
    pub pos: usize,
    #[serde(skip_serializing, skip_deserializing)]
    pub disabled: bool,

    // false: KiB (1024), true: KB (1000)
    #[serde(default = "Default::default")]
    pub si: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatsResp {
    pub updated: u64,
    pub servers: Vec<HostStat>,
}
impl StatsResp {
    pub fn new() -> Self {
        Self {
            updated: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            servers: Vec::new(),
        }
    }
}
