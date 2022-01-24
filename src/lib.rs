#![allow(unused)]
mod config;
mod notifier;
mod payload;

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;
