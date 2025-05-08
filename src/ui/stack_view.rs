//! Visualizzazione e gestione degli stack nell'interfaccia utente
//!
//! Questo modulo fornisce la visualizzazione e l'interazione con gli stack.

use std::sync::{Arc, Mutex};
use std::collections::HashSet;

use anyhow::{Result, anyhow};

use cursive::Cursive;
use cursive::views::{Dialog, SelectView, TextView, LinearLayout, DummyView, Panel, TextContent, Button, Checkbox};
use cursive::view::Scrollable;
use cursive::traits::*;
use cursive::align::HAlign;
use cursive::event::{Event, Key};

use crate::config::Config;
use crate::task::Task;
use crate::stack::Stack;
use crate::ui::app::{WINDOW_WIDTH, WINDOW_HEIGHT, PANEL_WIDTH, PANEL_HEIGHT};
use crate::ui::log_view;
use crate::logger;

// Implementazione della selezione multipla
struct StackSelection {
    // Indici degli stack selezionati
    selected_indices: HashSet<usize>,
}

impl StackSelection {
    fn new() -> Self {
        StackSelection {
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

/// Crea la vista per la gestione degli stack
pub fn create_stack_view(siv: &mut Cursive, config: Arc<Mutex<Config>>, stacks: Arc<Mutex<Vec<Stack>>>, tasks: Arc<Mutex<Vec<Task>>>) -> Result<()> {
    let stacks_clone = Arc::clone(&stacks);

    // Ottiene gli stack dal mutex
    let stacks_guard = stacks.lock().map_err(|_| anyhow!("Failed to lock stacks mutex"))?;

    // Stato della selezione multipla
    let stack_selection = Arc::new(Mutex::new(StackSelection::new()));

    // Crea la vista per selezionare gli stack
    let mut select_view = SelectView::new()
        .h_align(HAlign::Left)
        .autojump();

    // Popola la vista con gli stack
    for (idx, stack) in stacks_guard.iter().enumerate() {
        // Aggiungi lo stack alla vista
        let status = if stack.fully_installed {
            "[✓]"
        } else if stack.partially_installed {
            "[!]"
        } else {
            "[ ]"
        };

        let stack_line = format!("{} {} - {}",
                                 status,
                                 stack.name,
                                 stack.description
        );

        select_view.add_item(stack_line, idx);
    }

    // Rilascia il lock prima di creare le closure
    drop(stacks_guard);

    // Descrizione dettagliata dello stack selezionato
    let stack_detail = TextContent::new("Seleziona uno stack per vedere i dettagli");
    let stack_detail_view = TextView::new_with_content(stack_detail.clone())
        .scrollable();

    // Aggiungi handler per la selezione multipla con spazio
    let selection_clone = Arc::clone(&stack_selection);
    let mut select_view = select_view.with_name("stack_list");
    
    // Configura l'handler di eventi separatamente
    select_view.on_event(move |s, event| {
        match event {
            Event::Key(Key::Enter) => {
                if let Some(idx) = s.selected_id() {
                    if let Ok(mut selection) = selection_clone.lock() {
                        selection.toggle(idx);
                        
                        // Aggiorna l'interfaccia utente per mostrare la selezione
                        let is_selected = selection.is_selected(idx);
                        
                        let item_label = s.get_item(idx).unwrap().0;
                        let new_label = if is_selected {
                            format!("[*] {}", item_label.trim_start_matches("[ ] ")
                                             .trim_start_matches("[✓] ")
                                             .trim_start_matches("[!] "))
                        } else {
                            // Mantieni lo stato originale (installato, parziale, non installato)
                            if item_label.contains("[✓]") {
                                format!("[✓] {}", item_label.trim_start_matches("[*] ")
                                                 .trim_start_matches("[✓] "))
                            } else if item_label.contains("[!]") {
                                format!("[!] {}", item_label.trim_start_matches("[*] ")
                                                 .trim_start_matches("[!] "))
                            } else {
                                format!("[ ] {}", item_label.trim_start_matches("[*] ")
                                                 .trim_start_matches("[ ] "))
                            }
                        };
                        
                        s.set_item_label(idx, new_label);
                    }
                    return Some(event);
                }
            },
            _ => {}
        }
        None
    });

    // Informazioni sulla selezione
    let selection_info = TextContent::new("Premi 'Spazio' per selezionare/deselezionare stack. Nessuno stack selezionato.");
    let selection_info_view = TextView::new_with_content(selection_info.clone())
        .h_align(HAlign::Center);

    // Gestisci la selezione degli stack
    let tasks_clone = Arc::clone(&tasks);
    let stacks_clone2 = Arc::clone(&stacks_clone);
    select_view.set_on_select(move |_siv, idx| {
        if let Ok(stacks_guard) = stacks_clone2.lock() {
            if let Some(stack) = stacks_guard.get(*idx) {
                // Ottieni i task associati a questo stack
                if let Ok(tasks_guard) = tasks_clone.lock() {
                    // Crea una descrizione dettagliata dello stack
                    let mut details = format!("Nome: {}\n", stack.name);
                    details.push_str(&format!("Descrizione: {}\n", stack.description));
                    details.push_str(&format!("Stato: {}\n",
                                              if stack.fully_installed {
                                                  "Completamente installato"
                                              } else if stack.partially_installed {
                                                  "Parzialmente installato"
                                              } else {
                                                  "Non installato"
                                              }
                    ));

                    if !stack.tags.is_empty() {
                        details.push_str(&format!("Tag: {}\n", stack.tags.join(", ")));
                    }

                    details.push_str(&format!("Richiede riavvio: {}\n", if stack.requires_reboot { "Sì" } else { "No" }));

                    // Aggiungi l'elenco dei task inclusi
                    details.push_str("\nTask inclusi:\n");
                    for task_name in &stack.task_names {
                        // Verifica se il task è installato
                        let status = if let Some(task) = tasks_guard.iter().find(|t| &t.name == task_name) {
                            if task.installed {
                                "[✓]"
                            } else {
                                "[ ]"
                            }
                        } else {
                            "[?]" // Task non trovato
                        };

                        details.push_str(&format!("  {} {}\n", status, task_name));
                    }

                    // Aggiorna la vista dei dettagli
                    stack_detail.set_content(details);
                }
            }
        }
    });

    // Funzione per aggiornare la vista
    let update_stack_view = {
        let stacks = Arc::clone(&stacks);
        let selection = Arc::clone(&stack_selection);
        let selection_info_content = selection_info.clone();
        let select_view_cb = siv.cb_sink().clone();

        move || {
            if let Ok(stacks_guard) = stacks.lock() {
                // Crea una copia dei dati necessari
                let stack_data: Vec<(bool, bool, String, String)> = stacks_guard.iter()
                    .map(|s| (s.fully_installed, s.partially_installed, s.name.clone(), s.description.clone()))
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
                        selection_info_content.set_content(format!("Premi 'Spazio' per selezionare/deselezionare stack. {} stack selezionati.", selection_count));
                    } else {
                        selection_info_content.set_content("Premi 'Spazio' per selezionare/deselezionare stack. Nessuno stack selezionato.".to_string());
                    }

                    s.call_on_name("stack_list", |view: &mut SelectView<usize>| {
                        view.clear();

                        for (idx, (fully_installed, partially_installed, name, description)) in stack_data.iter().enumerate() {
                            let is_selected = {
                                if let Ok(sel) = selection.lock() {
                                    sel.is_selected(idx)
                                } else {
                                    false
                                }
                            };

                            let status = if is_selected {
                                "[*]"
                            } else if *fully_installed {
                                "[✓]"
                            } else if *partially_installed {
                                "[!]"
                            } else {
                                "[ ]"
                            };

                            let stack_line = format!("{} {} - {}", status, name, description);
                            view.add_item(stack_line, idx);
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
    let stacks_clone = Arc::clone(&stacks);
    let tasks_clone = Arc::clone(&tasks);
    let update_clone = update_stack_view.clone();
    let selection_clone = Arc::clone(&stack_selection);

    let install_all_button = {
        let stacks = Arc::clone(&stacks_clone);
        let tasks = Arc::clone(&tasks_clone);
        let config = Arc::clone(&config_clone);
        let update_fn = update_clone.clone();
        let selection = Arc::clone(&selection_clone);

        Button::new("Install Selezionati", move |s| {
            // Ottieni gli indici degli stack selezionati
            let selected_indices = {
                if let Ok(sel) = selection.lock() {
                    sel.get_selected_indices()
                } else {
                    vec![]
                }
            };

            if selected_indices.is_empty() {
                s.add_layer(Dialog::info("Nessuno stack selezionato")
                             .fixed_width(50)
                             .fixed_height(7));
                return;
            }

            // Chiedi conferma
            s.add_layer(Dialog::around(TextView::new(format!("Sei sicuro di voler installare {} stack selezionati?", selected_indices.len())))
                .title("Conferma installazione")
                .button("No", |s| { s.pop_layer(); })
                .button("Sì", {
                    let stacks = Arc::clone(&stacks);
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
                        
                        // Installa tutti gli stack selezionati
                        let mut success_count = 0;
                        let mut error_messages = Vec::new();
                        
                        for (i, idx) in selected_indices.iter().enumerate() {
                            let result = {
                                let mut stacks_guard = match stacks.lock() {
                                    Ok(guard) => guard,
                                    Err(e) => {
                                        error_messages.push(format!("Errore nel blocco degli stack: {}", e));
                                        continue;
                                    }
                                };
                                
                                let stack = match stacks_guard.get_mut(*idx) {
                                    Some(stack) => stack,
                                    None => {
                                        error_messages.push(format!("Stack con indice {} non trovato", idx));
                                        continue;
                                    }
                                };
                                
                                // Verifica che lo stack non sia già installato
                                if stack.fully_installed {
                                    // Considera già come successo se è già installato
                                    success_count += 1;
                                    continue;
                                }
                                
                                // Aggiorna il messaggio di progresso
                                progress_text.set_content(format!("Installazione dello stack {} ({}/{})...", 
                                                                stack.name, i+1, selected_indices.len()));
                                
                                // Crea un nuovo scope per limitare la durata del lock sulla configurazione
                                let install_result = {
                                    let config_guard = match config.lock() {
                                        Ok(guard) => guard,
                                        Err(e) => {
                                            error_messages.push(format!("Errore nel blocco della configurazione: {}", e));
                                            continue;
                                        }
                                    };
                                    
                                    let mut tasks_guard = match tasks.lock() {
                                        Ok(guard) => guard,
                                        Err(e) => {
                                            error_messages.push(format!("Errore nel blocco dei task: {}", e));
                                            continue;
                                        }
                                    };
                                    
                                    // Esegui l'installazione
                                    let result = stack.install(&config_guard, &mut tasks_guard);
                                    
                                    // I lock vengono rilasciati qui
                                    result
                                };
                                
                                install_result
                            };
                            
                            match result {
                                Ok(_) => success_count += 1,
                                Err(e) => error_messages.push(format!("Errore nell'installazione dello stack {}: {}", idx, e)),
                            }
                        }
                        
                        // Rimuovi la finestra di progresso
                        s.pop_layer();
                        
                        // Mostra il risultato
                        if error_messages.is_empty() {
                            s.add_layer(Dialog::info(format!("Tutti i {} stack sono stati installati con successo", success_count))
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
        let stacks = Arc::clone(&stacks_clone);
        let tasks = Arc::clone(&tasks_clone);
        let config = Arc::clone(&config_clone);
        let update_fn = update_clone.clone();

        Button::new("Install", move |s| {
            let idx = match s.call_on_name("stack_list", |view: &mut SelectView<usize>| view.selected_id()) {
                Some(Some(idx)) => idx,
                _ => return,
            };

            // Ottieni lo stack selezionato
            let stack_result = {
                let mut stacks_guard = match stacks.lock() {
                    Ok(guard) => guard,
                    Err(e) => {
                        s.add_layer(Dialog::info(format!("Errore nel blocco degli stack: {}", e))
                                     .fixed_width(50)
                                     .fixed_height(7));
                        return;
                    }
                };

                let stack = match stacks_guard.get_mut(idx) {
                    Some(stack) => stack,
                    None => {
                        s.add_layer(Dialog::info("Stack non trovato")
                                     .fixed_width(50)
                                     .fixed_height(7));
                        return;
                    }
                };

                // Verifica che lo stack non sia già installato
                if stack.fully_installed {
                    s.add_layer(Dialog::info("Lo stack è già completamente installato")
                                 .fixed_width(50)
                                 .fixed_height(7));
                    return;
                }

                // Crea un nuovo scope per limitare la durata dei lock
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

                    let mut tasks_guard = match tasks.lock() {
                        Ok(guard) => guard,
                        Err(e) => {
                            s.add_layer(Dialog::info(format!("Errore nel blocco dei task: {}", e))
                                         .fixed_width(50)
                                         .fixed_height(7));
                            return;
                        }
                    };

                    let result = stack.install(&config_guard, &mut tasks_guard);
                    
                    // I lock vengono rilasciati qui, alla fine dello scope
                    result
                };

                install_result
            };

            // Gestisci il risultato dell'installazione
            match stack_result {
                Ok(_) => {
                    s.add_layer(Dialog::info("Stack installato con successo")
                                 .fixed_width(50)
                                 .fixed_height(7));
                    update_fn();
                    
                    // Mostra i log recenti
                    log_view::show_recent_logs_popup(s);
                },
                Err(e) => {
                    s.add_layer(Dialog::info(format!("Errore durante l'installazione dello stack: {}", e))
                                 .fixed_width(50)
                                 .fixed_height(7));
                }
            }
        })
    };

    let uninstall_button = {
        let stacks = Arc::clone(&stacks_clone);
        let tasks = Arc::clone(&tasks_clone);
        let config = Arc::clone(&config_clone);
        let update_fn = update_clone.clone();
        let selection = Arc::clone(&selection_clone);

        Button::new("Uninstall", move |s| {
            // Verifica se ci sono stack selezionati
            let selected_indices = {
                if let Ok(sel) = selection.lock() {
                    sel.get_selected_indices()
                } else {
                    vec![]
                }
            };

            if !selected_indices.is_empty() {
                // Chiedi conferma per la disinstallazione multipla
                s.add_layer(Dialog::around(TextView::new(format!("Sei sicuro di voler disinstallare {} stack selezionati?", selected_indices.len())))
                    .title("Conferma disinstallazione")
                    .button("No", |s| { s.pop_layer(); })
                    .button("Sì", {
                        let stacks = Arc::clone(&stacks);
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
                            
                            // Disinstalla tutti gli stack selezionati
                            let mut success_count = 0;
                            let mut error_messages = Vec::new();
                            
                            for (i, idx) in selected_indices.iter().enumerate() {
                                let result = {
                                    let mut stacks_guard = match stacks.lock() {
                                        Ok(guard) => guard,
                                        Err(e) => {
                                            error_messages.push(format!("Errore nel blocco degli stack: {}", e));
                                            continue;
                                        }
                                    };
                                    
                                    let stack = match stacks_guard.get_mut(*idx) {
                                        Some(stack) => stack,
                                        None => {
                                            error_messages.push(format!("Stack con indice {} non trovato", idx));
                                            continue;
                                        }
                                    };
                                    
                                    // Verifica che lo stack sia almeno parzialmente installato
                                    if !stack.fully_installed && !stack.partially_installed {
                                        continue;
                                    }
                                    
                                    // Aggiorna il messaggio di progresso
                                    progress_text.set_content(format!("Disinstallazione dello stack {} ({}/{})...", 
                                                                    stack.name, i+1, selected_indices.len()));
                                    
                                    // Esegui la disinstallazione
                                    let uninstall_result = {
                                        let config_guard = match config.lock() {
                                            Ok(guard) => guard,
                                            Err(e) => {
                                                error_messages.push(format!("Errore nel blocco della configurazione: {}", e));
                                                continue;
                                            }
                                        };
                                        
                                        let mut tasks_guard = match tasks.lock() {
                                            Ok(guard) => guard,
                                            Err(e) => {
                                                error_messages.push(format!("Errore nel blocco dei task: {}", e));
                                                continue;
                                            }
                                        };
                                        
                                        stack.uninstall(&config_guard, &mut tasks_guard)
                                    };
                                    
                                    uninstall_result
                                };
                                
                                match result {
                                    Ok(_) => success_count += 1,
                                    Err(e) => error_messages.push(format!("Errore nella disinstallazione dello stack {}: {}", idx, e)),
                                }
                            }
                            
                            // Rimuovi la finestra di progresso
                            s.pop_layer();
                            
                            // Mostra il risultato
                            if error_messages.is_empty() {
                                s.add_layer(Dialog::info(format!("Tutti i {} stack sono stati disinstallati con successo", success_count))
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
            let idx = match s.call_on_name("stack_list", |view: &mut SelectView<usize>| view.selected_id()) {
                Some(Some(idx)) => idx,
                _ => return,
            };

            // Ottieni lo stack selezionato
            let stack_result = {
                let mut stacks_guard = match stacks.lock() {
                    Ok(guard) => guard,
                    Err(e) => {
                        s.add_layer(Dialog::info(format!("Errore nel blocco degli stack: {}", e))
                                     .fixed_width(50)
                                     .fixed_height(7));
                        return;
                    }
                };

                let stack = match stacks_guard.get_mut(idx) {
                    Some(stack) => stack,
                    None => {
                        s.add_layer(Dialog::info("Stack non trovato")
                                     .fixed_width(50)
                                     .fixed_height(7));
                        return;
                    }
                };

                // Verifica che lo stack sia almeno parzialmente installato
                if !stack.fully_installed && !stack.partially_installed {
                    s.add_layer(Dialog::info("Lo stack non è installato")
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

                    let mut tasks_guard = match tasks.lock() {
                        Ok(guard) => guard,
                        Err(e) => {
                            s.add_layer(Dialog::info(format!("Errore nel blocco dei task: {}", e))
                                         .fixed_width(50)
                                         .fixed_height(7));
                            return;
                        }
                    };

                    let result = stack.uninstall(&config_guard, &mut tasks_guard);
                    
                    // I lock vengono rilasciati qui
                    result
                };

                uninstall_result
            };

            // Gestisci il risultato della disinstallazione
            match stack_result {
                Ok(_) => {
                    s.add_layer(Dialog::info("Stack disinstallato con successo")
                                 .fixed_width(50)
                                 .fixed_height(7));
                    update_fn();
                    
                    // Mostra i log recenti
                    log_view::show_recent_logs_popup(s);
                },
                Err(e) => {
                    s.add_layer(Dialog::info(format!("Errore durante la disinstallazione dello stack: {}", e))
                                 .fixed_width(50)
                                 .fixed_height(7));
                }
            }
        })
    };

    let reset_button = {
        let stacks = Arc::clone(&stacks_clone);
        let tasks = Arc::clone(&tasks_clone);
        let config = Arc::clone(&config_clone);
        let selection = Arc::clone(&selection_clone);

        Button::new("Reset", move |s| {
            // Verifica se ci sono stack selezionati
            let selected_indices = {
                if let Ok(sel) = selection.lock() {
                    sel.get_selected_indices()
                } else {
                    vec![]
                }
            };

            if !selected_indices.is_empty() {
                // Chiedi conferma per il reset multiplo
                s.add_layer(Dialog::around(TextView::new(format!("Sei sicuro di voler resettare {} stack selezionati?", selected_indices.len())))
                    .title("Conferma reset")
                    .button("No", |s| { s.pop_layer(); })
                    .button("Sì", {
                        let stacks = Arc::clone(&stacks);
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
                            
                            // Reset di tutti gli stack selezionati
                            let mut success_count = 0;
                            let mut error_messages = Vec::new();
                            
                            for (i, idx) in selected_indices.iter().enumerate() {
                                let result = {
                                    let mut stacks_guard = match stacks.lock() {
                                        Ok(guard) => guard,
                                        Err(e) => {
                                            error_messages.push(format!("Errore nel blocco degli stack: {}", e));
                                            continue;
                                        }
                                    };
                                    
                                    let stack = match stacks_guard.get_mut(*idx) {
                                        Some(stack) => stack,
                                        None => {
                                            error_messages.push(format!("Stack con indice {} non trovato", idx));
                                            continue;
                                        }
                                    };
                                    
                                    // Verifica che lo stack sia almeno parzialmente installato
                                    if !stack.fully_installed && !stack.partially_installed {
                                        continue;
                                    }
                                    
                                    // Aggiorna il messaggio di progresso
                                    progress_text.set_content(format!("Reset dello stack {} ({}/{})...", 
                                                                    stack.name, i+1, selected_indices.len()));
                                    
                                    // Esegui il reset
                                    let reset_result = {
                                        let config_guard = match config.lock() {
                                            Ok(guard) => guard,
                                            Err(e) => {
                                                error_messages.push(format!("Errore nel blocco della configurazione: {}", e));
                                                continue;
                                            }
                                        };
                                        
                                        let mut tasks_guard = match tasks.lock() {
                                            Ok(guard) => guard,
                                            Err(e) => {
                                                error_messages.push(format!("Errore nel blocco dei task: {}", e));
                                                continue;
                                            }
                                        };
                                        
                                        stack.reset(&config_guard, &mut tasks_guard)
                                    };
                                    
                                    reset_result
                                };
                                
                                match result {
                                    Ok(_) => success_count += 1,
                                    Err(e) => error_messages.push(format!("Errore nel reset dello stack {}: {}", idx, e)),
                                }
                            }
                            
                            // Rimuovi la finestra di progresso
                            s.pop_layer();
                            
                            // Mostra il risultato
                            if error_messages.is_empty() {
                                s.add_layer(Dialog::info(format!("Tutti i {} stack sono stati resettati con successo", success_count))
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
            let idx = match s.call_on_name("stack_list", |view: &mut SelectView<usize>| view.selected_id()) {
                Some(Some(idx)) => idx,
                _ => return,
            };

            // Ottieni lo stack selezionato
            let stack_result = {
                let mut stacks_guard = match stacks.lock() {
                    Ok(guard) => guard,
                    Err(e) => {
                        s.add_layer(Dialog::info(format!("Errore nel blocco degli stack: {}", e))
                                     .fixed_width(50)
                                     .fixed_height(7));
                        return;
                    }
                };

                let stack = match stacks_guard.get_mut(idx) {
                    Some(stack) => stack,
                    None => {
                        s.add_layer(Dialog::info("Stack non trovato")
                                     .fixed_width(50)
                                     .fixed_height(7));
                        return;
                    }
                };

                // Verifica che lo stack sia almeno parzialmente installato
                if !stack.fully_installed && !stack.partially_installed {
                    s.add_layer(Dialog::info("Lo stack non è installato")
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

                    let mut tasks_guard = match tasks.lock() {
                        Ok(guard) => guard,
                        Err(e) => {
                            s.add_layer(Dialog::info(format!("Errore nel blocco dei task: {}", e))
                                         .fixed_width(50)
                                         .fixed_height(7));
                            return;
                        }
                    };

                    let result = stack.reset(&config_guard, &mut tasks_guard);
                    
                    // I lock vengono rilasciati qui
                    result
                };

                reset_result
            };

            // Gestisci il risultato del reset
            match stack_result {
                Ok(_) => {
                    s.add_layer(Dialog::info("Reset dello stack completato con successo")
                                 .fixed_width(50)
                                 .fixed_height(7));
                    
                    // Mostra i log recenti
                    log_view::show_recent_logs_popup(s);
                },
                Err(e) => {
                    s.add_layer(Dialog::info(format!("Errore durante il reset dello stack: {}", e))
                                 .fixed_width(50)
                                 .fixed_height(7));
                }
            }
        })
    };

    let remediate_button = {
        let stacks = Arc::clone(&stacks_clone);
        let tasks = Arc::clone(&tasks_clone);
        let config = Arc::clone(&config_clone);
        let selection = Arc::clone(&selection_clone);

        Button::new("Remediate", move |s| {
            // Verifica se ci sono stack selezionati
            let selected_indices = {
                if let Ok(sel) = selection.lock() {
                    sel.get_selected_indices()
                } else {
                    vec![]
                }
            };

            if !selected_indices.is_empty() {
                // Chiedi conferma per la remediation multipla
                s.add_layer(Dialog::around(TextView::new(format!("Sei sicuro di voler rimediare {} stack selezionati?", selected_indices.len())))
                    .title("Conferma remediation")
                    .button("No", |s| { s.pop_layer(); })
                    .button("Sì", {
                        let stacks = Arc::clone(&stacks);
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
                            
                            // Remediation di tutti gli stack selezionati
                            let mut success_count = 0;
                            let mut error_messages = Vec::new();
                            
                            for (i, idx) in selected_indices.iter().enumerate() {
                                let result = {
                                    let mut stacks_guard = match stacks.lock() {
                                        Ok(guard) => guard,
                                        Err(e) => {
                                            error_messages.push(format!("Errore nel blocco degli stack: {}", e));
                                            continue;
                                        }
                                    };
                                    
                                    let stack = match stacks_guard.get_mut(*idx) {
                                        Some(stack) => stack,
                                        None => {
                                            error_messages.push(format!("Stack con indice {} non trovato", idx));
                                            continue;
                                        }
                                    };
                                    
                                    // Verifica che lo stack sia almeno parzialmente installato
                                    if !stack.fully_installed && !stack.partially_installed {
                                        continue;
                                    }
                                    
                                    // Aggiorna il messaggio di progresso
                                    progress_text.set_content(format!("Remediation dello stack {} ({}/{})...", 
                                                                    stack.name, i+1, selected_indices.len()));
                                    
                                    // Esegui la remediation
                                    let remediate_result = {
                                        let config_guard = match config.lock() {
                                            Ok(guard) => guard,
                                            Err(e) => {
                                                error_messages.push(format!("Errore nel blocco della configurazione: {}", e));
                                                continue;
                                            }
                                        };
                                        
                                        let mut tasks_guard = match tasks.lock() {
                                            Ok(guard) => guard,
                                            Err(e) => {
                                                error_messages.push(format!("Errore nel blocco dei task: {}", e));
                                                continue;
                                            }
                                        };
                                        
                                        stack.remediate(&config_guard, &mut tasks_guard)
                                    };
                                    
                                    remediate_result
                                };
                                
                                match result {
                                    Ok(_) => success_count += 1,
                                    Err(e) => error_messages.push(format!("Errore nella remediation dello stack {}: {}", idx, e)),
                                }
                            }
                            
                            // Rimuovi la finestra di progresso
                            s.pop_layer();
                            
                            // Mostra il risultato
                            if error_messages.is_empty() {
                                s.add_layer(Dialog::info(format!("Tutti i {} stack sono stati rimediati con successo", success_count))
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
            let idx = match s.call_on_name("stack_list", |view: &mut SelectView<usize>| view.selected_id()) {
                Some(Some(idx)) => idx,
                _ => return,
            };

            // Ottieni lo stack selezionato
            let stack_result = {
                let mut stacks_guard = match stacks.lock() {
                    Ok(guard) => guard,
                    Err(e) => {
                        s.add_layer(Dialog::info(format!("Errore nel blocco degli stack: {}", e))
                                     .fixed_width(50)
                                     .fixed_height(7));
                        return;
                    }
                };

                let stack = match stacks_guard.get_mut(idx) {
                    Some(stack) => stack,
                    None => {
                        s.add_layer(Dialog::info("Stack non trovato")
                                     .fixed_width(50)
                                     .fixed_height(7));
                        return;
                    }
                };

                // Verifica che lo stack sia almeno parzialmente installato
                if !stack.fully_installed && !stack.partially_installed {
                    s.add_layer(Dialog::info("Lo stack non è installato")
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

                    let mut tasks_guard = match tasks.lock() {
                        Ok(guard) => guard,
                        Err(e) => {
                            s.add_layer(Dialog::info(format!("Errore nel blocco dei task: {}", e))
                                         .fixed_width(50)
                                         .fixed_height(7));
                            return;
                        }
                    };

                    let result = stack.remediate(&config_guard, &mut tasks_guard);
                    
                    // I lock vengono rilasciati qui
                    result
                };

                remediate_result
            };

            // Gestisci il risultato della remediation
            match stack_result {
                Ok(_) => {
                    s.add_layer(Dialog::info("Remediation dello stack completata con successo")
                                 .fixed_width(50)
                                 .fixed_height(7));
                    
                    // Mostra i log recenti
                    log_view::show_recent_logs_popup(s);
                },
                Err(e) => {
                    s.add_layer(Dialog::info(format!("Errore durante la remediation dello stack: {}", e))
                                 .fixed_width(50)
                                 .fixed_height(7));
                }
            }
        })
    };

    // Aggiungi anche un bottone per la gestione di massa
    let clear_selection_button = {
        let selection = Arc::clone(&stack_selection);
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
        .child(clear_selection_button);
        
    // Layout principale
    let layout = LinearLayout::vertical()
        .child(Panel::new(select_view.scrollable())
            .title("Stack disponibili")
            .fixed_width(PANEL_WIDTH)
            .fixed_height(PANEL_HEIGHT))
        .child(selection_info_view)
        .child(DummyView.fixed_height(1))
        .child(Panel::new(stack_detail_view)
            .title("Dettagli dello stack")
            .fixed_width(PANEL_WIDTH)
            .fixed_height(10))
        .child(DummyView.fixed_height(1))
        .child(buttons);

    // Aggiungi la vista alla UI
    siv.add_layer(Dialog::around(layout)
        .title("Gestione Stack")
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


