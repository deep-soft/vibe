use eyre::{Context, Result};
use serde_json::Value;
use std::{fs::OpenOptions, path::PathBuf};
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_store::Store;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Layer, Registry};

use crate::{cmd::is_portable, config, utils::get_current_dir};

fn get_log_path(app: &AppHandle) -> Result<PathBuf> {
    let config_path = if is_portable() {
        get_current_dir()?
    } else {
        app.path().app_config_dir()?
    };
    let mut log_path = config_path.join(format!("{}.txt", config::LOG_FILENAME_PREFIX));
    let mut count = 0;
    while log_path.exists() {
        log_path = config_path.join(format!("{}_{}.txt", config::LOG_FILENAME_PREFIX, count));
        count += 1;
    }
    Ok(log_path)
}

pub fn setup_logging(app: &AppHandle, store: Store<Wry>) -> Result<()> {
    let sub = Registry::default().with(
        tracing_subscriber::fmt::layer()
            .with_ansi(true)
            .with_filter(EnvFilter::from_default_env()),
    );

    if store
        .get("prefs_log_to_file")
        .unwrap_or(&Value::Bool(false))
        .as_bool()
        .unwrap_or_default()
    {
        // with log to file
        let path = get_log_path(app)?;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path.clone())
            .context(format!("failed to open file at {}", path.display()))?;

        tracing::subscriber::set_global_default(sub.with(tracing_subscriber::fmt::layer().json().with_writer(file)))?;
    } else {
        tracing::subscriber::set_global_default(sub)?;
    }
    Ok(())
}