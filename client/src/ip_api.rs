#![deny(warnings)]
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use stat_common::server_status::IpInfo;

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct IpApiResp {
    pub status: String,
    pub continent: String,
    #[serde(rename = "continentCode")]
    pub continent_code: String,
    pub country: String,
    #[serde(rename = "countryCode")]
    pub country_code: String,
    pub region: String,
    #[serde(rename = "regionName")]
    pub region_name: String,
    pub city: String,
    pub district: String,
    pub zip: String,
    pub lat: f64,
    pub lon: f64,
    pub timezone: String,
    pub isp: String,
    pub org: String,
    pub r#as: String,
    pub asname: String,
    pub query: String,
}

impl From<IpApiResp> for IpInfo {
    fn from(resp: IpApiResp) -> Self {
        IpInfo {
            query: resp.query.to_string(),
            source: "ip-api.com".to_string(),
            continent: resp.continent.to_string(),
            country: resp.country.to_string(),
            region_name: resp.region_name.to_string(),
            city: resp.city.to_string(),
            isp: resp.isp.to_string(),
            org: resp.org.to_string(),
            r#as: resp.r#as.to_string(),
            asname: resp.asname.to_string(),
            lat: resp.lat,
            lon: resp.lon,
        }
    }
}

const IP_API_URL:&str = "http://ip-api.com/json?fields=status,message,continent,continentCode,country,countryCode,region,regionName,city,district,zip,lat,lon,timezone,isp,org,as,asname,query&lang=zh-CN";

pub async fn get_ip_info(ipv6: bool) -> Result<IpInfo> {
    let mut ip_api_url = IP_API_URL;
    if ipv6 {
        // ipv6 only: forward to ip-api.com
        ip_api_url = "https://ip.zdz.workers.dev";
    }

    let http_client = reqwest::Client::builder()
        .pool_max_idle_per_host(1)
        .connect_timeout(Duration::from_secs(5))
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_14_6) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/99.0.4844.74 Safari/537.36")
        .build()?;

    match http_client.get(ip_api_url).send().await {
        Ok(resp) => resp
            .json::<IpApiResp>()
            .await
            .map(|resp| resp.into())
            .map_err(anyhow::Error::new),
        Err(err) => Err(anyhow::Error::new(err)),
    }
}
