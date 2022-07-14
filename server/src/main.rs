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
mod http;
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

static G_CONFIG: OnceCell<crate::config::Config> = OnceCell::new();
static G_STATS_MGR: OnceCell<crate::stats::StatsMgr> = OnceCell::new();

#[derive(RustEmbed)]
#[folder = "../web"]
#[prefix = "/"]
struct Asset;

#[derive(Parser, Debug)]
#[clap(author, version = env!("APP_VERSION"), about, long_about = None)]
struct Args {
    #[clap(short, long, value_parser, default_value = "config.toml")]
    config: String,
    #[clap(short = 't', long, value_parser, help = "config test, default:false")]
    config_test: bool,
    #[clap(long = "notify-test", value_parser, help = "notify test, default:false")]
    notify_test: bool,
    #[clap(long = "cloud", value_parser, help = "cloud mode, load cfg from env var: SRV_CONF")]
    cloud: bool,
}

// stat report
async fn stats_report(req: Request<Body>) -> Result<Response<Body>> {
    let req_header = req.headers();
    // auth
    let mut auth_ok = false;
    let mut group_auth = false;
    if let Some(ssr_auth) = req_header.get("ssr-auth") {
        group_auth = "group".eq(ssr_auth);
    }

    if let Some(auth) = req_header.get(hyper::header::AUTHORIZATION) {
        let auth_header_value = auth.to_str()?.to_string();
        if let Ok(credentials) = Credentials::from_header(auth_header_value) {
            if let Some(cfg) = G_CONFIG.get() {
                if group_auth {
                    auth_ok = cfg.group_auth(&credentials.user_id, &credentials.password);
                } else {
                    auth_ok = cfg.auth(&credentials.user_id, &credentials.password);
                }
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
    if let Ok(content_type) = req_header.get(hyper::header::CONTENT_TYPE).unwrap().clone().to_str() {
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

async fn main_service_func(req: Request<Body>) -> Result<Response<Body>> {
    let req_path = req.uri().path();
    match (req.method(), req_path) {
        (&Method::POST, "/report") => stats_report(req).await,
        (&Method::GET, "/json/stats.json") => get_stats_json().await,
        (&Method::GET, "/detail") => http::get_detail(req).await,
        (&Method::GET, "/detail_ht") => http::render_jinja_ht_tpl("detail_ht", req).await,
        (&Method::GET, "/map") => http::render_jinja_ht_tpl("map", req).await,
        (&Method::GET, "/i") => http::init_client(req).await,
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

    eprintln!("✨ {} {}", env!("CARGO_BIN_NAME"), env!("APP_VERSION"));

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
        eprintln!("✨ run in normal mode, load conf from local file `{}", &args.config);
        config::from_file(&args.config)
    } {
        debug!("{}", serde_json::to_string_pretty(&cfg).unwrap());
        G_CONFIG.set(cfg).unwrap();
    } else {
        error!("can't parse config");
        process::exit(1);
    }

    // init tpl
    http::init_jinja_tpl().unwrap();

    // init notifier
    *notifier::NOTIFIER_HANDLE.lock().unwrap() = Some(Handle::current());
    let cfg = G_CONFIG.get().unwrap();
    let notifies: Arc<Mutex<Vec<Box<dyn notifier::Notifier + Send>>>> = Arc::new(Mutex::new(Vec::new()));
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
    if cfg.log.enabled {
        let o = Box::new(notifier::log::Log::new(&cfg.log));
        notifies.lock().unwrap().push(o);
    }
    if cfg.webhook.enabled {
        let o = Box::new(notifier::webhook::Webhook::new(&cfg.webhook));
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
    let http_service = make_service_fn(|_| async { Ok::<_, GenericError>(service_fn(main_service_func)) });

    let http_addr = G_CONFIG.get().unwrap().http_addr.parse()?;
    eprintln!("🚀 listening on http://{}", http_addr);
    let server = Server::bind(&http_addr).serve(http_service);
    let graceful = server.with_graceful_shutdown(shutdown_signal());
    if let Err(e) = graceful.await {
        eprintln!("server error: {}", e);
    }

    Ok(())
}
