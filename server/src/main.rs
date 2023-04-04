#![deny(warnings)]
// #![allow(unused)]
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
#[macro_use]
extern crate prettytable;

use clap::Parser;
use once_cell::sync::OnceCell;
use std::process;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::signal;

use axum::{
    http::{Method, Uri},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};

mod assets;
mod auth;
mod config;
mod grpc;
mod http;
mod jinja;
mod jwt;
mod notifier;
mod payload;
mod stats;

static G_CONFIG: OnceCell<crate::config::Config> = OnceCell::new();
static G_STATS_MGR: OnceCell<crate::stats::StatsMgr> = OnceCell::new();

#[derive(Parser, Debug)]
#[command(author, version = env!("APP_VERSION"), about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "config.toml")]
    config: String,
    #[arg(short = 't', long, help = "config test, default:false")]
    config_test: bool,
    #[arg(long = "notify-test", help = "notify test, default:false")]
    notify_test: bool,
    #[arg(long = "cloud", help = "cloud mode, load cfg from env var: SRV_CONF")]
    cloud: bool,
}

fn create_app_router() -> Router {
    let cors_layer = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST]);

    Router::new()
        .route("/report", post(http::report))
        .route("/json/stats.json", get(http::get_stats_json)) // 兼容就旧主题
        // .route("/config.pub.json", get(http::get_site_config_json)) // TODO
        .route("/api/admin/authorize", post(jwt::authorize))
        .route("/api/admin/:path", get(http::admin_api)) // stats.json || config.json
        // .route("/admin", get(assets::admin_index_handler))
        .route("/detail", get(http::get_detail))
        .route("/map", get(http::get_map))
        .route("/i", get(http::init_client))
        .route("/", get(assets::index_handler))
        .fallback(fallback)
        .layer(cors_layer)
}

async fn fallback(uri: Uri) -> impl IntoResponse {
    assets::static_handler(uri).await
}

pub async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("signal received, starting graceful shutdown");
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
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

    let http_addr = G_CONFIG.get().unwrap().http_addr.to_string();
    eprintln!("🚀 listening on http://{http_addr}");

    axum::Server::bind(&http_addr.parse().unwrap())
        .serve(create_app_router().into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    Ok(())
}
