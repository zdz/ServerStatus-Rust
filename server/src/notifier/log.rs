#![deny(warnings)]
use anyhow::Result;
use chrono::Local;
use minijinja::context;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

use crate::jinja::{add_template, render_template};
use crate::notifier::{Event, HostStat, NOTIFIER_HANDLE};

const KIND: &str = "log";

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Config {
    pub enabled: bool,
    pub log_dir: String,
    pub tpl: String,
}

pub struct Log {
    config: &'static Config,
}

impl Log {
    pub fn new(cfg: &'static Config) -> Self {
        let o = Self { config: cfg };

        add_template(KIND, "tpl", o.config.tpl.to_string());

        // build dir
        fs::create_dir_all(&cfg.log_dir).unwrap_or_else(|_| panic!("can't create dir `{}", cfg.log_dir));
        o
    }
}

impl crate::notifier::Notifier for Log {
    fn kind(&self) -> &'static str {
        KIND
    }

    fn send_notify(&self, content: String) -> Result<()> {
        if content.is_empty() {
            return Ok(());
        }

        let dt = Local::now().format("%Y-%m-%d").to_string();
        let log_file = Path::new(&self.config.log_dir)
            .join(format!("ssr.log.{dt}"))
            .to_string_lossy()
            .to_string();

        let handle = NOTIFIER_HANDLE.lock().unwrap().as_ref().unwrap().clone();
        handle.spawn(async move {
            //
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .append(true)
                .open(&log_file)
                .await
                .unwrap_or_else(|_| panic!("can't create log `{log_file}"));

            let _ = file
                .write(content.as_bytes())
                .await
                .unwrap_or_else(|_| panic!("can't write log `{log_file}"));

            if !content.ends_with('\n') {
                let _ = file
                    .write(b"\n")
                    .await
                    .unwrap_or_else(|_| panic!("can't write log `{log_file}"));
            }

            file.flush()
                .await
                .unwrap_or_else(|_| panic!("can't flush log `{log_file}"));
        });
        Ok(())
    }

    fn notify(&self, e: &Event, stat: &HostStat) -> Result<()> {
        render_template(
            self.kind(),
            "tpl",
            context!(event => e, host => stat, config => self.config, ip_info => stat.ip_info, sys_info => stat.sys_info),
            true,
        )
        .map(|content| self.send_notify(content).unwrap())
    }
}
