use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::geoip::do_http_req;
use crate::Args;
use stat_common::server_status::IpInfo;

const SOURCE: &str = "ip-api.com";

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct IpApiResp {
    pub query: String,

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

    pub isp: String,
    pub org: String,
    pub r#as: String,
    pub asname: String,

    pub lat: f64,
    pub lon: f64,

    // pub district: String,
    // pub zip: String,
    pub timezone: String,
    // pub status: String,
}

impl From<IpApiResp> for IpInfo {
    fn from(resp: IpApiResp) -> Self {
        IpInfo {
            source: SOURCE.to_string(),
            query: resp.query.to_string(),

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

            timezone: resp.timezone,
        }
    }
}

const IP_API_URL:&str = "http://ip-api.com/json?fields=status,message,continent,continentCode,country,countryCode,region,regionName,city,district,zip,lat,lon,timezone,isp,org,as,asname,query";

pub async fn get_ip_info(args: &Args) -> Result<IpInfo> {
    let mut ip_api_url = IP_API_URL;
    if args.ipv6 {
        // ipv6 only: forward to ip-api.com
        ip_api_url = "https://ip.zdz.workers.dev";
    }

    do_http_req::<IpApiResp>(ip_api_url, args).await
}
