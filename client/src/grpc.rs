// #![allow(unused)]
use std::str::FromStr;
use std::thread;
use std::time::Duration;

use tonic::{metadata::MetadataValue, Request};
use tower::timeout::Timeout;
use url::Url;

use stat_common::server_status::server_status_client::ServerStatusClient;
use stat_common::server_status::StatRequest;
use tonic::transport::Channel;
use tonic::transport::{ClientTlsConfig,Identity,Certificate};
use crate::sample_all;
use crate::Args;


pub async fn report(args: &Args, stat_base: &mut StatRequest) -> anyhow::Result<()> {
    let auth_user: String;
    let ssr_auth: &[u8];
    if args.gid.is_empty() {
        auth_user = args.user.clone();
        ssr_auth = b"single";
    } else {
        auth_user = args.gid.clone();
        ssr_auth = b"group";
    }
    let token = MetadataValue::try_from(format!("{}@_@{}", auth_user, args.pass))?;

    let addr = args.addr.replace("grpcs://", "https://");
    let channel: Channel;
    if args.mtls {
        // === mTLS 模式 ===
        let u = Url::parse(&addr)?;
        let domain_name = u.host_str().ok_or_else(|| anyhow::anyhow!("invalid URL: missing host"))?;

        let tls_dir = std::path::PathBuf::from_str(&args.tls_dir)?;
        let ca_pem = std::fs::read(tls_dir.join("ca.pem"))?;
        let client_cert_pem = std::fs::read(tls_dir.join("client.pem"))?;
        let client_key_pem = std::fs::read(tls_dir.join("client.key"))?;
        let client_identity = Identity::from_pem(client_cert_pem, client_key_pem);
        let ca = Certificate::from_pem(ca_pem);

        let tls_config = ClientTlsConfig::new()
            .domain_name(domain_name)
            .ca_certificate(ca)
            .identity(client_identity);
            
        channel = Channel::from_shared(addr)?
            .tls_config(tls_config)?
            .connect().await?;
    } else if addr.starts_with("https://") {
        // TLS
        let tls_config = ClientTlsConfig::new();
        channel = Channel::from_shared(addr)?.tls_config(tls_config)?.connect().await?;
    } else {
        channel = Channel::from_shared(addr)?.connect().await?;
    }

    let timeout_channel = Timeout::new(channel, Duration::from_millis(3000));
    let grpc_client = ServerStatusClient::with_interceptor(timeout_channel, move |mut req: Request<()>| {
        req.metadata_mut().insert("authorization", token.clone());
        req.metadata_mut()
            .insert("ssr-auth", MetadataValue::try_from(ssr_auth).unwrap());
        Ok(req)
    });

    loop {
        let stat_rt = sample_all(args, stat_base);
        let mut client = grpc_client.clone();

        tokio::spawn(async move {
            let request = tonic::Request::new(stat_rt);
            match client.report(request).await {
                Ok(resp) => {
                    info!("grpc report resp => {resp:?}");
                }
                Err(status) => {
                    error!("grpc report status => {status:?}");
                }
            }
        });

        thread::sleep(Duration::from_secs(args.report_interval));
    }
}