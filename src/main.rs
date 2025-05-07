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

    // Verifica se l'applicazione è eseguita come root
    if !utils::is_running_as_root() {
        eprintln!("Errore: Galatea deve essere eseguito con privilegi di root.");
        eprintln!("Riprova con 'sudo galatea'");
        process::exit(1);
    }

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
        .get_matches();

    // Gestione dell'opzione per creare un file di configurazione di esempio
    if let Some(example_path) = matches.get_one::<String>("create-example") {
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
                println!("File di configurazione di esempio creato con successo in: {}", example_path);
                process::exit(0);
            },
            Err(e) => {
                eprintln!("Errore durante la creazione del file di configurazione di esempio: {}", e);
                process::exit(1);
            }
        }
    }

    // Caricamento della configurazione
    let config_path = matches.get_one::<String>("config").map(|s| s.as_str());
    let config = match Config::load(config_path) {
        Ok(config) => {
            config
        },
        Err(e) => {
            eprintln!("Errore durante il caricamento della configurazione: {}", e);
            eprintln!("Prova ad eseguire il programma con l'opzione --create-example per creare una configurazione di esempio");
            process::exit(1);
        }
    };

    // Avvio dell'applicazione
    run_app(config).context("Errore durante l'esecuzione dell'applicazione")?;

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
                std::process::exit(130); // Exit con codice standard per SIGINT
            }
        });
    }
    
    Ok(())
}