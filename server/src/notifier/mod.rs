use anyhow::Result;
use once_cell::sync::Lazy;
use std::sync::Mutex;
use tokio::runtime::Handle;

use crate::payload::HostStat;

pub mod email;
pub mod tgbot;
pub mod wechat;

pub static NOTIFIER_HANDLE: Lazy<Mutex<Option<Handle>>> = Lazy::new(Default::default);

#[derive(Debug)]
pub enum Event {
    NodeUp,
    NodeDown,
    Custom,
}

fn get_tag(e: &Event) -> &'static str {
    match *e {
        Event::NodeUp => "online",
        Event::NodeDown => "offline",
        Event::Custom => "custom",
    }
}

pub trait Notifier {
    fn kind(&self) -> &'static str;
    fn notify(&self, e: &Event, stat: &HostStat) -> Result<()>;
    // send notify impl
    fn send_notify(&self, content: String) -> Result<()>;
    fn notify_test(&self) -> Result<()> {
        self.send_notify("❗ServerStatus test msg".to_string())
    }
}
