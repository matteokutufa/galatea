use std::path::Path;
use std::process;
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use clap::{Arg, Command};
use anyhow::{Result, Context, anyhow};

mod config;
mod downloader;
mod executor;
mod stack;
mod task;
mod ui;
mod utils;
mod logger;

use crate::config::{Config, create_example_config};
use crate::ui::app::run_app;

fn main() -> Result<()> {
    // Configura i gestori di segnali
    setup_signal_handlers()?;

    // Parsing degli argomenti da linea di comando
    let matches = Command::new("Galatea")
        .version("0.1.0")
        .author("Galatea Team")
        .about("Strumento di installazione e configurazione server e workstation")
        .arg(Arg::new("config")
            .short('c')
            .long("config")
            .value_name("FILE")
            .help("Specifica un file di configurazione personalizzato"))
        .arg(Arg::new("create-example")
            .long("create-example")
            .value_name("FILE")
            .help("Crea un file di configurazione di esempio"))
        .arg(Arg::new("log-dir")
            .long("log-dir")
            .value_name("DIR")
            .help("Specifica una directory per i file di log"))
        .arg(Arg::new("no-root-check")
            .long("no-root-check")
            .help("Disabilita il controllo dei permessi di root"))
        .get_matches();

    // Configura il logger il prima possibile
    let log_dir = matches.get_one::<String>("log-dir")
        .map(|s| s.as_str())
        .unwrap_or("/var/log/galatea");

    // Inizializza il logger
    logger::init_file_logger(log_dir)?;
    log::info!("Galatea è stata avviata");

    // Verifica se l'applicazione è eseguita come root (a meno che --no-root-check sia specificato)
    if !matches.contains_id("no-root-check") && !utils::is_running_as_root() {
        log::error!("Galatea deve essere eseguito con privilegi di root");
        eprintln!("Errore: Galatea deve essere eseguito con privilegi di root.");
        eprintln!("Riprova con 'sudo galatea'");
        eprintln!("(Puoi disabilitare questo controllo con --no-root-check)");
        process::exit(1);
    }

    // Gestione dell'opzione per creare un file di configurazione di esempio
    if let Some(example_path) = matches.get_one::<String>("create-example") {
        log::info!("Tentativo di creare config di esempio in: {}", example_path);
        println!("Tentativo di creare config in: {}", example_path);
        
        let path = Path::new(example_path);
        if let Some(parent) = path.parent() {
            println!("Directory genitore: {:?}", parent);
            println!("Esiste directory genitore: {}", parent.exists());
            
            // Tenta di creare manualmente la directory
            match fs::create_dir_all(parent) {
                Ok(_) => println!("Directory creata con successo"),
                Err(e) => println!("Errore nella creazione directory: {}", e)
            }
        }
        
        match create_example_config(path) {
            Ok(_) => {
                log::info!("File di configurazione di esempio creato con successo in: {}", example_path);
                println!("File di configurazione di esempio creato con successo in: {}", example_path);
                process::exit(0);
            },
            Err(e) => {
                log::error!("Errore durante la creazione del file di configurazione di esempio: {}", e);
                eprintln!("Errore durante la creazione del file di configurazione di esempio: {}", e);
                process::exit(1);
            }
        }
    }

    // Caricamento della configurazione
    let config_path = matches.get_one::<String>("config").map(|s| s.as_str());
    let config = match Config::load(config_path) {
        Ok(config) => {
            log::info!("Configurazione caricata con successo");
            config
        },
        Err(e) => {
            log::error!("Errore durante il caricamento della configurazione: {}", e);
            eprintln!("Errore durante il caricamento della configurazione: {}", e);
            eprintln!("Prova ad eseguire il programma con l'opzione --create-example per creare una configurazione di esempio");
            process::exit(1);
        }
    };

    // Avvio dell'applicazione
    log::info!("Avvio dell'interfaccia utente");
    match run_app(config) {
        Ok(_) => {
            log::info!("Applicazione terminata con successo");
            println!("Applicazione terminata con successo");
        },
        Err(e) => {
            log::error!("Errore durante l'esecuzione dell'applicazione: {}", e);
            eprintln!("Errore durante l'esecuzione dell'applicazione: {}", e);
            process::exit(1);
        }
    }

    Ok(())
}

/// Configura i gestori di segnali
fn setup_signal_handlers() -> Result<()> {
    #[cfg(unix)]
    {
        use signal_hook::{consts::SIGINT, flag};
        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();
        
        flag::register(SIGINT, r).map_err(|e| anyhow!("Failed to register signal handler: {}", e))?;
        
        // For custom handler behavior, use signal_hook::iterator
        std::thread::spawn(move || {
            if !running.load(Ordering::SeqCst) {
                println!("\nRicevuto segnale di interruzione, chiusura in corso...");
                log::info!("Ricevuto segnale di interruzione, chiusura in corso...");
                std::process::exit(130); // Exit con codice standard per SIGINT
            }
        });
    }
    
    Ok(())
}
