use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::geoip::do_http_req;
use crate::Args;
use stat_common::server_status::IpInfo;

const SOURCE: &str = "myip.la";
const IP_API_URL: &str = "https://api.myip.la/en?json";

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
struct Location {
    pub city: String,
    pub country_code: String,
    pub country_name: String,
    pub latitude: String,
    pub longitude: String,
    pub province: String,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
struct ApiResp {
    pub ip: String,
    pub location: Location,
}

impl From<ApiResp> for IpInfo {
    fn from(resp: ApiResp) -> Self {
        IpInfo {
            source: SOURCE.to_string(),
            query: resp.ip,

            // continent: resp.continent.to_string(),
            country: resp.location.country_name,
            region_name: resp.location.province,
            city: resp.location.city,

            lat: resp.location.latitude.parse().unwrap_or_default(),
            lon: resp.location.longitude.parse().unwrap_or_default(),

            ..Default::default()
        }
    }
}

pub async fn get_ip_info(args: &Args) -> Result<IpInfo> {
    do_http_req::<ApiResp>(IP_API_URL, args).await
}
