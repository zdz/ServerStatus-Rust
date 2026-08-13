#![allow(unused)]
use anyhow::Result;
use chrono::{Datelike, Local, TimeZone, Timelike};
use once_cell::sync::OnceCell;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::sync::mpsc::sync_channel;
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, LazyLock, Mutex};
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::Host;
use crate::notifier::{Event, Notifier};
use crate::payload::{HostStat, StatsResp};

const SAVE_INTERVAL: u64 = 60;
const OS_LIST: [&str; 10] = [
    "centos", "debian", "ubuntu", "arch", "windows", "macos", "pi", "android", "linux", "freebsd",
];

static STAT_SENDER: OnceCell<SyncSender<Cow<HostStat>>> = OnceCell::new();

fn date_key(year: i32, month: u32, day: u32) -> u32 {
    u32::try_from(year).unwrap_or_default() * 10_000 + month * 100 + day
}

fn current_date_key() -> u32 {
    let now = Local::now();
    date_key(now.year(), now.month(), now.day())
}

fn timestamp_date_key(timestamp: u64) -> Option<u32> {
    let timestamp = i64::try_from(timestamp).ok()?;
    Local
        .timestamp_opt(timestamp, 0)
        .single()
        .map(|dt| date_key(dt.year(), dt.month(), dt.day()))
}

fn apply_daily_traffic(info: &mut Host, stat: &mut HostStat, today: u32) {
    if stat.vnstat && (stat.daily_network_in > 0 || stat.daily_network_out > 0) {
        info.daily_network_in_base = stat.network_in.saturating_sub(stat.daily_network_in);
        info.daily_network_out_base = stat.network_out.saturating_sub(stat.daily_network_out);
        info.daily_base_date = today;
        return;
    }

    let reset_baseline = info.daily_base_date != today
        || info.daily_network_in_base > stat.network_in
        || info.daily_network_out_base > stat.network_out;

    if reset_baseline {
        info.daily_network_in_base = stat.network_in;
        info.daily_network_out_base = stat.network_out;
        info.daily_base_date = today;
        stat.daily_network_in = 0;
        stat.daily_network_out = 0;
        return;
    }

    stat.daily_network_in = stat.network_in.saturating_sub(info.daily_network_in_base);
    stat.daily_network_out = stat.network_out.saturating_sub(info.daily_network_out_base);
}

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

    fn load_last_network(hosts_map: &mut HashMap<String, Host>) {
        let contents = fs::read_to_string("stats.json").unwrap_or_default();
        if contents.is_empty() {
            return;
        }

        if let Ok(stats_json) = serde_json::from_str::<serde_json::Value>(contents.as_str()) {
            if let Some(servers) = stats_json["servers"].as_array() {
                let today = current_date_key();
                for v in servers {
                    let Some(name) = v["name"].as_str() else {
                        error!("invalid json => {v:?}");
                        continue;
                    };
                    let Some(srv) = hosts_map.get_mut(name) else {
                        continue;
                    };

                    if let (Some(last_network_in), Some(last_network_out)) =
                        (v["last_network_in"].as_u64(), v["last_network_out"].as_u64())
                    {
                        srv.last_network_in = last_network_in;
                        srv.last_network_out = last_network_out;
                        trace!("{} => last in/out ({}/{}))", &name, last_network_in, last_network_out);
                    }

                    if let (
                        Some(latest_ts),
                        Some(network_in),
                        Some(network_out),
                        Some(daily_network_in),
                        Some(daily_network_out),
                    ) = (
                        v["latest_ts"].as_u64(),
                        v["network_in"].as_u64(),
                        v["network_out"].as_u64(),
                        v["daily_network_in"].as_u64(),
                        v["daily_network_out"].as_u64(),
                    ) {
                        if timestamp_date_key(latest_ts) == Some(today) {
                            srv.daily_network_in_base = network_in.saturating_sub(daily_network_in);
                            srv.daily_network_out_base = network_out.saturating_sub(daily_network_out);
                            srv.daily_base_date = today;
                            trace!(
                                "{} => daily base ({}/{})",
                                &name,
                                srv.daily_network_in_base,
                                srv.daily_network_out_base
                            );
                        }
                    }
                }
                trace!("load stats.json succ!");
            }
        } else {
            warn!("ignore invalid stats.json");
        }
    }

    #[allow(clippy::too_many_lines)]
    #[allow(clippy::unnecessary_wraps)]
    pub fn init(
        &mut self,
        cfg: &'static crate::config::Config,
        notifies: Arc<Mutex<Vec<Box<dyn Notifier + Send>>>>,
    ) -> Result<()> {
        let hosts_map_base = Arc::new(Mutex::new(cfg.hosts_map.clone()));

        // load last_network_in/out
        if let Ok(mut hosts_map_guard) = hosts_map_base.lock() {
            Self::load_last_network(&mut hosts_map_guard);
        }

        let (stat_tx, stat_rx) = sync_channel(512);
        STAT_SENDER.set(stat_tx).unwrap();
        let (notifier_tx, notifier_rx) = sync_channel(512);

        let stat_map: Arc<Mutex<HashMap<String, Arc<HostStat>>>> = Arc::new(Mutex::new(HashMap::new()));

        // stat_rx thread
        thread::spawn({
            let hosts_map = hosts_map_base.clone();
            let stat_map = stat_map.clone();
            let notifier_tx = notifier_tx.clone();

            move || loop {
                while let Ok(mut stat) = stat_rx.recv() {
                    trace!("recv stat `{stat:?}");

                    let mut stat_t = stat.to_mut();

                    // group mode
                    if !stat_t.gid.is_empty() {
                        if stat_t.alias.is_empty() {
                            stat_t.alias = stat_t.name.clone();
                        }

                        if let Ok(mut hosts_map) = hosts_map.lock() {
                            let host = hosts_map.get(&stat_t.name);
                            if host.is_none() || !host.unwrap().gid.eq(&stat_t.gid) {
                                if let Some(group) = cfg.hosts_group_map.get(&stat_t.gid) {
                                    // 名称不变，换组了，更新组配置 & last in/out
                                    let mut inst = group.inst_host(&stat_t.name);
                                    if let Some(o) = host {
                                        inst.last_network_in = o.last_network_in;
                                        inst.last_network_out = o.last_network_out;
                                        inst.daily_network_in_base = o.daily_network_in_base;
                                        inst.daily_network_out_base = o.daily_network_out_base;
                                        inst.daily_base_date = o.daily_base_date;
                                    }
                                    hosts_map.insert(stat_t.name.clone(), inst);
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
                            error!("invalid stat `{stat_t:?}");
                            continue;
                        }
                        let info = host_info.unwrap();

                        if info.disabled {
                            continue;
                        }

                        // 补齐
                        if stat_t.location.is_empty() {
                            stat_t.location = info.location.clone();
                        }
                        if stat_t.host_type.is_empty() {
                            stat_t.host_type = info.r#type.clone();
                        }
                        stat_t.notify = info.notify && stat_t.notify;
                        stat_t.pos = info.pos;
                        stat_t.disabled = info.disabled;
                        stat_t.weight += info.weight;
                        stat_t.labels = info.labels.clone();

                        // !group
                        if !info.alias.is_empty() {
                            stat_t.alias = info.alias.clone();
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

                        apply_daily_traffic(info, stat_t, current_date_key());

                        // uptime str
                        let day = stat_t.uptime / (3600 * 24);
                        if day > 0 {
                            stat_t.uptime_str = format!("{day} 天");
                        } else {
                            stat_t.uptime_str = format!(
                                "{:02}:{:02}:{:02}",
                                stat_t.uptime / 3600,
                                (stat_t.uptime / 60) % 60,
                                stat_t.uptime % 60
                            );
                        }

                        info!("update stat `{stat_t:?}");
                        if let Ok(mut host_stat_map) = stat_map.lock() {
                            let mut notify_up = false;
                            if let Some(pre_stat) = host_stat_map.get(&stat_t.name) {
                                if stat_t.ip_info.is_none() {
                                    stat_t.ip_info = pre_stat.ip_info.clone();
                                }

                                if stat_t.notify && (pre_stat.latest_ts + cfg.offline_threshold < stat_t.latest_ts) {
                                    notify_up = true;
                                }
                            }
                            let arc_stat = Arc::new(stat.into_owned());
                            if notify_up {
                                // node up notify
                                notifier_tx.send((Event::NodeUp, Arc::clone(&arc_stat)));
                            }
                            host_stat_map.insert(arc_stat.name.clone(), arc_stat);
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
                let mut any_notified = false;

                // group gc
                if latest_group_gc + cfg.group_gc < now {
                    latest_group_gc = now;
                    //
                    if let Ok(mut hm) = hosts_map.lock() {
                        hm.retain(|_, o| o.gid.is_empty() || o.latest_ts + cfg.group_gc >= now);
                    }
                    //
                    if let Ok(mut sm) = stat_map.lock() {
                        sm.retain(|_, o| o.gid.is_empty() || o.latest_ts + cfg.group_gc >= now);
                    }
                }

                if let Ok(mut host_stat_map) = stat_map.lock() {
                    for (_, stat) in host_stat_map.iter_mut() {
                        if stat.disabled {
                            resp.servers.push(Arc::clone(stat));
                            continue;
                        }
                        let notify_event = {
                            let o = Arc::make_mut(stat);
                            // 30s 下线
                            if o.latest_ts + cfg.offline_threshold < now {
                                o.online4 = false;
                                o.online6 = false;
                            }

                            // labels
                            if !o.labels.contains("os=") {
                                if let Some(sys_info) = &o.sys_info {
                                    let os_r = sys_info.os_release.to_lowercase();
                                    for s in &OS_LIST {
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

                            // determine notify event (o is dropped after this block)
                            if o.notify && latest_notify_ts + cfg.notify_interval < now {
                                if o.online4 || o.online6 {
                                    Some(Event::Custom)
                                } else {
                                    o.disabled = true;
                                    Some(Event::NodeDown)
                                }
                            } else {
                                None
                            }
                        };

                        // client notify — Arc::clone is O(1), no HostStat copy
                        if let Some(event) = notify_event {
                            notifier_tx.send((event, Arc::clone(stat)));
                            any_notified = true;
                        }

                        resp.servers.push(Arc::clone(stat));
                    }
                    if any_notified {
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
                            file.write_all(serde_json::to_string(&resp).unwrap().as_bytes());
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
                let notify_list = &*notifies.lock().unwrap();
                trace!("recv notify => {e:?}, {stat:?}");
                for n in notify_list {
                    trace!("{} notify {:?} => {:?}", n.kind(), e, stat);
                    n.notify(&e, &stat);
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

    #[allow(clippy::unused_self)]
    #[allow(clippy::unnecessary_wraps)]
    pub fn report(&self, data: serde_json::Value) -> Result<()> {
        static SENDER: LazyLock<SyncSender<Cow<'static, HostStat>>> =
            LazyLock::new(|| STAT_SENDER.get().unwrap().clone());

        match serde_json::from_value(data) {
            Ok(stat) => {
                trace!("send stat => {stat:?} ");
                SENDER.send(Cow::Owned(stat));
            }
            Err(err) => {
                error!("report error => {err:?}");
            }
        }
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
        }

        Ok(resp_json)
    }
}

#[cfg(test)]
mod tests {
    use super::apply_daily_traffic;
    use crate::config::Host;
    use crate::payload::HostStat;

    #[test]
    fn daily_traffic_uses_same_day_baseline() {
        let mut host = Host {
            daily_network_in_base: 100,
            daily_network_out_base: 200,
            daily_base_date: 20260813,
            ..Default::default()
        };
        let mut stat = HostStat {
            network_in: 150,
            network_out: 260,
            ..Default::default()
        };

        apply_daily_traffic(&mut host, &mut stat, 20260813);

        assert_eq!(stat.daily_network_in, 50);
        assert_eq!(stat.daily_network_out, 60);
    }

    #[test]
    fn daily_traffic_resets_baseline_on_date_change() {
        let mut host = Host {
            daily_network_in_base: 100,
            daily_network_out_base: 200,
            daily_base_date: 20260812,
            ..Default::default()
        };
        let mut stat = HostStat {
            network_in: 150,
            network_out: 260,
            ..Default::default()
        };

        apply_daily_traffic(&mut host, &mut stat, 20260813);

        assert_eq!(host.daily_network_in_base, 150);
        assert_eq!(host.daily_network_out_base, 260);
        assert_eq!(host.daily_base_date, 20260813);
        assert_eq!(stat.daily_network_in, 0);
        assert_eq!(stat.daily_network_out, 0);
    }

    #[test]
    fn daily_traffic_resets_baseline_when_counter_regresses() {
        let mut host = Host {
            daily_network_in_base: 500,
            daily_network_out_base: 600,
            daily_base_date: 20260813,
            ..Default::default()
        };
        let mut stat = HostStat {
            network_in: 100,
            network_out: 120,
            ..Default::default()
        };

        apply_daily_traffic(&mut host, &mut stat, 20260813);

        assert_eq!(host.daily_network_in_base, 100);
        assert_eq!(host.daily_network_out_base, 120);
        assert_eq!(stat.daily_network_in, 0);
        assert_eq!(stat.daily_network_out, 0);
    }

    #[test]
    fn vnstat_daily_values_are_authoritative_when_present() {
        let mut host = Host::default();
        let mut stat = HostStat {
            vnstat: true,
            network_in: 1_000,
            network_out: 2_000,
            daily_network_in: 100,
            daily_network_out: 200,
            ..Default::default()
        };

        apply_daily_traffic(&mut host, &mut stat, 20260813);

        assert_eq!(stat.daily_network_in, 100);
        assert_eq!(stat.daily_network_out, 200);
        assert_eq!(host.daily_network_in_base, 900);
        assert_eq!(host.daily_network_out_base, 1_800);
        assert_eq!(host.daily_base_date, 20260813);
    }

    #[test]
    fn old_vnstat_client_falls_back_to_server_baseline() {
        let mut host = Host {
            daily_network_in_base: 900,
            daily_network_out_base: 1_800,
            daily_base_date: 20260813,
            ..Default::default()
        };
        let mut stat = HostStat {
            vnstat: true,
            network_in: 1_000,
            network_out: 2_000,
            ..Default::default()
        };

        apply_daily_traffic(&mut host, &mut stat, 20260813);

        assert_eq!(stat.daily_network_in, 100);
        assert_eq!(stat.daily_network_out, 200);
    }
}
