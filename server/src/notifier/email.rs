#![deny(warnings)]
use anyhow::Result;
use lettre::{
    message::{header, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use log::{error, info};
use minijinja::context;
use serde::{Deserialize, Serialize};

use crate::jinja::{add_template, render_template};
use crate::notifier::{get_tag, Event, HostStat, NOTIFIER_HANDLE};

const KIND: &str = "email";

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Config {
    pub enabled: bool,
    pub server: String,
    pub username: String,
    pub password: String,
    pub to: String,
    pub subject: String,
    pub title: String,
    pub online_tpl: String,
    pub offline_tpl: String,
    pub custom_tpl: String,
}

pub struct Email {
    config: &'static Config,
}

impl Email {
    pub fn new(cfg: &'static Config) -> Self {
        let o = Self { config: cfg };
        add_template(KIND, get_tag(&Event::NodeUp), o.config.online_tpl.to_string());
        add_template(KIND, get_tag(&Event::NodeDown), o.config.offline_tpl.to_string());
        add_template(KIND, get_tag(&Event::Custom), o.config.custom_tpl.to_string());
        o
    }
}

impl crate::notifier::Notifier for Email {
    fn kind(&self) -> &'static str {
        KIND
    }

    fn send_notify(&self, html_content: String) -> Result<()> {
        let email = Message::builder()
            .from(format!("ServerStatus <{}>", self.config.username).parse().unwrap())
            .to(self.config.to.parse().unwrap())
            .subject(self.config.subject.to_string())
            .multipart(
                MultiPart::alternative().singlepart(
                    SinglePart::builder()
                        .header(header::ContentType::TEXT_HTML)
                        .body(html_content),
                ),
            )
            .unwrap();

        let creds = Credentials::new(self.config.username.to_string(), self.config.password.to_string());

        let smtp_server = self.config.server.to_string();
        let handle = NOTIFIER_HANDLE.lock().unwrap().as_ref().unwrap().clone();
        handle.spawn(async move {
            // Open a remote connection to gmail
            let mailer: AsyncSmtpTransport<Tokio1Executor> =
                AsyncSmtpTransport::<Tokio1Executor>::relay(smtp_server.as_str())
                    .unwrap()
                    .credentials(creds)
                    .build();

            // Send the email
            match mailer.send(email).await {
                Ok(_) => {
                    info!("Email sent successfully!");
                }
                Err(err) => {
                    error!("Could not send email: {:?}", err);
                }
            }
        });

        Ok(())
    }

    fn notify(&self, e: &Event, stat: &HostStat) -> Result<()> {
        render_template(
            self.kind(),
            get_tag(e),
            context!(host => stat, config => self.config, ip_info => stat.ip_info, sys_info => stat.sys_info),
            true,
        )
        .map(|content| match *e {
            Event::NodeUp | Event::NodeDown => self.send_notify(content).unwrap(),
            Event::Custom => {
                info!("render.custom.tpl => {}", content);
                if !content.is_empty() {
                    self.send_notify(format!("{}\n{}", self.config.title, content))
                        .unwrap_or_else(|err| {
                            error!("send_msg err => {:?}", err);
                        });
                }
            }
        })
    }
}
