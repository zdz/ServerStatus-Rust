#![deny(warnings)]
// #![allow(unused)]
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
#[macro_use]
extern crate prettytable;
use bytes::Buf;
use clap::Parser;
use http_auth_basic::Credentials;
use minijinja::context;
use once_cell::sync::OnceCell;
use prost::Message;
use rust_embed::RustEmbed;
use stat_common::server_status::StatRequest;
use std::collections::HashMap;
use std::process;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use tokio::runtime::Handle;

mod config;
mod grpc;
mod jinja;
mod notifier;
mod payload;
mod stats;

use hyper::service::{make_service_fn, service_fn};
use hyper::{header, Body, Method, Request, Response, Server, StatusCode};
type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;

static NOTFOUND: &[u8] = b"Not Found";
static UNAUTHORIZED: &[u8] = b"Unauthorized";
static INTERNAL_SERVER_ERROR: &[u8] = b"Internal Server Error";

static G_CONFIG: OnceCell<crate::config::Config> = OnceCell::new();
static G_STATS_MGR: OnceCell<crate::stats::StatsMgr> = OnceCell::new();

#[derive(RustEmbed)]
#[folder = "../web"]
#[prefix = "/"]
struct Asset;

#[derive(Parser, Debug)]
#[clap(author, version = env!("APP_VERSION"), about, long_about = None)]
struct Args {
    #[clap(short, long, default_value = "config.toml")]
    config: String,
    #[clap(short = 't', long, help = "config test, default:false")]
    config_test: bool,
    #[clap(long = "notify-test", help = "notify test, default:false")]
    notify_test: bool,
    #[clap(long = "cloud", help = "cloud mode, load cfg from env var: SRV_CONF")]
    cloud: bool,
}

// stat report
async fn stats_report(req: Request<Body>) -> Result<Response<Body>> {
    let req_header = req.headers();
    // auth
    let mut auth_ok = false;
    if let Some(auth) = req_header.get(hyper::header::AUTHORIZATION) {
        let auth_header_value = auth.to_str()?.to_string();
        if let Ok(credentials) = Credentials::from_header(auth_header_value) {
            if let Some(cfg) = G_CONFIG.get() {
                auth_ok = cfg.auth(&credentials.user_id, &credentials.password);
            }
        }
    }
    if !auth_ok {
        return Ok(Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(UNAUTHORIZED.into())?);
    }
    // auth end

    let mut json_data: Option<serde_json::Value> = None;
    if let Ok(content_type) = req_header
        .get(hyper::header::CONTENT_TYPE)
        .unwrap()
        .clone()
        .to_str()
    {
        let whole_body = hyper::body::aggregate(req).await?;
        // dbg!(content_type);
        if content_type.eq(&mime::APPLICATION_JSON.to_string()) {
            // json
            json_data = Some(serde_json::from_reader(whole_body.reader())?);
        } else if content_type.eq(&mime::APPLICATION_OCTET_STREAM.to_string()) {
            // protobuf
            let stat = StatRequest::decode(whole_body)?;
            json_data = Some(serde_json::to_value(stat)?);
        }
    }

    // report
    if let Some(mgr) = G_STATS_MGR.get() {
        mgr.report(json_data.unwrap())?;
    }

    let mut resp = HashMap::new();
    resp.insert(&"code", serde_json::Value::from(0_i32));
    let resp_str = serde_json::to_string(&resp)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(resp_str))?)
}

// get json data
async fn get_stats_json() -> Result<Response<Body>> {
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(G_STATS_MGR.get().unwrap().get_stats_json()))?)
}

// admin auth
fn is_admin(req: &Request<Body>) -> bool {
    if let Some(auth) = req.headers().get(hyper::header::AUTHORIZATION) {
        let auth_header_value = auth.to_str().unwrap().to_string();
        if let Ok(credentials) = Credentials::from_header(auth_header_value) {
            if let Some(cfg) = G_CONFIG.get() {
                return cfg.admin_auth(&credentials.user_id, &credentials.password);
            }
        }
    }
    false
}

fn init_jinja_tpl() -> Result<()> {
    let detail_data = Asset::get("/jinja/detail.jinja.html").expect("detail.jinja.html not found");
    let detail_html: String = String::from_utf8(detail_data.data.try_into()?).unwrap();
    jinja::add_template("main", "detail", detail_html);

    let map_data = Asset::get("/jinja/map.jinja.html").expect("map.jinja.html not found");
    let map_html: String = String::from_utf8(map_data.data.try_into()?).unwrap();
    jinja::add_template("main", "map", map_html);

    let detail_ht_data =
        Asset::get("/jinja/detail_ht.jinja.html").expect("detail_ht.jinja.html not found");
    let detail_ht_html: String = String::from_utf8(detail_ht_data.data.try_into()?).unwrap();
    jinja::add_template("main", "detail_ht", detail_ht_html);

    Ok(())
}

//
async fn render_jinja_ht_tpl(tag: &'static str, req: Request<Body>) -> Result<Response<Body>> {
    if !is_admin(&req) {
        return Ok(Response::builder()
            .header(header::WWW_AUTHENTICATE, "Basic realm=\"Restricted\"")
            .status(StatusCode::UNAUTHORIZED)
            .body(UNAUTHORIZED.into())?);
    }

    // skip_serializing
    let resp = G_STATS_MGR.get().unwrap().get_stats();
    let o = resp.lock().unwrap();
    let mut sys_info_list = Vec::new();
    let mut ip_info_list = Vec::new();
    for stat in &*o.servers {
        ip_info_list.push(stat.ip_info.as_ref());
        sys_info_list.push(stat.sys_info.as_ref());
    }

    Ok(jinja::render_template(
        "main",
        tag,
        context!(resp => &*o, ip_info_list => ip_info_list, sys_info_list => sys_info_list),
    )
    .map(|contents| {
        Response::builder()
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Body::from(contents))
    })?
    .unwrap_or(
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(INTERNAL_SERVER_ERROR.into())?,
    ))
}

use prettytable::Table;
async fn get_detail(req: Request<Body>) -> Result<Response<Body>> {
    if !is_admin(&req) {
        return Ok(Response::builder()
            .header(header::WWW_AUTHENTICATE, "Basic realm=\"Restricted\"")
            .status(StatusCode::UNAUTHORIZED)
            .body(UNAUTHORIZED.into())?);
    }

    let resp = G_STATS_MGR.get().unwrap().get_stats();
    let o = resp.lock().unwrap();

    let mut table = Table::new();
    table.set_titles(row![
        "#",
        "Id",
        "节点名",
        "位置",
        "在线时间",
        "IP",
        "系统信息",
        "IP信息"
    ]);
    for (idx, host) in o.servers.iter().enumerate() {
        let sys_info = host
            .sys_info
            .as_ref()
            .map(|o| {
                let mut s = String::new();
                s.push_str(format!("version:        {}\n", o.version).as_str());
                s.push_str(format!("host_name:      {}\n", o.host_name).as_str());
                s.push_str(format!("os_name:        {}\n", o.os_name).as_str());
                s.push_str(format!("os_arch:        {}\n", o.os_arch).as_str());
                s.push_str(format!("os_family:      {}\n", o.os_family).as_str());
                s.push_str(format!("os_release:     {}\n", o.os_release).as_str());
                s.push_str(format!("kernel_version: {}\n", o.kernel_version).as_str());
                s.push_str(format!("cpu_num:        {}\n", o.cpu_num).as_str());
                s.push_str(format!("cpu_brand:      {}\n", o.cpu_brand).as_str());
                s.push_str(format!("cpu_vender_id:  {}", o.cpu_vender_id).as_str());
                s
            })
            .unwrap_or_default();
        if let Some(ip_info) = &host.ip_info {
            let addrs = vec![
                ip_info.continent.as_str(),
                ip_info.country.as_str(),
                ip_info.region_name.as_str(),
                ip_info.city.as_str(),
            ]
            .iter()
            .map(|s| s.trim())
            .filter(|&s| !s.is_empty())
            .collect::<Vec<&str>>()
            .join("/");

            let isp = vec![
                ip_info.isp.as_str(),
                ip_info.org.as_str(),
                ip_info.r#as.as_str(),
                ip_info.asname.as_str(),
            ]
            .iter()
            .map(|s| s.trim())
            .filter(|&s| !s.is_empty())
            .collect::<Vec<&str>>()
            .join("\n");

            table.add_row(row![
                idx.to_string(),
                host.name,
                host.alias,
                host.location,
                host.uptime_str,
                ip_info.query,
                sys_info,
                format!("{}\n{}", addrs, isp)
            ]);
        } else {
            table.add_row(row![
                idx.to_string(),
                host.name,
                host.alias,
                host.location,
                host.uptime_str,
                "xx.xx.xx.xx".to_string(),
                sys_info,
                "".to_string()
            ]);
        }
    }
    // table.printstd();

    Ok(jinja::render_template(
        "main",
        "detail",
        context!(pretty_content => table.to_string()),
    )
    .map(|contents| {
        Response::builder()
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Body::from(contents))
    })?
    .unwrap_or(
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(INTERNAL_SERVER_ERROR.into())?,
    ))
}

async fn main_service_func(req: Request<Body>) -> Result<Response<Body>> {
    let req_path = req.uri().path();
    match (req.method(), req_path) {
        (&Method::POST, "/report") => stats_report(req).await,
        (&Method::GET, "/json/stats.json") => get_stats_json().await,
        (&Method::GET, "/detail") => get_detail(req).await,
        (&Method::GET, "/detail_ht") => render_jinja_ht_tpl("detail_ht", req).await,
        (&Method::GET, "/map") => render_jinja_ht_tpl("map", req).await,
        (&Method::GET, "/") | (&Method::GET, "/index.html") => {
            let body = Body::from(Asset::get("/index.html").unwrap().data);
            Ok(Response::builder()
                .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                .body(body)?)
        }
        _ => {
            if req.method() == Method::GET
                && (req_path.starts_with("/js/")
                    || req_path.starts_with("/css/")
                    || req_path.starts_with("/img/")
                    || req_path.eq("/favicon.ico"))
            {
                if let Some(data) = Asset::get(req_path) {
                    let ct = mime_guess::from_path(req_path);
                    return Ok(Response::builder()
                        .header(header::CONTENT_TYPE, ct.first_raw().unwrap())
                        .body(Body::from(data.data))?);
                } else {
                    error!("can't get => {:?}", req_path);
                }
            }

            // Return 404 not found response.
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(NOTFOUND.into())?)
        }
    }
}

async fn shutdown_signal() {
    // Wait for the CTRL+C signal
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();
    let args = Args::parse();

    // config test
    if args.config_test {
        config::test_from_file(&args.config).unwrap();
        eprintln!("✨ the conf file {} syntax is ok", &args.config);
        eprintln!("✨ the conf file {} test is successful", &args.config);
        process::exit(0);
    }

    // config load
    if let Some(cfg) = if args.cloud {
        // export SRV_CONF=$(cat config.toml)
        // echo "$SRV_CONF"
        eprintln!("✨ run in cloud mode, load config from env");
        config::from_env()
    } else {
        eprintln!(
            "✨ run in normal mode, load conf from local file `{}",
            &args.config
        );
        config::from_file(&args.config)
    } {
        debug!("{:?}", cfg);
        G_CONFIG.set(cfg).unwrap();
    } else {
        error!("can't parse config");
        process::exit(1);
    }

    // init tpl
    init_jinja_tpl().unwrap();

    // init notifier
    *notifier::NOTIFIER_HANDLE.lock().unwrap() = Some(Handle::current());
    let cfg = G_CONFIG.get().unwrap();
    let notifies: Arc<Mutex<Vec<Box<dyn notifier::Notifier + Send>>>> =
        Arc::new(Mutex::new(Vec::new()));
    if cfg.tgbot.enabled {
        let o = Box::new(notifier::tgbot::TGBot::new(&cfg.tgbot));
        notifies.lock().unwrap().push(o);
    }
    if cfg.wechat.enabled {
        let o = Box::new(notifier::wechat::WeChat::new(&cfg.wechat));
        notifies.lock().unwrap().push(o);
    }
    if cfg.email.enabled {
        let o = Box::new(notifier::email::Email::new(&cfg.email));
        notifies.lock().unwrap().push(o);
    }
    // init notifier end

    // notify test
    if args.notify_test {
        for notifier in &*notifies.lock().unwrap() {
            eprintln!("send test message to {}", notifier.kind());
            notifier.notify_test().unwrap();
        }
        thread::sleep(Duration::from_millis(7000)); // TODO: wait
        eprintln!("Please check for notifications");
        process::exit(0);
    }

    // init mgr
    let mut mgr = crate::stats::StatsMgr::new();
    mgr.init(G_CONFIG.get().unwrap(), notifies)?;
    if G_STATS_MGR.set(mgr).is_err() {
        error!("can't set G_STATS_MGR");
        process::exit(1);
    }

    // serv grpc
    tokio::spawn(async move {
        let addr = &*G_CONFIG.get().unwrap().grpc_addr;
        grpc::serv_grpc(addr).await
    });

    // serv http
    let http_service =
        make_service_fn(|_| async { Ok::<_, GenericError>(service_fn(main_service_func)) });

    let http_addr = G_CONFIG.get().unwrap().http_addr.parse()?;
    eprintln!("🚀 listening on http://{}", http_addr);
    let server = Server::bind(&http_addr).serve(http_service);
    let graceful = server.with_graceful_shutdown(shutdown_signal());
    if let Err(e) = graceful.await {
        eprintln!("server error: {}", e);
    }

    Ok(())
}
