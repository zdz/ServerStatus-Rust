use once_cell::sync::Lazy;
use std::sync::Mutex;
use tokio::runtime::Handle;

use crate::payload::HostStat;
use crate::Result;

pub mod tgbot;
pub mod wechat;

pub static NOTIFIER_HANDLE: Lazy<Mutex<Option<Handle>>> = Lazy::new(Default::default);

#[derive(Debug)]
pub enum Event {
    NodeUp,
    NodeDown,
    Custom,
}

pub trait Notifier {
    fn do_notify(&self, e: &Event, stat: &HostStat) -> Result<()>;
}
