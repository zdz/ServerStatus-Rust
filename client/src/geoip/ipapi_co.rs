use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::geoip::do_http_req;
use crate::Args;
use stat_common::server_status::IpInfo;

const SOURCE: &str = "ipapi.co";
const IP_API_URL: &str = "https://ipapi.co/json";

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
struct ApiResp {
    pub ip: String,

    pub continent_code: String,
    // pub country: String,
    pub country_name: String,
    pub region: String,
    pub city: String,

    pub org: String,
    pub asn: String,

    pub latitude: f64,
    pub longitude: f64,

    pub timezone: String,
    // pub region_code: String,
    // pub in_eu: bool,
    // pub postal: String,
    // pub utc_offset: String,
    // pub country_calling_code: String,
    // pub currency: String,
    // pub languages: String,
}

impl From<ApiResp> for IpInfo {
    fn from(resp: ApiResp) -> Self {
        IpInfo {
            source: SOURCE.to_string(),
            query: resp.ip,

            continent: resp.continent_code,
            country: resp.country_name,
            region_name: resp.region,
            city: resp.city,

            // isp: resp.org,
            org: resp.org,
            r#as: resp.asn,
            // asname: resp.asn,
            lat: resp.latitude,
            lon: resp.longitude,

            timezone: resp.timezone,

            ..Default::default()
        }
    }
}

pub async fn get_ip_info(args: &Args) -> Result<IpInfo> {
    do_http_req::<ApiResp>(IP_API_URL, args).await
}
