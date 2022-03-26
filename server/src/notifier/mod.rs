use anyhow::Result;
use minijinja::{context, Environment};
use once_cell::sync::Lazy;
use std::sync::Mutex;
use tokio::runtime::Handle;

use crate::payload::HostStat;

pub mod email;
pub mod tgbot;
pub mod wechat;

pub static NOTIFIER_HANDLE: Lazy<Mutex<Option<Handle>>> = Lazy::new(Default::default);
pub static JINJA_ENV: Lazy<Mutex<Environment>> = Lazy::new(|| Mutex::new(Environment::new()));

#[derive(Debug)]
pub enum Event {
    NodeUp,
    NodeDown,
    Custom,
}

pub trait Notifier {
    fn kind(&self) -> &'static str;
    fn notify(&self, e: &Event, stat: &HostStat) -> Result<()>;
}

fn add_template(kind: &'static str, tpl: &'static str) -> Result<()> {
    Ok(JINJA_ENV.lock().unwrap().add_template(kind, tpl)?)
}

fn render_template(kind: &'static str, stat: &HostStat) -> Result<String> {
    Ok(JINJA_ENV
        .lock()
        .map(|e| {
            e.get_template(kind).map(|tmpl| {
                tmpl.render(context!(host => stat))
                    .map(|content| {
                        content
                            .split('\n')
                            .map(|t| t.trim())
                            .filter(|&t| !t.is_empty())
                            .collect::<Vec<&str>>()
                            .join("\n")
                    })
                    .unwrap_or_else(|err| {
                        error!("tmpl.render err => {:?}", err);
                        "".to_string()
                    })
            })
        })
        .unwrap_or_else(|err| {
            error!("render_template err => {:?}", err);
            Ok("".to_string())
        })?)
}
