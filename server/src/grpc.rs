// #![allow(unused)]
use anyhow::Result;
use std::str::FromStr;
use tonic::{
    transport::{Certificate, Identity, Server, ServerTlsConfig},
    Request, Response, Status,
};

use stat_common::server_status;
use stat_common::server_status::server_status_server::{ServerStatus, ServerStatusServer};
use stat_common::server_status::StatRequest;

use crate::config::Config;
use crate::G_CONFIG;
use crate::G_STATS_MGR;

#[derive(Default)]
pub struct ServerStatusSrv {}

#[tonic::async_trait]
impl ServerStatus for ServerStatusSrv {
    async fn report(&self, request: Request<StatRequest>) -> Result<Response<server_status::Response>, Status> {
        if let Some(mgr) = G_STATS_MGR.get() {
            match serde_json::to_value(request.get_ref()) {
                Ok(v) => {
                    let _ = mgr.report(v);
                }
                Err(err) => {
                    error!("serde_json::to_value err => {:?}", err);
                }
            }
        }

        Ok(Response::new(server_status::Response {
            code: 0,
            message: "ok".to_string(),
        }))
    }
}

fn check_auth(req: Request<()>) -> Result<Request<()>, Status> {
    let mut group_auth = false;
    req.metadata().get("ssr-auth").map(|v| {
        v.to_str().map(|s| {
            group_auth = s.eq("group");
        })
    });

    match req.metadata().get("authorization") {
        Some(token) => {
            let tuple = token.to_str().unwrap_or("").split("@_@").collect::<Vec<_>>();

            if tuple.len() == 2 {
                if let Some(cfg) = G_CONFIG.get() {
                    if group_auth {
                        if cfg.group_auth(tuple[0], tuple[1]) {
                            return Ok(req);
                        }
                    } else if cfg.auth(tuple[0], tuple[1]) {
                        return Ok(req);
                    }
                }
            }

            Err(Status::unauthenticated("invalid user/group && pass"))
        }

        _ => Err(Status::unauthenticated("invalid user/group && pass")),
    }
}

pub async fn serv_grpc(cfg: &Config) -> anyhow::Result<()> {
    let sock_addr = cfg.grpc_addr.parse().unwrap();
    let sss = ServerStatusSrv::default();
    let svc = ServerStatusServer::with_interceptor(sss, check_auth);

    if cfg.grpc_tls > 0 {
        let mut proto = " + TLS";
        let tls_dir = std::path::PathBuf::from_str(&cfg.tls_dir)?;
        let cert = std::fs::read_to_string(tls_dir.join("server.pem"))?;
        let key = std::fs::read_to_string(tls_dir.join("server.key"))?;
        let identity = Identity::from_pem(cert, key);

        let mut tls = ServerTlsConfig::new().identity(identity);
        if cfg.grpc_tls > 1 {
            let ca = Certificate::from_pem(std::fs::read_to_string(tls_dir.join("ca.pem"))?);
            tls = tls.client_ca_root(ca);
            proto = " + mTLS";
        }

        eprintln!("🚀 listening on grpc://{sock_addr}{proto}");
        Server::builder()
            .tls_config(tls)?
            .add_service(svc)
            .serve(sock_addr)
            .await
            .map_err(anyhow::Error::new)
    } else {
        eprintln!("🚀 listening on grpc://{sock_addr}");
        Server::builder()
            .accept_http1(true)
            .add_service(svc)
            .serve(sock_addr)
            .await
            .map_err(anyhow::Error::new)
    }
}
