use anyhow::Result;
use serde::Deserialize;
use stat_common::server_status::IpInfo;
use std::time::Duration;

use crate::Args;

mod ip_api_com;
mod ip_sb;
mod ipapi_co;
mod myip_la;

pub const STATIC_AGENTS: & [& str; 10] = &[
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; Trident/7.0; rv:11.0) like Gecko",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:51.0) Gecko/20100101 Firefox/51.0",
    "Mozilla/5.0 (Windows NT 6.1; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/50.0.2676.43 Safari/537.36",
    "Mozilla/5.0 (Android 4.4.1; Tablet; rv:50.0) Gecko/50.0 Firefox/50.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.8; rv:48.0) Gecko/20100101 Firefox/48.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_14_6) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/99.0.4844.74 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_9_4) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/52.0.2760.0 Safari/537.36",
    "Mozilla/5.0 (Android 5.1.1; Tablet; rv:45.0) Gecko/45.0 Firefox/45.0",
    "Mozilla/5.0 (X11; Linux i686 on x86_64; rv:49.0) Gecko/20100101 Firefox/49.0",
    "Mozilla/5.0 (X11; Linux i686) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/52.0.2743.72 Safari/537.36",
];

pub fn random_agent() -> &'static str {
    STATIC_AGENTS[fastrand::usize(..STATIC_AGENTS.len())]
}

pub async fn do_http_req<T>(ip_api_url: &'static str, args: &Args) -> Result<IpInfo>
where
    T: for<'de> Deserialize<'de> + Default + Clone + Send + Sync + 'static + Into<IpInfo>,
{
    let http_client = reqwest::Client::builder()
        .pool_max_idle_per_host(1)
        .connect_timeout(Duration::from_secs(5))
        .user_agent(random_agent())
        .build()?;

    match http_client.get(ip_api_url).send().await {
        Ok(resp) => {
            if args.debug {
                dbg!(&ip_api_url);
                dbg!(&resp);
            }
            resp.json::<T>()
                .await
                .map(|resp| resp.into())
                .map_err(anyhow::Error::new)
        }
        Err(err) => Err(anyhow::Error::new(err)),
    }
}

pub async fn get_ip_info(args: &Args) -> Result<IpInfo> {
    let source = args.ip_source.as_str();
    match source {
        "ip-api.com" => ip_api_com::get_ip_info(args).await,
        "ip.sb" => ip_sb::get_ip_info(args).await,
        "ipapi.co" => ipapi_co::get_ip_info(args).await,
        "myip.la" => myip_la::get_ip_info(args).await,
        _ => ip_sb::get_ip_info(args).await,
    }
}
