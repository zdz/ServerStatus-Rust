use anyhow::Result;
use minijinja::{value::Value, Environment, Source};
use once_cell::sync::Lazy;
use std::sync::Mutex;

pub static JINJA_ENV: Lazy<Mutex<Environment>> = Lazy::new(|| Mutex::new(Environment::new()));

pub fn add_template<K, T, S>(kind: K, tag: T, tpl: S)
where
    K: Into<String> + std::fmt::Display,
    T: Into<String> + std::fmt::Display,
    S: Into<String>,
{
    let name = format!("{kind}.{tag}");
    JINJA_ENV
        .lock()
        .as_mut()
        .map(|env| {
            let mut s = env.source().unwrap_or(&Source::new()).to_owned();
            s.add_template(name, tpl).unwrap();
            env.set_source(s);
        })
        .unwrap();
}

pub fn render_template<'a>(kind: &'a str, tag: &'a str, ctx: Value, trim: bool) -> Result<String> {
    let name = format!("{kind}.{tag}");
    Ok(JINJA_ENV
        .lock()
        .map(|e| {
            e.get_template(name.as_str()).map(|tmpl| {
                tmpl.render(ctx)
                    .map(|content| {
                        if trim {
                            return content
                                .split('\n')
                                .map(|t| t.trim())
                                .filter(|&t| !t.is_empty())
                                .collect::<Vec<&str>>()
                                .join("\n");
                        }
                        content
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
