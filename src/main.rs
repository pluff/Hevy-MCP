use std::env;
use std::io::stderr;
use std::net::AddrParseError;

use thiserror::Error;
use tracing::{Level, warn};
use tracing_subscriber::filter::Targets;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, registry};

use hevy_mcp_server::{HevyRouter, http};

#[tokio::main]
async fn main() -> Result<(), AppError> {
    // LOG_LEVEL: error | warn | info | debug (default: info). Applies to this crate;
    // dependencies only log warnings and errors.
    let requested = env::var("LOG_LEVEL").ok();
    let (level, invalid_level) = match requested.as_deref().map(|v| v.trim().parse::<Level>()) {
        Some(Ok(level)) => (level, None),
        Some(Err(_)) => (Level::INFO, requested.clone()),
        None => (Level::INFO, None),
    };
    registry()
        .with(
            fmt::layer()
                .with_target(false)
                .with_writer(stderr)
                .with_filter(
                    Targets::new()
                        .with_default(Level::WARN)
                        .with_target("hevy_mcp_server", level),
                ),
        )
        .init();
    if let Some(value) = invalid_level {
        warn!("Invalid LOG_LEVEL {value:?}, using info (expected error, warn, info or debug)");
    }

    let router = HevyRouter::new();
    let addr = match env::var("HEVY_HTTP_ADDR") {
        Ok(value) => value,
        Err(env::VarError::NotPresent) => match env::var("PORT") {
            Ok(port) => format!("0.0.0.0:{port}"),
            Err(_) => "127.0.0.1:3000".to_string(),
        },
        Err(env::VarError::NotUnicode(_)) => {
            return Err(AppError::EnvVar(
                "HEVY_HTTP_ADDR must be valid UTF-8".to_string(),
            ));
        }
    };
    let addr = addr.parse().map_err(AppError::InvalidAddr)?;
    http::serve(router, addr)
        .await
        .map_err(|err| AppError::HttpServe(err.to_string()))?;

    Ok(())
}

#[derive(Error, Debug)]
enum AppError {
    #[error("{0}")]
    EnvVar(String),
    #[error("HEVY_HTTP_ADDR must be a valid socket address: {0}")]
    InvalidAddr(AddrParseError),
    #[error("HTTP server error: {0}")]
    HttpServe(String),
}
