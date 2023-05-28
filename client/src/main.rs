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
mod geoip;
mod grpc;
mod status;
mod sys_info;
mod vnstat;

static CU: &str = "cu.tz.cloudcpp.com:80";
static CT: &str = "ct.tz.cloudcpp.com:80";
static CM: &str = "cm.tz.cloudcpp.com:80";

#[derive(Default)]
pub struct ClientConfig {
    ip_info: Option<IpInfo>,
    sys_info: Option<SysInfo>,
}

pub static G_CONFIG: Lazy<Mutex<ClientConfig>> = Lazy::new(|| Mutex::new(ClientConfig::default()));

// https://docs.rs/clap/latest/clap/_derive/index.html#command-attributes
#[derive(Parser, Debug, Clone)]
#[command(author, version = env!("APP_VERSION"), about, long_about = None)]
pub struct Args {
    #[arg(short, long, env = "SSR_ADDR", default_value = "http://127.0.0.1:8080/report")]
    addr: String,
    #[arg(short, long, env = "SSR_USER", default_value = "h1", help = "username")]
    user: String,
    #[arg(short, long, env = "SSR_PASS", default_value = "p1", help = "password")]
    pass: String,
    #[arg(short = 'n', long, env = "SSR_VNSTAT", help = "enable vnstat, default:false")]
    vnstat: bool,
    #[arg(
        long = "vnstat-mr",
        env = "SSR_VNSTAT_MR",
        default_value_t = 1,
        help = "vnstat month rotate 1-28"
    )]
    vnstat_mr: u32,
    #[arg(
        long = "interval",
        env = "SSR_INTERVAL",
        default_value_t = 1,
        help = "data report interval (s)"
    )]
    report_interval: u64,
    #[arg(
        long = "disable-tupd",
        env = "SSR_DISABLE_TUPD",
        help = "disable t/u/p/d, default:false"
    )]
    disable_tupd: bool,
    #[arg(
        long = "disable-ping",
        env = "SSR_DISABLE_PING",
        help = "disable ping, default:false"
    )]
    disable_ping: bool,
    #[arg(
        long = "disable-extra",
        env = "SSR_DISABLE_EXTRA",
        help = "disable extra info report, default:false"
    )]
    disable_extra: bool,
    #[arg(long = "ct",  env = "SSR_CT_ADDR", default_value = CT, help = "China Telecom probe addr")]
    ct_addr: String,
    #[arg(long = "cm",  env = "SSR_CM_ADDR", default_value = CM, help = "China Mobile probe addr")]
    cm_addr: String,
    #[arg(long = "cu",  env = "SSR_CU_ADDR", default_value = CU, help = "China Unicom probe addr")]
    cu_addr: String,
    #[arg(long = "sys-info", help = "show sys info, default:false")]
    sys_info: bool,
    #[arg(long = "ip-info", help = "show ip info, default:false")]
    ip_info: bool,
    #[arg(
        long = "ip-source",
        env = "SSR_IP_SOURCE",
        default_value = "ip-api.com",
        help = "ip info source"
    )]
    ip_source: String,
    #[arg(long = "json", help = "use json protocol, default:false")]
    json: bool,
    #[arg(short = '6', long = "ipv6", help = "ipv6 only, default:false")]
    ipv6: bool,
    // for group
    #[arg(short, long, env = "SSR_GID", default_value = "", help = "group id")]
    gid: String,
    #[arg(
        long = "alias",
        env = "SSR_ALIAS",
        default_value = "unknown",
        help = "alias for host"
    )]
    alias: String,
    #[arg(short, long, env = "SSR_WEIGHT", default_value = "0", help = "weight for rank")]
    weight: u64,
    #[arg(
        long = "disable-notify",
        env = "SSR_DISABLE_NOTIFY",
        help = "disable notify, default:false"
    )]
    disable_notify: bool,
    #[arg(short = 't', long = "type", env = "SSR_TYPE", default_value = "", help = "host type")]
    host_type: String,
    #[arg(long, env = "SSR_LOC", default_value = "", help = "location")]
    location: String,
    #[arg(short = 'd', long = "debug", env = "SSR_DEBUG", help = "debug mode, default:false")]
    debug: bool,
    #[arg(
        short = 'i',
        long = "iface",
        env = "SSR_IFACE",
        default_values_t = Vec::<String>::new(),
        value_delimiter = ',',
        help = "iface list, eg: eth0,eth1"
    )]
    iface: Vec<String>,
    #[arg(
        short = 'e',
        long = "exclude-iface",
        env = "SSR_EXCLUDE_IFACE",
        default_value = "lo,docker,vnet,veth,vmbr,kube,br-",
        value_delimiter = ',',
        help = "exclude iface"
    )]
    exclude_iface: Vec<String>,
    #[arg(long, env = "SSR_PROXY", default_value = "", help = "proxy")]
    proxy: String,
    #[arg(long, env = "SSR_NO_PROXY", default_value = "", help = "no proxy, eg: ip-api.com")]
    no_proxy: String,
}

impl Args {
    pub fn skip_iface(&self, name: &str) -> bool {
        if !self.iface.is_empty() {
            if self.iface.iter().any(|fa| name.eq(fa)) {
                return false;
            }
            return true;
        }
        if self.exclude_iface.iter().any(|sk| name.contains(sk)) {
            return true;
        }
        false
    }
}

fn sample_all(args: &Args, stat_base: &StatRequest) -> StatRequest {
    // dbg!(&stat_base);
    let mut stat_rt = stat_base.clone();

    #[cfg(all(feature = "native", not(feature = "sysinfo"), target_os = "linux"))]
    status::sample(args, &mut stat_rt);
    #[cfg(all(feature = "sysinfo", not(feature = "native")))]
    sys_info::sample(args, &mut stat_rt);

    stat_rt.latest_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

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
            domain = format!("{domain}:443");
        } else {
            domain = format!("{domain}:80");
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

    let mut http_client_builder = reqwest::Client::builder()
        .pool_max_idle_per_host(1)
        .connect_timeout(Duration::from_secs(5))
        .user_agent(format!("{}/{}", env!("CARGO_BIN_NAME"), env!("CARGO_PKG_VERSION")));

    if !args.proxy.is_empty() {
        let mut proxy = reqwest::Proxy::all(&args.proxy)?;
        if !args.no_proxy.is_empty() {
            proxy = proxy.no_proxy(reqwest::NoProxy::from_string(&args.no_proxy));
        }

        http_client_builder = http_client_builder.proxy(proxy);
    }

    let http_client = http_client_builder.build()?;
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
        let auth_pass = args.pass.to_string();
        let auth_user: String;
        let ssr_auth: &str;
        if args.gid.is_empty() {
            auth_user = args.user.to_string();
            ssr_auth = "single";
        } else {
            auth_user = args.gid.to_string();
            ssr_auth = "group";
        }

        // http
        tokio::spawn(async move {
            match client
                .post(&url)
                .basic_auth(auth_user, Some(auth_pass))
                .timeout(Duration::from_secs(3))
                .header(header::CONTENT_TYPE, content_type)
                .header("ssr-auth", ssr_auth)
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

        thread::sleep(Duration::from_secs(args.report_interval));
    }
}

async fn refresh_ip_info(args: &Args) {
    // refresh/1 hour
    let mut interval = time::interval(time::Duration::from_secs(3600));
    loop {
        info!("get ip info from ip-api.com");
        match geoip::get_ip_info(args).await {
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
    let mut args = Args::parse();
    args.iface.retain(|e| !e.trim().is_empty());
    args.exclude_iface.retain(|e| !e.trim().is_empty());
    if args.debug {
        dbg!(&args);
    }

    if args.ip_info {
        let info = geoip::get_ip_info(&args).await?;
        dbg!(info);
        process::exit(0);
    }

    // support check
    if !System::IS_SUPPORTED {
        panic!("当前系统不支持，请切换到Python跨平台版本!");
    }

    let sys_info = sys_info::collect_sys_info(&args);
    let sys_info_json = serde_json::to_string(&sys_info)?;
    let sys_id = sys_info::gen_sys_id(&sys_info);
    eprintln!("sys id: {sys_id}");
    eprintln!("sys info: {sys_info_json}");

    if args.sys_info {
        sys_info::print_sysinfo();
        process::exit(0);
    }

    if let Ok(mut o) = G_CONFIG.lock() {
        o.sys_info = Some(sys_info);
    }

    // use native
    #[cfg(all(feature = "native", not(feature = "sysinfo"), target_os = "linux"))]
    {
        eprintln!("feature native enabled");
        status::start_cpu_percent_collect_t();
        status::start_net_speed_collect_t(&args);
    }

    // use sysinfo
    #[cfg(all(feature = "sysinfo", not(feature = "native")))]
    {
        eprintln!("feature sysinfo enabled");
        sys_info::start_cpu_percent_collect_t();
        sys_info::start_net_speed_collect_t(&args);
    }

    status::start_all_ping_collect_t(&args);
    let (ipv4, ipv6) = status::get_network();
    eprintln!("get_network (ipv4, ipv6) => ({ipv4}, {ipv6})");

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
        weight: args.weight,
        notify: true,
        version: env!("CARGO_PKG_VERSION").to_string(),
        ..Default::default()
    };
    if !args.gid.is_empty() {
        stat_base.gid = args.gid.to_owned();
        if stat_base.name.eq("h1") {
            stat_base.name = sys_id;
        }
        if args.alias.eq("unknown") {
            args.alias = stat_base.name.to_owned();
        } else {
            stat_base.alias = args.alias.to_owned();
        }
    }
    if args.disable_notify {
        stat_base.notify = false;
    }
    if !args.host_type.is_empty() {
        stat_base.r#type = args.host_type.to_owned();
    }
    if !args.location.is_empty() {
        stat_base.location = args.location.to_owned();
    }
    // dbg!(&stat_base);

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
