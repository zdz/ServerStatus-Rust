#![deny(warnings)]
use anyhow::Result;
use log::{error, info};
use minijinja::context;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use tokio::time::Duration;

use crate::jinja::{add_template, render_template};
use crate::notifier::{get_tag, Event, HostStat, NOTIFIER_HANDLE};

// https://qydev.weixin.qq.com/wiki/index.php?title=%E4%B8%BB%E5%8A%A8%E8%B0%83%E7%94%A8
// https://qydev.weixin.qq.com/wiki/index.php?title=%E5%8F%91%E9%80%81%E6%8E%A5%E5%8F%A3%E8%AF%B4%E6%98%8E
static TOKEN_URL: &str = "https://qyapi.weixin.qq.com/cgi-bin/gettoken";
const KIND: &str = "wechat";

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Config {
    pub enabled: bool,
    pub corp_id: String,
    pub corp_secret: String,
    pub agent_id: String,
    pub title: String,
    pub online_tpl: String,
    pub offline_tpl: String,
    pub custom_tpl: String,
}

pub struct WeChat {
    config: &'static Config,
    http_client: reqwest::Client,
}

impl WeChat {
    pub fn new(cfg: &'static Config) -> Self {
        let o = Self {
            config: cfg,
            http_client: reqwest::Client::new(),
        };
        add_template(KIND, get_tag(&Event::NodeUp), o.config.online_tpl.to_string());
        add_template(KIND, get_tag(&Event::NodeDown), o.config.offline_tpl.to_string());
        add_template(KIND, get_tag(&Event::Custom), o.config.custom_tpl.to_string());

        o
    }
}

impl crate::notifier::Notifier for WeChat {
    fn kind(&self) -> &'static str {
        KIND
    }

    fn send_notify(&self, text_content: String) -> Result<()> {
        // get access_token
        let mut data = HashMap::new();
        data.insert("corpid", self.config.corp_id.to_string());
        data.insert("corpsecret", self.config.corp_secret.to_string());

        let http_client = self.http_client.clone();
        let handle = NOTIFIER_HANDLE.lock().unwrap().as_ref().unwrap().clone();
        let agent_id = self.config.agent_id.to_string();
        handle.spawn(async move {
            match http_client
                .post(TOKEN_URL)
                .timeout(Duration::from_secs(5))
                .json(&data)
                .send()
                .await
            {
                Ok(resp) => {
                    info!("wechat get access token resp => {:?}", resp);
                    let json_res = resp.json::<HashMap<String, serde_json::Value>>().await;
                    if let Ok(json_data) = json_res {
                        if let Some(access_token) = json_data.get("access_token") {
                            if let Some(token) = access_token.as_str() {
                                let req_url =
                                    format!("https://qyapi.weixin.qq.com/cgi-bin/message/send?access_token={token}");
                                let req_data = serde_json::json!({
                                    "touser": "@all",
                                    "agentid": agent_id,
                                    "msgtype": "text",
                                    "text": {
                                        "content": text_content,
                                    },
                                    "safe": 0
                                });

                                match http_client
                                    .post(&req_url)
                                    .timeout(Duration::from_secs(5))
                                    .json(&req_data)
                                    .send()
                                    .await
                                {
                                    Ok(resp) => {
                                        info!("wechat send msg resp => {:?}", resp);
                                    }
                                    Err(err) => {
                                        error!("wechat send msg error => {:?}", err);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(err) => {
                    error!("wechat get access_token error => {:?}", err);
                }
            }
        });

        Ok(())
    }

    fn notify(&self, e: &Event, stat: &HostStat) -> Result<()> {
        render_template(
            self.kind(),
            get_tag(e),
            context!(host => stat, config => self.config, ip_info => stat.ip_info, sys_info => stat.sys_info),
            true,
        )
        .map(|content| match *e {
            Event::NodeUp | Event::NodeDown => self.send_notify(content).unwrap(),
            Event::Custom => {
                info!("render.custom.tpl => {}", content);
                if !content.is_empty() {
                    self.send_notify(format!("{}\n{}", self.config.title, content))
                        .unwrap_or_else(|err| {
                            error!("send_msg err => {:?}", err);
                        });
                }
            }
        })
    }
}
