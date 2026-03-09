#![deny(warnings)]
#![allow(unused)]
use lazy_static::lazy_static;
use prettytable::{row, Table};
use std::collections::HashSet;
use std::fs;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use sysinfo::CpuRefreshKind;
use sysinfo::{Components, Disks, MemoryRefreshKind, Networks, RefreshKind, System};
use std::env::consts::ARCH;
use crate::status;
use crate::vnstat;
use crate::Args;
use stat_common::{
    server_status::{DiskInfo, StatRequest, SysInfo},
    utils::bytes2human,
};

const SAMPLE_PERIOD: u64 = 1000; //ms

lazy_static! {
    pub static ref G_EXPECT_FS: Vec<&'static str> = [
        "apfs",
        "hfs",
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

/// A minimal disk descriptor used by [`calc_hdd_stats`] so the logic can be
/// tested without OS-level disk enumeration.
#[derive(Debug)]
pub(crate) struct DiskCalcInput {
    pub name: String,
    pub fs_type: String,
    pub total: u64,
    pub avail: u64,
}

/// Compute `(hdd_total_mb, hdd_used_mb)` from a slice of disk descriptors.
///
/// * `si = true`  → SI units (1 MB  = 1 000 000 bytes, macOS style)
/// * `si = false` → IEC units (1 MiB = 1 048 576 bytes, Linux/Windows style)
///
/// On non-Windows platforms the same physical disk can appear multiple times
/// (once per mount point); `is_windows = false` enables deduplication by disk
/// name so each physical device is counted only once.
pub(crate) fn calc_hdd_stats(disks: &[DiskCalcInput], si: bool, is_windows: bool) -> (u64, u64) {
    let (mut total_bytes, mut avail_bytes) = (0_u64, 0_u64);
    // Dedup set is only needed on non-Windows platforms.
    let mut seen: Option<HashSet<String>> = if !is_windows { Some(HashSet::new()) } else { None };

    for disk in disks {
        let fs = disk.fs_type.to_lowercase();
        if G_EXPECT_FS.iter().any(|&k| fs.contains(k)) {
            if let Some(ref mut s) = seen {
                if s.contains(&disk.name) {
                    continue;
                }
                s.insert(disk.name.clone());
            }
            total_bytes += disk.total;
            avail_bytes += disk.avail;
        }
    }

    // SI:  1 MB  = 1_000_000 bytes  (10^6,  used on macOS)
    // IEC: 1 MiB = 1_048_576 bytes  (2^20, used on Linux / Windows)
    let divisor = if si { 1_000_000_u64 } else { 1_048_576_u64 };
    (total_bytes / divisor, (total_bytes - avail_bytes) / divisor)
}

pub fn start_cpu_percent_collect_t() {
    let mut sys = System::new_with_specifics(RefreshKind::nothing().with_cpu(CpuRefreshKind::nothing().with_cpu_usage()));
    thread::spawn(move || loop {
        sys.refresh_cpu_all();

        let global_cpu = sys.global_cpu_usage();
        if let Ok(mut cpu_percent) = G_CPU_PERCENT.lock() {
            *cpu_percent = global_cpu.round() as f64;
        }

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
    let mut networks = Networks::new_with_refreshed_list();
    let args_1 = args.clone();
    thread::spawn(move || loop {
        let (mut net_rx, mut net_tx) = (0_u64, 0_u64);
        for (name, data) in &networks {
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

        networks.refresh(true);
        thread::sleep(Duration::from_millis(SAMPLE_PERIOD));
    });
}

pub fn sample(args: &Args, stat: &mut StatRequest) {
    stat.version = env!("CARGO_PKG_VERSION").to_string();
    stat.vnstat = args.vnstat;

    // mac系统 下面使用 KB 展示
    #[cfg(target_os = "macos")]
    {
        stat.si = true;
    }

    let mut sys = System::new_with_specifics(RefreshKind::nothing().with_memory(MemoryRefreshKind::everything()));

    // uptime
    stat.uptime = System::uptime();
    // load average
    let load_avg = System::load_average();
    stat.load_1 = load_avg.one;
    stat.load_5 = load_avg.five;
    stat.load_15 = load_avg.fifteen;

    // mem 不用转。。。(KB -> KiB)
    stat.memory_total = sys.total_memory() / 1024;
    #[cfg(target_os = "macos")]
    {
        stat.memory_used = (sys.total_memory() - sys.available_memory()) / 1024;
    }
    #[cfg(not(target_os = "macos"))]
    {
        stat.memory_used = sys.used_memory() / 1024;
    }
    stat.swap_total = sys.total_swap() / 1024;
    stat.swap_used = (sys.total_swap() - sys.free_swap()) / 1024;

    // hdd
    let disks = Disks::new_with_refreshed_list();
    stat.disks.clear();

    for disk in &disks {
        let fs_type = disk.file_system().to_string_lossy().to_lowercase();
        let total_space = disk.total_space();

        if G_EXPECT_FS.iter().any(|&k| fs_type.contains(k)) {
            stat.disks.push(DiskInfo {
                name: disk.name().to_string_lossy().to_string(),
                mount_point: disk.mount_point().to_string_lossy().to_string(),
                file_system: fs_type,
                total: total_space,
                used: total_space - disk.available_space(),
                free: disk.available_space(),
            });
        }
    }

    let disk_inputs: Vec<DiskCalcInput> = disks
        .iter()
        .map(|d| DiskCalcInput {
            name: d.name().to_string_lossy().into_owned(),
            fs_type: d.file_system().to_string_lossy().into_owned(),
            total: d.total_space(),
            avail: d.available_space(),
        })
        .collect();

    let is_windows = cfg!(target_os = "windows");
    let (hdd_total, hdd_used) = calc_hdd_stats(&disk_inputs, stat.si, is_windows);
    stat.hdd_total = hdd_total;
    stat.hdd_used = hdd_used;
    // t/u/p/d
    let (t, u, p, d) = if args.disable_tupd {
        (0, 0, 0, 0)
    } else if "linux".eq(std::env::consts::OS) {
        status::tupd()
    } else {
        // sys.processes()
        (0, 0, 0, 0)
    };
    stat.tcp = t;
    stat.udp = u;
    stat.process = p;
    stat.thread = d;

    // traffic
    if args.vnstat {
        #[cfg(target_os = "linux")]
        {
            let (network_in, network_out, m_network_in, m_network_out) = vnstat::get_traffic(args).unwrap();
            stat.network_in = network_in;
            stat.network_out = network_out;
            stat.last_network_in = network_in - m_network_in;
            stat.last_network_out = network_out - m_network_out;
        }
    } else {
        let (mut network_in, mut network_out) = (0_u64, 0_u64);

        let networks = Networks::new_with_refreshed_list();
        for (name, data) in &networks {
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

    let mut sys = System::new();
    sys.refresh_cpu_all();

    info_pb.name = args.user.to_owned();
    info_pb.version = env!("CARGO_PKG_VERSION").to_string();

    info_pb.os_name = std::env::consts::OS.to_string();
    info_pb.os_arch = std::env::consts::ARCH.to_string();
    info_pb.os_family = std::env::consts::FAMILY.to_string();
    info_pb.os_release = System::long_os_version().unwrap_or_default();
    info_pb.kernel_version = System::kernel_version().unwrap_or_default();

    // cpu
    let cpus = sys.cpus();
    info_pb.cpu_num = cpus.len() as u32;
    if let Some(cpu) = cpus.iter().next() {
        info_pb.cpu_brand = cpu.brand().to_string();
        info_pb.cpu_vender_id = cpu.vendor_id().to_string();
    }

    info_pb.host_name = System::host_name().unwrap_or_default();

    info_pb
}

pub fn gen_sys_id(sys_info: &SysInfo) -> String {
    // read from .server_status_sys_id
    const SYS_ID_FILE: &str = ".server_status_sys_id";

    match fs::read_to_string(SYS_ID_FILE) {
        Ok(content) => {
            if (!content.is_empty()) {
                info!("{}", format!("read sys_id from {SYS_ID_FILE}"));
                return content.trim().to_string();
            }
        }
        Err(_) => {
            warn!("{}", format!("can't read {SYS_ID_FILE}, regen sys_id"));
        }
    }

    let mut sys = System::new();
    let bt = System::boot_time();

    let sys_id = format!(
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
    );

    match fs::write(SYS_ID_FILE, &sys_id) {
        Ok(()) => {
            info!("{}", format!("save sys_id to {SYS_ID_FILE} succ"));
        }
        Err(_) => {
            warn!("{}", format!("save sys_id to {SYS_ID_FILE} fail"));
        }
    }

    sys_id
}

pub fn print_sysinfo() {
    use sysinfo::{Components, Disks, MemoryRefreshKind, Networks, RefreshKind, System};
    let mut sys = System::new_all();
    sys.refresh_all();

    let mut si = false;
    #[cfg(target_os = "macos")]
    {
        si = true;
    }

    let mut sysinfo_t = Table::new();
    sysinfo_t.set_titles(row!["Category", "Detail"]);

    // Components temperature:
    let mut components_sb = String::new();
    let components = Components::new_with_refreshed_list();
    for component in &components {
        components_sb.push_str(&format!("{component:?}\n"));
    }
    sysinfo_t.add_row(row!["Components", components_sb]);

    // Network interfaces name, data received and data transmitted:
    let mut network_t = Table::new();
    network_t.set_format(*prettytable::format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    network_t.set_titles(row!["Name", "received", "transmitted"]);
    let networks = Networks::new_with_refreshed_list();
    for (interface_name, data) in &networks {
        network_t.add_row(row![interface_name, data.received(), data.transmitted()]);
    }
    sysinfo_t.add_row(row!["Networks", network_t]);

    let mut system_t = Table::new();
    system_t.set_format(*prettytable::format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    system_t.set_titles(row!["type", "value"]);

    system_t.add_row(row!["System name", System::name().unwrap_or_default()]);
    system_t.add_row(row!["Kernel version", System::kernel_version().unwrap_or_default()]);
    system_t.add_row(row!["OS version", System::os_version().unwrap_or_default()]);
    system_t.add_row(row!["Long OS version", System::long_os_version().unwrap_or_default()]);
    system_t.add_row(row!["Distribution ID", System::distribution_id()]);
    system_t.add_row(row!["Host name", System::host_name().unwrap_or_default()]);
    system_t.add_row(row!["CPU arch", ARCH]);

    let mut cpu_t = Table::new();
    cpu_t.set_format(*prettytable::format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    cpu_t.set_titles(row!["#", "Brand", "VerderId", "Frequency"]);
    for (idx, cpu) in sys.cpus().iter().enumerate() {
        cpu_t.add_row(row![idx, cpu.brand(), cpu.vendor_id(), cpu.frequency()]);
    }
    system_t.add_row(row!["CPU", cpu_t]);

    let load_avg = System::load_average();
    system_t.add_row(row![
        "Load Average",
        format!("{:.2}, {:.2}, {:.2}", load_avg.one, load_avg.five, load_avg.fifteen)
    ]);

    let mut mem_t = Table::new();
    mem_t.set_format(*prettytable::format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    mem_t.set_titles(row!["-", "bytes", "human"]);
    mem_t.add_row(row![
        "total memory",
        sys.total_memory(),
        bytes2human(sys.total_memory(), 2, si)
    ]);
    mem_t.add_row(row![
        "used memory",
        sys.used_memory(),
        bytes2human(sys.used_memory(), 2, si)
    ]);
    mem_t.add_row(row![
        "avai memory",
        sys.available_memory(),
        bytes2human(sys.available_memory(), 2, si)
    ]);
    mem_t.add_row(row![
        "free memory",
        sys.free_memory(),
        bytes2human(sys.free_memory(), 2, si)
    ]);
    mem_t.add_row(row![
        "total swap",
        sys.total_swap(),
        bytes2human(sys.total_swap(), 2, si)
    ]);
    mem_t.add_row(row!["used swap", sys.used_swap(), bytes2human(sys.used_swap(), 2, si)]);

    system_t.add_row(row!["Mem", mem_t]);

    sysinfo_t.add_row(row!["System", system_t]);

    let mut dt = Table::new();
    dt.set_format(*prettytable::format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    dt.set_titles(row!["name", "mount_point", "fs", "total", "available", "is_removable"]);
    let disks = Disks::new_with_refreshed_list();
    for disk in &disks {
        dt.add_row(row![
            disk.name().to_str().unwrap_or_default(),
            disk.mount_point().to_str().unwrap_or_default(),
            disk.file_system().to_str().unwrap_or_default(),
            bytes2human(disk.total_space(), 2, si),
            bytes2human(disk.available_space(), 2, si),
            disk.is_removable(),
        ]);
    }
    sysinfo_t.add_row(row!["Disks", dt]);

    sysinfo_t.printstd();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disk(name: &str, fs: &str, total: u64, avail: u64) -> DiskCalcInput {
        DiskCalcInput {
            name: name.to_string(),
            fs_type: fs.to_string(),
            total,
            avail,
        }
    }

    // ── SI units (macOS, 1 MB = 1_000_000 bytes) ────────────────────────────

    #[test]
    fn test_si_units_basic() {
        // 1 GB (SI) = 1_000_000_000 bytes; used = 500 MB
        let disks = vec![disk("/dev/sda1", "apfs", 1_000_000_000, 500_000_000)];
        let (total, used) = calc_hdd_stats(&disks, true, false);
        assert_eq!(total, 1000);
        assert_eq!(used, 500);
    }

    // ── IEC units (Linux/Windows, 1 MiB = 1_048_576 bytes) ──────────────────

    #[test]
    fn test_iec_units_basic() {
        // 1 GiB = 1_073_741_824 bytes = 1024 MiB; used = 512 MiB
        let disks = vec![disk("/dev/sda1", "ext4", 1_073_741_824, 536_870_912)];
        let (total, used) = calc_hdd_stats(&disks, false, false);
        assert_eq!(total, 1024);
        assert_eq!(used, 512);
    }

    // ── Filesystem filtering ─────────────────────────────────────────────────

    #[test]
    fn test_unknown_fs_excluded() {
        // tmpfs / devtmpfs are not in G_EXPECT_FS and should not be counted
        let disks = vec![
            disk("/dev/sda1", "ext4", 1_073_741_824, 536_870_912),
            disk("tmpfs", "tmpfs", 536_870_912, 536_870_912),
        ];
        let (total, used) = calc_hdd_stats(&disks, false, false);
        assert_eq!(total, 1024);
        assert_eq!(used, 512);
    }

    #[test]
    fn test_all_unknown_fs_gives_zero() {
        let disks = vec![
            disk("tmpfs", "tmpfs", 1_073_741_824, 1_073_741_824),
            disk("devtmpfs", "devtmpfs", 1_073_741_824, 1_073_741_824),
        ];
        let (total, used) = calc_hdd_stats(&disks, false, false);
        assert_eq!(total, 0);
        assert_eq!(used, 0);
    }

    // ── Disk deduplication on non-Windows ────────────────────────────────────

    #[test]
    fn test_dedup_same_name_counted_once() {
        // The same physical disk (/dev/sda) may surface under multiple mount
        // points; only the first entry should contribute to the totals.
        let disks = vec![
            disk("/dev/sda", "ext4", 1_073_741_824, 536_870_912),
            disk("/dev/sda", "ext4", 1_073_741_824, 536_870_912),
        ];
        let (total, used) = calc_hdd_stats(&disks, false, false);
        assert_eq!(total, 1024);
        assert_eq!(used, 512);
    }

    #[test]
    fn test_different_disk_names_both_counted() {
        let disks = vec![
            disk("/dev/sda1", "ext4", 1_073_741_824, 536_870_912),
            disk("/dev/sdb1", "xfs", 1_073_741_824, 0),
        ];
        let (total, used) = calc_hdd_stats(&disks, false, false);
        assert_eq!(total, 2048);
        assert_eq!(used, 1536);
    }

    // ── Windows: no deduplication ────────────────────────────────────────────

    #[test]
    fn test_windows_no_dedup() {
        // On Windows (is_windows = true) every entry is always counted,
        // even when two entries share the same name.
        let disks = vec![
            disk("C:", "ntfs", 1_073_741_824, 536_870_912),
            disk("C:", "ntfs", 1_073_741_824, 536_870_912),
        ];
        let (total, used) = calc_hdd_stats(&disks, false, true);
        assert_eq!(total, 2048);
        assert_eq!(used, 1024);
    }

    // ── Edge cases ───────────────────────────────────────────────────────────

    #[test]
    fn test_empty_disk_list() {
        let (total, used) = calc_hdd_stats(&[], false, false);
        assert_eq!(total, 0);
        assert_eq!(used, 0);
    }

    #[test]
    fn test_fully_used_disk() {
        let disks = vec![disk("/dev/sda1", "ext4", 1_073_741_824, 0)];
        let (total, used) = calc_hdd_stats(&disks, false, false);
        assert_eq!(total, 1024);
        assert_eq!(used, 1024);
    }

    #[test]
    fn test_case_insensitive_fs_match() {
        // Filesystem strings from the OS may be uppercase or mixed case.
        let disks = vec![disk("/dev/sda1", "EXT4", 1_073_741_824, 536_870_912)];
        let (total, used) = calc_hdd_stats(&disks, false, false);
        assert_eq!(total, 1024);
        assert_eq!(used, 512);
    }
}
