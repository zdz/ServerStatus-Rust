#![deny(warnings)]
use anyhow::Result;
use log::{error, info, trace};
use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::time::Duration;

use crate::notifier;
use crate::notifier::Event;
use crate::notifier::HostStat;
use crate::notifier::Notifier;
use crate::notifier::NOTIFIER_HANDLE;

const KIND: &str = "tgbot";

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Config {
    pub enabled: bool,
    pub bot_token: String,
    pub chat_id: String,
    pub custom_tpl: String,
}

pub struct TGBot {
    config: &'static Config,
    tg_url: String,
    http_client: reqwest::Client,
}

impl TGBot {
    pub fn new(cfg: &'static Config) -> Self {
        let o = Self {
            config: cfg,
            tg_url: format!("https://api.telegram.org/bot{}/sendMessage", &cfg.bot_token),
            http_client: reqwest::Client::new(),
        };

        notifier::add_template(KIND, o.config.custom_tpl.as_str()).unwrap();
        o
    }

    fn custom_notify(&self, stat: &HostStat) -> Result<()> {
        trace!("{} custom_notify => {:?}", self.kind(), stat);

        notifier::render_template(KIND, stat).map(|content| {
            info!("tmpl.render => {}", content);
            if !content.is_empty() {
                self.send_msg(format!("❗<b>Server Status</b>\n{}", content))
                    .unwrap_or_else(|err| {
                        error!("send_msg err => {:?}", err);
                    });
            }
        })
    }

    fn send_msg(&self, html_content: String) -> Result<()> {
        let mut data = HashMap::new();
        data.insert("chat_id", self.config.chat_id.to_string());
        data.insert("parse_mode", "HTML".to_string());
        data.insert("text", html_content);

        let tg_url = self.tg_url.to_string();
        let handle = NOTIFIER_HANDLE.lock().unwrap().as_ref().unwrap().clone();
        let http_client = self.http_client.clone();
        handle.spawn(async move {
            match http_client
                .post(&tg_url)
                .timeout(Duration::from_secs(5))
                .json(&data)
                .send()
                .await
            {
                Ok(resp) => {
                    info!("tg send msg resp => {:?}", resp);
                }
                Err(err) => {
                    error!("tg send msg error => {:?}", err);
                }
            }
        });

        Ok(())
    }
}

impl crate::notifier::Notifier for TGBot {
    fn kind(&self) -> &'static str {
        KIND
    }

    fn notify(&self, e: &Event, stat: &HostStat) -> Result<()> {
        trace!("{} notify {:?} => {:?}", self.kind(), e, stat);
        match *e {
            Event::NodeUp => {
                let content = format!("❗<b>Server Status</b>\n❗ {} 主机上线 🟢", stat.name);
                let _ = self.send_msg(content);
            }
            Event::NodeDown => {
                let content = format!("❗<b>Server Status</b>\n❗ {} 主机下线 🔴", stat.name);
                let _ = self.send_msg(content);
            }
            Event::Custom => {
                let _ = self.custom_notify(stat);
            }
        }

        Ok(())
    }
}
