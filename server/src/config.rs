#![deny(warnings)]
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;

use crate::notifier;

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
    #[serde(default = "bool::default")]
    pub disable_notify: bool,
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
    pub http_addr: String,
    pub tcp_addr: String,
    #[serde(default = "Default::default")]
    pub notify_interval: u64,
    #[serde(default = "Default::default")]
    pub offline_threshold: u64,
    // pub admin_user: String,
    // pub admin_pass: String,
    // #[serde(default = "bool::default")]
    // pub vnstat: bool,
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
