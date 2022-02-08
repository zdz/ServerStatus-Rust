#![deny(warnings)]

use log::{error, info, trace};
use minijinja::{context, Environment};
use reqwest;
use serde_json;
use std::borrow::Cow;
use std::collections::HashMap;
use tokio::time::Duration;

use crate::notifier::Event;
use crate::notifier::HostStat;
use crate::notifier::Result;
use crate::notifier::NOTIFIER_HANDLE;

// https://qydev.weixin.qq.com/wiki/index.php?title=%E4%B8%BB%E5%8A%A8%E8%B0%83%E7%94%A8
// https://qydev.weixin.qq.com/wiki/index.php?title=%E5%8F%91%E9%80%81%E6%8E%A5%E5%8F%A3%E8%AF%B4%E6%98%8E
static TOKEN_URL: &str = "https://qyapi.weixin.qq.com/cgi-bin/gettoken";

pub struct WeChat<'a> {
    corp_id: &'static String,
    corp_secret: &'static String,
    agent_id: &'static String,
    custom_tpl: &'static String,
    jinja_env: Environment<'a>,
    http_client: reqwest::Client,
}

impl WeChat<'_> {
    pub fn new(cfg: &'static crate::config::WeChat) -> Self {
        let mut o = Self {
            corp_id: &cfg.corp_id,
            corp_secret: &cfg.corp_secret,
            agent_id: &cfg.agent_id,
            custom_tpl: &cfg.custom_tpl,
            jinja_env: Environment::new(),
            http_client: reqwest::Client::new(),
        };

        o.jinja_env.add_template("tpl", o.custom_tpl).unwrap();
        o
    }

    fn do_custom_notify(&self, stat: &Cow<HostStat>) -> Result<()> {
        trace!("do_custom_notify => {:?}", stat);
        let tmpl = self.jinja_env.get_template("tpl").unwrap();
        match tmpl.render(context!(host => stat)) {
            Ok(content) => {
                info!("tmpl.render => {}", content);
                let s = content
                    .split("\n")
                    .map(|t| t.trim())
                    .filter(|&t| !t.is_empty())
                    .collect::<Vec<&str>>()
                    .join("\n");
                if s.len() > 0 {
                    let _ = self.send_wechat_msg(format!("❗Server Status\n{}", s));
                }
            }
            Err(err) => {
                error!("tmpl.render err => {:?}", err);
            }
        }

        Ok(())
    }

    fn send_wechat_msg(&self, text_content: String) -> Result<()> {
        // get access_token
        let mut data = HashMap::new();
        data.insert("corpid", self.corp_id.to_string());
        data.insert("corpsecret", self.corp_secret.to_string());

        let http_client = self.http_client.clone();
        let handle = NOTIFIER_HANDLE.lock().unwrap().as_ref().unwrap().clone();
        let agent_id = self.agent_id.to_string();
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
                                let req_url = format!(
                                "https://qyapi.weixin.qq.com/cgi-bin/message/send?access_token={}",
                                token
                            );
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
}

impl crate::notifier::Notifier for WeChat<'_> {
    fn do_notify(&self, e: &Event, stat: &Cow<HostStat>) -> Result<()> {
        trace!("WeChat do_notify {:?} => {:?}", e, stat);
        match e {
            &Event::NodeUp => {
                let content = format!("❗Server Status\n❗ {} 主机上线 🟢", stat.name);
                let _ = self.send_wechat_msg(content);
            }
            &Event::NodeDown => {
                let content = format!("❗Server Status\n❗ {} 主机下线 🔴", stat.name);
                let _ = self.send_wechat_msg(content);
            }
            &Event::Custom => {
                let _ = self.do_custom_notify(stat);
            }
        }

        Ok(())
    }
}
