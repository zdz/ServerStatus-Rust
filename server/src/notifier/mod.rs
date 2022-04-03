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

fn get_tag(e: &Event) -> &'static str {
    match *e {
        Event::NodeUp => "online",
        Event::NodeDown => "offline",
        Event::Custom => "custom",
    }
}

fn add_template<
    K: Into<String> + std::fmt::Display,
    T: Into<String> + std::fmt::Display,
    S: Into<String>,
>(
    kind: K,
    tag: T,
    tpl: S,
) -> Result<()> {
    let tpl_name = format!("{}.{}", kind, tag);
    JINJA_ENV
        .lock()
        .as_mut()
        .map(|env| {
            let mut s = env.source().unwrap().to_owned();
            s.add_template(tpl_name, tpl).unwrap();
            env.set_source(s);
        })
        .unwrap();

    Ok(())
}

fn render_template(kind: &'static str, tag: &'static str, stat: &HostStat) -> Result<String> {
    let tpl_name = format!("{}.{}", kind, tag);
    Ok(JINJA_ENV
        .lock()
        .map(|e| {
            e.get_template(tpl_name.as_str()).map(|tmpl| {
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
