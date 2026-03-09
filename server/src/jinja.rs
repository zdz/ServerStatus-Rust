use anyhow::Result;
use minijinja::{value::Value, Environment};
use once_cell::sync::Lazy;
use std::sync::Mutex;

pub static JINJA_ENV: Lazy<Mutex<Environment>> = Lazy::new(|| Mutex::new(Environment::new()));

pub fn add_template(kind: &str, tag: &str, tpl: String) {
    let name = format!("{kind}.{tag}");
    JINJA_ENV
        .lock()
        .as_mut()
        .map(|env| env.add_template_owned(name, tpl).unwrap())
        .unwrap();
}

pub fn render_template<'a>(kind: &'a str, tag: &'a str, ctx: Value, trim: bool) -> Result<String> {
    let name = format!("{kind}.{tag}");
    Ok(JINJA_ENV
        .lock().map_or_else(|err| {
            error!("render_template err => {err:?}");
            Ok(String::new())
        }, |e| {
            e.get_template(name.as_str()).map(|tmpl| {
                tmpl.render(ctx).map_or_else(|err| {
                        error!("tmpl.render err => {err:?}");
                        String::new()
                    }, |content| {
                        if trim {
                            return content
                                .split('\n')
                                .map(str::trim)
                                .filter(|&t| !t.is_empty())
                                .collect::<Vec<&str>>()
                                .join("\n");
                        }
                        content
                    })
            })
        })?)
}
