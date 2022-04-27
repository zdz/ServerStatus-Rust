// #![allow(unused)]
use tonic::{transport::Server, Request, Response, Status};

use stat_common::server_status;
use stat_common::server_status::server_status_server::{ServerStatus, ServerStatusServer};
use stat_common::server_status::StatRequest;

use crate::G_CONFIG;
use crate::G_STATS_MGR;

#[derive(Default)]
pub struct ServerStatusSrv {}

#[tonic::async_trait]
impl ServerStatus for ServerStatusSrv {
    async fn report(
        &self,
        request: Request<StatRequest>,
    ) -> Result<Response<server_status::Response>, Status> {
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
    match req.metadata().get("authorization") {
        Some(token) => {
            let tuple = token
                .to_str()
                .unwrap_or("")
                .split("@_@")
                .collect::<Vec<_>>();

            if tuple.len() == 2 {
                if let Some(mgr) = G_CONFIG.get() {
                    if mgr.auth(tuple[0], tuple[1]) {
                        return Ok(req);
                    }
                }
            }

            Err(Status::unauthenticated("invalid user && pass"))
        }

        _ => Err(Status::unauthenticated("invalid user && pass")),
    }
}

pub async fn serv_grpc(addr: &str) -> anyhow::Result<()> {
    let sock_addr = addr.parse().unwrap();
    let sss = ServerStatusSrv::default();
    eprintln!("🚀 listening on grpc://{}", sock_addr);
    let svc = ServerStatusServer::with_interceptor(sss, check_auth);
    Server::builder()
        .add_service(svc)
        .serve(sock_addr)
        .await
        .map_err(anyhow::Error::new)
}
