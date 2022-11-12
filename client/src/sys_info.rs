#![deny(warnings)]
#![allow(unused)]
use lazy_static::lazy_static;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use sysinfo::{CpuExt, DiskExt, NetworkExt, RefreshKind, System, SystemExt};

use crate::status;
use crate::vnstat;
use crate::Args;
use stat_common::server_status::{StatRequest, SysInfo};

const SAMPLE_PERIOD: u64 = 1000; //ms

lazy_static! {
    pub static ref G_EXPECT_FS: Vec<&'static str> = [
        "apfs",
        "ext4",
        "ext3",
        "ext2",
        "f2fs",
        "reiserfs",
        "jfs",
        "btrfs",
        "fuseblk",
        "zfs",
        "simfs",
        "ntfs",
        "fat32",
        "exfat",
        "xfs",
        "fuse.rclone",
    ]
    .to_vec();
    pub static ref G_CPU_PERCENT: Arc<Mutex<f64>> = Arc::new(Default::default());
}
pub fn start_cpu_percent_collect_t() {
    let mut sys = System::new_all();
    sys.refresh_cpu();
    thread::spawn(move || loop {
        let global_cpu = sys.global_cpu_info();
        if let Ok(mut cpu_percent) = G_CPU_PERCENT.lock() {
            *cpu_percent = global_cpu.cpu_usage().round() as f64;
        }

        sys.refresh_cpu();
        thread::sleep(Duration::from_millis(SAMPLE_PERIOD));
    });
}

#[derive(Debug, Default)]
pub struct NetSpeed {
    pub net_rx: u64,
    pub net_tx: u64,
}

lazy_static! {
    pub static ref G_NET_SPEED: Arc<Mutex<NetSpeed>> = Arc::new(Default::default());
}

pub fn start_net_speed_collect_t(args: &Args) {
    let mut sys = System::new_all();
    sys.refresh_all();
    let args_1 = args.clone();
    thread::spawn(move || loop {
        let (mut net_rx, mut net_tx) = (0_u64, 0_u64);
        for (name, data) in sys.networks() {
            // spec iface
            if args_1.skip_iface(name) {
                continue;
            }
            net_rx += data.received();
            net_tx += data.transmitted();
        }
        if let Ok(mut t) = G_NET_SPEED.lock() {
            t.net_rx = net_rx;
            t.net_tx = net_tx;
        }

        sys.refresh_networks();
        thread::sleep(Duration::from_millis(SAMPLE_PERIOD));
    });
}

// TODO
pub fn sample(args: &Args, stat: &mut StatRequest) {
    stat.version = env!("CARGO_PKG_VERSION").to_string();
    stat.vnstat = args.vnstat;

    // 注意：sysinfo 统一使用 KB, 非KiB，需要转换一下
    let mut sys = System::new_with_specifics(RefreshKind::new().with_disks_list().with_memory());

    sys.refresh_system();
    // sys.refresh_processes();
    // sys.refresh_memory();
    // sys.refresh_disks();
    sys.refresh_disks_list();

    // uptime
    stat.uptime = sys.uptime();
    // load average
    let load_avg = sys.load_average();
    stat.load_1 = load_avg.one;
    stat.load_5 = load_avg.five;
    stat.load_15 = load_avg.fifteen;

    // mem KB -> KiB
    let (mem_total, mem_used, swap_total, swap_free) = (
        sys.total_memory() * 1000 / 1024,
        sys.used_memory() * 1000 / 1024,
        sys.total_swap() * 1000 / 1024,
        sys.free_swap() * 1000 / 1024,
    );
    stat.memory_total = mem_total;
    stat.memory_used = mem_used;
    stat.swap_total = swap_total;
    stat.swap_used = swap_total - swap_free;

    // hdd  KB -> KiB
    let (mut hdd_total, mut hdd_avail) = (0_u64, 0_u64);
    for disk in sys.disks() {
        let fs = String::from_utf8_lossy(disk.file_system()).to_lowercase();
        if G_EXPECT_FS.iter().any(|&k| fs.contains(k)) {
            hdd_total += disk.total_space();
            hdd_avail += disk.available_space();
        }
    }
    stat.hdd_total = hdd_total / 1024 / 1024;
    stat.hdd_used = (hdd_total - hdd_avail) / 1024 / 1024;

    // t/u/p/d TODO
    let (t, u, p, d) = if args.disable_tupd {
        (0, 0, 0, 0)
    } else if "linux".eq(std::env::consts::OS) {
        status::tupd()
    } else {
        (0, 0, 0, 0)
    };
    stat.tcp = t;
    stat.udp = u;
    stat.process = p;
    stat.thread = d;

    // traffic
    if args.vnstat {
        let (network_in, network_out, m_network_in, m_network_out) = vnstat::get_traffic(args).unwrap();
        stat.network_in = network_in;
        stat.network_out = network_out;
        stat.last_network_in = network_in - m_network_in;
        stat.last_network_out = network_out - m_network_out;
    } else {
        sys.refresh_networks();
        let (mut network_in, mut network_out) = (0_u64, 0_u64);
        for (name, data) in sys.networks() {
            // spec iface
            if args.skip_iface(name) {
                continue;
            }
            network_in += data.total_received();
            network_out += data.total_transmitted();
        }
        stat.network_in = network_in;
        stat.network_out = network_out;
    }

    if let Ok(o) = G_CPU_PERCENT.lock() {
        stat.cpu = *o;
    }
    if let Ok(o) = G_NET_SPEED.lock() {
        stat.network_rx = o.net_rx;
        stat.network_tx = o.net_tx;
    }
    {
        let o = &*status::G_PING_10010.get().unwrap().lock().unwrap();
        stat.ping_10010 = o.lost_rate.into();
        stat.time_10010 = o.ping_time.into();
    }
    {
        let o = &*status::G_PING_189.get().unwrap().lock().unwrap();
        stat.ping_189 = o.lost_rate.into();
        stat.time_189 = o.ping_time.into();
    }
    {
        let o = &*status::G_PING_10086.get().unwrap().lock().unwrap();
        stat.ping_10086 = o.lost_rate.into();
        stat.time_10086 = o.ping_time.into();
    }
}

pub fn collect_sys_info(args: &Args) -> SysInfo {
    let mut info_pb = SysInfo::default();

    let mut sys = System::new_all();
    sys.refresh_all();

    info_pb.name = args.user.to_owned();
    info_pb.version = env!("CARGO_PKG_VERSION").to_string();

    info_pb.os_name = std::env::consts::OS.to_string();
    info_pb.os_arch = std::env::consts::ARCH.to_string();
    info_pb.os_family = std::env::consts::FAMILY.to_string();
    info_pb.os_release = sys.long_os_version().unwrap_or_default();
    info_pb.kernel_version = sys.kernel_version().unwrap_or_default();

    // cpu
    let global_cpu = sys.global_cpu_info();
    info_pb.cpu_num = sys.cpus().len() as u32;
    info_pb.cpu_brand = global_cpu.brand().to_string();
    info_pb.cpu_vender_id = global_cpu.vendor_id().to_string();

    info_pb.host_name = sys.host_name().unwrap_or_default();

    info_pb
}

pub fn gen_sys_id(sys_info: &SysInfo) -> String {
    let mut sys = System::new_all();
    let bt = sys.boot_time();

    format!(
        "{:x}",
        md5::compute(format!(
            "{}/{}/{}/{}/{}/{}/{}/{}",
            sys_info.host_name,
            sys_info.os_name,
            sys_info.os_arch,
            sys_info.os_family,
            sys_info.os_release,
            sys_info.kernel_version,
            sys_info.cpu_brand,
            bt,
        ))
    )
}
