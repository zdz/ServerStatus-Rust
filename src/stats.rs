#![allow(unused)]
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::mpsc::sync_channel;
use std::sync::mpsc::SyncSender;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::runtime::Handle;
use tokio::{runtime::Runtime, time};

use crate::Result;

use crate::notifier::Notifier;
use crate::notifier::NOTIFIER_HANDLE;
use crate::payload::{HostStat, StatsResp};

static OFFLINE_THRESHOLD: u64 = 10; // 10s 下线
static NOTIFY_INTERVAL: u64 = 30; // 30s

pub struct StatsMgr {
    stat_sender: Option<SyncSender<HostStat>>,
    stat_rx_t: Option<thread::JoinHandle<()>>,
    timer_t: Option<thread::JoinHandle<()>>,
    notify_t: Option<thread::JoinHandle<()>>,
    resp_json: Arc<Mutex<String>>,
    notifier_list: Arc<Mutex<Vec<Box<dyn Notifier + Send>>>>,
}

use chrono::{DateTime, Datelike, Local, Timelike, Utc};

impl StatsMgr {
    pub fn new() -> Self {
        Self {
            stat_sender: None,
            resp_json: Arc::new(Mutex::new(String::from("{}"))),
            stat_rx_t: None,
            timer_t: None,
            notify_t: None,
            notifier_list: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn init(&mut self, cfg: &'static crate::config::Config) -> Result<()> {
        let mut host_map = HashMap::new();
        for host in &cfg.hosts {
            host_map.insert(String::from(&host.name), host.clone());
        }

        // init notifier, // ?plugins system?
        if cfg.tgbot.enabled {
            let o = Box::new(crate::notifier::tgbot::TGBot::new(&cfg.tgbot));
            self.notifier_list.lock().unwrap().push(o);
        }
        if cfg.wechat.enabled {
            let o = Box::new(crate::notifier::wechat::WeChat::new(&cfg.wechat));
            self.notifier_list.lock().unwrap().push(o);
        }

        let (stat_tx, stat_rx) = sync_channel(256);
        self.stat_sender = Some(stat_tx.clone());
        let (notifier_tx, notifier_rx) = sync_channel(256);

        let mut stat_dict: Arc<Mutex<HashMap<String, Box<HostStat>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        // stat_rx thread
        let mut stat_dict_1 = stat_dict.clone();
        let mut notifier_tx_1 = notifier_tx.clone();
        self.stat_rx_t = Some(thread::spawn(move || loop {
            while let Ok(stat) = stat_rx.recv() {
                info!("recv stat `{:?}", stat);
                if let Some(info) = host_map.get_mut(&stat.name) {
                    let local_now = Local::now();
                    // 补齐
                    let mut stat_t = stat; //.clone();
                    stat_t.name = String::from(&info.name);
                    stat_t.host = String::from(&info.host);
                    stat_t.location = String::from(&info.location);
                    stat_t.host_type = String::from(&info.host_type);
                    stat_t.lastest_ts = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    // last_network_in/out
                    if info.last_network_in == 0
                        || (stat_t.network_in != 0 && info.last_network_in > stat_t.network_in)
                        || (local_now.day() == info.monthstart
                            && local_now.hour() == 0
                            && local_now.minute() < 5)
                    {
                        info.last_network_in = stat_t.network_in;
                        info.last_network_out = stat_t.network_out;
                    }
                    stat_t.last_network_in = info.last_network_in;
                    stat_t.last_network_out = info.last_network_out;

                    // uptime str
                    let day = (stat_t.uptime as f64 / 3600.0 / 24.0) as i64;
                    if day > 0 {
                        stat_t.uptime_str = format!("{} 天", day);
                    } else {
                        stat_t.uptime_str = format!(
                            "{:02}:{:02}:{:02}",
                            (stat_t.uptime as f64 / 3600.0) as i64,
                            (stat_t.uptime as f64 / 60.0) as i64 % 60,
                            stat_t.uptime % 60
                        );
                    }

                    info!("update stat `{:?}", stat_t);
                    {
                        let mut host_stat_map = stat_dict_1.lock().unwrap();
                        if let Some(pre_stat) = host_stat_map.get(&info.name) {
                            if pre_stat.lastest_ts + OFFLINE_THRESHOLD < stat_t.lastest_ts {
                                // node up notify
                                notifier_tx_1
                                    .send((crate::notifier::Event::NodeUp, stat_t.clone()));
                            }
                        } else {
                            // node up notify
                            notifier_tx_1.send((crate::notifier::Event::NodeUp, stat_t.clone()));
                        }
                        host_stat_map.insert(String::from(&info.name), Box::new(stat_t));
                        info!("{:?}", host_stat_map);
                    }
                } else {
                    info!("invalid stat `{:?}", stat);
                }
            }
        }));

        // timer thread
        let mut resp_json = self.resp_json.clone();
        let mut stat_dict_2 = stat_dict.clone();
        let mut notifier_tx_2 = notifier_tx.clone();
        let mut last_notify_ts: u64 = 0;
        self.timer_t = Some(thread::spawn(move || loop {
            let mut resp = StatsResp::new();
            let mut notified = false;
            {
                let host_stat_map = stat_dict_2.lock().unwrap();
                for (k, v) in &*host_stat_map {
                    let mut o = (**v).clone();
                    // 10s 下线
                    if v.lastest_ts + OFFLINE_THRESHOLD < resp.updated {
                        o.online4 = false;
                        o.online6 = false;
                    }

                    // notify check /30 s
                    if last_notify_ts + NOTIFY_INTERVAL < resp.updated {
                        if o.online4 || o.online6 {
                            notifier_tx_2.send((crate::notifier::Event::Custom, o.clone()));
                        } else {
                            notifier_tx_2.send((crate::notifier::Event::NodeDown, o.clone()));
                        }
                        notified = true;
                    }

                    resp.servers.push(o);
                }
                if notified {
                    last_notify_ts = resp.updated;
                }
            }
            {
                let mut o = resp_json.lock().unwrap();
                *o = serde_json::to_string(&resp).unwrap();
                // info!("{}", *o);
            }

            thread::sleep(Duration::from_millis(500))
        }));

        // notify thread
        *NOTIFIER_HANDLE.lock().unwrap() = Some(Handle::current().clone());
        let notifier_list = self.notifier_list.clone();
        self.notify_t = Some(thread::spawn(move || loop {
            while let Ok(msg) = notifier_rx.recv() {
                let (e, stat) = msg;
                let notifiers = &*notifier_list.lock().unwrap();
                trace!("recv notify => {}, {:?}, {:?}", notifiers.len(), e, stat);
                for notifier in notifiers {
                    notifier.do_notify(&e, &stat);
                }
            }
        }));

        Ok(())
    }

    pub fn get_stats_json(&self) -> String {
        String::from(self.resp_json.lock().unwrap().as_str())
    }

    pub fn report(&self, data: &String) -> Result<()> {
        match serde_json::from_str(data) {
            Ok(stat) => {
                if let Some(ref sender) = self.stat_sender {
                    trace!("send stat => {:?} ", stat);
                    sender.send(stat);
                }
            }
            Err(err) => {
                error!("report error => {:?}", err);
            }
        };
        Ok(())
    }
}
