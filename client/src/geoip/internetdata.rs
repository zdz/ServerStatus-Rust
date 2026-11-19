use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::geoip::do_http_req;
use crate::Args;
use stat_common::server_status::IpInfo;

const SOURCE: &str = "internetdata.io";
const IP_API_URL: &str = "https://api.internetdata.io/myip";

// region, city and the coordinates come with a paid API key only
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
struct Location {
    pub continent_name: String,
    pub country_name: String,
    pub region: String,
    pub city: String,
    pub latitude: f64,
    pub longitude: f64,
    pub timezone: String,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
struct Asn {
    pub asn: i64,
    pub name: String,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
struct Company {
    pub name: String,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
struct ApiResp {
    pub ip: String,
    pub location: Location,
    pub asn: Asn,
    pub company: Company,
}

impl From<ApiResp> for IpInfo {
    fn from(resp: ApiResp) -> Self {
        IpInfo {
            source: SOURCE.to_string(),
            query: resp.ip,

            continent: resp.location.continent_name,
            country: resp.location.country_name,
            region_name: resp.location.region,
            city: resp.location.city,

            org: resp.company.name,
            r#as: resp.asn.asn.to_string(),
            asname: resp.asn.name,

            lat: resp.location.latitude,
            lon: resp.location.longitude,

            timezone: resp.location.timezone,

            ..Default::default()
        }
    }
}

pub async fn get_ip_info(args: &Args) -> Result<IpInfo> {
    do_http_req::<ApiResp>(IP_API_URL, args).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_keyless_answer() {
        let body = r#"{"ip":"8.8.8.8","is_anycast":true,"location":{"continent":"NA","continent_name":"North America","country":"US","country_name":"United States","timezone":"America/Los_Angeles"},"asn":{"asn":15169,"name":"Google LLC","domain":"google.com","type":"hosting","prefix":"8.8.8.0/24"},"company":{"name":"Google LLC","domain":"google.com"},"hostname":"dns.google"}"#;
        let info: IpInfo = serde_json::from_str::<ApiResp>(body).unwrap().into();
        assert_eq!(info.query, "8.8.8.8");
        assert_eq!(info.country, "United States");
        assert_eq!(info.r#as, "15169");
        assert_eq!(info.asname, "Google LLC");
        assert_eq!(info.city, "");
    }
}
