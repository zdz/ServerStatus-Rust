// #![allow(unused)]
use std::net::ToSocketAddrs;
use std::thread;
use std::time::Duration;
use tonic::transport::Channel;
use tonic::{metadata::MetadataValue, Request};
use tower::timeout::Timeout;

use stat_common::server_status::server_status_client::ServerStatusClient;
use stat_common::server_status::StatRequest;

use crate::sample_all;
use crate::Args;
use crate::INTERVAL_MS;

// TODO TLS

pub async fn report(args: &Args, stat_base: &mut StatRequest) -> anyhow::Result<()> {
    if !vec![stat_base.online4, stat_base.online6]
        .iter()
        .any(|&x| x)
    {
        eprintln!("try get target network...");
        let addr = args.addr.replace("grpc://", "");
        let sock_addr = addr.to_socket_addrs()?.next().unwrap();

        stat_base.online4 = sock_addr.is_ipv4();
        stat_base.online6 = sock_addr.is_ipv6();

        eprintln!(
            "get target network (ipv4, ipv6) => ({}, {})",
            stat_base.online4, stat_base.online6
        );
    }

    let token = MetadataValue::try_from(format!("{}@_@{}", args.user, args.pass))?;

    let channel = Channel::from_shared(args.addr.to_string())?
        .connect()
        .await?;
    let timeout_channel = Timeout::new(channel, Duration::from_millis(3000));

    let grpc_client =
        ServerStatusClient::with_interceptor(timeout_channel, move |mut req: Request<()>| {
            req.metadata_mut().insert("authorization", token.clone());
            Ok(req)
        });

    loop {
        let stat_rt = sample_all(args, stat_base);
        let mut client = grpc_client.clone();
        tokio::spawn(async move {
            let request = tonic::Request::new(stat_rt);

            match client.report(request).await {
                Ok(resp) => {
                    info!("grpc report resp => {:?}", resp);
                }
                Err(status) => {
                    error!("grpc report status => {:?}", status);
                }
            }
        });

        thread::sleep(Duration::from_millis(INTERVAL_MS));
    }
}
