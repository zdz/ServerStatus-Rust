#![deny(warnings)]
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
use bytes::Buf;
use clap::Parser;
use http_auth_basic::Credentials;
use once_cell::sync::OnceCell;
use rust_embed::RustEmbed;
use std::collections::HashMap;
use std::io::BufRead;
use std::io::BufReader;
use std::process;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

mod config;
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

async fn stats_report(req: Request<Body>) -> Result<Response<Body>> {
    // auth
    let mut auth_ok = false;
    if let Some(auth) = req.headers().get(hyper::header::AUTHORIZATION) {
        let auth_header_value = auth.to_str()?.to_string();
        if let Ok(credentials) = Credentials::from_header(auth_header_value) {
            if G_CONFIG
                .get()
                .unwrap()
                .auth(&credentials.user_id, &credentials.password)
            {
                auth_ok = true;
            }
        }
    }
    if !auth_ok {
        return Ok(Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(UNAUTHORIZED.into())?);
    }
    // auth end

    let whole_body = hyper::body::aggregate(req).await?;
    let json_data: serde_json::Value = serde_json::from_reader(whole_body.reader())?;

    // report
    {
        G_STATS_MGR.get().unwrap().report(json_data)?;
    }

    let mut resp = HashMap::new();
    resp.insert(&"code", serde_json::Value::from(0_i32));
    let resp_str = serde_json::to_string(&resp)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(resp_str))?)
}

async fn get_stats_json() -> Result<Response<Body>> {
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(G_STATS_MGR.get().unwrap().get_stats_json()))?)
}

#[allow(unused)]
async fn handle_admin_cmd(req: Request<Body>) -> Result<Response<Body>> {
    // TODO
    Ok(Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .body(UNAUTHORIZED.into())?)
}

async fn main_service_func(req: Request<Body>) -> Result<Response<Body>> {
    let req_path = req.uri().path();
    match (req.method(), req_path) {
        (&Method::POST, "/report") => stats_report(req).await,
        (&Method::GET, "/json/stats.json") => get_stats_json().await,
        (&Method::POST, "/admin") => handle_admin_cmd(req).await,
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
                    || req_path.starts_with("/img/"))
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

#[derive(Parser, Debug)]
#[clap(author, version = env!("APP_VERSION"), about, long_about = None)]
struct Args {
    #[clap(short, long, default_value = "config.toml")]
    config: String,
}

async fn serv_tcp() -> Result<()> {
    let addr = &*G_CONFIG.get().unwrap().tcp_addr;
    let listener = TcpListener::bind(addr).await?;
    println!("Listening on tcp://{}", addr);

    loop {
        let (mut socket, _) = listener.accept().await?;
        let mut auth_ok = false;
        let peer_addr = socket.peer_addr()?;
        trace!("accept conn {:?}", peer_addr);

        tokio::spawn(async move {
            if socket.write(b"Authentication required\n").await.is_err() {
                return;
            }

            loop {
                let mut buf = vec![0; 1460];
                match socket.read(&mut buf).await {
                    // Return value of `Ok(0)` signifies that the remote has closed
                    Ok(0) => return,
                    Ok(n) => {
                        debug!("read buf size `{}", n);

                        let mut reader = BufReader::new(&*buf);
                        let mut line = String::new();
                        let ln = reader.read_line(&mut line).unwrap();
                        if ln < 1 {
                            continue;
                        }
                        debug!("read line `{}", line);
                        match serde_json::from_str::<serde_json::Value>(&line) {
                            Ok(stat) => {
                                let frame = stat["frame"].as_str().unwrap();
                                // dbg!(&stat);
                                if frame.eq("data") {
                                    if !auth_ok {
                                        return;
                                    }
                                    G_STATS_MGR.get().unwrap().report(stat).unwrap();
                                } else if frame.eq("auth") {
                                    let user = stat["user"].as_str().unwrap();
                                    let pass = stat["pass"].as_str().unwrap();
                                    if !G_CONFIG.get().unwrap().auth(user, pass) {
                                        return;
                                    }
                                    auth_ok = true;
                                    if socket
                                        .write_all(b"Authentication successful. Access granted.")
                                        .await
                                        .is_err()
                                    {
                                        // Unexpected socket error.
                                        return;
                                    }
                                }
                            }
                            Err(e) => {
                                error!("serde_json::from_str err `{:?}", e);
                                error!("invalid data line `{}", line);
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        error!("{:?}", e);
                        // Unexpected socket error.
                        return;
                    }
                }
            }
        });
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();
    let args = Args::parse();

    // config load
    if let Some(cfg) = crate::config::from_file(&args.config) {
        debug!("{:?}", cfg);
        G_CONFIG.set(cfg).unwrap();
    } else {
        error!("can't parse config");
        process::exit(1);
    }

    let cfg = G_CONFIG.get().unwrap();
    let notifier_list: Arc<Mutex<Vec<Box<dyn notifier::Notifier + Send>>>> =
        Arc::new(Mutex::new(Vec::new()));
    // init notifier
    if cfg.tgbot.enabled {
        let o = Box::new(notifier::tgbot::TGBot::new(&cfg.tgbot));
        notifier_list.lock().unwrap().push(o);
    }
    if cfg.wechat.enabled {
        let o = Box::new(notifier::wechat::WeChat::new(&cfg.wechat));
        notifier_list.lock().unwrap().push(o);
    }

    let mut mgr = crate::stats::StatsMgr::new();
    mgr.init(G_CONFIG.get().unwrap(), notifier_list)?;
    if G_STATS_MGR.set(mgr).is_err() {
        error!("can't set G_STATS_MGR");
        process::exit(1);
    }

    tokio::spawn(async move {
        let _ = serv_tcp().await;
    });

    let http_service =
        make_service_fn(|_| async { Ok::<_, GenericError>(service_fn(main_service_func)) });

    let http_addr = G_CONFIG.get().unwrap().http_addr.parse()?;
    println!("Listening on http://{}", http_addr);
    let server = Server::bind(&http_addr).serve(http_service);
    let graceful = server.with_graceful_shutdown(shutdown_signal());
    if let Err(e) = graceful.await {
        eprintln!("server error: {}", e);
    }

    Ok(())
}
