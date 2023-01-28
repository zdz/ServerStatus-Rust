#![deny(warnings)]
// #![allow(unused)]
use anyhow::Result;
use chrono::Local;
use hyper::Body;
use reqwest;
use rhai::serde::{from_dynamic, to_dynamic};
use rhai::{Array, Dynamic, Engine, ImmutableString, Scope, AST};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::time::Duration;

use crate::notifier::{get_tag, Event, HostStat, NOTIFIER_HANDLE};

const KIND: &str = "webhook";

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct Receiver {
    pub enabled: bool,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub timeout: u32,
    pub script: String,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct Config {
    pub enabled: bool,
    pub receiver: Vec<Receiver>,
}

pub struct Webhook {
    config: &'static Config,
    http_client: reqwest::Client,
    engine: Engine,
    ast_list: Vec<Option<AST>>,
}

fn join(arr: Array, sep: ImmutableString) -> ImmutableString {
    arr.iter()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join(sep.as_str())
        .into()
}

fn now_str() -> ImmutableString {
    Local::now().format("%Y-%m-%d %H:%M:%S %Z").to_string().into()
}

fn to_json(o: Dynamic) -> ImmutableString {
    serde_json::to_string(&o).map(|s| s.into()).unwrap_or_default()
}

impl Webhook {
    pub fn new(cfg: &'static Config) -> Self {
        let mut o = Self {
            config: cfg,
            http_client: reqwest::Client::new(),
            engine: Engine::new(),
            ast_list: Vec::new(),
        };

        o.engine.register_fn("to_json", to_json);
        o.engine.register_fn("join", join);
        o.engine.register_fn("now_str", now_str);

        for r in o.config.receiver.iter() {
            if r.enabled {
                let ast = o.engine.compile(&r.script).unwrap();
                o.ast_list.push(Some(ast));
            } else {
                o.ast_list.push(None);
            }
        }

        o
    }
    fn call_webhook(&self, r: &'static Receiver, content: String) -> Result<()> {
        if content.is_empty() {
            return Ok(());
        }

        let handle = NOTIFIER_HANDLE.lock().unwrap().as_ref().unwrap().clone();
        let http_client = self.http_client.clone();
        handle.spawn(async move {
            let mut http_client_builder = http_client
                .post(&r.url)
                .timeout(Duration::from_secs(r.timeout.into()))
                .body(Body::from(content));

            for (k, v) in r.headers.iter() {
                http_client_builder = http_client_builder.header(k, v);
            }

            if r.username.is_some()
                && r.password.is_some()
                && !r.username.as_ref().unwrap().is_empty()
                && !r.password.as_ref().unwrap().is_empty()
            {
                http_client_builder = http_client_builder.basic_auth(r.username.as_ref().unwrap(), r.password.as_ref());
            }

            //
            match http_client_builder.send().await {
                Ok(resp) => {
                    info!("webhook send msg resp => {:?}", resp);
                }
                Err(err) => {
                    error!("webhook send msg error => {:?}", err);
                }
            }
        });
        Ok(())
    }
}
impl crate::notifier::Notifier for Webhook {
    fn kind(&self) -> &'static str {
        KIND
    }

    fn send_notify(&self, content: String) -> Result<()> {
        info!("{}", content);
        Ok(())
    }

    fn notify_test(&self) -> Result<()> {
        for r in self.config.receiver.iter() {
            if !r.enabled {
                continue;
            }
            self.call_webhook(r, "❗ServerStatus test msg".into())?;
        }
        Ok(())
    }

    fn notify(&self, e: &Event, stat: &HostStat) -> Result<()> {
        for (idx, r) in self.config.receiver.iter().enumerate() {
            if !r.enabled {
                continue;
            }

            let mut scope = Scope::new();
            scope.push("event", get_tag(e));
            scope.push("host", to_dynamic(stat)?);
            scope.push("config", to_dynamic(r)?);
            scope.push("ip_info", to_dynamic(stat.ip_info.as_ref())?);
            scope.push("sys_info", to_dynamic(stat.sys_info.as_ref())?);

            let res: Dynamic = self
                .engine
                .eval_ast_with_scope(&mut scope, self.ast_list[idx].as_ref().unwrap())?;

            // [notify, json_body/content]
            if let Ok(v) = from_dynamic::<Array>(&res) {
                if v.len() >= 2 && from_dynamic::<bool>(&v[0]).unwrap_or_default() {
                    self.call_webhook(r, serde_json::to_string(&v[1]).unwrap_or_default())?
                }
            }
        }

        Ok(())
    }
}
