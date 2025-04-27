use std::path::Path;
use std::process;
use clap::{Arg, Command};
use anyhow::{Result, Context};
use log::{info, error, LevelFilter};
use env_logger::Builder;

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
    // Configurazione del logger
    let mut builder = Builder::new();
    builder.filter_level(LevelFilter::Info).init();

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
        match create_example_config(Path::new(example_path)) {
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
            info!("Configurazione caricata con successo");
            config
        },
        Err(e) => {
            error!("Errore durante il caricamento della configurazione: {}", e);
            eprintln!("Errore durante il caricamento della configurazione: {}", e);
            eprintln!("Prova ad eseguire il programma con l'opzione --create-example per creare una configurazione di esempio");
            process::exit(1);
        }
    };

    // Avvio dell'applicazione
    run_app(config).context("Errore durante l'esecuzione dell'applicazione")?;

    Ok(())
}