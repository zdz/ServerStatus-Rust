#![deny(warnings)]
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
use clap::Parser;
use hyper::header;
use once_cell::sync::Lazy;
use prost::Message;
use std::net::ToSocketAddrs;
use std::process;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::{System, SystemExt};
use tokio::time;

use stat_common::server_status::{IpInfo, StatRequest, SysInfo};
type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;
mod grpc;
mod ip_api;
mod status;
mod sys_info;

const INTERVAL_MS: u64 = 1000;
static CU: &str = "cu.tz.cloudcpp.com:80";
static CT: &str = "ct.tz.cloudcpp.com:80";
static CM: &str = "cm.tz.cloudcpp.com:80";

#[derive(Default)]
pub struct ClientConfig {
    ip_info: Option<IpInfo>,
    sys_info: Option<SysInfo>,
}

pub static G_CONFIG: Lazy<Mutex<ClientConfig>> = Lazy::new(|| Mutex::new(ClientConfig::default()));

#[derive(Parser, Debug, Clone)]
#[clap(author, version = env!("APP_VERSION"), about, long_about = None)]
pub struct Args {
    #[clap(short, long, default_value = "http://127.0.0.1:8080/report")]
    addr: String,
    #[clap(short, long, default_value = "h1", help = "username")]
    user: String,
    #[clap(short, long, default_value = "p1", help = "password")]
    pass: String,
    #[clap(short = 'n', long, help = "enable vnstat, default:false")]
    vnstat: bool,
    #[clap(long = "disable-tupd", help = "disable t/u/p/d, default:false")]
    disable_tupd: bool,
    #[clap(long = "disable-ping", help = "disable ping, default:false")]
    disable_ping: bool,
    #[clap(
        long = "disable-extra",
        help = "disable extra info report, default:false"
    )]
    disable_extra: bool,
    #[clap(long = "ct", default_value = CT, help = "China Telecom probe addr")]
    ct_addr: String,
    #[clap(long = "cm", default_value = CM, help = "China Mobile probe addr")]
    cm_addr: String,
    #[clap(long = "cu", default_value = CU, help = "China Unicom probe addr")]
    cu_addr: String,
    #[clap(long = "ip-info", help = "show ip info, default:false")]
    ip_info: bool,
    #[clap(long = "json", help = "use json protocol, default:false")]
    json: bool,
    #[clap(short = '6', long = "ipv6", help = "ipv6 only, default:false")]
    ipv6: bool,
}

fn sample_all(args: &Args, stat_base: &StatRequest) -> StatRequest {
    // dbg!(&stat_base);
    let mut stat_rt = stat_base.clone();

    #[cfg(all(feature = "native", not(feature = "sysinfo")))]
    status::sample(args, &mut stat_rt);
    #[cfg(all(feature = "sysinfo", not(feature = "native")))]
    sys_info::sample(args, &mut stat_rt);

    stat_rt.latest_ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if !args.disable_extra {
        if let Ok(o) = G_CONFIG.lock() {
            if let Some(ip_info) = o.ip_info.as_ref() {
                stat_rt.ip_info = Some(ip_info.clone());
            }
            if let Some(sys_info) = o.sys_info.as_ref() {
                stat_rt.sys_info = Some(sys_info.clone());
            }
        }
    }

    stat_rt
}

fn http_report(args: &Args, stat_base: &mut StatRequest) -> Result<()> {
    let mut domain = args.addr.split('/').collect::<Vec<&str>>()[2].to_owned();
    if !domain.contains(':') {
        if args.addr.contains("https") {
            domain = format!("{}:443", domain);
        } else {
            domain = format!("{}:80", domain);
        }
    }
    let tcp_addr = domain.to_socket_addrs()?.next().unwrap();
    let (ipv4, ipv6) = (tcp_addr.is_ipv4(), tcp_addr.is_ipv6());
    if ipv4 {
        stat_base.online4 = ipv4;
    }
    if ipv6 {
        stat_base.online6 = ipv6;
    }

    let http_client = reqwest::Client::builder()
        .pool_max_idle_per_host(1)
        .connect_timeout(Duration::from_secs(5))
        .user_agent(format!(
            "{}/{}",
            env!("CARGO_BIN_NAME"),
            env!("CARGO_PKG_VERSION")
        ))
        .build()?;
    loop {
        let stat_rt = sample_all(args, stat_base);

        let body_data: Option<Vec<u8>>;
        let mut content_type = "application/octet-stream";
        if args.json {
            let data = serde_json::to_string(&stat_rt)?;
            trace!("json_str => {:?}", serde_json::to_string(&data)?);
            body_data = Some(data.into());
            content_type = "application/json";
        } else {
            let buf = stat_rt.encode_to_vec();
            body_data = Some(buf);
            // content_type = "application/octet-stream";
        }
        // byte 581, json str 1281
        // dbg!(&body_data.as_ref().unwrap().len());

        let client = http_client.clone();
        let url = args.addr.to_string();
        let auth_user = args.user.to_string();
        let auth_pass = args.pass.to_string();

        // http
        tokio::spawn(async move {
            match client
                .post(&url)
                .basic_auth(auth_user, Some(auth_pass))
                .timeout(Duration::from_secs(3))
                .header(header::CONTENT_TYPE, content_type)
                .body(body_data.unwrap())
                .send()
                .await
            {
                Ok(resp) => {
                    info!("report resp => {:?}", resp);
                }
                Err(err) => {
                    error!("report error => {:?}", err);
                }
            }
        });

        thread::sleep(Duration::from_millis(INTERVAL_MS));
    }
}

async fn refresh_ip_info(args: &Args) {
    // refresh/1 hour
    let mut interval = time::interval(time::Duration::from_secs(3600));
    loop {
        info!("get ip info from ip-api.com");
        match ip_api::get_ip_info(args.ipv6).await {
            Ok(ip_info) => {
                info!("refresh_ip_info succ => {:?}", ip_info);
                if let Ok(mut o) = G_CONFIG.lock() {
                    o.ip_info = Some(ip_info);
                }
            }
            Err(err) => {
                error!("refresh_ip_info error => {:?}", err);
            }
        }

        interval.tick().await;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();
    let args = Args::parse();
    dbg!(&args);

    if args.ip_info {
        let info = ip_api::get_ip_info(args.ipv6).await?;
        dbg!(info);
        process::exit(0);
    }

    let sys_info = sys_info::collect_sys_info(&args);
    let sys_info_json = serde_json::to_string(&sys_info)?;
    eprintln!("sys info: {}", sys_info_json);

    if let Ok(mut o) = G_CONFIG.lock() {
        o.sys_info = Some(sys_info);
    }

    // support check
    if !System::IS_SUPPORTED {
        panic!("当前系统不支持，请切换到Python跨平台版本!");
    }

    // use native
    #[cfg(all(feature = "native", not(feature = "sysinfo")))]
    {
        eprintln!("enable feature native");
        status::start_cpu_percent_collect_t();
        status::start_net_speed_collect_t();
    }

    // use sysinfo
    #[cfg(all(feature = "sysinfo", not(feature = "native")))]
    {
        eprintln!("enable feature sysinfo");
        sys_info::start_cpu_percent_collect_t();
        sys_info::start_net_speed_collect_t();
    }

    status::start_all_ping_collect_t(&args);
    let (ipv4, ipv6) = status::get_network();
    eprintln!("get_network (ipv4, ipv6) => ({}, {})", ipv4, ipv6);

    if !args.disable_extra {
        // refresh ip info
        let args_1 = args.clone();
        tokio::spawn(async move { refresh_ip_info(&args_1).await });
    }

    let mut stat_base = StatRequest {
        name: args.user.to_string(),
        frame: "data".to_string(),
        online4: ipv4,
        online6: ipv6,
        vnstat: args.vnstat,
        ..Default::default()
    };

    if args.addr.starts_with("http") {
        let result = http_report(&args, &mut stat_base);
        dbg!(&result);
    } else if args.addr.starts_with("grpc") {
        let result = grpc::report(&args, &mut stat_base).await;
        dbg!(&result);
    } else {
        eprint!("invalid addr scheme!");
    }

    Ok(())
}
