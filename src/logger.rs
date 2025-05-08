//! Modulo per la gestione centralizzata dei log su file
//!
//! Questo modulo fornisce funzionalità per scrivere i log su file invece che su console.

use std::path::{Path, PathBuf};
use std::fs::{self, File, OpenOptions};
use std::io::{Write, Read, BufReader, BufRead};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use anyhow::{Context, Result};
use chrono::Local;
use lazy_static::lazy_static;

// Singleton per il file di log e il percorso del file corrente
lazy_static! {
    static ref LOG_FILE: Mutex<Option<File>> = Mutex::new(None);
    static ref LOG_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
    static ref LOG_DIR: Mutex<Option<String>> = Mutex::new(None);
    static ref LOG_INITIALIZED: AtomicBool = AtomicBool::new(false);
}

/// Inizializza il sistema di logging su file (solo su file, non su console)
pub fn init_file_logger(log_dir: &str) -> Result<()> {
    // Verifica se il logger è già stato inizializzato
    if LOG_INITIALIZED.load(Ordering::SeqCst) {
        // Il logger è già inizializzato, non fare nulla
        return Ok(());
    }

    // Crea la directory dei log se non esiste
    fs::create_dir_all(log_dir).context("Failed to create log directory")?;

    // Salva la directory di log per riferimento futuro
    {
        let mut log_dir_guard = LOG_DIR.lock().unwrap();
        *log_dir_guard = Some(log_dir.to_string());
    }

    // Crea il nome del file di log con timestamp
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let log_file_path = Path::new(log_dir).join(format!("galatea_{}.log", timestamp));

    // Imposta il percorso del file di log corrente
    {
        let mut log_path_guard = LOG_PATH.lock().unwrap();
        *log_path_guard = Some(log_file_path.clone());
    }

    // Apri il file in modalità append
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)
        .context("Failed to open log file")?;

    // Salva il file nel singleton
    {
        let mut log_file_guard = LOG_FILE.lock().unwrap();
        *log_file_guard = Some(file);
    }

    // Configura il logger per scrivere solo sul file (non su stdout)
    env_logger::Builder::from_default_env()
        .format(|buf, record| {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
            writeln!(
                buf,
                "[{}] {} {}: {}",
                timestamp,
                record.level(),
                record.module_path().unwrap_or("unknown"),
                record.args()
            )
        })
        .init();

    // Inizializza il logger
    log::info!("Logger initialized, writing to: {:?}", log_file_path);

    // Imposta il flag di inizializzazione
    LOG_INITIALIZED.store(true, Ordering::SeqCst);

    // Logga l'inizio della sessione
    log_to_file(&format!("=== Galatea session started at {} ===", Local::now().format("%Y-%m-%d %H:%M:%S")))?;

    Ok(())
}

/// Ottiene la directory dei log
pub fn get_log_directory() -> Option<String> {
    LOG_DIR.lock().unwrap().clone()
}

/// Ottiene il percorso completo del file di log corrente
pub fn get_current_log_path() -> Option<PathBuf> {
    LOG_PATH.lock().unwrap().clone()
}

/// Scrive un messaggio di log manualmente
pub fn log_to_file(message: &str) -> Result<()> {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    let formatted = format!("[{}] INFO: {}\n", timestamp, message);

    // Scrivi sul file di log
    if let Ok(log_file_guard) = LOG_FILE.lock() {
        if let Some(mut file) = log_file_guard.as_ref() {
            file.write_all(formatted.as_bytes())
                .context("Failed to write to log file")?;
            file.flush().context("Failed to flush log file")?;
        }
    }

    // Non scrivere sulla console (rimosso)
    // println!("{}", formatted.trim());

    Ok(())
}

/// Ottiene il contenuto recente del file di log
pub fn get_recent_logs(lines: usize) -> Result<Vec<String>> {
    if let Ok(log_path_guard) = LOG_PATH.lock() {
        if let Some(log_path) = log_path_guard.as_ref() {
            // Il file di log esiste, leggi le ultime `lines` righe
            let file = File::open(log_path).context("Failed to open log file")?;
            let buf_reader = BufReader::new(file);
            
            // Leggi tutte le righe in memoria
            let all_lines: Vec<String> = buf_reader
                .lines()
                .map(|line| line.unwrap_or_else(|_| String::from("Error reading line")))
                .collect();

            // Prendi le ultime `lines` righe
            let start_idx = if all_lines.len() > lines { all_lines.len() - lines } else { 0 };
            return Ok(all_lines[start_idx..].to_vec());
        }
    }

    // Se il file di log non esiste o non può essere letto, restituisci un vettore vuoto
    Ok(Vec::new())
}

/// Ottiene l'elenco di tutti i file di log nella directory dei log
pub fn get_log_files() -> Result<Vec<PathBuf>> {
    let mut log_files = Vec::new();

    if let Ok(log_dir_guard) = LOG_DIR.lock() {
        if let Some(log_dir) = log_dir_guard.as_ref() {
            let dir_path = Path::new(log_dir);
            
            // Verifica che la directory esista
            if dir_path.exists() && dir_path.is_dir() {
                // Leggi tutti i file nella directory
                for entry in fs::read_dir(dir_path).context("Failed to read log directory")? {
                    let entry = entry.context("Failed to read directory entry")?;
                    let path = entry.path();
                    
                    // Aggiungi solo i file .log
                    if path.is_file() && path.extension().map_or(false, |ext| ext == "log") {
                        log_files.push(path);
                    }
                }
            }
        }
    }

    // Ordina i file per data di modifica (dal più recente al più vecchio)
    log_files.sort_by(|a, b| {
        if let (Ok(meta_a), Ok(meta_b)) = (fs::metadata(a), fs::metadata(b)) {
            if let (Ok(time_a), Ok(time_b)) = (meta_a.modified(), meta_b.modified()) {
                return time_b.cmp(&time_a);
            }
        }
        b.file_name().cmp(&a.file_name())  // Fallback: ordina per nome file in ordine inverso
    });

    Ok(log_files)
}

/// Legge il contenuto di un file di log
pub fn read_log_file(path: &Path) -> Result<String> {
    let mut content = String::new();
    let mut file = File::open(path).context("Failed to open log file")?;
    file.read_to_string(&mut content).context("Failed to read log file")?;
    Ok(content)
}

/// Implementazione di un logger personalizzato che scrive solo su file (non su console)
pub struct FileAndConsoleLogger;

impl log::Log for FileAndConsoleLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Debug
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

            // Scrivi sul file di log
            if let Ok(log_file_guard) = LOG_FILE.lock() {
                if let Some(mut file) = log_file_guard.as_ref() {
                    let _ = file.write_all(formatted.as_bytes());
                    let _ = file.flush();
                }
            }

            // Non scrivere sulla console (rimosso)
            // println!("{}", formatted.trim());
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