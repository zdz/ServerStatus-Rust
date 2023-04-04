#![allow(unused)]
use anyhow::Result;
use chrono::{Datelike, Local, Timelike};
use lazy_static::lazy_static;
use once_cell::sync::OnceCell;
use std::borrow::Borrow;
use std::borrow::BorrowMut;
use std::borrow::Cow;
use std::collections::binary_heap::Iter;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::sync::mpsc::sync_channel;
use std::sync::mpsc::SyncSender;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::Host;
use crate::notifier::{Event, Notifier};
use crate::payload::{HostStat, StatsResp};

const SAVE_INTERVAL: u64 = 60;

static STAT_SENDER: OnceCell<SyncSender<Cow<HostStat>>> = OnceCell::new();

pub struct StatsMgr {
    resp_json: Arc<Mutex<String>>,
    stats_data: Arc<Mutex<StatsResp>>,
}

impl StatsMgr {
    pub fn new() -> Self {
        Self {
            resp_json: Arc::new(Mutex::new("{}".to_string())),
            stats_data: Arc::new(Mutex::new(StatsResp::new())),
        }
    }

    fn load_last_network(&mut self, hosts_map: &mut HashMap<String, Host>) {
        let contents = fs::read_to_string("stats.json").unwrap_or_default();
        if contents.is_empty() {
            return;
        }

        if let Ok(stats_json) = serde_json::from_str::<serde_json::Value>(contents.as_str()) {
            if let Some(servers) = stats_json["servers"].as_array() {
                for v in servers {
                    if let (Some(name), Some(last_network_in), Some(last_network_out)) = (
                        v["name"].as_str(),
                        v["last_network_in"].as_u64(),
                        v["last_network_out"].as_u64(),
                    ) {
                        if let Some(srv) = hosts_map.get_mut(name) {
                            srv.last_network_in = last_network_in;
                            srv.last_network_out = last_network_out;

                            trace!("{} => last in/out ({}/{}))", &name, last_network_in, last_network_out);
                        }
                    } else {
                        error!("invalid json => {:?}", v);
                    }
                }
                trace!("load stats.json succ!");
            }
        } else {
            warn!("ignore invalid stats.json");
        }
    }

    pub fn init(
        &mut self,
        cfg: &'static crate::config::Config,
        notifies: Arc<Mutex<Vec<Box<dyn Notifier + Send>>>>,
    ) -> Result<()> {
        let hosts_map_base = Arc::new(Mutex::new(cfg.hosts_map.clone()));

        // load last_network_in/out
        if let Ok(mut hosts_map) = hosts_map_base.lock() {
            self.load_last_network(&mut hosts_map);
        }

        let (stat_tx, stat_rx) = sync_channel(512);
        STAT_SENDER.set(stat_tx).unwrap();
        let (notifier_tx, notifier_rx) = sync_channel(512);

        let stat_map: Arc<Mutex<HashMap<String, Cow<HostStat>>>> = Arc::new(Mutex::new(HashMap::new()));

        // stat_rx thread
        thread::spawn({
            let hosts_group_map = cfg.hosts_group_map.clone();
            let hosts_map = hosts_map_base.clone();
            let stat_map = stat_map.clone();
            let notifier_tx = notifier_tx.clone();

            move || loop {
                while let Ok(mut stat) = stat_rx.recv() {
                    trace!("recv stat `{:?}", stat);

                    let mut stat_t = stat.to_mut();

                    // group mode
                    if !stat_t.gid.is_empty() {
                        if stat_t.alias.is_empty() {
                            stat_t.alias = stat_t.name.to_string();
                        }

                        if let Ok(mut hosts_map) = hosts_map.lock() {
                            let host = hosts_map.get(&stat_t.name);
                            if host.is_none() || !host.unwrap().gid.eq(&stat_t.gid) {
                                if let Some(group) = hosts_group_map.get(&stat_t.gid) {
                                    // 名称不变，换组了，更新组配置 & last in/out
                                    let mut inst = group.inst_host(&stat_t.name);
                                    if let Some(o) = host {
                                        inst.last_network_in = o.last_network_in;
                                        inst.last_network_out = o.last_network_out;
                                    };
                                    hosts_map.insert(stat_t.name.to_string(), inst);
                                } else {
                                    continue;
                                }
                            }
                        }
                    }

                    //
                    if let Ok(mut hosts_map) = hosts_map.lock() {
                        let host_info = hosts_map.get_mut(&stat_t.name);
                        if host_info.is_none() {
                            error!("invalid stat `{:?}", stat_t);
                            continue;
                        }
                        let info = host_info.unwrap();

                        if info.disabled {
                            continue;
                        }

                        // 补齐
                        if stat_t.location.is_empty() {
                            stat_t.location = info.location.to_string();
                        }
                        if stat_t.host_type.is_empty() {
                            stat_t.host_type = info.r#type.to_owned();
                        }
                        stat_t.notify = info.notify && stat_t.notify;
                        stat_t.pos = info.pos;
                        stat_t.disabled = info.disabled;
                        stat_t.weight += info.weight;
                        stat_t.labels = info.labels.to_owned();

                        // !group
                        if !info.alias.is_empty() {
                            stat_t.alias = info.alias.to_owned();
                        }

                        info.latest_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                        stat_t.latest_ts = info.latest_ts;

                        // last_network_in/out
                        if !stat_t.vnstat {
                            let local_now = Local::now();
                            if info.last_network_in == 0
                                || (stat_t.network_in != 0 && info.last_network_in > stat_t.network_in)
                                || (local_now.day() == info.monthstart
                                    && local_now.hour() == 0
                                    && local_now.minute() < 5)
                            {
                                info.last_network_in = stat_t.network_in;
                                info.last_network_out = stat_t.network_out;
                            } else {
                                stat_t.last_network_in = info.last_network_in;
                                stat_t.last_network_out = info.last_network_out;
                            }
                        }

                        // uptime str
                        let day = (stat_t.uptime as f64 / 3600.0 / 24.0) as i64;
                        if day > 0 {
                            stat_t.uptime_str = format!("{day} 天");
                        } else {
                            stat_t.uptime_str = format!(
                                "{:02}:{:02}:{:02}",
                                (stat_t.uptime as f64 / 3600.0) as i64,
                                (stat_t.uptime as f64 / 60.0) as i64 % 60,
                                stat_t.uptime % 60
                            );
                        }

                        info!("update stat `{:?}", stat_t);
                        if let Ok(mut host_stat_map) = stat_map.lock() {
                            if let Some(pre_stat) = host_stat_map.get(&stat_t.name) {
                                if stat_t.ip_info.is_none() {
                                    stat_t.ip_info = pre_stat.ip_info.to_owned();
                                }

                                if stat_t.notify && (pre_stat.latest_ts + cfg.offline_threshold < stat_t.latest_ts) {
                                    // node up notify
                                    notifier_tx.send((Event::NodeUp, stat.clone()));
                                }
                            }
                            host_stat_map.insert(stat.name.to_string(), stat);
                            //trace!("{:?}", host_stat_map);
                        }
                    }
                }
            }
        });

        // timer thread
        thread::spawn({
            let resp_json = self.resp_json.clone();
            let stats_data = self.stats_data.clone();
            let hosts_map = hosts_map_base.clone();
            let stat_map = stat_map.clone();
            let notifier_tx = notifier_tx.clone();
            let mut latest_notify_ts = 0_u64;
            let mut latest_save_ts = 0_u64;
            let mut latest_group_gc = 0_u64;
            let mut latest_alert_check_ts = 0_u64;
            move || loop {
                thread::sleep(Duration::from_millis(500));

                let mut resp = StatsResp::new();
                let now = resp.updated;
                let mut notified = false;

                // group gc
                if latest_group_gc + cfg.group_gc < now {
                    latest_group_gc = now;
                    //
                    if let Ok(mut hosts_map) = hosts_map.lock() {
                        hosts_map.retain(|_, o| o.gid.is_empty() || o.latest_ts + cfg.group_gc >= now);
                    }
                    //
                    if let Ok(mut stat_map) = stat_map.lock() {
                        stat_map.retain(|_, o| o.gid.is_empty() || o.latest_ts + cfg.group_gc >= now);
                    }
                }

                if let Ok(mut host_stat_map) = stat_map.lock() {
                    for (_, stat) in host_stat_map.iter_mut() {
                        if stat.disabled {
                            resp.servers.push(stat.as_ref().clone());
                            continue;
                        }
                        let stat = stat.borrow_mut();
                        let o = stat.to_mut();
                        // 30s 下线
                        if o.latest_ts + cfg.offline_threshold < now {
                            o.online4 = false;
                            o.online6 = false;
                        }

                        // labels
                        const OS_LIST: [&str; 9] = [
                            "centos", "debian", "ubuntu", "arch", "windows", "macos", "pi", "android", "linux",
                        ];
                        if !o.labels.contains("os=") {
                            if let Some(sys_info) = &o.sys_info {
                                let os_r = sys_info.os_release.to_lowercase();
                                for s in OS_LIST.iter() {
                                    if os_r.contains(s) {
                                        if o.labels.is_empty() {
                                            write!(o.labels, "os={s}");
                                        } else {
                                            write!(o.labels, ";os={s}");
                                        }
                                        break;
                                    }
                                }
                            }
                        }

                        // client notify
                        if o.notify {
                            // notify check /30 s
                            if latest_notify_ts + cfg.notify_interval < now {
                                if o.online4 || o.online6 {
                                    notifier_tx.send((Event::Custom, stat.clone()));
                                } else {
                                    o.disabled = true;
                                    notifier_tx.send((Event::NodeDown, stat.clone()));
                                }
                                notified = true;
                            }
                        }

                        resp.servers.push(stat.as_ref().clone());
                    }
                    if notified {
                        latest_notify_ts = now;
                    }
                }

                resp.servers.sort_by(|a, b| {
                    if a.weight != b.weight {
                        return a.weight.cmp(&b.weight).reverse();
                    }
                    if a.pos != b.pos {
                        return a.pos.cmp(&b.pos);
                    }
                    // same group
                    a.alias.cmp(&b.alias)
                });

                // last_network_in/out save /60s
                if latest_save_ts + SAVE_INTERVAL < now {
                    latest_save_ts = now;
                    if !resp.servers.is_empty() {
                        if let Ok(mut file) = File::create("stats.json") {
                            file.write(serde_json::to_string(&resp).unwrap().as_bytes());
                            file.flush();
                            trace!("save stats.json succ!");
                        } else {
                            error!("save stats.json fail!");
                        }
                    }
                }
                //
                if let Ok(mut o) = resp_json.lock() {
                    *o = serde_json::to_string(&resp).unwrap();
                }
                if let Ok(mut o) = stats_data.lock() {
                    *o = resp;
                }
            }
        });

        // notify thread
        thread::spawn(move || loop {
            while let Ok(msg) = notifier_rx.recv() {
                let (e, stat) = msg;
                let notifiers = &*notifies.lock().unwrap();
                trace!("recv notify => {:?}, {:?}", e, stat);
                for notifier in notifiers {
                    trace!("{} notify {:?} => {:?}", notifier.kind(), e, stat);
                    notifier.notify(&e, stat.borrow());
                }
            }
        });

        Ok(())
    }

    pub fn get_stats(&self) -> Arc<Mutex<StatsResp>> {
        self.stats_data.clone()
    }

    pub fn get_stats_json(&self) -> String {
        self.resp_json.lock().unwrap().to_string()
    }

    pub fn report(&self, data: serde_json::Value) -> Result<()> {
        lazy_static! {
            static ref SENDER: SyncSender<Cow<'static, HostStat>> = STAT_SENDER.get().unwrap().clone();
        }

        match serde_json::from_value(data) {
            Ok(stat) => {
                trace!("send stat => {:?} ", stat);
                SENDER.send(Cow::Owned(stat));
            }
            Err(err) => {
                error!("report error => {:?}", err);
            }
        };
        Ok(())
    }

    pub fn get_all_info(&self) -> Result<serde_json::Value> {
        let data = self.stats_data.lock().unwrap();
        let mut resp_json = serde_json::to_value(&*data)?;
        // for skip_serializing
        if let Some(srv_list) = resp_json["servers"].as_array_mut() {
            for (idx, stat) in data.servers.iter().enumerate() {
                if let Some(srv) = srv_list[idx].as_object_mut() {
                    srv.insert("ip_info".into(), serde_json::to_value(stat.ip_info.as_ref())?);
                    srv.insert("sys_info".into(), serde_json::to_value(stat.sys_info.as_ref())?);
                }
            }
        } else {
            // todo!()
        };

        Ok(resp_json)
    }
}
