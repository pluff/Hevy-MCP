use std::env;
use std::io::stderr;
use std::net::AddrParseError;

use thiserror::Error;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

use hevy_mcp_server::{HevyRouter, http};

#[tokio::main]
async fn main() -> Result<(), AppError> {
    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_writer(stderr)
        .init();

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
