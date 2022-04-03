#![deny(warnings)]
use anyhow::Result;
use log::{error, info};
use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::time::Duration;

use crate::notifier::{add_template, get_tag, render_template, Event, HostStat, NOTIFIER_HANDLE};

const KIND: &str = "tgbot";

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Config {
    pub enabled: bool,
    pub bot_token: String,
    pub chat_id: String,
    pub online_tpl: String,
    pub offline_tpl: String,
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

        add_template(
            KIND,
            get_tag(&Event::NodeUp),
            o.config.online_tpl.to_string(),
        )
        .unwrap();
        add_template(
            KIND,
            get_tag(&Event::NodeDown),
            o.config.offline_tpl.to_string(),
        )
        .unwrap();
        add_template(
            KIND,
            get_tag(&Event::Custom),
            o.config.custom_tpl.to_string(),
        )
        .unwrap();

        o
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

    fn notify_test(&self) -> Result<()> {
        self.send_msg("❗ServerStatus test msg".to_string())
    }

    fn notify(&self, e: &Event, stat: &HostStat) -> Result<()> {
        match *e {
            Event::NodeUp | Event::NodeDown => render_template(KIND, get_tag(e), stat)
                .map(|content| self.send_msg(content))
                .unwrap(),
            Event::Custom => render_template(KIND, get_tag(e), stat).map(|content| {
                info!("tmpl.render => {}", content);
                if !content.is_empty() {
                    self.send_msg(format!("❗<b>Server Status</b>\n{}", content))
                        .unwrap_or_else(|err| {
                            error!("send_msg err => {:?}", err);
                        });
                }
            }),
        }
    }
}
