//! Modulo per la gestione centralizzata dei log su file
//!
//! Questo modulo fornisce funzionalità per scrivere i log su file invece che su console.

use std::path::Path;
use std::fs::{self, File};
use std::io::Write;
use std::sync::Mutex;
use anyhow::{Context, Result};
use chrono::Local;
use lazy_static::lazy_static;

// Singleton per il file di log
lazy_static! {
    static ref LOG_FILE: Mutex<Option<File>> = Mutex::new(None);
}

/// Inizializza il sistema di logging su file (solo su file, non su console)
pub fn init_file_logger(log_dir: &str) -> Result<()> {
    // Crea la directory dei log se non esiste
    fs::create_dir_all(log_dir).context("Failed to create log directory")?;

    // Crea il nome del file di log con timestamp
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let log_file_path = Path::new(log_dir).join(format!("galatea_{}.log", timestamp));

    // Apri il file in modalità append
    let file = File::options()
        .create(true)
        .append(true)
        .open(&log_file_path)
        .context("Failed to open log file")?;

    // Salva il file nel singleton
    let mut log_file_guard = LOG_FILE.lock().unwrap();
    *log_file_guard = Some(file);

    // Configura il logger per scrivere SOLO sul file, non su stdout
    log::set_boxed_logger(Box::new(FileLogger))
        .map(|()| log::set_max_level(log::LevelFilter::Info))
        .context("Failed to set logger")?;

    log::info!("Logger initialized, writing to: {:?}", log_file_path);
    Ok(())
}

/// Scrive un messaggio di log manualmente
pub fn log_to_file(message: &str) -> Result<()> {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    let formatted = format!("[{}] {}\n", timestamp, message);

    if let Ok(log_file_guard) = LOG_FILE.lock() {
        if let Some(mut file) = log_file_guard.as_ref() {
            file.write_all(formatted.as_bytes())
                .context("Failed to write to log file")?;
            file.flush().context("Failed to flush log file")?;
        }
    }

    Ok(())
}

/// Ottiene il contenuto recente del file di log
pub fn get_recent_logs(lines: usize) -> Result<Vec<String>> {
    if let Ok(log_file_guard) = LOG_FILE.lock() {
        if let Some(file) = log_file_guard.as_ref() {
            // Per ottenere il path del file di log, possiamo usare una tecnica alternativa
            // oppure mantenerlo in memoria durante l'inizializzazione
            // Per ora restituiamo un vettore vuoto
            return Ok(Vec::new());
        }
    }

    Ok(Vec::new())
}

/// Implementazione di un logger personalizzato che scrive solo su file
struct FileLogger;

impl log::Log for FileLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
            let formatted = format!(
                "[{}] {} {}: {}\n",
                timestamp,
                record.level(),
                record.module_path().unwrap_or("unknown"),
                record.args()
            );

            if let Ok(log_file_guard) = LOG_FILE.lock() {
                if let Some(mut file) = log_file_guard.as_ref() {
                    let _ = file.write_all(formatted.as_bytes());
                    let _ = file.flush();
                }
            }
        }
    }

    fn flush(&self) {
        if let Ok(log_file_guard) = LOG_FILE.lock() {
            if let Some(mut file) = log_file_guard.as_ref() {
                let _ = file.flush();
            }
        }
    }
}