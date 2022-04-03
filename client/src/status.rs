// #![allow(unused)]
use chrono::{Datelike, Local};
use lazy_static::lazy_static;
use once_cell::sync::OnceCell;
use regex::Regex;
use std::collections::HashMap;
use std::collections::LinkedList;
use std::fs;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::ErrorKind::ConnectionRefused;
use std::net::TcpStream;
use std::net::{Shutdown, ToSocketAddrs};
use std::process::Command;
use std::str;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::Args;

const SAMPLE_PERIOD: u64 = 1000; //ms
const TIMEOUT_MS: u64 = 1000;
static IPV4_ADDR: &str = "ipv4.google.com:80";
static IPV6_ADDR: &str = "ipv6.google.com:80";

pub fn get_uptime() -> u64 {
    fs::read_to_string("/proc/uptime")
        .map(|contents| {
            if let Some(s) = contents.split('.').next() {
                return s.parse::<u64>().unwrap_or(0);
            }
            0
        })
        .unwrap()
}

pub fn get_loadavg() -> (f64, f64, f64) {
    fs::read_to_string("/proc/loadavg")
        .map(|contents| {
            let vec = contents.split_whitespace().collect::<Vec<_>>();
            // dbg!(&vec);
            if vec.len() >= 3 {
                let a = vec[0..3]
                    .iter()
                    .map(|v| v.parse::<f64>().unwrap())
                    .collect::<Vec<f64>>();

                return (a[0], a[1], a[2]);
            }
            (0.0, 0.0, 0.0)
        })
        .unwrap()
}

static MEMORY_REGEX: &str = r#"^(?P<key>\S*):\s*(?P<value>\d*)\s*kB"#;
lazy_static! {
    static ref MEMORY_REGEX_RE: Regex = Regex::new(MEMORY_REGEX).unwrap();
}
pub fn get_memory() -> (u64, u64, u64, u64) {
    let file = File::open("/proc/meminfo").unwrap();
    let buf_reader = BufReader::new(file);
    let mut res_dict = HashMap::new();
    for line in buf_reader.lines() {
        let l = line.unwrap();
        if let Some(caps) = MEMORY_REGEX_RE.captures(&l) {
            res_dict.insert(
                caps["key"].to_string(),
                caps["value"].parse::<u64>().unwrap(),
            );
        };
    }

    let mem_total = res_dict["MemTotal"];
    let swap_total = res_dict["SwapTotal"];
    let swap_free = res_dict["SwapFree"];

    let mem_used = mem_total
        - res_dict["MemFree"]
        - res_dict["Buffers"]
        - res_dict["Cached"]
        - res_dict["SReclaimable"];

    (mem_total, mem_used, swap_total, swap_free)
}

macro_rules! exec_shell_cmd_fetch_u32 {
    ($shell_cmd:expr) => {{
        let a = &Command::new("/bin/sh")
            .args(&["-c", $shell_cmd])
            .output()
            .expect("failed to execute process")
            .stdout;
        str::from_utf8(a).unwrap().trim().parse::<u32>().unwrap()
    }};
}

pub fn tupd() -> (u32, u32, u32, u32) {
    let t = exec_shell_cmd_fetch_u32!("ss -t | wc -l") - 1;
    let u = exec_shell_cmd_fetch_u32!("ss -u | wc -l") - 1;
    let p = exec_shell_cmd_fetch_u32!("ps -ef | wc -l") - 2;
    let d = exec_shell_cmd_fetch_u32!("ps -eLf | wc -l") - 2;

    (t, u, p, d)
}

static IFACE_IGNORE_VEC: &[&str] = &["lo", "docker", "vnet", "veth", "vmbr", "kube", "br-"];
pub fn get_vnstat_traffic() -> (u64, u64, u64, u64) {
    let local_now = Local::now();
    let (mut network_in, mut network_out, mut m_network_in, mut m_network_out) = (0, 0, 0, 0);
    let a = Command::new("/usr/bin/vnstat")
        .args(&["--json", "m"])
        .output()
        .expect("failed to execute vnstat")
        .stdout;
    let b = str::from_utf8(&a).unwrap();
    let j: HashMap<&str, serde_json::Value> = serde_json::from_str(b).unwrap();
    for iface in j["interfaces"].as_array().unwrap() {
        let name = iface["name"].as_str().unwrap();
        if IFACE_IGNORE_VEC.iter().any(|sk| name.contains(*sk)) {
            continue;
        }
        let total_o = iface["traffic"]["total"].as_object().unwrap();
        let month_v = iface["traffic"]["month"].as_array().unwrap();
        network_in += total_o["rx"].as_u64().unwrap();
        network_out += total_o["tx"].as_u64().unwrap();

        for data in month_v {
            let year = data["date"]["year"].as_i64().unwrap() as i32;
            let month = data["date"]["month"].as_i64().unwrap() as u32;
            if local_now.year() != year || local_now.month() != month {
                continue;
            }

            m_network_in += data["rx"].as_u64().unwrap();
            m_network_out += data["tx"].as_u64().unwrap();
        }
    }

    (network_in, network_out, m_network_in, m_network_out)
}

static TRAFFIC_REGEX: &str = r#"([^\s]+):[\s]{0,}(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)"#;
lazy_static! {
    static ref TRAFFIC_REGEX_RE: Regex = Regex::new(TRAFFIC_REGEX).unwrap();
}
pub fn get_sys_traffic() -> (u64, u64) {
    let (mut network_in, mut network_out) = (0, 0);
    let file = File::open("/proc/net/dev").unwrap();
    let buf_reader = BufReader::new(file);
    for line in buf_reader.lines() {
        let l = line.unwrap();

        TRAFFIC_REGEX_RE.captures(&l).and_then(|caps| {
            // println!("caps[0]=>{:?}", caps.get(0).unwrap().as_str());
            let name = caps.get(1).unwrap().as_str();
            if IFACE_IGNORE_VEC.iter().any(|sk| name.contains(*sk)) {
                return None;
            }
            let net_in = caps.get(2).unwrap().as_str().parse::<u64>().unwrap();
            let net_out = caps.get(10).unwrap().as_str().parse::<u64>().unwrap();

            network_in += net_in;
            network_out += net_out;
            Some(())
        });
    }

    (network_in, network_out)
}

static DF_CMD:&str = "df -Tlm --total -t ext4 -t ext3 -t ext2 -t reiserfs -t jfs -t ntfs -t fat32 -t btrfs -t fuseblk -t zfs -t simfs -t xfs";
pub fn get_hdd() -> (u64, u64) {
    let (mut hdd_total, mut hdd_used) = (0, 0);
    let a = &Command::new("/bin/sh")
        .args(&["-c", DF_CMD])
        .output()
        .expect("failed to execute df")
        .stdout;
    let _ = str::from_utf8(a).map(|s| {
        s.trim().split('\n').last().map(|s| {
            let vec: Vec<&str> = s.split_whitespace().collect();
            // dbg!(&vec);
            hdd_total = vec[2].parse::<u64>().unwrap();
            hdd_used = vec[3].parse::<u64>().unwrap();
            Some(())
        });
    });

    (hdd_total, hdd_used)
}

#[derive(Debug, Default)]
pub struct NetSpeed {
    pub diff: f64,
    pub clock: f64,
    pub netrx: u64,
    pub nettx: u64,
    pub avgrx: u64,
    pub avgtx: u64,
}

lazy_static! {
    pub static ref G_NET_SPEED: Arc<Mutex<NetSpeed>> = Arc::new(Default::default());
}

pub fn start_net_speed_collect_t() {
    thread::spawn(|| loop {
        let _ = File::open("/proc/net/dev").map(|file| {
            let buf_reader = BufReader::new(file);
            let (mut avgrx, mut avgtx) = (0, 0);
            for line in buf_reader.lines() {
                let l = line.unwrap();
                let v: Vec<&str> = l.split(':').collect();
                if v.len() < 2 {
                    continue;
                }

                if IFACE_IGNORE_VEC.iter().any(|sk| v[0].contains(*sk)) {
                    continue;
                }
                let v1: Vec<&str> = v[1].split_whitespace().collect();
                avgrx += v1[0].parse::<u64>().unwrap();
                avgtx += v1[8].parse::<u64>().unwrap();
            }

            {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as f64;
                let mut t = G_NET_SPEED.lock().unwrap();
                t.diff = now - t.clock;
                t.clock = now;
                t.netrx = ((avgrx - t.avgrx) as f64 / t.diff) as u64;
                t.nettx = ((avgtx - t.avgtx) as f64 / t.diff) as u64;
                t.avgrx = avgrx;
                t.avgtx = avgtx;

                // dbg!(&t);
            }
        });
        thread::sleep(Duration::from_millis(SAMPLE_PERIOD));
    });
}

lazy_static! {
    pub static ref G_CPU_PERCENT: Arc<Mutex<f64>> = Arc::new(Default::default());
}
pub fn start_cpu_percent_collect_t() {
    let mut pre_cpu: Vec<u64> = vec![0, 0, 0, 0];
    thread::spawn(move || loop {
        let _ = File::open("/proc/stat").map(|file| {
            let mut buf_reader = BufReader::new(file);
            let mut buf = String::new();
            let _ = buf_reader.read_line(&mut buf).map(|_| {
                let cur_cpu = buf
                    .split_whitespace()
                    .enumerate()
                    .filter(|&(idx, _)| idx > 0 && idx < 5)
                    .map(|(_, e)| e.parse::<u64>().unwrap())
                    .collect::<Vec<_>>();

                let pre: u64 = pre_cpu.iter().sum();
                let cur: u64 = cur_cpu.iter().sum();
                let mut st = cur - pre;
                if st == 0 {
                    st = 1;
                }

                let res = 100.0 - (100.0 * (cur_cpu[3] - pre_cpu[3]) as f64 / st as f64);

                // dbg!(&pre_cpu);
                // dbg!(&cur_cpu);

                pre_cpu = cur_cpu;

                let cpu_percent = &mut *G_CPU_PERCENT.lock().unwrap();
                *cpu_percent = res.round();
                // dbg!(cpu_percent);
            });
        });

        thread::sleep(Duration::from_millis(SAMPLE_PERIOD));
    });
}

pub fn get_network() -> (bool, bool) {
    let mut network: [bool; 2] = [false, false];
    let addrs = vec![IPV4_ADDR, IPV6_ADDR];
    for (idx, probe_addr) in addrs.into_iter().enumerate() {
        let _ = probe_addr.to_socket_addrs().map(|mut iter| {
            if let Some(addr) = iter.next() {
                info!("{} => {}", probe_addr, addr);

                let r =
                    TcpStream::connect_timeout(&addr, Duration::from_millis(TIMEOUT_MS)).map(|s| {
                        network[idx] = true;
                        s.shutdown(Shutdown::Both)
                    });

                info!("{:?}", r);
            };
        });
    }

    (network[0], network[1])
}

#[derive(Debug, Default)]
pub struct PingData {
    pub probe_uri: String,
    pub lost_rate: u32,
    pub ping_time: u32,
}

fn start_ping_collect_t(data: &Arc<Mutex<PingData>>) {
    let mut package_list: LinkedList<i32> = LinkedList::new();
    let mut package_lost: u32 = 0;
    let pt = &*data.lock().unwrap();
    let addr = pt
        .probe_uri
        .to_socket_addrs()
        .unwrap()
        .next()
        .expect("can't get addr info");
    info!("{} => {:?}", pt.probe_uri, addr);

    let ping_data = data.clone();
    thread::spawn(move || loop {
        if package_list.len() > 100 && package_list.pop_front().unwrap() == 0 {
            package_lost -= 1;
        }

        let st = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        match TcpStream::connect_timeout(&addr, Duration::from_millis(TIMEOUT_MS)) {
            Ok(s) => {
                let _ = s.shutdown(Shutdown::Both);
                package_list.push_back(1);
            }
            Err(e) => {
                // error!("{:?}", e);
                if e.kind() == ConnectionRefused {
                    package_list.push_back(1);
                } else {
                    package_lost += 1;
                    package_list.push_back(0);
                }
            }
        }
        let et = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let time_cost_ms = et - st;

        {
            let mut o = &mut *ping_data.lock().unwrap();
            o.ping_time = time_cost_ms as u32;
            if package_list.len() > 30 {
                o.lost_rate = package_lost * 100 / package_list.len() as u32;
            }
        }
        thread::sleep(Duration::from_millis(SAMPLE_PERIOD));
    });
}

pub static G_PING_10010: OnceCell<Arc<Mutex<PingData>>> = OnceCell::new();
pub static G_PING_189: OnceCell<Arc<Mutex<PingData>>> = OnceCell::new();
pub static G_PING_10086: OnceCell<Arc<Mutex<PingData>>> = OnceCell::new();

pub fn start_all_ping_collect_t(args: &Args) {
    G_PING_10010
        .set(Arc::new(Mutex::new(PingData {
            probe_uri: args.cu_addr.to_owned(),
            lost_rate: 0,
            ping_time: 0,
        })))
        .unwrap();
    G_PING_189
        .set(Arc::new(Mutex::new(PingData {
            probe_uri: args.ct_addr.to_owned(),
            lost_rate: 0,
            ping_time: 0,
        })))
        .unwrap();
    G_PING_10086
        .set(Arc::new(Mutex::new(PingData {
            probe_uri: args.cm_addr.to_owned(),
            lost_rate: 0,
            ping_time: 0,
        })))
        .unwrap();

    if !args.disable_ping {
        start_ping_collect_t(G_PING_10010.get().unwrap());
        start_ping_collect_t(G_PING_189.get().unwrap());
        start_ping_collect_t(G_PING_10086.get().unwrap());
    }
}
