#![deny(warnings)]
use reqwest;
use std::collections::HashMap;
use tokio::time::Duration;

use log::{error, info, trace};
use minijinja::{context, Environment};

use crate::notifier::Event;
use crate::notifier::HostStat;
use crate::notifier::Result;
use crate::notifier::NOTIFIER_HANDLE;

pub struct TGBot<'a> {
    pub bot_token: &'static String,
    chat_id: &'static String,
    custom_tpl: &'static String,
    jinja_env: Environment<'a>,
    tg_url: String,
    http_client: reqwest::Client,
}

impl TGBot<'_> {
    pub fn new(cfg: &'static crate::config::TGBot) -> Self {
        let mut o = Self {
            bot_token: &cfg.bot_token,
            chat_id: &cfg.chat_id,
            custom_tpl: &cfg.custom_tpl,
            jinja_env: Environment::new(),
            tg_url: format!("https://api.telegram.org/bot{}/sendMessage", &cfg.bot_token),
            http_client: reqwest::Client::new(),
        };

        o.jinja_env.add_template("tpl", o.custom_tpl).unwrap();
        o
    }

    fn do_custom_notify(&self, stat: &HostStat) -> Result<()> {
        trace!("do_custom_notify => {:?}", stat);
        let tmpl = self.jinja_env.get_template("tpl").unwrap();
        match tmpl.render(context!(host => stat)) {
            Ok(content) => {
                info!("tmpl.render => {}", content);
                let s = content
                    .split('\n')
                    .map(|t| t.trim())
                    .filter(|&t| !t.is_empty())
                    .collect::<Vec<&str>>()
                    .join("\n");
                if !s.is_empty() {
                    let _ = self.send_tg_msg(format!("❗<b>Server Status</b>\n{}", s));
                }
            }
            Err(err) => {
                error!("tmpl.render err => {:?}", err);
            }
        }

        Ok(())
    }

    fn send_tg_msg(&self, html_content: String) -> Result<()> {
        let mut data = HashMap::new();
        data.insert("chat_id", self.chat_id.to_string());
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

impl crate::notifier::Notifier for TGBot<'_> {
    fn do_notify(&self, e: &Event, stat: &HostStat) -> Result<()> {
        trace!("TGBot do_notify {:?} => {:?}", e, stat);
        match *e {
            Event::NodeUp => {
                let content = format!("❗<b>Server Status</b>\n❗ {} 主机上线 🟢", stat.name);
                let _ = self.send_tg_msg(content);
            }
            Event::NodeDown => {
                let content = format!("❗<b>Server Status</b>\n❗ {} 主机下线 🔴", stat.name);
                let _ = self.send_tg_msg(content);
            }
            Event::Custom => {
                let _ = self.do_custom_notify(stat);
            }
        }

        Ok(())
    }
}
