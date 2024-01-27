#![deny(warnings)]
#![allow(unused)]
use lazy_static::lazy_static;
use std::collections::HashSet;
use std::fs;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use sysinfo::CpuRefreshKind;
use sysinfo::{Components, Disks, MemoryRefreshKind, Networks, RefreshKind, System};

use crate::status;
use crate::vnstat;
use crate::Args;
use stat_common::server_status::{StatRequest, SysInfo};

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
pub fn start_cpu_percent_collect_t() {
    let mut sys = System::new_with_specifics(RefreshKind::new().with_cpu(CpuRefreshKind::new().with_cpu_usage()));
    thread::spawn(move || loop {
        sys.refresh_cpu();

        let global_cpu = sys.global_cpu_info();
        if let Ok(mut cpu_percent) = G_CPU_PERCENT.lock() {
            *cpu_percent = global_cpu.cpu_usage().round() as f64;
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

        networks.refresh_list();
        thread::sleep(Duration::from_millis(SAMPLE_PERIOD));
    });
}

pub fn sample(args: &Args, stat: &mut StatRequest) {
    stat.version = env!("CARGO_PKG_VERSION").to_string();
    stat.vnstat = args.vnstat;

    // 注意：sysinfo 统一使用 KB, 非KiB，需要转换一下
    let mut unit: u64 = 1024;

    // mac系统 下面使用 KB 展示
    #[cfg(target_os = "macos")]
    {
        stat.si = true;
        unit = 1000;
    }

    let mut sys = System::new_with_specifics(RefreshKind::new().with_memory(MemoryRefreshKind::everything()));

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

    // hdd KB -> KiB
    let (mut hdd_total, mut hdd_avail) = (0_u64, 0_u64);
    let mut uniq_disk_set = HashSet::new();
    let disks = Disks::new_with_refreshed_list();
    for disk in &disks {
        let fs = disk.file_system().to_str().unwrap().to_lowercase();
        if G_EXPECT_FS.iter().any(|&k| fs.contains(k)) {
            if uniq_disk_set.contains(disk.name()) {
                continue;
            }
            uniq_disk_set.insert(disk.name());

            hdd_total += disk.total_space();
            hdd_avail += disk.available_space();
        }
    }

    stat.hdd_total = hdd_total / unit.pow(2);
    stat.hdd_used = (hdd_total - hdd_avail) / unit.pow(2);

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
    sys.refresh_cpu();

    info_pb.name = args.user.to_owned();
    info_pb.version = env!("CARGO_PKG_VERSION").to_string();

    info_pb.os_name = std::env::consts::OS.to_string();
    info_pb.os_arch = std::env::consts::ARCH.to_string();
    info_pb.os_family = std::env::consts::FAMILY.to_string();
    info_pb.os_release = System::long_os_version().unwrap_or_default();
    info_pb.kernel_version = System::kernel_version().unwrap_or_default();

    // cpu
    let global_cpu = sys.global_cpu_info();
    info_pb.cpu_num = sys.cpus().len() as u32;
    info_pb.cpu_brand = global_cpu.brand().to_string();
    info_pb.cpu_vender_id = global_cpu.vendor_id().to_string();

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

    println!("=> disks:");
    let disks = Disks::new_with_refreshed_list();
    for disk in &disks {
        println!(
            "\t{}\t{}\t{}/{} B\tremovable:{}\tmounted on:{}",
            disk.name().to_str().unwrap_or_default(),
            disk.file_system().to_str().unwrap_or_default(),
            disk.available_space(),
            disk.total_space(),
            disk.is_removable(),
            disk.mount_point().to_str().unwrap_or_default(),
        );
        // println!("\t{:?}", disk);
    }

    // Network interfaces name, data received and data transmitted:
    println!("=> networks:");
    let networks = Networks::new_with_refreshed_list();
    for (interface_name, data) in &networks {
        println!("\t{interface_name}: \t{}/{} B", data.received(), data.transmitted());
    }

    // Components temperature:
    println!("=> components:");
    let components = Components::new_with_refreshed_list();
    for component in &components {
        println!("\t{component:?}");
    }

    println!("=> system:");
    // RAM and swap information:
    println!("\ttotal memory: {} bytes", sys.total_memory());
    println!("\tused memory : {} bytes", sys.used_memory());
    println!("\tavai memory : {} bytes", sys.available_memory());
    println!("\tfree memory : {} bytes", sys.free_memory());
    println!("\ttotal swap  : {} bytes", sys.total_swap());
    println!("\tused swap   : {} bytes", sys.used_swap());

    // Display system information:
    println!("\tSystem name:             {:?}", System::name());
    println!("\tSystem kernel version:   {:?}", System::kernel_version());
    println!("\tSystem OS version:       {:?}", System::os_version());
    println!("\tSystem long OS version:  {:?}", System::long_os_version());
    println!("\tSystem Dist ID:          {:?}", System::distribution_id());
    println!("\tSystem host name:        {:?}", System::host_name());
    println!("\tSystem cpu arch:         {:?}", System::cpu_arch());

    let load_avg = System::load_average();
    println!(
        "\tone minute: {:.2}, five minutes: {:.2}, fifteen minutes: {:.2}",
        load_avg.one, load_avg.five, load_avg.fifteen,
    );

    // Number of CPUs:
    let global_cpu = sys.global_cpu_info();
    println!("\tCPU Num: {}", sys.cpus().len());

    for cpu in sys.cpus() {
        println!("\t\tCPU Brand: {}", cpu.brand());
        println!("\t\tCPU VerderId: {}", cpu.vendor_id());
        println!("\t\tCPU Frequency: {}", cpu.frequency());
    }
    // println!("\tCPU Frequency: {}", global_cpu.frequency());

    // Display processes ID, name na disk usage:
    // for (pid, process) in sys.processes() {
    //     println!("[{}] {} {:?}", pid, process.name(), process.disk_usage());
    // }
}
