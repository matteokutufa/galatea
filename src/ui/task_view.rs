//! Visualizzazione e gestione dei task nell'interfaccia utente
//!
//! Questo modulo fornisce la visualizzazione e l'interazione con i task.

use std::sync::{Arc, Mutex};
use std::collections::HashSet;

use anyhow::{Result, anyhow};

use cursive::Cursive;
use cursive::view::Scrollable;
use cursive::views::{Dialog, SelectView, TextView, LinearLayout, DummyView, Panel, TextContent, Button, OnEventView};
use cursive::views::NamedView;
use cursive::traits::*;
use cursive::align::HAlign;
use cursive::event::{Event, Key};

use crate::config::Config;
use crate::task::Task;
use crate::ui::app::{WINDOW_WIDTH, WINDOW_HEIGHT, PANEL_WIDTH, PANEL_HEIGHT};
use crate::ui::log_view;

// Implementazione della selezione multipla
struct TaskSelection {
    // Indici dei task selezionati
    selected_indices: HashSet<usize>,
}

impl TaskSelection {
    fn new() -> Self {
        TaskSelection {
            selected_indices: HashSet::new(),
        }
    }

    fn toggle(&mut self, idx: usize) {
        if self.selected_indices.contains(&idx) {
            self.selected_indices.remove(&idx);
        } else {
            self.selected_indices.insert(idx);
        }
    }

    fn is_selected(&self, idx: usize) -> bool {
        self.selected_indices.contains(&idx)
    }

    fn clear(&mut self) {
        self.selected_indices.clear();
    }

    fn count(&self) -> usize {
        self.selected_indices.len()
    }

    fn get_selected_indices(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = self.selected_indices.iter().cloned().collect();
        indices.sort();
        indices
    }
}

/// Crea la vista per la gestione dei task
pub fn create_task_view(siv: &mut Cursive, config: Arc<Mutex<Config>>, tasks: Arc<Mutex<Vec<Task>>>) -> Result<()> {
    let tasks_clone = Arc::clone(&tasks);

    // Ottiene i task dal mutex
    let tasks_guard = tasks.lock().map_err(|_| anyhow!("Failed to lock tasks mutex"))?;

    // Stato della selezione multipla
    let task_selection = Arc::new(Mutex::new(TaskSelection::new()));

    // Crea la vista normale di SelectView
    let mut select_view = SelectView::new()
        .h_align(HAlign::Left)
        .autojump();

    // Popola la vista con i task
    for (idx, task) in tasks_guard.iter().enumerate() {
        // Aggiungi il task alla vista
        let status = if task.installed { "[✓]" } else { "[ ]" };
        let task_type = format!("[{}]", task.script_type.get_letter());

        let task_line = format!("{} {} {} - {}",
                                status,
                                task_type,
                                task.name,
                                task.description
        );

        select_view.add_item(task_line, idx);
    }

    // Rilascia il lock prima di creare le closure
    drop(tasks_guard);

    // Descrizione dettagliata del task selezionato
    let task_detail = TextContent::new("Seleziona un task per vedere i dettagli");
    let task_detail_view = TextView::new_with_content(task_detail.clone())
        .scrollable();

    // Aggiungi handler per la selezione multipla con spazio
    let selection_clone = Arc::clone(&task_selection);

    // Avvolgi SelectView in OnEventView per gestire gli eventi personalizzati
    let select_view_with_events = OnEventView::new(select_view.with_name("task_list"))
    .on_event_inner(Event::Key(Key::Enter), move |view: &mut NamedView<SelectView<usize>>, event: &Event| {
        let mut view = view.get_mut(); // Access the inner SelectView
        if let Some(idx) = view.selected_id() {
            if let Ok(mut selection) = selection_clone.lock() {
                selection.toggle(idx);
                // Aggiorna l'interfaccia utente per mostrare la selezione
                let is_selected = selection.is_selected(idx);
                if let Some((item_label, _)) = view.get_item(idx) { // Nota: ho corretto get*item a get_item e * a _
                    let item_label = item_label.to_string();
                    let new_label = if is_selected {
                        format!("[*] {}", item_label.trim_start_matches("[ ] ").trim_start_matches("[✓] "))
                    } else {
                        if item_label.contains("[✓]") {
                            format!("[✓] {}", item_label.trim_start_matches("[*] ").trim_start_matches("[✓] "))
                        } else {
                            format!("[ ] {}", item_label.trim_start_matches("[*] ").trim_start_matches("[ ] "))
                        }
                    };
                    // Rimuovi e aggiungi l'item con il nuovo label
                    let value = view.selection().map(|i| *i);
                    view.remove_item(idx);
                    view.insert_item(idx, new_label, idx);
                    // Ripristina la selezione
                    if let Some(val) = value {
                        view.set_selection(val);
                    }
                }
            }
            Some(cursive::event::EventResult::Consumed(None))
        } else {
            None // Ritorna None se non abbiamo un indice selezionato
        }
    }).with_name("task_list");

    // Informazioni sulla selezione
    let selection_info = TextContent::new("Premi 'Spazio' per selezionare/deselezionare task. Nessun task selezionato.");
    let selection_info_view = TextView::new_with_content(selection_info.clone())
        .h_align(HAlign::Center);

    // Gestisci la selezione dei task
    let tasks_clone2 = Arc::clone(&tasks_clone);
    select_view.set_on_select(move |_siv, idx| {
        if let Ok(tasks_guard) = tasks_clone2.lock() {
            if let Some(task) = tasks_guard.get(*idx) {
                // Crea una descrizione dettagliata del task
                let mut details = format!("Nome: {}\n", task.name);
                details.push_str(&format!("Tipo: {} ({})\n", task.script_type.to_str(), task.script_type.get_letter()));
                details.push_str(&format!("Descrizione: {}\n", task.description));
                details.push_str(&format!("URL: {}\n", task.url));
                details.push_str(&format!("Stato: {}\n", if task.installed { "Installato" } else { "Non installato" }));

                if !task.dependencies.is_empty() {
                    details.push_str(&format!("Dipendenze: {}\n", task.dependencies.join(", ")));
                }

                if !task.tags.is_empty() {
                    details.push_str(&format!("Tag: {}\n", task.tags.join(", ")));
                }

                details.push_str(&format!("Richiede riavvio: {}\n", if task.requires_reboot { "Sì" } else { "No" }));

                if let Some(cmd) = &task.cleanup_command {
                    details.push_str(&format!("Comando di pulizia: {}\n", cmd));
                }

                if let Some(path) = &task.local_path {
                    details.push_str(&format!("Percorso locale: {:?}\n", path));
                }

                // Aggiorna la vista dei dettagli
                task_detail.set_content(details);
            }
        }
    });

    // Funzione per aggiornare la vista
    let update_task_view = {
        let tasks = Arc::clone(&tasks);
        let selection = Arc::clone(&task_selection);
        let selection_info_content = selection_info.clone();
        let select_view_cb = siv.cb_sink().clone();

        move || {
            if let Ok(tasks_guard) = tasks.lock() {
                // Crea una copia dei dati necessari
                let task_data: Vec<(bool, String, String, String)> = tasks_guard.iter()
                    .map(|t| (t.installed, t.script_type.get_letter().to_string(), t.name.clone(), t.description.clone()))
                    .collect();

                // Invia i dati al callback
                if let Err(e) = select_view_cb.send(Box::new(move |s: &mut Cursive| {
                    let selection_count = {
                        if let Ok(sel) = selection.lock() {
                            sel.count()
                        } else {
                            0
                        }
                    };

                    // Aggiorna il testo informativo sulla selezione
                    if selection_count > 0 {
                        selection_info_content.set_content(format!("Premi 'Spazio' per selezionare/deselezionare task. {} task selezionati.", selection_count));
                    } else {
                        selection_info_content.set_content("Premi 'Spazio' per selezionare/deselezionare task. Nessun task selezionato.".to_string());
                    }

                    s.call_on_name("task_list", |view: &mut SelectView<usize>| {
                        view.clear();

                        for (idx, (installed, type_letter, name, description)) in task_data.iter().enumerate() {
                            let is_selected = {
                                if let Ok(sel) = selection.lock() {
                                    sel.is_selected(idx)
                                } else {
                                    false
                                }
                            };

                            let status = if is_selected {
                                "[*]"
                            } else if *installed {
                                "[✓]"
                            } else {
                                "[ ]"
                            };

                            let task_type = format!("[{}]", type_letter);

                            let task_line = format!("{} {} {} - {}", status, task_type, name, description);
                            view.add_item(task_line, idx);
                        }
                    });
                })) {
                    eprintln!("Errore nell'aggiornamento della vista: {}", e);
                }
            }
        }
    };

    // Crea i bottoni per le azioni
    let config_clone = Arc::clone(&config);
    let tasks_clone = Arc::clone(&tasks);
    let update_clone = update_task_view.clone();
    let selection_clone = Arc::clone(&task_selection);

    let install_all_button = {
        let tasks = Arc::clone(&tasks_clone);
        let config = Arc::clone(&config_clone);
        let update_fn = update_clone.clone();
        let selection = Arc::clone(&selection_clone);

        Button::new("Install Selezionati", move |s| {
            // Ottieni gli indici dei task selezionati
            let selected_indices = {
                if let Ok(sel) = selection.lock() {
                    sel.get_selected_indices()
                } else {
                    vec![]
                }
            };

            if selected_indices.is_empty() {
                s.add_layer(Dialog::info("Nessun task selezionato")
                             .fixed_width(50)
                             .fixed_height(7));
                return;
            }

            // Chiedi conferma
            s.add_layer(Dialog::around(TextView::new(format!("Sei sicuro di voler installare {} task selezionati?", selected_indices.len())))
                .title("Conferma installazione")
                .button("No", |s| { s.pop_layer(); })
                .button("Sì", {
                    let tasks = Arc::clone(&tasks);
                    let config = Arc::clone(&config);
                    let update_fn = update_fn.clone();
                    let selected_indices = selected_indices.clone();
                    
                    move |s| {
                        s.pop_layer();
                        
                        // Mostra una finestra di progresso
                        let progress_text = TextContent::new("Inizializzazione dell'installazione...");
                        let progress_view = Dialog::around(TextView::new_with_content(progress_text.clone()))
                            .title("Installazione in corso")
                            .fixed_width(60)
                            .fixed_height(10);
                        
                        s.add_layer(progress_view);
                        
                        // Installa tutti i task selezionati
                        let mut success_count = 0;
                        let mut error_messages = Vec::new();
                        
                        for (i, idx) in selected_indices.iter().enumerate() {
                            let result = {
                                let mut tasks_guard = match tasks.lock() {
                                    Ok(guard) => guard,
                                    Err(e) => {
                                        error_messages.push(format!("Errore nel blocco dei task: {}", e));
                                        continue;
                                    }
                                };
                                
                                let task = match tasks_guard.get_mut(*idx) {
                                    Some(task) => task,
                                    None => {
                                        error_messages.push(format!("Task con indice {} non trovato", idx));
                                        continue;
                                    }
                                };
                                
                                // Verifica che il task non sia già installato
                                if task.installed {
                                    // Considera già come successo se è già installato
                                    success_count += 1;
                                    continue;
                                }
                                
                                // Aggiorna il messaggio di progresso
                                progress_text.set_content(format!("Installazione del task {} ({}/{})...", 
                                                                task.name, i+1, selected_indices.len()));
                                
                                // Crea un nuovo scope per limitare la durata del lock sulla configurazione
                                let install_result = {
                                    let config_guard = match config.lock() {
                                        Ok(guard) => guard,
                                        Err(e) => {
                                            error_messages.push(format!("Errore nel blocco della configurazione: {}", e));
                                            continue;
                                        }
                                    };
                                    
                                    // Esegui l'installazione
                                    let result = task.install(&config_guard);
                                    
                                    // Il lock su config_guard viene rilasciato qui
                                    result
                                };
                                
                                install_result
                            };
                            
                            match result {
                                Ok(_) => success_count += 1,
                                Err(e) => error_messages.push(format!("Errore nell'installazione del task {}: {}", idx, e)),
                            }
                        }
                        
                        // Rimuovi la finestra di progresso
                        s.pop_layer();
                        
                        // Mostra il risultato
                        if error_messages.is_empty() {
                            s.add_layer(Dialog::info(format!("Tutti i {} task sono stati installati con successo", success_count))
                                         .fixed_width(60)
                                         .fixed_height(10));
                        } else {
                            let mut result_message = format!("Installati con successo: {}/{}\n\nErrori:\n", 
                                                          success_count, selected_indices.len());
                            for error in &error_messages {
                                result_message.push_str(&format!("- {}\n", error));
                            }
                            
                            s.add_layer(Dialog::around(TextView::new(result_message).scrollable())
                                .title("Risultato installazione")
                                .button("OK", |s| { s.pop_layer(); })
                                .fixed_width(70)
                                .fixed_height(15));
                        }
                        
                        // Aggiorna la vista
                        update_fn();
                        
                        // Mostra i log recenti
                        log_view::show_recent_logs_popup(s);
                    }
                })
                .fixed_width(60)
                .fixed_height(10));
        })
    };

    let install_button = {
        let tasks = Arc::clone(&tasks_clone);
        let config = Arc::clone(&config_clone);
        let update_fn = update_clone.clone();

        Button::new("Install", move |s| {
            let idx = match s.call_on_name("task_list", |view: &mut SelectView<usize>| view.selected_id()) {
                Some(Some(idx)) => idx,
                _ => return,
            };

            // Ottieni il task selezionato
            let task_result = {
                let mut tasks_guard = match tasks.lock() {
                    Ok(guard) => guard,
                    Err(e) => {
                        s.add_layer(Dialog::info(format!("Errore nel blocco dei task: {}", e))
                                     .fixed_width(50)
                                     .fixed_height(7));
                        return;
                    }
                };

                let task = match tasks_guard.get_mut(idx) {
                    Some(task) => task,
                    None => {
                        s.add_layer(Dialog::info("Task non trovato")
                                     .fixed_width(50)
                                     .fixed_height(7));
                        return;
                    }
                };

                // Verifica che il task non sia già installato
                if task.installed {
                    s.add_layer(Dialog::info("Il task è già installato")
                                 .fixed_width(50)
                                 .fixed_height(7));
                    return;
                }

                // Crea un nuovo scope per limitare la durata del lock sulla configurazione
                let install_result = {
                    let config_guard = match config.lock() {
                        Ok(guard) => guard,
                        Err(e) => {
                            s.add_layer(Dialog::info(format!("Errore nel blocco della configurazione: {}", e))
                                         .fixed_width(50)
                                         .fixed_height(7));
                            return;
                        }
                    };

                    // Esegui l'installazione
                    let result = task.install(&config_guard);
                    
                    // Il lock su config_guard viene rilasciato qui, alla fine dello scope
                    result
                };

                install_result
            };

            // Gestisci il risultato dell'installazione
            match task_result {
                Ok(_) => {
                    s.add_layer(Dialog::info("Task installato con successo")
                                 .fixed_width(50)
                                 .fixed_height(7));
                    update_fn();
                    
                    // Mostra i log recenti
                    log_view::show_recent_logs_popup(s);
                },
                Err(e) => {
                    s.add_layer(Dialog::info(format!("Errore durante l'installazione del task: {}", e))
                                 .fixed_width(50)
                                 .fixed_height(7));
                }
            }
        })
    };

    let uninstall_button = {
        let tasks = Arc::clone(&tasks_clone);
        let config = Arc::clone(&config_clone);
        let update_fn = update_clone.clone();
        let selection = Arc::clone(&selection_clone);

        Button::new("Uninstall", move |s| {
            // Verifica se ci sono task selezionati
            let selected_indices = {
                if let Ok(sel) = selection.lock() {
                    sel.get_selected_indices()
                } else {
                    vec![]
                }
            };

            if !selected_indices.is_empty() {
                // Chiedi conferma per la disinstallazione multipla
                s.add_layer(Dialog::around(TextView::new(format!("Sei sicuro di voler disinstallare {} task selezionati?", selected_indices.len())))
                    .title("Conferma disinstallazione")
                    .button("No", |s| { s.pop_layer(); })
                    .button("Sì", {
                        let tasks = Arc::clone(&tasks);
                        let config = Arc::clone(&config);
                        let update_fn = update_fn.clone();
                        let selected_indices = selected_indices.clone();
                        
                        move |s| {
                            s.pop_layer();
                            
                            // Mostra una finestra di progresso
                            let progress_text = TextContent::new("Inizializzazione della disinstallazione...");
                            let progress_view = Dialog::around(TextView::new_with_content(progress_text.clone()))
                                .title("Disinstallazione in corso")
                                .fixed_width(60)
                                .fixed_height(10);
                            
                            s.add_layer(progress_view);
                            
                            // Disinstalla tutti i task selezionati
                            let mut success_count = 0;
                            let mut error_messages = Vec::new();
                            
                            for (i, idx) in selected_indices.iter().enumerate() {
                                let result = {
                                    let mut tasks_guard = match tasks.lock() {
                                        Ok(guard) => guard,
                                        Err(e) => {
                                            error_messages.push(format!("Errore nel blocco dei task: {}", e));
                                            continue;
                                        }
                                    };
                                    
                                    let task = match tasks_guard.get_mut(*idx) {
                                        Some(task) => task,
                                        None => {
                                            error_messages.push(format!("Task con indice {} non trovato", idx));
                                            continue;
                                        }
                                    };
                                    
                                    // Verifica che il task sia installato
                                    if !task.installed {
                                        continue;
                                    }
                                    
                                    // Aggiorna il messaggio di progresso
                                    progress_text.set_content(format!("Disinstallazione del task {} ({}/{})...", 
                                                                    task.name, i+1, selected_indices.len()));
                                    
                                    // Esegui la disinstallazione
                                    let uninstall_result = {
                                        let config_guard = match config.lock() {
                                            Ok(guard) => guard,
                                            Err(e) => {
                                                error_messages.push(format!("Errore nel blocco della configurazione: {}", e));
                                                continue;
                                            }
                                        };
                                        
                                        task.uninstall(&config_guard)
                                    };
                                    
                                    uninstall_result
                                };
                                
                                match result {
                                    Ok(_) => success_count += 1,
                                    Err(e) => error_messages.push(format!("Errore nella disinstallazione del task {}: {}", idx, e)),
                                }
                            }
                            
                            // Rimuovi la finestra di progresso
                            s.pop_layer();
                            
                            // Mostra il risultato
                            if error_messages.is_empty() {
                                s.add_layer(Dialog::info(format!("Tutti i {} task sono stati disinstallati con successo", success_count))
                                             .fixed_width(60)
                                             .fixed_height(10));
                            } else {
                                let mut result_message = format!("Disinstallati con successo: {}/{}\n\nErrori:\n", 
                                                              success_count, selected_indices.len());
                                for error in &error_messages {
                                    result_message.push_str(&format!("- {}\n", error));
                                }
                                
                                s.add_layer(Dialog::around(TextView::new(result_message).scrollable())
                                    .title("Risultato disinstallazione")
                                    .button("OK", |s| { s.pop_layer(); })
                                    .fixed_width(70)
                                    .fixed_height(15));
                            }
                            
                            // Aggiorna la vista
                            update_fn();
                            
                            // Mostra i log recenti
                            log_view::show_recent_logs_popup(s);
                        }
                    })
                    .fixed_width(60)
                    .fixed_height(10));
                return;
            }

            // Se non ci sono selezioni multiple, procedi con la singola selezione
            let idx = match s.call_on_name("task_list", |view: &mut SelectView<usize>| view.selected_id()) {
                Some(Some(idx)) => idx,
                _ => return,
            };

            // Ottieni il task selezionato
            let task_result = {
                let mut tasks_guard = match tasks.lock() {
                    Ok(guard) => guard,
                    Err(e) => {
                        s.add_layer(Dialog::info(format!("Errore nel blocco dei task: {}", e))
                                     .fixed_width(50)
                                     .fixed_height(7));
                        return;
                    }
                };

                let task = match tasks_guard.get_mut(idx) {
                    Some(task) => task,
                    None => {
                        s.add_layer(Dialog::info("Task non trovato")
                                     .fixed_width(50)
                                     .fixed_height(7));
                        return;
                    }
                };

                // Verifica che il task sia installato
                if !task.installed {
                    s.add_layer(Dialog::info("Il task non è installato")
                                 .fixed_width(50)
                                 .fixed_height(7));
                    return;
                }

                // Esegui la disinstallazione in un nuovo scope
                let uninstall_result = {
                    let config_guard = match config.lock() {
                        Ok(guard) => guard,
                        Err(e) => {
                            s.add_layer(Dialog::info(format!("Errore nel blocco della configurazione: {}", e))
                                         .fixed_width(50)
                                         .fixed_height(7));
                            return;
                        }
                    };

                    let result = task.uninstall(&config_guard);
                    
                    // Il lock su config_guard viene rilasciato qui
                    result
                };

                uninstall_result
            };

            // Gestisci il risultato della disinstallazione
            match task_result {
                Ok(_) => {
                    s.add_layer(Dialog::info("Task disinstallato con successo")
                                 .fixed_width(50)
                                 .fixed_height(7));
                    update_fn();
                    
                    // Mostra i log recenti
                    log_view::show_recent_logs_popup(s);
                },
                Err(e) => {
                    s.add_layer(Dialog::info(format!("Errore durante la disinstallazione del task: {}", e))
                                 .fixed_width(50)
                                 .fixed_height(7));
                }
            }
        })
    };

    let reset_button = {
        let tasks = Arc::clone(&tasks_clone);
        let config = Arc::clone(&config_clone);
        let selection = Arc::clone(&selection_clone);

        Button::new("Reset", move |s| {
            // Verifica se ci sono task selezionati
            let selected_indices = {
                if let Ok(sel) = selection.lock() {
                    sel.get_selected_indices()
                } else {
                    vec![]
                }
            };

            if !selected_indices.is_empty() {
                // Chiedi conferma per il reset multiplo
                s.add_layer(Dialog::around(TextView::new(format!("Sei sicuro di voler resettare {} task selezionati?", selected_indices.len())))
                    .title("Conferma reset")
                    .button("No", |s| { s.pop_layer(); })
                    .button("Sì", {
                        let tasks = Arc::clone(&tasks);
                        let config = Arc::clone(&config);
                        let selected_indices = selected_indices.clone();
                        
                        move |s| {
                            s.pop_layer();
                            
                            // Mostra una finestra di progresso
                            let progress_text = TextContent::new("Inizializzazione del reset...");
                            let progress_view = Dialog::around(TextView::new_with_content(progress_text.clone()))
                                .title("Reset in corso")
                                .fixed_width(60)
                                .fixed_height(10);
                            
                            s.add_layer(progress_view);
                            
                            // Reset di tutti i task selezionati
                            let mut success_count = 0;
                            let mut error_messages = Vec::new();
                            
                            for (i, idx) in selected_indices.iter().enumerate() {
                                let result = {
                                    let mut tasks_guard = match tasks.lock() {
                                        Ok(guard) => guard,
                                        Err(e) => {
                                            error_messages.push(format!("Errore nel blocco dei task: {}", e));
                                            continue;
                                        }
                                    };
                                    
                                    let task = match tasks_guard.get_mut(*idx) {
                                        Some(task) => task,
                                        None => {
                                            error_messages.push(format!("Task con indice {} non trovato", idx));
                                            continue;
                                        }
                                    };
                                    
                                    // Verifica che il task sia installato
                                    if !task.installed {
                                        continue;
                                    }
                                    
                                    // Aggiorna il messaggio di progresso
                                    progress_text.set_content(format!("Reset del task {} ({}/{})...", 
                                                                    task.name, i+1, selected_indices.len()));
                                    
                                    // Esegui il reset
                                    let reset_result = {
                                        let config_guard = match config.lock() {
                                            Ok(guard) => guard,
                                            Err(e) => {
                                                error_messages.push(format!("Errore nel blocco della configurazione: {}", e));
                                                continue;
                                            }
                                        };
                                        
                                        task.reset(&config_guard)
                                    };
                                    
                                    reset_result
                                };
                                
                                match result {
                                    Ok(_) => success_count += 1,
                                    Err(e) => error_messages.push(format!("Errore nel reset del task {}: {}", idx, e)),
                                }
                            }
                            
                            // Rimuovi la finestra di progresso
                            s.pop_layer();
                            
                            // Mostra il risultato
                            if error_messages.is_empty() {
                                s.add_layer(Dialog::info(format!("Tutti i {} task sono stati resettati con successo", success_count))
                                             .fixed_width(60)
                                             .fixed_height(10));
                            } else {
                                let mut result_message = format!("Reset completato con successo: {}/{}\n\nErrori:\n", 
                                                              success_count, selected_indices.len());
                                for error in &error_messages {
                                    result_message.push_str(&format!("- {}\n", error));
                                }
                                
                                s.add_layer(Dialog::around(TextView::new(result_message).scrollable())
                                    .title("Risultato reset")
                                    .button("OK", |s| { s.pop_layer(); })
                                    .fixed_width(70)
                                    .fixed_height(15));
                            }
                            
                            // Mostra i log recenti
                            log_view::show_recent_logs_popup(s);
                        }
                    })
                    .fixed_width(60)
                    .fixed_height(10));
                return;
            }

            // Se non ci sono selezioni multiple, procedi con la singola selezione
            let idx = match s.call_on_name("task_list", |view: &mut SelectView<usize>| view.selected_id()) {
                Some(Some(idx)) => idx,
                _ => return,
            };

            // Ottieni il task selezionato
            let task_result = {
                let mut tasks_guard = match tasks.lock() {
                    Ok(guard) => guard,
                    Err(e) => {
                        s.add_layer(Dialog::info(format!("Errore nel blocco dei task: {}", e))
                                     .fixed_width(50)
                                     .fixed_height(7));
                        return;
                    }
                };

                let task = match tasks_guard.get_mut(idx) {
                    Some(task) => task,
                    None => {
                        s.add_layer(Dialog::info("Task non trovato")
                                     .fixed_width(50)
                                     .fixed_height(7));
                        return;
                    }
                };

                // Verifica che il task sia installato
                if !task.installed {
                    s.add_layer(Dialog::info("Il task non è installato")
                                 .fixed_width(50)
                                 .fixed_height(7));
                    return;
                }

                // Esegui il reset in un nuovo scope
                let reset_result = {
                    let config_guard = match config.lock() {
                        Ok(guard) => guard,
                        Err(e) => {
                            s.add_layer(Dialog::info(format!("Errore nel blocco della configurazione: {}", e))
                                         .fixed_width(50)
                                         .fixed_height(7));
                            return;
                        }
                    };

                    let result = task.reset(&config_guard);
                    
                    // Il lock su config_guard viene rilasciato qui
                    result
                };

                reset_result
            };

            // Gestisci il risultato del reset
            match task_result {
                Ok(_) => {
                    s.add_layer(Dialog::info("Reset del task completato con successo")
                                 .fixed_width(50)
                                 .fixed_height(7));
                    
                    // Mostra i log recenti
                    log_view::show_recent_logs_popup(s);
                },
                Err(e) => {
                    s.add_layer(Dialog::info(format!("Errore durante il reset del task: {}", e))
                                 .fixed_width(50)
                                 .fixed_height(7));
                }
            }
        })
    };
    
    let remediate_button = {
        let tasks = Arc::clone(&tasks_clone);
        let config = Arc::clone(&config_clone);
        let selection = Arc::clone(&selection_clone);

        Button::new("Remediate", move |s| {
            // Verifica se ci sono task selezionati
            let selected_indices = {
                if let Ok(sel) = selection.lock() {
                    sel.get_selected_indices()
                } else {
                    vec![]
                }
            };

            if !selected_indices.is_empty() {
                // Chiedi conferma per la remediation multipla
                s.add_layer(Dialog::around(TextView::new(format!("Sei sicuro di voler rimediare {} task selezionati?", selected_indices.len())))
                    .title("Conferma remediation")
                    .button("No", |s| { s.pop_layer(); })
                    .button("Sì", {
                        let tasks = Arc::clone(&tasks);
                        let config = Arc::clone(&config);
                        let selected_indices = selected_indices.clone();
                        
                        move |s| {
                            s.pop_layer();
                            
                            // Mostra una finestra di progresso
                            let progress_text = TextContent::new("Inizializzazione della remediation...");
                            let progress_view = Dialog::around(TextView::new_with_content(progress_text.clone()))
                                .title("Remediation in corso")
                                .fixed_width(60)
                                .fixed_height(10);
                            
                            s.add_layer(progress_view);
                            
                            // Remediation di tutti i task selezionati
                            let mut success_count = 0;
                            let mut error_messages = Vec::new();
                            
                            for (i, idx) in selected_indices.iter().enumerate() {
                                let result = {
                                    let mut tasks_guard = match tasks.lock() {
                                        Ok(guard) => guard,
                                        Err(e) => {
                                            error_messages.push(format!("Errore nel blocco dei task: {}", e));
                                            continue;
                                        }
                                    };
                                    
                                    let task = match tasks_guard.get_mut(*idx) {
                                        Some(task) => task,
                                        None => {
                                            error_messages.push(format!("Task con indice {} non trovato", idx));
                                            continue;
                                        }
                                    };
                                    
                                    // Verifica che il task sia installato
                                    if !task.installed {
                                        continue;
                                    }
                                    
                                    // Aggiorna il messaggio di progresso
                                    progress_text.set_content(format!("Remediation del task {} ({}/{})...", 
                                                                    task.name, i+1, selected_indices.len()));
                                    
                                    // Esegui la remediation
                                    let remediate_result = {
                                        let config_guard = match config.lock() {
                                            Ok(guard) => guard,
                                            Err(e) => {
                                                error_messages.push(format!("Errore nel blocco della configurazione: {}", e));
                                                continue;
                                            }
                                        };
                                        
                                        task.remediate(&config_guard)
                                    };
                                    
                                    remediate_result
                                };
                                
                                match result {
                                    Ok(_) => success_count += 1,
                                    Err(e) => error_messages.push(format!("Errore nella remediation del task {}: {}", idx, e)),
                                }
                            }
                            
                            // Rimuovi la finestra di progresso
                            s.pop_layer();
                            
                            // Mostra il risultato
                            if error_messages.is_empty() {
                                s.add_layer(Dialog::info(format!("Tutti i {} task sono stati rimediati con successo", success_count))
                                             .fixed_width(60)
                                             .fixed_height(10));
                            } else {
                                let mut result_message = format!("Remediation completata con successo: {}/{}\n\nErrori:\n", 
                                                              success_count, selected_indices.len());
                                for error in &error_messages {
                                    result_message.push_str(&format!("- {}\n", error));
                                }
                                
                                s.add_layer(Dialog::around(TextView::new(result_message).scrollable())
                                    .title("Risultato remediation")
                                    .button("OK", |s| { s.pop_layer(); })
                                    .fixed_width(70)
                                    .fixed_height(15));
                            }
                            
                            // Mostra i log recenti
                            log_view::show_recent_logs_popup(s);
                        }
                    })
                    .fixed_width(60)
                    .fixed_height(10));
                return;
            }

            // Se non ci sono selezioni multiple, procedi con la singola selezione
            let idx = match s.call_on_name("task_list", |view: &mut SelectView<usize>| view.selected_id()) {
                Some(Some(idx)) => idx,
                _ => return,
            };

            // Ottieni il task selezionato
            let task_result = {
                let mut tasks_guard = match tasks.lock() {
                    Ok(guard) => guard,
                    Err(e) => {
                        s.add_layer(Dialog::info(format!("Errore nel blocco dei task: {}", e))
                                     .fixed_width(50)
                                     .fixed_height(7));
                        return;
                    }
                };

                let task = match tasks_guard.get_mut(idx) {
                    Some(task) => task,
                    None => {
                        s.add_layer(Dialog::info("Task non trovato")
                                     .fixed_width(50)
                                     .fixed_height(7));
                        return;
                    }
                };

                // Verifica che il task sia installato
                if !task.installed {
                    s.add_layer(Dialog::info("Il task non è installato")
                                 .fixed_width(50)
                                 .fixed_height(7));
                    return;
                }

                // Esegui la remediation in un nuovo scope
                let remediate_result = {
                    let config_guard = match config.lock() {
                        Ok(guard) => guard,
                        Err(e) => {
                            s.add_layer(Dialog::info(format!("Errore nel blocco della configurazione: {}", e))
                                         .fixed_width(50)
                                         .fixed_height(7));
                            return;
                        }
                    };

                    let result = task.remediate(&config_guard);
                    
                    // Il lock su config_guard viene rilasciato qui
                    result
                };

                remediate_result
            };

            // Gestisci il risultato della remediation
            match task_result {
                Ok(_) => {
                    s.add_layer(Dialog::info("Remediation del task completata con successo")
                                 .fixed_width(50)
                                 .fixed_height(7));
                    
                    // Mostra i log recenti
                    log_view::show_recent_logs_popup(s);
                },
                Err(e) => {
                    s.add_layer(Dialog::info(format!("Errore durante la remediation del task: {}", e))
                                 .fixed_width(50)
                                 .fixed_height(7));
                }
            }
        })
    };

    // Aggiungi anche un bottone per la gestione di massa
    let clear_selection_button = {
        let selection = Arc::clone(&task_selection);
        let update_fn = update_clone.clone();
        
        Button::new("Pulisci Selezione", move |s| {
            {
                if let Ok(mut sel) = selection.lock() {
                    sel.clear();
                }
            }
            update_fn();
        })
    };

    // Crea la barra dei pulsanti
    let buttons = LinearLayout::horizontal()
        .child(install_all_button)
        .child(DummyView.fixed_width(1))
        .child(install_button)
        .child(DummyView.fixed_width(1))
        .child(uninstall_button)
        .child(DummyView.fixed_width(1))
        .child(reset_button)
        .child(DummyView.fixed_width(1))
        .child(remediate_button)
        .child(DummyView.fixed_width(1))
        .child(clear_selection_button);

    // Layout principale
    let layout = LinearLayout::vertical()
        .child(Panel::new(select_view.with_name("task_list").scrollable())
            .fixed_width(PANEL_WIDTH)
            .fixed_height(PANEL_HEIGHT))
        .child(selection_info_view)
        .child(DummyView.fixed_height(1))
        .child(Panel::new(task_detail_view)
            .title("Dettagli del task")
            .fixed_width(PANEL_WIDTH)
            .fixed_height(10))
        .child(DummyView.fixed_height(1))
        .child(buttons);

    // Aggiungi la vista alla UI
    siv.add_layer(Dialog::around(layout)
        .title("Gestione Task")
        .button("Log", |s| {
            log_view::show_recent_logs_popup(s);
        })
        .button("Back", |s| {
            s.pop_layer();
        })
        .fixed_width(WINDOW_WIDTH)
        .fixed_height(WINDOW_HEIGHT));

    Ok(())
}