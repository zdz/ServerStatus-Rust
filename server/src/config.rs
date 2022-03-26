#![deny(warnings)]
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    pub monthstart: u32,
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
    #[serde(default = "bool::default")]
    pub vnstat: bool,
    #[serde(default = "Default::default")]
    pub tgbot: notifier::tgbot::Config,
    #[serde(default = "Default::default")]
    pub wechat: notifier::wechat::Config,
    #[serde(default = "Default::default")]
    pub email: notifier::email::Config,
    pub hosts: Vec<Host>,

    #[serde(skip_deserializing)]
    auth_map: HashMap<String, String>,
}

impl Config {
    pub fn auth(&self, user: &str, pass: &str) -> bool {
        if let Some(o) = self.auth_map.get(user) {
            return pass.eq(o);
        }
        false
    }
}

pub fn from_file(cfg: &str) -> Option<Config> {
    fs::read_to_string(cfg)
        .map(|contents| {
            let mut o: Config = toml::from_str(&contents).unwrap();
            o.auth_map = HashMap::new();
            for (idx, host) in o.hosts.iter_mut().enumerate() {
                host.pos = idx;
                if host.alias.is_empty() {
                    host.alias = host.name.to_owned();
                }
                o.auth_map
                    .insert(host.name.to_owned(), host.password.to_owned());
            }
            if o.notify_interval < 30 {
                o.notify_interval = 30;
            }
            if o.offline_threshold < 30 {
                o.offline_threshold = 30;
            }

            o
        })
        .ok()?
        .into()
}
