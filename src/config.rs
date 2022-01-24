#![deny(warnings)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

#[derive(Debug, Deserialize, Serialize)]
pub struct TGBot {
    #[serde(default = "bool::default")]
    pub enabled: bool,
    pub bot_token: String,
    pub chat_id: String,
    pub custom_tpl: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WeChat {
    #[serde(default = "bool::default")]
    pub enabled: bool,
    pub corp_id: String,
    pub corp_secret: String,
    pub agent_id: String,
    pub custom_tpl: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Host {
    pub name: String,
    pub host: String,
    pub location: String,
    // pub username: String,
    pub password: String,
    #[serde(rename = "type")]
    pub host_type: String,
    pub monthstart: u32,
    #[serde(default = "bool::default")]
    pub disabled: bool,

    #[serde(skip_deserializing)]
    pub last_network_in: u64,
    #[serde(skip_deserializing)]
    pub last_network_out: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub addr: String,
    pub log_level: String,
    pub admin_user: String,
    pub admin_pass: String,
    pub tgbot: TGBot,
    pub wechat: WeChat,
    pub hosts: Vec<Host>,

    #[serde(skip_deserializing)]
    auth_map: HashMap<String, String>,
}

impl Config {
    pub fn auth(&self, user: &str, pass: &str) -> bool {
        if let Some(o) = self.auth_map.get(user) {
            return pass.eq(o);
        }
        return false;
    }
}

pub fn parse_config(cfg: &String) -> Config {
    let file = File::open(cfg).unwrap();
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    buf_reader.read_to_string(&mut contents).unwrap();

    let mut o: Config = toml::from_str(&contents).unwrap();

    o.auth_map = HashMap::new();
    for host in &o.hosts {
        o.auth_map
            .insert(String::from(&host.name), String::from(&host.password));
    }

    o
}
