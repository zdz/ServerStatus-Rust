use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::geoip::do_http_req;
use crate::Args;
use stat_common::server_status::IpInfo;

const SOURCE: &str = "ip.sb";
const IP_API_URL: &str = "https://api.ip.sb/geoip";

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
struct ApiResp {
    pub ip: String,

    pub continent_code: String,
    pub country: String,
    pub region: String,
    pub city: String,

    pub isp: String,
    pub organization: String,
    pub asn: i64,
    pub asn_organization: String,

    pub latitude: f64,
    pub longitude: f64,

    pub timezone: String,
    // pub offset: i64,
    // pub postal_code: String,
    // pub country_code: String,
    // pub region_code: String,
}

impl From<ApiResp> for IpInfo {
    fn from(resp: ApiResp) -> Self {
        IpInfo {
            source: SOURCE.to_string(),
            query: resp.ip,

            continent: resp.continent_code,
            country: resp.country,
            region_name: resp.region,
            city: resp.city,

            isp: resp.isp.to_string(),
            org: resp.organization.to_string(),
            r#as: resp.asn.to_string(),
            asname: resp.asn_organization.to_string(),

            lat: resp.latitude,
            lon: resp.longitude,

            timezone: resp.timezone,
            // ..Default::default()
        }
    }
}

pub async fn get_ip_info(args: &Args) -> Result<IpInfo> {
    do_http_req::<ApiResp>(IP_API_URL, args).await
}
