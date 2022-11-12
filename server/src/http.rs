// #![allow(unused)]
use http_auth_basic::Credentials;
use hyper::{header, Body, Request, Response, StatusCode};
use minijinja::context;
use prettytable::Table;
use std::collections::HashMap;
use std::fmt::Write as _;

use crate::jinja;
use crate::Asset;
use crate::G_CONFIG;
use crate::G_STATS_MGR;

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;

static UNAUTHORIZED: &[u8] = b"Unauthorized";
static INTERNAL_SERVER_ERROR: &[u8] = b"Internal Server Error";
const KIND: &str = "http";

// client auth
pub fn client_auth(req: &Request<Body>) -> bool {
    let req_header = req.headers();
    // auth
    let mut auth_ok = false;
    let mut group_auth = false;
    if let Some(ssr_auth) = req_header.get("ssr-auth") {
        group_auth = "group".eq(ssr_auth);
    }

    if let Some(auth) = req_header.get(hyper::header::AUTHORIZATION) {
        let auth_header_value = auth.to_str().unwrap().to_string();
        if let Ok(credentials) = Credentials::from_header(auth_header_value) {
            if let Some(cfg) = G_CONFIG.get() {
                if group_auth {
                    auth_ok = cfg.group_auth(&credentials.user_id, &credentials.password);
                } else {
                    auth_ok = cfg.auth(&credentials.user_id, &credentials.password);
                }
            }
        }
    }
    auth_ok
}

// admin auth
pub fn admin_auth(req: &Request<Body>) -> bool {
    if let Some(auth) = req.headers().get(hyper::header::AUTHORIZATION) {
        let auth_header_value = auth.to_str().unwrap().to_string();
        if let Ok(credentials) = Credentials::from_header(auth_header_value) {
            if let Some(cfg) = G_CONFIG.get() {
                return cfg.admin_auth(&credentials.user_id, &credentials.password);
            }
        }
    }
    false
}

pub async fn get_admin_stats_json(req: Request<Body>) -> Result<Response<Body>> {
    if !admin_auth(&req) {
        return Ok(Response::builder()
            .header(header::WWW_AUTHENTICATE, "Basic realm=\"Restricted\"")
            .status(StatusCode::UNAUTHORIZED)
            .body(UNAUTHORIZED.into())?);
    }

    let resp = G_STATS_MGR.get().unwrap().get_all_info()?;

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&resp)?))?)
}

pub async fn init_client(req: Request<Body>) -> Result<Response<Body>> {
    // dbg!(&req);
    let params: HashMap<String, String> = req
        .uri()
        .query()
        .map(|v| url::form_urlencoded::parse(v.as_bytes()).into_owned().collect())
        .unwrap_or_else(HashMap::new);

    // query args
    let invalid = "".to_string();
    let pass = params.get("pass").unwrap_or(&invalid);
    let uid = params.get("uid").unwrap_or(&invalid);
    let gid = params.get("gid").unwrap_or(&invalid);
    let alias = params.get("alias").unwrap_or(&invalid);

    if pass.is_empty() || (uid.is_empty() && gid.is_empty()) || (uid.is_empty() && alias.is_empty()) {
        return Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(StatusCode::BAD_REQUEST.canonical_reason().unwrap().into())?);
    }

    // auth
    let mut auth_ok = false;
    if let Some(cfg) = G_CONFIG.get() {
        if gid.is_empty() {
            auth_ok = cfg.auth(uid, pass)
        } else {
            auth_ok = cfg.group_auth(gid, pass)
        }
    }
    if !auth_ok {
        return Ok(Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(UNAUTHORIZED.into())?);
    }

    let mut domain = "localhost".to_string();
    let mut scheme = "http".to_string();
    let mut server_url = "".to_string();
    let mut workspace = "".to_string();

    // load deploy config
    if let Some(cfg) = G_CONFIG.get() {
        server_url = cfg.server_url.to_string();
        workspace = cfg.workspace.to_string();
    }
    // build server url
    if server_url.is_empty() {
        let req_header = req.headers();

        if let Some(v) = req.uri().scheme() {
            scheme = v.to_string();
            debug!("Http Scheme => {}", scheme);
        }
        req_header.get("x-forwarded-proto").map(|v| {
            v.to_str().map(|s| {
                debug!("x-forwarded-proto => {}", s);
                scheme = s.to_string();
            })
        });

        req_header.get("Host").map(|v| {
            v.to_str().map(|host| {
                debug!("Http Host => {}", host);
                domain = host.to_string();
            })
        });
        req_header.get("x-forwarded-host").map(|v| {
            v.to_str().map(|host| {
                debug!("x-forwarded-host => {}", host);
                domain = host.to_string();
            })
        });
        server_url = format!("{}://{}/report", scheme, domain);
    }

    let vnstat = params.get("vnstat").map(|p| p.eq("1")).unwrap_or(false);
    let disable_ping = params.get("ping").map(|p| p.eq("0")).unwrap_or(false);
    let disable_tupd = params.get("tupd").map(|p| p.eq("0")).unwrap_or(false);
    let disable_extra = params.get("extra").map(|p| p.eq("0")).unwrap_or(false);
    let cn = params.get("cn").map(|p| p.eq("1")).unwrap_or(false);
    let weight = params
        .get("weight")
        .map(|p| p.parse::<u64>().unwrap_or(0_u64))
        .unwrap_or(0_u64);
    let vnstat_mr = params
        .get("vnstat-mr")
        .map(|p| p.parse::<u32>().unwrap_or(1_u32))
        .unwrap_or(1_u32);

    let notify = params.get("notify").map(|p| !p.eq("0")).unwrap_or(true);
    let host_type = params.get("type").unwrap_or(&invalid);
    let location = params.get("loc").unwrap_or(&invalid);

    // build client opts
    let mut client_opts = format!(r#"-a "{}" -p "{}""#, server_url, pass);
    if vnstat {
        client_opts.push_str(" -n");
    }
    if 1 < vnstat_mr && vnstat_mr <= 28 {
        let _ = write!(client_opts, r#" --vnstat-mr "{}""#, vnstat_mr);
    }
    if disable_ping {
        client_opts.push_str(" --disable-ping");
    }
    if disable_tupd {
        client_opts.push_str(" --disable-tupd");
    }
    if disable_extra {
        client_opts.push_str(" --disable-extra");
    }
    if weight > 0 {
        let _ = write!(client_opts, r#" -w {}"#, weight);
    }
    if !gid.is_empty() {
        let _ = write!(client_opts, r#" -g "{}""#, gid);
        let _ = write!(client_opts, r#" --alias "{}""#, alias);
    }
    if !uid.is_empty() {
        let _ = write!(client_opts, r#" -u "{}""#, uid);
    }
    if !notify {
        client_opts.push_str(" --disable-notify");
    }
    if !host_type.is_empty() {
        let _ = write!(client_opts, r#" -t "{}""#, host_type);
    }
    if !location.is_empty() {
        let _ = write!(client_opts, r#" --location "{}""#, location);
    }

    Ok(jinja::render_template(
        KIND,
        "client-init",
        context!(
            pass => pass, uid => uid, gid => gid, alias => alias,
            vnstat => vnstat, weight => weight, cn => cn,
            domain => domain, scheme => scheme,
            server_url => server_url, workspace => workspace,
            client_opts => client_opts,
            pkg_version => env!("CARGO_PKG_VERSION"),
        ),
        false,
    )
    .map(|contents| {
        Response::builder()
            .header(header::CONTENT_TYPE, "text/x-sh")
            .header(
                header::CONTENT_DISPOSITION,
                r#"attachment; filename="ssr-client-init.sh""#,
            )
            .body(Body::from(contents))
    })?
    .unwrap_or(
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(INTERNAL_SERVER_ERROR.into())?,
    ))
}

pub fn init_jinja_tpl() -> Result<()> {
    let detail_data = Asset::get("/jinja/detail.jinja.html").expect("detail.jinja.html not found");
    let detail_html: String = String::from_utf8(detail_data.data.try_into()?).unwrap();
    jinja::add_template(KIND, "detail", detail_html);

    let map_data = Asset::get("/jinja/map.jinja.html").expect("map.jinja.html not found");
    let map_html: String = String::from_utf8(map_data.data.try_into()?).unwrap();
    jinja::add_template(KIND, "map", map_html);

    let client_init_sh = Asset::get("/jinja/client-init.jinja.sh").expect("client-init.jinja.sh not found");
    let client_init_sh_s: String = String::from_utf8(client_init_sh.data.try_into()?).unwrap();
    jinja::add_template(KIND, "client-init", client_init_sh_s);
    Ok(())
}

//
pub async fn render_jinja_ht_tpl(tag: &'static str, req: Request<Body>) -> Result<Response<Body>> {
    if !admin_auth(&req) {
        return Ok(Response::builder()
            .header(header::WWW_AUTHENTICATE, "Basic realm=\"Restricted\"")
            .status(StatusCode::UNAUTHORIZED)
            .body(UNAUTHORIZED.into())?);
    }

    let o = G_STATS_MGR.get().unwrap().get_all_info()?;

    Ok(jinja::render_template(KIND, tag, context!(resp => &o), false)
        .map(|contents| {
            Response::builder()
                .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                .body(Body::from(contents))
        })?
        .unwrap_or(
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(INTERNAL_SERVER_ERROR.into())?,
        ))
}

pub async fn get_detail(req: Request<Body>) -> Result<Response<Body>> {
    if !admin_auth(&req) {
        return Ok(Response::builder()
            .header(header::WWW_AUTHENTICATE, "Basic realm=\"Restricted\"")
            .status(StatusCode::UNAUTHORIZED)
            .body(UNAUTHORIZED.into())?);
    }

    let resp = G_STATS_MGR.get().unwrap().get_stats();
    let o = resp.lock().unwrap();

    let mut table = Table::new();
    table.set_titles(row![
        "#",
        "Id",
        "节点名",
        "位置",
        "在线时间",
        "IP",
        "系统信息",
        "IP信息"
    ]);
    for (idx, host) in o.servers.iter().enumerate() {
        let sys_info = host
            .sys_info
            .as_ref()
            .map(|o| {
                let mut s = String::new();
                s.push_str(format!("version:        {}\n", o.version).as_str());
                s.push_str(format!("host_name:      {}\n", o.host_name).as_str());
                s.push_str(format!("os_name:        {}\n", o.os_name).as_str());
                s.push_str(format!("os_arch:        {}\n", o.os_arch).as_str());
                s.push_str(format!("os_family:      {}\n", o.os_family).as_str());
                s.push_str(format!("os_release:     {}\n", o.os_release).as_str());
                s.push_str(format!("kernel_version: {}\n", o.kernel_version).as_str());
                s.push_str(format!("cpu_num:        {}\n", o.cpu_num).as_str());
                s.push_str(format!("cpu_brand:      {}\n", o.cpu_brand).as_str());
                s.push_str(format!("cpu_vender_id:  {}", o.cpu_vender_id).as_str());
                s
            })
            .unwrap_or_default();
        if let Some(ip_info) = &host.ip_info {
            let addrs = vec![
                ip_info.continent.as_str(),
                ip_info.country.as_str(),
                ip_info.region_name.as_str(),
                ip_info.city.as_str(),
            ]
            .iter()
            .map(|s| s.trim())
            .filter(|&s| !s.is_empty())
            .collect::<Vec<&str>>()
            .join("/");

            let isp = vec![
                ip_info.isp.as_str(),
                ip_info.org.as_str(),
                ip_info.r#as.as_str(),
                ip_info.asname.as_str(),
            ]
            .iter()
            .map(|s| s.trim())
            .filter(|&s| !s.is_empty())
            .collect::<Vec<&str>>()
            .join("\n");

            table.add_row(row![
                idx.to_string(),
                host.name,
                host.alias,
                host.location,
                host.uptime_str,
                ip_info.query,
                sys_info,
                format!("{}\n{}", addrs, isp)
            ]);
        } else {
            table.add_row(row![
                idx.to_string(),
                host.name,
                host.alias,
                host.location,
                host.uptime_str,
                "xx.xx.xx.xx".to_string(),
                sys_info,
                "".to_string()
            ]);
        }
    }
    // table.printstd();

    Ok(
        jinja::render_template(KIND, "detail", context!(pretty_content => table.to_string()), true)
            .map(|contents| {
                Response::builder()
                    .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                    .body(Body::from(contents))
            })?
            .unwrap_or(
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(INTERNAL_SERVER_ERROR.into())?,
            ),
    )
}
