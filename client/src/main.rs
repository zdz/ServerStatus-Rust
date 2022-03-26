#![deny(warnings)]
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
use clap::Parser;
use std::collections::HashMap;
use std::net::ToSocketAddrs;
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;
mod status;

const INTERVAL_MS: u64 = 1000;

#[derive(Parser, Debug)]
#[clap(author, version = env!("APP_VERSION"), about, long_about = None)]
struct Args {
    #[clap(short, long, default_value = "http://127.0.0.1:8080/report")]
    addr: String,
    #[clap(short, long, default_value = "h1", help = "username")]
    user: String,
    #[clap(short, long, default_value = "p1", help = "password")]
    pass: String,
    #[clap(short = 'n', long, help = "enable vnstat, default:false")]
    vnstat: bool,
    #[clap(short = 'd', long, help = "disable t/u/p/d, default:false")]
    disable_tupd: bool,
}

fn sample(stat: &mut HashMap<&'static str, serde_json::Value>, args: &Args) {
    let (load_1, load_5, load_15) = status::get_loadavg();
    stat.insert("load_1", serde_json::Value::from(load_1));
    stat.insert("load_5", serde_json::Value::from(load_5));
    stat.insert("load_15", serde_json::Value::from(load_15));

    let uptime = status::get_uptime();
    stat.insert("uptime", serde_json::Value::from(uptime));

    let (mem_total, mem_used, swap_total, swap_free) = status::get_memory();
    stat.insert("memory_total", serde_json::Value::from(mem_total));
    stat.insert("memory_used", serde_json::Value::from(mem_used));
    stat.insert("swap_total", serde_json::Value::from(swap_total));
    stat.insert("swap_used", serde_json::Value::from(swap_total - swap_free));

    let (t, u, p, d) = if args.disable_tupd {
        (0, 0, 0, 0)
    } else {
        status::tupd()
    };
    stat.insert("tcp", serde_json::Value::from(t));
    stat.insert("udp", serde_json::Value::from(u));
    stat.insert("process", serde_json::Value::from(p));
    stat.insert("thread", serde_json::Value::from(d));

    if args.vnstat {
        let (network_in, network_out, m_network_in, m_network_out) = status::get_vnstat_traffic();
        stat.insert("network_in", serde_json::Value::from(network_in));
        stat.insert("network_out", serde_json::Value::from(network_out));
        stat.insert(
            "last_network_in",
            serde_json::Value::from(network_in - m_network_in),
        );
        stat.insert(
            "last_network_out",
            serde_json::Value::from(network_out - m_network_out),
        );
    } else {
        let (network_in, network_out) = status::get_sys_traffic();
        stat.insert("network_in", serde_json::Value::from(network_in));
        stat.insert("network_out", serde_json::Value::from(network_out));
    }

    let (hdd_total, hdd_used) = status::get_hdd();
    stat.insert("hdd_total", serde_json::Value::from(hdd_total));
    stat.insert("hdd_used", serde_json::Value::from(hdd_used));

    {
        let o = *status::G_CPU_PERCENT.lock().unwrap();
        stat.insert("cpu", serde_json::Value::from(o));
    }
    {
        let o = &*status::G_NET_SPEED.lock().unwrap();
        stat.insert("network_rx", serde_json::Value::from(o.netrx));
        stat.insert("network_tx", serde_json::Value::from(o.nettx));
    }
    {
        let o = &*status::G_PING_10010.lock().unwrap();
        stat.insert("ping_10010", serde_json::Value::from(o.lost_rate));
        stat.insert("time_10010", serde_json::Value::from(o.ping_time));
    }
    {
        let o = &*status::G_PING_189.lock().unwrap();
        stat.insert("ping_189", serde_json::Value::from(o.lost_rate));
        stat.insert("time_189", serde_json::Value::from(o.ping_time));
    }
    {
        let o = &*status::G_PING_10086.lock().unwrap();
        stat.insert("ping_10086", serde_json::Value::from(o.lost_rate));
        stat.insert("time_10086", serde_json::Value::from(o.ping_time));
    }
}

async fn do_tcp_report(
    args: &Args,
    stat_base: &mut HashMap<&'static str, serde_json::Value>,
) -> Result<()> {
    // "127.0.0.1:34512";
    let tcp_addr = args
        .addr
        .replace("tcp://", "")
        .to_socket_addrs()?
        .next()
        .unwrap();
    let (ipv4, ipv6) = (tcp_addr.is_ipv4(), tcp_addr.is_ipv6());
    if ipv4 {
        stat_base.insert("online4", serde_json::Value::from(ipv4));
    }
    if ipv6 {
        stat_base.insert("online6", serde_json::Value::from(ipv6));
    }
    // dbg!(&stat_base);

    loop {
        thread::sleep(Duration::from_millis(INTERVAL_MS));

        let result = TcpStream::connect(&tcp_addr).await;
        if result.is_err() {
            error!("{:?}", result);
            continue;
        }
        let mut socket = result.unwrap();
        info!("{}", format!("connected {}", args.addr));

        let mut buf = vec![0; 1024];
        let result = socket.read(&mut buf).await;
        if String::from_utf8(buf)
            .unwrap()
            .contains("Authentication required")
        {
            dbg!("Authentication required");
        }
        if result.is_err() {
            error!("{:?}", result);
            drop(socket);
            continue;
        }

        let mut auth_map = HashMap::new();
        auth_map.insert("frame", "auth".to_string());
        auth_map.insert("user", args.user.to_string());
        auth_map.insert("pass", args.pass.to_string());
        let auth_dat = serde_json::to_string(&auth_map).unwrap() + "\n";

        let result = socket.write_all(auth_dat.as_bytes()).await;
        if result.is_err() {
            error!("{:?}", result);
            drop(socket);
            continue;
        }

        let mut buf = vec![0; 1024];
        let result = socket.read(&mut buf).await;
        if !String::from_utf8(buf)
            .unwrap()
            .contains("Authentication successful")
        {
            dbg!(&result);
            error!("Authentication failed!");
            drop(socket);
            continue;
        }

        loop {
            let mut stat = stat_base.clone();
            stat.insert(
                "latest_ts",
                serde_json::Value::from(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                ),
            );

            sample(&mut stat, args);
            let json_str = serde_json::to_string(&stat).unwrap();
            trace!("json_str => {:?}", json_str);

            //
            let frame_data = json_str + "\n";
            let result = socket.write_all(frame_data.as_bytes()).await;
            if result.is_err() {
                error!("{:?}", result);
                break;
            }

            thread::sleep(Duration::from_millis(INTERVAL_MS));
        }
    }
}

fn do_http_report(
    args: &Args,
    stat_base: &mut HashMap<&'static str, serde_json::Value>,
) -> Result<()> {
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
        stat_base.insert("online4", serde_json::Value::from(ipv4));
    }
    if ipv6 {
        stat_base.insert("online6", serde_json::Value::from(ipv6));
    }
    // dbg!(&stat_base);

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
        let mut stat = stat_base.clone();
        stat.insert(
            "latest_ts",
            serde_json::Value::from(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            ),
        );

        sample(&mut stat, args);
        let json_str = serde_json::to_string(&stat).unwrap();
        trace!("json_str => {:?}", json_str);

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
                .json(&stat)
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

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();
    let args = Args::parse();
    dbg!(&args);

    status::start_net_speed_collect_t();
    status::start_all_ping_collect_t();
    status::start_cpu_percent_collect_t();
    let (ipv4, ipv6) = status::get_network();

    let mut stat_base: HashMap<&str, serde_json::Value> = HashMap::new();
    stat_base.insert("name", serde_json::Value::from(args.user.to_string()));
    stat_base.insert("online4", serde_json::Value::from(ipv4));
    stat_base.insert("online6", serde_json::Value::from(ipv6));
    stat_base.insert("frame", serde_json::Value::from("data"));

    if args.addr.starts_with("http") {
        let result = do_http_report(&args, &mut stat_base);
        dbg!(&result);
    } else if args.addr.starts_with("tcp://") {
        let result = do_tcp_report(&args, &mut stat_base).await;
        dbg!(&result);
    } else {
        eprint!("invalid addr scheme!");
    }

    Ok(())
}
