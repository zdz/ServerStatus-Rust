#![deny(warnings)]
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use uuid::Uuid;

use crate::notifier;

fn default_as_true() -> bool {
    true
}
fn default_grpc_addr() -> String {
    "0.0.0.0:9394".to_string()
}
fn default_http_addr() -> String {
    "0.0.0.0:8080".to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Host {
    pub name: String,
    pub password: String,
    #[serde(default = "Default::default")]
    pub alias: String,
    pub location: String,
    #[serde(rename = "type")]
    pub host_type: String,
    #[serde(default = "u32::default")]
    pub monthstart: u32,
    #[serde(default = "default_as_true")]
    pub notify: bool,
    #[serde(default = "bool::default")]
    pub disabled: bool,

    #[serde(skip_deserializing)]
    pub last_network_in: u64,
    #[serde(skip_deserializing)]
    pub last_network_out: u64,

    // user data
    #[serde(skip_serializing, skip_deserializing)]
    pub pos: usize,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_http_addr")]
    pub http_addr: String,
    #[serde(default = "default_grpc_addr")]
    pub grpc_addr: String,
    #[serde(default = "Default::default")]
    pub notify_interval: u64,
    #[serde(default = "Default::default")]
    pub offline_threshold: u64,
    // admin user&pass
    pub admin_user: Option<String>,
    pub admin_pass: Option<String>,

    #[serde(default = "Default::default")]
    pub tgbot: notifier::tgbot::Config,
    #[serde(default = "Default::default")]
    pub wechat: notifier::wechat::Config,
    #[serde(default = "Default::default")]
    pub email: notifier::email::Config,
    pub hosts: Vec<Host>,

    #[serde(skip_deserializing)]
    pub hosts_map: HashMap<String, Host>,
}

impl Config {
    pub fn auth(&self, user: &str, pass: &str) -> bool {
        if let Some(o) = self.hosts_map.get(user) {
            return pass.eq(o.password.as_str());
        }
        false
    }
    pub fn admin_auth(&self, user: &str, pass: &str) -> bool {
        if let (Some(u), Some(p)) = (self.admin_user.as_ref(), self.admin_pass.as_ref()) {
            return user.eq(u.as_str()) && pass.eq(p.as_str());
        }
        false
    }
    pub fn get_host(&self, name: &str) -> Option<&Host> {
        self.hosts_map.get(name)
    }
}

pub fn test_from_file(cfg: &str) -> Result<Config> {
    fs::read_to_string(cfg)
        .map(|contents| toml::from_str::<Config>(&contents))
        .unwrap()
        .map_err(anyhow::Error::new)
}

pub fn from_str(content: &str) -> Option<Config> {
    let mut o = toml::from_str::<Config>(content).unwrap();
    o.hosts_map = HashMap::new();

    for (idx, host) in o.hosts.iter_mut().enumerate() {
        host.pos = idx;
        if host.alias.is_empty() {
            host.alias = host.name.to_owned();
        }
        if host.monthstart < 1 || host.monthstart > 31 {
            host.monthstart = 1;
        }
        o.hosts_map.insert(host.name.to_owned(), host.clone());
    }
    if o.notify_interval < 30 {
        o.notify_interval = 30;
    }
    if o.offline_threshold < 30 {
        o.offline_threshold = 30;
    }
    if o.admin_user.is_none() || o.admin_user.as_ref()?.is_empty() {
        o.admin_user = Some("admin".to_string());
    }
    if o.admin_pass.is_none() || o.admin_pass.as_ref()?.is_empty() {
        o.admin_pass = Some(Uuid::new_v4().to_string());
    }

    eprintln!("✨ admin_user: {}", o.admin_user.as_ref()?);
    eprintln!("✨ admin_pass: {}", o.admin_pass.as_ref()?);

    Some(o)
}

pub fn from_env() -> Option<Config> {
    from_str(
        env::var("SRV_CONF")
            .expect("can't load config from env `SRV_CONF")
            .as_str(),
    )
}

pub fn from_file(cfg: &str) -> Option<Config> {
    fs::read_to_string(cfg)
        .map(|contents| from_str(contents.as_str()))
        .ok()?
}
