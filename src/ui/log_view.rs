//! Visualizzazione e gestione dei log nell'interfaccia utente
//!
//! Questo modulo fornisce la visualizzazione dei log di sistema.

use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::thread;

use cursive::Cursive;
use cursive::views::{Dialog, TextView, LinearLayout, DummyView, Panel, Button, ScrollView};
use cursive::view::Scrollable;
use cursive::traits::*;
use cursive::theme::{BaseColor, Color, ColorStyle};
use cursive::align::HAlign;

use crate::config::Config;
use crate::logger;

// Dimensioni standard per le finestre
const WINDOW_WIDTH: usize = 80;
const WINDOW_HEIGHT: usize = 24;
const PANEL_WIDTH: usize = 78;
const LOG_HEIGHT: usize = 10;

/// Struttura per contenere lo stato della visualizzazione dei log
pub struct LogState {
    pub log_dir: String,
    pub current_log_file: Option<String>,
    pub auto_refresh: bool,
}

impl LogState {
    pub fn new(log_dir: String) -> Self {
        LogState {
            log_dir,
            current_log_file: None,
            auto_refresh: false,
        }
    }

    pub fn get_log_files(&self) -> Vec<String> {
        let mut log_files = Vec::new();
        
        if let Ok(entries) = fs::read_dir(&self.log_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() && path.extension().map_or(false, |ext| ext == "log") {
                        if let Some(file_name) = path.file_name() {
                            log_files.push(file_name.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
        
        // Ordina i file per nome (in ordine decrescente, per avere i più recenti prima)
        log_files.sort_by(|a, b| b.cmp(a));
        
        log_files
    }

    pub fn get_log_content(&self, file_name: &str) -> String {
        let path = Path::new(&self.log_dir).join(file_name);
        
        match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(e) => format!("Errore nella lettura del file di log: {}", e),
        }
    }
}

/// Crea la vista per la visualizzazione dei log
pub fn create_log_view(siv: &mut Cursive) {
    // Crea uno stato iniziale per la vista dei log
    let log_state = LogState::new("/var/log/galatea".to_string());
    
    // Ottieni l'elenco dei file di log
    let log_files = log_state.get_log_files();
    
    // Contenuto iniziale
    let initial_content = if let Some(first_log) = log_files.first() {
        log_state.get_log_content(first_log)
    } else {
        "Nessun file di log trovato".to_string()
    };

    // Crea la vista di testo per i log
    let log_text = TextView::new(initial_content)
        .with_name("log_content")
        .scrollable();

    // Crea il selettore dei file di log
    let mut log_selector = LinearLayout::horizontal()
        .child(TextView::new("File: "));

    // Aggiungi pulsanti per ogni file di log
    for log_file in &log_files {
        let file_name = log_file.clone();
        let file_name_for_button = file_name.clone(); // Clone for button label
        let file_name_for_closure = file_name.clone(); // Clone for closure
        log_selector = log_selector.child(Button::new_raw(&file_name_for_button, move |s| {
            let log_content = {
                let log_dir = "/var/log/galatea".to_string();
                let log_state = LogState::new(log_dir);
                log_state.get_log_content(&file_name_for_closure)
            };
            
            s.call_on_name("log_content", |view: &mut TextView| {
                view.set_content(log_content);
            });
        }));
        log_selector = log_selector.child(DummyView.fixed_width(1));
    }

    // Layout principale
    let layout = LinearLayout::vertical()
        .child(log_selector)
        .child(DummyView.fixed_height(1))
        .child(Panel::new(log_text)
            .title("Contenuto del log")
            .fixed_width(PANEL_WIDTH)
            .fixed_height(LOG_HEIGHT * 2));

    // Aggiungi la vista alla UI
    siv.add_layer(Dialog::around(layout)
        .title("Visualizzazione Log")
        .button("Aggiorna", |s| {
            // Ricarica il contenuto del log corrente
            if let Some(first_log) = LogState::new("/var/log/galatea".to_string()).get_log_files().first() {
                let file_name = first_log.clone();
                let log_content = {
                    let log_dir = "/var/log/galatea".to_string();
                    let log_state = LogState::new(log_dir);
                    log_state.get_log_content(&file_name)
                };
                
                s.call_on_name("log_content", |view: &mut TextView| {
                    view.set_content(log_content);
                });
            }
        })
        .button("Attiva Auto-Refresh", |s| {
            // Configura un timer che aggiorna i log ogni 2 secondi
            let cb_sink = s.cb_sink().clone();
            thread::spawn(move || {
                loop {
                    thread::sleep(Duration::from_secs(2));
                    
                    // Invia un callback per aggiornare i log
                    if let Some(first_log) = LogState::new("/var/log/galatea".to_string()).get_log_files().first() {
                        let file_name = first_log.clone();
                        let log_content = {
                            let log_dir = "/var/log/galatea".to_string();
                            let log_state = LogState::new(log_dir);
                            log_state.get_log_content(&file_name)
                        };
                        
                        // Aggiorna la vista dei log
                        let content = log_content.clone();
                        if let Err(e) = cb_sink.send(Box::new(move |s| {
                            s.call_on_name("log_content", |view: &mut TextView| {
                                view.set_content(content.clone());
                            });
                        })) {
                            break; // Interrompi il loop se c'è un errore
                        }
                    }
                }
            });
            
            s.add_layer(Dialog::info("Auto-refresh dei log attivato")
                         .fixed_width(50)
                         .fixed_height(7));
        })
        .button("Chiudi", |s| { s.pop_layer(); })
        .fixed_width(WINDOW_WIDTH)
        .fixed_height(WINDOW_HEIGHT));
}

/// Legge i log recenti e li formatta per la visualizzazione
pub fn read_recent_logs() -> String {
    // Percorso della directory dei log
    let log_dir = "/var/log/galatea";
    
    // Ottieni l'elenco dei file di log
    let log_state = LogState::new(log_dir.to_string());
    let log_files = log_state.get_log_files();
    
    // Se non ci sono file di log, restituisci un messaggio
    if log_files.is_empty() {
        return "Nessun file di log trovato".to_string();
    }
    
    // Prendi il file di log più recente
    let most_recent_log = &log_files[0];
    
    // Leggi il contenuto del file
    let content = log_state.get_log_content(most_recent_log);
    
    // Prendi le ultime 50 righe (o meno se il file è più corto)
    let lines: Vec<&str> = content.lines().collect();
    let start_idx = if lines.len() > 50 { lines.len() - 50 } else { 0 };
    
    // Formatta le righe
    lines[start_idx..].join("\n")
}

/// Crea una finestra popup per mostrare i log recenti
pub fn show_recent_logs_popup(siv: &mut Cursive) {
    let recent_logs = read_recent_logs();
    
    siv.add_layer(Dialog::around(TextView::new(recent_logs).scrollable())
        .title("Log recenti")
        .button("Chiudi", |s| { s.pop_layer(); })
        .button("Visualizza tutti i log", |s| {
            s.pop_layer();
            create_log_view(s);
        })
        .fixed_width(WINDOW_WIDTH - 10)
        .fixed_height(WINDOW_HEIGHT - 5));
}


