#![deny(warnings)]
use anyhow::Result;
use lettre::{
    message::{header, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use log::{error, info, trace};
use serde::{Deserialize, Serialize};

use crate::notifier;
use crate::notifier::Event;
use crate::notifier::HostStat;
use crate::notifier::Notifier;
use crate::notifier::NOTIFIER_HANDLE;

const KIND: &str = "email";

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Config {
    pub enabled: bool,
    pub server: String,
    pub username: String,
    pub password: String,
    pub to: String,
    pub subject: String,
    pub custom_tpl: String,
}

pub struct Email {
    config: &'static Config,
}

impl Email {
    pub fn new(cfg: &'static Config) -> Self {
        let o = Self { config: cfg };
        notifier::add_template(KIND, o.config.custom_tpl.as_str()).unwrap();
        o
    }

    fn custom_notify(&self, stat: &HostStat) -> Result<()> {
        trace!("{} custom_notify => {:?}", self.kind(), stat);

        notifier::render_template(KIND, stat).map(|content| {
            info!("tmpl.render => {}", content);
            if !content.is_empty() {
                self.send_msg(format!("❗<b>Server Status</b>\n{}", content))
                    .unwrap_or_else(|err| {
                        error!("send_msg err => {:?}", err);
                    });
            }
        })
    }

    fn send_msg(&self, html_content: String) -> Result<()> {
        let email = Message::builder()
            .from(
                format!("ServerStatus <{}>", self.config.username)
                    .parse()
                    .unwrap(),
            )
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

        let creds = Credentials::new(
            self.config.username.to_string(),
            self.config.password.to_string(),
        );

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
}

impl crate::notifier::Notifier for Email {
    fn kind(&self) -> &'static str {
        KIND
    }

    fn notify(&self, e: &Event, stat: &HostStat) -> Result<()> {
        trace!("{} notify {:?} => {:?}", self.kind(), e, stat);
        match *e {
            Event::NodeUp => {
                let content = format!("❗<b>Server Status</b>\n❗ {} 主机上线 🟢", stat.name);
                let _ = self.send_msg(content);
            }
            Event::NodeDown => {
                let content = format!("❗<b>Server Status</b>\n❗ {} 主机下线 🔴", stat.name);
                let _ = self.send_msg(content);
            }
            Event::Custom => {
                let _ = self.custom_notify(stat);
            }
        }

        Ok(())
    }
}
