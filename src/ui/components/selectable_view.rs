// File: src/ui/components/selectable_view.rs

use std::sync::{Arc, Mutex};
use anyhow::{Result, anyhow};

use cursive::Cursive;
use cursive::views::{Dialog, SelectView, TextView, LinearLayout, DummyView, Panel, TextContent, Button, OnEventView};
use cursive::view::Scrollable;
use cursive::traits::*;
use cursive::align::HAlign;
use cursive::event::{Event, Key};

use crate::config::Config;
use crate::ui::app::{WINDOW_WIDTH, WINDOW_HEIGHT, PANEL_WIDTH, PANEL_HEIGHT};
use crate::ui::log_view;
use crate::ui::components::selection::{SelectableItem, SharedSelection};

/// Trait per implementare le operazioni eseguibili su un tipo
pub trait Executable<T: SelectableItem> {
    /// Installa l'elemento
    fn install(&mut self, config: &Config) -> Result<()>;
    
    /// Disinstalla l'elemento
    fn uninstall(&mut self, config: &Config) -> Result<()>;
    
    /// Resetta l'elemento
    fn reset(&mut self, config: &Config) -> Result<()>;
    
    /// Ripara l'elemento
    fn remediate(&mut self, config: &Config) -> Result<()>;
}

/// Crea una vista per gestire una collezione di elementi selezionabili
pub fn create_selectable_view<T, E>(
    siv: &mut Cursive,
    config: Arc<Mutex<Config>>,
    items: Arc<Mutex<Vec<E>>>, 
    selection: SharedSelection<T>,
    view_title: &str,
    _can_modify_items: bool, // Se gli elementi possono essere modificati (es: task installati)
) -> Result<()> 
where
    T: 'static + Send + Sync, // Aggiunto vincolo Send + Sync per T
    E: SelectableItem + Executable<E> + Clone + 'static + Send + Sync, // Aggiunto vincolo Send + Sync per E
{
    // Ottiene gli elementi dal mutex
    let items_guard = items.lock().map_err(|_| anyhow!("Failed to lock items mutex"))?;

    // Crea la vista per selezionare gli elementi
    let mut select_view = SelectView::new()
        .h_align(HAlign::Left)
        .autojump();

    // Popola la vista con gli elementi
    for (idx, item) in items_guard.iter().enumerate() {
        select_view.add_item(item.format_for_list(), idx);
    }

    // Dettagli dell'elemento selezionato
    let item_detail = TextContent::new("Seleziona un elemento per vedere i dettagli");
    let item_detail_view = TextView::new_with_content(item_detail.clone())
        .scrollable();

    // Gestisci la selezione degli elementi (prima di avvolgere in OnEventView)
    let items_clone = Arc::clone(&items);
    let item_detail_clone = item_detail.clone();
    select_view.set_on_select(move |_siv, idx| {
        if let Ok(items_guard) = items_clone.lock() {
            if let Some(item) = items_guard.get(*idx) {
                // Aggiorna il testo dei dettagli
                item_detail_clone.set_content(item.format_details());
            }
        }
    });

    // Rilascia il lock prima di creare le closure
    drop(items_guard);

    // Aggiungi handler per la selezione multipla con Invio
    let selection_clone = Arc::clone(&selection);
    let select_view = select_view.with_name("item_list");
    
    // Avvolgi con OnEventView per gestire gli eventi
    let select_view_with_events = OnEventView::new(select_view)
    .on_event(Event::Key(Key::Enter), move |s| {
        // Ottieni l'indice selezionato dalla vista originale
        if let Some(idx) = s.call_on_name("item_list", |view: &mut SelectView<usize>| {
            view.selected_id()
        }).unwrap_or(None) {
            if let Ok(mut sel) = selection_clone.lock() {
                sel.toggle(idx);
                
                // Aggiorna l'interfaccia utente per mostrare la selezione
                let is_selected = sel.is_selected(idx);
                
                // Modifica l'etichetta nella vista
                s.call_on_name("item_list", |view: &mut SelectView<usize>| {
                    if let Some((item_label, _)) = view.get_item(idx) {
                        let item_label = item_label.to_string();
                        
                        // Aggiorna l'etichetta basata sulla selezione
                        let new_label = if is_selected {
                            format!("[*] {}", item_label.trim_start_matches("[").split(']').nth(1).unwrap_or(""))
                        } else {
                            // Ripristina lo stato originale
                            if item_label.contains("[✓]") {
                                format!("[✓]{}", item_label.trim_start_matches("[*]").split(']').nth(1).unwrap_or(""))
                            } else if item_label.contains("[!]") {
                                format!("[!]{}", item_label.trim_start_matches("[*]").split(']').nth(1).unwrap_or(""))
                            } else {
                                format!("[ ]{}", item_label.trim_start_matches("[*]").split(']').nth(1).unwrap_or(""))
                            }
                        };
                        
                        // Aggiorna l'item nella vista
                        let value = view.selection().map(|i| *i);
                        view.remove_item(idx);
                        view.insert_item(idx, new_label, idx);
                        
                        // Ripristina la selezione
                        if let Some(val) = value {
                            view.set_selection(val);
                        }
                    }
                });
            }
        }
    });

    // Informazioni sulla selezione
    let selection_info = TextContent::new("Premi 'Invio' per selezionare/deselezionare. Nessun elemento selezionato.");
    let selection_info_view = TextView::new_with_content(selection_info.clone())
        .h_align(HAlign::Center);

    // Funzione di aggiornamento UI - implementata come funzione libera
    // che verrà chiamata direttamente dalle closure dei bottoni
    fn update_ui<T: Send + Sync + 'static, E: SelectableItem + Clone + 'static>(
        items: &Arc<Mutex<Vec<E>>>,
        selection: &SharedSelection<T>,
        selection_info_content: &TextContent,
        cb_sink: &cursive::CbSink,
    ) {
        if let Ok(items_guard) = items.lock() {
            // Raccogli i dati necessari per l'aggiornamento
            let items_data: Vec<(String, usize)> = items_guard.iter().enumerate()
                .map(|(idx, item)| (item.format_for_list(), idx))
                .collect();

            // Clona ciò che serve per il callback
            let items_data = items_data.clone();
            let selection = Arc::clone(selection);
            let selection_info_content = selection_info_content.clone();
            
            // Invia i dati al callback
            if let Err(_) = cb_sink.send(Box::new(move |s: &mut Cursive| {
                let selection_count = {
                    if let Ok(sel) = selection.lock() {
                        sel.count()
                    } else {
                        0
                    }
                };

                // Aggiorna il testo informativo
                if selection_count > 0 {
                    selection_info_content.set_content(format!("Premi 'Invio' per selezionare/deselezionare. {} elementi selezionati.", selection_count));
                } else {
                    selection_info_content.set_content("Premi 'Invio' per selezionare/deselezionare. Nessun elemento selezionato.".to_string());
                }

                // Aggiorna la lista
                s.call_on_name("item_list", |view: &mut SelectView<usize>| {
                    view.clear();

                    for (item_str, idx) in &items_data {
                        let is_selected = {
                            if let Ok(sel) = selection.lock() {
                                sel.is_selected(*idx)
                            } else {
                                false
                            }
                        };

                        let display_str = if is_selected {
                            // Sostituisci il marker di stato con [*]
                            let without_marker = item_str.trim_start_matches("[").split(']').nth(1).unwrap_or("");
                            format!("[*]{}", without_marker)
                        } else {
                            item_str.clone()
                        };

                        view.add_item(display_str, *idx);
                    }
                });
            })) {
                eprintln!("Errore nell'aggiornamento della vista");
            }
        }
    }

    // Crea i bottoni per le azioni
    let install_all_button = Button::new("Install Selezionati", {
        let items = Arc::clone(&items);
        let config = Arc::clone(&config);
        let selection = Arc::clone(&selection);
        let selection_info = selection_info.clone();
        let cb_sink = siv.cb_sink().clone();
        
        move |s| {
            // Ottieni gli indici degli elementi selezionati
            let selected_indices = {
                if let Ok(sel) = selection.lock() {
                    sel.get_selected_indices()
                } else {
                    vec![]
                }
            };

            if selected_indices.is_empty() {
                s.add_layer(Dialog::info("Nessun elemento selezionato")
                             .fixed_width(50)
                             .fixed_height(7));
                return;
            }

            // Chiedi conferma
            s.add_layer(Dialog::around(TextView::new(format!("Sei sicuro di voler installare {} elementi selezionati?", 
                                                           selected_indices.len())))
                .title("Conferma Installazione")
                .button("No", |s| { s.pop_layer(); })
                .button("Sì", {
                    let items = Arc::clone(&items);
                    let config = Arc::clone(&config);
                    let selected_indices = selected_indices.clone();
                    let selection_info = selection_info.clone();
                    let cb_sink = cb_sink.clone();
                    // Create unique names for each clone to avoid shadowing issues
                    let outer_selection = Arc::clone(&selection);
                    let selection_clone = Arc::clone(&selection);
                    let selection_for_update = Arc::clone(&selection);
                    
                    move |s| {
                        s.pop_layer();
                        
                        // Mostra una finestra di progresso
                        let progress_text = TextContent::new("Inizializzazione installazione...");
                        let progress_view = Dialog::around(TextView::new_with_content(progress_text.clone()))
                            .title("Installazione in corso")
                            .fixed_width(60)
                            .fixed_height(10);
                        
                        s.add_layer(progress_view);
                        
                        // Esegui l'operazione su tutti gli elementi selezionati
                        let mut success_count = 0;
                        let mut error_messages = Vec::new();
                        
                        for (i, idx) in selected_indices.iter().enumerate() {
                            let result = {
                                let mut items_guard = match items.lock() {
                                    Ok(guard) => guard,
                                    Err(e) => {
                                        error_messages.push(format!("Errore nel blocco degli elementi: {}", e));
                                        continue;
                                    }
                                };
                                
                                let item = match items_guard.get_mut(*idx) {
                                    Some(item) => item,
                                    None => {
                                        error_messages.push(format!("Elemento con indice {} non trovato", idx));
                                        continue;
                                    }
                                };
                                
                                // Verifica la condizione (es: può essere installato)
                                if !item.can_install() {
                                    continue;
                                }
                                
                                // Aggiorna il messaggio di progresso
                                progress_text.set_content(format!("Installazione dell'elemento {} ({}/{})...", 
                                                                item, i+1, selected_indices.len()));
                                
                                // Esegui l'operazione
                                let config_guard = match config.lock() {
                                    Ok(guard) => guard,
                                    Err(e) => {
                                        error_messages.push(format!("Errore nel blocco della configurazione: {}", e));
                                        continue;
                                    }
                                };
                                
                                item.install(&config_guard)
                            };
                            
                            match result {
                                Ok(_) => success_count += 1,
                                Err(e) => error_messages.push(format!("Errore nell'operazione su {}: {}", idx, e)),
                            }
                        }
                        
                        // Rimuovi la finestra di progresso
                        s.pop_layer();
                        
                        // Mostra il risultato
                        if error_messages.is_empty() {
                            s.add_layer(Dialog::info(format!("Tutti i {} elementi sono stati elaborati con successo", success_count))
                                         .fixed_width(60)
                                         .fixed_height(10));
                        } else {
                            let mut result_message = format!("Operazioni completate con successo: {}/{}\n\nErrori:\n", 
                                                          success_count, selected_indices.len());
                            for error in &error_messages {
                                result_message.push_str(&format!("- {}\n", error));
                            }
                            
                            s.add_layer(Dialog::around(TextView::new(result_message).scrollable())
                                .title("Risultato Installazione")
                                .button("OK", |s| { s.pop_layer(); })
                                .fixed_width(70)
                                .fixed_height(15));
                        }
                        
                        // Aggiorna la vista
                        update_ui(&items, &selection_for_update, &selection_info, &cb_sink);
                        
                        // Mostra i log recenti
                        log_view::show_recent_logs_popup(s);
                    }
                })
                .fixed_width(60)
                .fixed_height(10));
        }
    });

    let install_button = Button::new("Install", {
        let items = Arc::clone(&items);
        let config = Arc::clone(&config);
        let selection = Arc::clone(&selection);
        let selection_info = selection_info.clone();
        let cb_sink = siv.cb_sink().clone();
        
        move |s| {
            let idx = match s.call_on_name("item_list", |view: &mut SelectView<usize>| view.selected_id()) {
                Some(Some(idx)) => idx,
                _ => return,
            };

            // Ottieni l'elemento selezionato
            let item_result = {
                let mut items_guard = match items.lock() {
                    Ok(guard) => guard,
                    Err(e) => {
                        s.add_layer(Dialog::info(format!("Errore nel blocco degli elementi: {}", e))
                                     .fixed_width(50)
                                     .fixed_height(7));
                        return;
                    }
                };

                let item = match items_guard.get_mut(idx) {
                    Some(item) => item,
                    None => {
                        s.add_layer(Dialog::info("Elemento non trovato")
                                     .fixed_width(50)
                                     .fixed_height(7));
                        return;
                    }
                };

                // Verifica la condizione
                if !item.can_install() {
                    s.add_layer(Dialog::info("L'elemento non può essere installato")
                                 .fixed_width(50)
                                 .fixed_height(7));
                    return;
                }

                // Esegui l'operazione
                let config_guard = match config.lock() {
                    Ok(guard) => guard,
                    Err(e) => {
                        s.add_layer(Dialog::info(format!("Errore nel blocco della configurazione: {}", e))
                                     .fixed_width(50)
                                     .fixed_height(7));
                        return;
                    }
                };

                item.install(&config_guard)
            };

            // Gestisci il risultato dell'operazione
            match item_result {
                Ok(_) => {
                    s.add_layer(Dialog::info("Operazione installazione completata con successo")
                                 .fixed_width(50)
                                 .fixed_height(7));
                    update_ui(&items, &selection, &selection_info, &cb_sink);
                    
                    // Mostra i log recenti
                    log_view::show_recent_logs_popup(s);
                },
                Err(e) => {
                    s.add_layer(Dialog::info(format!("Errore durante l'operazione installazione: {}", e))
                                 .fixed_width(50)
                                 .fixed_height(7));
                }
            }
        }
    });

    /*
    let uninstall_button = Button::new("Uninstall", {
        let items = Arc::clone(&items);
        let config = Arc::clone(&config);
        let selection = Arc::clone(&selection);
        let selection_info = selection_info.clone();
        let cb_sink = siv.cb_sink().clone();
        
        move |s| {
            // Check if we have a multi-selection
            let selected_indices = {
                if let Ok(sel) = selection.lock() {
                    sel.get_selected_indices()
                } else {
                    vec![]
                }
            };

            if !selected_indices.is_empty() {
                // Disinstallazione per elementi multipli
                s.add_layer(Dialog::around(TextView::new(format!("Sei sicuro di voler disinstallare {} elementi selezionati?", 
                                                               selected_indices.len())))
                    .title("Conferma Disinstallazione")
                    .button("No", |s| { s.pop_layer(); })
                    .button("Sì", {
                        let items = Arc::clone(&items);
                        let config = Arc::clone(&config);
                        let selected_indices = selected_indices.clone();
                        let selection_info = selection_info.clone();
                        let cb_sink = cb_sink.clone();
                        let selection_for_update = Arc::clone(&selection);
                        
                        move |s| {
                            s.pop_layer();
                            
                            // Mostra una finestra di progresso
                            let progress_text = TextContent::new("Inizializzazione disinstallazione...");
                            let progress_view = Dialog::around(TextView::new_with_content(progress_text.clone()))
                                .title("Disinstallazione in corso")
                                .fixed_width(60)
                                .fixed_height(10);
                            
                            s.add_layer(progress_view);
                            
                            // Esegui l'operazione su tutti gli elementi selezionati
                            let mut success_count = 0;
                            let mut error_messages = Vec::new();
                            
                            for (i, idx) in selected_indices.iter().enumerate() {
                                let result = {
                                    let mut items_guard = match items.lock() {
                                        Ok(guard) => guard,
                                        Err(e) => {
                                            error_messages.push(format!("Errore nel blocco degli elementi: {}", e));
                                            continue;
                                        }
                                    };
                                    
                                    let item = match items_guard.get_mut(*idx) {
                                        Some(item) => item,
                                        None => {
                                            error_messages.push(format!("Elemento con indice {} non trovato", idx));
                                            continue;
                                        }
                                    };
                                    
                                    // Verifica la condizione
                                    if !item.can_uninstall() {
                                        continue;
                                    }
                                    
                                    // Aggiorna il messaggio di progresso
                                    progress_text.set_content(format!("Disinstallazione dell'elemento {} ({}/{})...", 
                                                                    item, i+1, selected_indices.len()));
                                    
                                    // Esegui l'operazione
                                    let config_guard = match config.lock() {
                                        Ok(guard) => guard,
                                        Err(e) => {
                                            error_messages.push(format!("Errore nel blocco della configurazione: {}", e));
                                            continue;
                                        }
                                    };
                                    
                                    item.uninstall(&config_guard)
                                };
                                
                                match result {
                                    Ok(_) => success_count += 1,
                                    Err(e) => error_messages.push(format!("Errore nell'operazione su {}: {}", idx, e)),
                                }
                            }
                            
                            // Rimuovi la finestra di progresso
                            s.pop_layer();
                            
                            // Mostra il risultato
                            if error_messages.is_empty() {
                                s.add_layer(Dialog::info(format!("Tutti i {} elementi sono stati disinstallati con successo", success_count))
                                             .fixed_width(60)
                                             .fixed_height(10));
                            } else {
                                let mut result_message = format!("Disinstallazioni completate con successo: {}/{}\n\nErrori:\n", 
                                                              success_count, selected_indices.len());
                                for error in &error_messages {
                                    result_message.push_str(&format!("- {}\n", error));
                                }
                                
                                s.add_layer(Dialog::around(TextView::new(result_message).scrollable())
                                    .title("Risultato Disinstallazione")
                                    .button("OK", |s| { s.pop_layer(); })
                                    .fixed_width(70)
                                    .fixed_height(15));
                            }
                            
                            // Aggiorna la vista
                            update_ui(&items, &selection_for_update, &selection_info, &cb_sink);
                            
                            // Mostra i log recenti
                            log_view::show_recent_logs_popup(s);
                        }
                    })
                    .fixed_width(60)
                    .fixed_height(10));
            } else {
                // Disinstallazione per singolo elemento
                let idx = match s.call_on_name("item_list", |view: &mut SelectView<usize>| view.selected_id()) {
                    Some(Some(idx)) => idx,
                    _ => return,
                };

                // Ottieni l'elemento selezionato
                let item_result = {
                    let mut items_guard = match items.lock() {
                        Ok(guard) => guard,
                        Err(e) => {
                            s.add_layer(Dialog::info(format!("Errore nel blocco degli elementi: {}", e))
                                        .fixed_width(50)
                                        .fixed_height(7));
                            return;
                        }
                    };

                    let item = match items_guard.get_mut(idx) {
                        Some(item) => item,
                        None => {
                            s.add_layer(Dialog::info("Elemento non trovato")
                                        .fixed_width(50)
                                        .fixed_height(7));
                            return;
                        }
                    };

                    // Verifica la condizione
                    if !item.can_uninstall() {
                        s.add_layer(Dialog::info("L'elemento non può essere disinstallato")
                                    .fixed_width(50)
                                    .fixed_height(7));
                        return;
                    }

                    // Esegui l'operazione
                    let config_guard = match config.lock() {
                        Ok(guard) => guard,
                        Err(e) => {
                            s.add_layer(Dialog::info(format!("Errore nel blocco della configurazione: {}", e))
                                        .fixed_width(50)
                                        .fixed_height(7));
                            return;
                        }
                    };

                    item.uninstall(&config_guard)
                };

                // Gestisci il risultato dell'operazione
                match item_result {
                    Ok(_) => {
                        s.add_layer(Dialog::info("Operazione disinstallazione completata con successo")
                                    .fixed_width(50)
                                    .fixed_height(7));
                        update_ui(&items, &selection, &selection_info, &cb_sink);
                        
                        // Mostra i log recenti
                        log_view::show_recent_logs_popup(s);
                    },
                    Err(e) => {
                        s.add_layer(Dialog::info(format!("Errore durante l'operazione disinstallazione: {}", e))
                                    .fixed_width(50)
                                    .fixed_height(7));
                    }
                }
            }
        }
    });
    */
    /*
    let reset_button = Button::new("Reset", {
        let items = Arc::clone(&items);
        let config = Arc::clone(&config);
        let selection = Arc::clone(&selection);
        let selection_info = selection_info.clone();
        let cb_sink = siv.cb_sink().clone();
        
        move |s| {
            // Check if we have a multi-selection
            let selected_indices = {
                if let Ok(sel) = selection.lock() {
                    sel.get_selected_indices()
                } else {
                    vec![]
                }
            };

            if !selected_indices.is_empty() {
                // Reset per elementi multipli
                s.add_layer(Dialog::around(TextView::new(format!("Sei sicuro di voler resettare {} elementi selezionati?", 
                                                               selected_indices.len())))
                    .title("Conferma Reset")
                    .button("No", |s| { s.pop_layer(); })
                    .button("Sì", {
                        let items = Arc::clone(&items);
                        let config = Arc::clone(&config);
                        let selected_indices = selected_indices.clone();
                        let selection_info = selection_info.clone();
                        let cb_sink = cb_sink.clone();
                        
                        move |s| {
                            // Implementazione del reset multiplo
                            s.pop_layer();
                            
                            // Mostra una finestra di progresso
                            let progress_text = TextContent::new("Inizializzazione reset...");
                            let progress_view = Dialog::around(TextView::new_with_content(progress_text.clone()))
                                .title("Reset in corso")
                                .fixed_width(60)
                                .fixed_height(10);
                            
                            s.add_layer(progress_view);
                            
                            // Esegui l'operazione su tutti gli elementi selezionati
                            let mut success_count = 0;
                            let mut error_messages = Vec::new();
                            
                            for (i, idx) in selected_indices.iter().enumerate() {
                                let result = {
                                    let mut items_guard = match items.lock() {
                                        Ok(guard) => guard,
                                        Err(e) => {
                                            error_messages.push(format!("Errore nel blocco degli elementi: {}", e));
                                            continue;
                                        }
                                    };
                                    
                                    let item = match items_guard.get_mut(*idx) {
                                        Some(item) => item,
                                        None => {
                                            error_messages.push(format!("Elemento con indice {} non trovato", idx));
                                            continue;
                                        }
                                    };
                                    
                                    // Verifica la condizione
                                    if !item.can_reset() {
                                        continue;
                                    }
                                    
                                    // Aggiorna il messaggio di progresso
                                    progress_text.set_content(format!("Reset dell'elemento {} ({}/{})...", 
                                                                    item, i+1, selected_indices.len()));
                                    
                                    // Esegui l'operazione
                                    let config_guard = match config.lock() {
                                        Ok(guard) => guard,
                                        Err(e) => {
                                            error_messages.push(format!("Errore nel blocco della configurazione: {}", e));
                                            continue;
                                        }
                                    };
                                    
                                    item.reset(&config_guard)
                                };
                                
                                match result {
                                    Ok(_) => success_count += 1,
                                    Err(e) => error_messages.push(format!("Errore nell'operazione su {}: {}", idx, e)),
                                }
                            }
                            
                            // Rimuovi la finestra di progresso
                            s.pop_layer();
                            
                            // Mostra il risultato
                            if error_messages.is_empty() {
                                s.add_layer(Dialog::info(format!("Tutti i {} elementi sono stati resettati con successo", success_count))
                                             .fixed_width(60)
                                             .fixed_height(10));
                            } else {
                                let mut result_message = format!("Reset completati con successo: {}/{}\n\nErrori:\n", 
                                                              success_count, selected_indices.len());
                                for error in &error_messages {
                                    result_message.push_str(&format!("- {}\n", error));
                                }
                                
                                s.add_layer(Dialog::around(TextView::new(result_message).scrollable())
                                    .title("Risultato Reset")
                                    .button("OK", |s| { s.pop_layer(); })
                                    .fixed_width(70)
                                    .fixed_height(15));
                            }
                            
                            // Aggiorna la vista
                            update_ui(&items, &selection, &selection_info, &cb_sink);
                            
                            // Mostra i log recenti
                            log_view::show_recent_logs_popup(s);
                        }
                    })
                    .fixed_width(60)
                    .fixed_height(10));
            } else {
                // Reset per singolo elemento
                let idx = match s.call_on_name("item_list", |view: &mut SelectView<usize>| view.selected_id()) {
                    Some(Some(idx)) => idx,
                    _ => return,
                };

                // Ottieni l'elemento selezionato
                let item_result = {
                    let mut items_guard = match items.lock() {
                        Ok(guard) => guard,
                        Err(e) => {
                            s.add_layer(Dialog::info(format!("Errore nel blocco degli elementi: {}", e))
                                        .fixed_width(50)
                                        .fixed_height(7));
                            return;
                        }
                    };

                    let item = match items_guard.get_mut(idx) {
                        Some(item) => item,
                        None => {
                            s.add_layer(Dialog::info("Elemento non trovato")
                                        .fixed_width(50)
                                        .fixed_height(7));
                            return;
                        }
                    };

                    // Verifica la condizione
                    if !item.can_reset() {
                        s.add_layer(Dialog::info("L'elemento non può essere resettato")
                                    .fixed_width(50)
                                    .fixed_height(7));
                        return;
                    }

                    // Esegui l'operazione
                    let config_guard = match config.lock() {
                        Ok(guard) => guard,
                        Err(e) => {
                            s.add_layer(Dialog::info(format!("Errore nel blocco della configurazione: {}", e))
                                        .fixed_width(50)
                                        .fixed_height(7));
                            return;
                        }
                    };

                    item.reset(&config_guard)
                };

                // Gestisci il risultato dell'operazione
                match item_result {
                    Ok(_) => {
                        s.add_layer(Dialog::info("Operazione reset completata con successo")
                                    .fixed_width(50)
                                    .fixed_height(7));
                        update_ui(&items, &selection, &selection_info, &cb_sink);
                        
                        // Mostra i log recenti
                        log_view::show_recent_logs_popup(s);
                    },
                    Err(e) => {
                        s.add_layer(Dialog::info(format!("Errore durante l'operazione reset: {}", e))
                                    .fixed_width(50)
                                    .fixed_height(7));
                    }
                }
            }
        }
    });
    */
    /*
    let remediate_button = Button::new("Remediate", {
        let items = Arc::clone(&items);
        let config = Arc::clone(&config);
        let selection = Arc::clone(&selection);
        let selection_info = selection_info.clone();
        let cb_sink = siv.cb_sink().clone();
        
        move |s| {
            // Check if we have a multi-selection
            let selected_indices = {
                if let Ok(sel) = selection.lock() {
                    sel.get_selected_indices()
                } else {
                    vec![]
                }
            };

            if !selected_indices.is_empty() {
                // Remediate per elementi multipli
                s.add_layer(Dialog::around(TextView::new(format!("Sei sicuro di voler applicare remediation a {} elementi selezionati?", 
                                                               selected_indices.len())))
                    .title("Conferma Remediation")
                    .button("No", |s| { s.pop_layer(); })
                    .button("Sì", {
                        let items = Arc::clone(&items);
                        let config = Arc::clone(&config);
                        let selected_indices = selected_indices.clone();
                        let selection_info = selection_info.clone();
                        let cb_sink = cb_sink.clone();
                        
                        move |s| {
                            // Implementazione di remediate multiplo
                            s.pop_layer();
                            
                            // Mostra una finestra di progresso
                            let progress_text = TextContent::new("Inizializzazione remediation...");
                            let progress_view = Dialog::around(TextView::new_with_content(progress_text.clone()))
                                .title("Remediation in corso")
                                .fixed_width(60)
                                .fixed_height(10);
                            
                            s.add_layer(progress_view);
                            
                            // Esegui l'operazione su tutti gli elementi selezionati
                            let mut success_count = 0;
                            let mut error_messages = Vec::new();
                            
                            for (i, idx) in selected_indices.iter().enumerate() {
                                let result = {
                                    let mut items_guard = match items.lock() {
                                        Ok(guard) => guard,
                                        Err(e) => {
                                            error_messages.push(format!("Errore nel blocco degli elementi: {}", e));
                                            continue;
                                        }
                                    };
                                    
                                    let item = match items_guard.get_mut(*idx) {
                                        Some(item) => item,
                                        None => {
                                            error_messages.push(format!("Elemento con indice {} non trovato", idx));
                                            continue;
                                        }
                                    };
                                    
                                    // Verifica la condizione
                                    if !item.can_remediate() {
                                        continue;
                                    }
                                    
                                    // Aggiorna il messaggio di progresso
                                    progress_text.set_content(format!("Remediation dell'elemento {} ({}/{})...", 
                                                                    item, i+1, selected_indices.len()));
                                    
                                    // Esegui l'operazione
                                    let config_guard = match config.lock() {
                                        Ok(guard) => guard,
                                        Err(e) => {
                                            error_messages.push(format!("Errore nel blocco della configurazione: {}", e));
                                            continue;
                                        }
                                    };
                                    
                                    item.remediate(&config_guard)
                                };
                                
                                match result {
                                    Ok(_) => success_count += 1,
                                    Err(e) => error_messages.push(format!("Errore nell'operazione su {}: {}", idx, e)),
                                }
                            }
                            
                            // Rimuovi la finestra di progresso
                            s.pop_layer();
                            
                            // Mostra il risultato
                            if error_messages.is_empty() {
                                s.add_layer(Dialog::info(format!("Tutti i {} elementi sono stati rimediati con successo", success_count))
                                             .fixed_width(60)
                                             .fixed_height(10));
                            } else {
                                let mut result_message = format!("Remediation completate con successo: {}/{}\n\nErrori:\n", 
                                                              success_count, selected_indices.len());
                                for error in &error_messages {
                                    result_message.push_str(&format!("- {}\n", error));
                                }
                                
                                s.add_layer(Dialog::around(TextView::new(result_message).scrollable())
                                    .title("Risultato Remediation")
                                    .button("OK", |s| { s.pop_layer(); })
                                    .fixed_width(70)
                                    .fixed_height(15));
                            }
                            
                            // Aggiorna la vista
                            update_ui(&items, &selection, &selection_info, &cb_sink);
                            
                            // Mostra i log recenti
                            log_view::show_recent_logs_popup(s);
                        }
                    })
                    .fixed_width(60)
                    .fixed_height(10));
            } else {
                // Remediate per singolo elemento
                let idx = match s.call_on_name("item_list", |view: &mut SelectView<usize>| view.selected_id()) {
                    Some(Some(idx)) => idx,
                    _ => return,
                };

                // Ottieni l'elemento selezionato
                let item_result = {
                    let mut items_guard = match items.lock() {
                        Ok(guard) => guard,
                        Err(e) => {
                            s.add_layer(Dialog::info(format!("Errore nel blocco degli elementi: {}", e))
                                        .fixed_width(50)
                                        .fixed_height(7));
                            return;
                        }
                    };

                    let item = match items_guard.get_mut(idx) {
                        Some(item) => item,
                        None => {
                            s.add_layer(Dialog::info("Elemento non trovato")
                                        .fixed_width(50)
                                        .fixed_height(7));
                            return;
                        }
                    };

                    // Verifica la condizione
                    if !item.can_remediate() {
                        s.add_layer(Dialog::info("L'elemento non può essere rimediato")
                                    .fixed_width(50)
                                    .fixed_height(7));
                        return;
                    }

                    // Esegui l'operazione
                    let config_guard = match config.lock() {
                        Ok(guard) => guard,
                        Err(e) => {
                            s.add_layer(Dialog::info(format!("Errore nel blocco della configurazione: {}", e))
                                        .fixed_width(50)
                                        .fixed_height(7));
                            return;
                        }
                    };

                    item.remediate(&config_guard)
                };

                // Gestisci il risultato dell'operazione
                match item_result {
                    Ok(_) => {
                        s.add_layer(Dialog::info("Operazione remediation completata con successo")
                                    .fixed_width(50)
                                    .fixed_height(7));
                        update_ui(&items, &selection, &selection_info, &cb_sink);
                        
                        // Mostra i log recenti
                        log_view::show_recent_logs_popup(s);
                    },
                    Err(e) => {
                        s.add_layer(Dialog::info(format!("Errore durante l'operazione remediation: {}", e))
                                    .fixed_width(50)
                                    .fixed_height(7));
                    }
                }
            }
        }
    });
    */

    // Bottone per pulire la selezione
    let clear_selection_button = {
        let selection = Arc::clone(&selection);
        let items = Arc::clone(&items);
        let selection_info = selection_info.clone();
        let cb_sink = siv.cb_sink().clone();
        
        Button::new("Pulisci Selezione", move |_s| {
            {
                if let Ok(mut sel) = selection.lock() {
                    sel.clear();
                }
            }
            update_ui(&items, &selection, &selection_info, &cb_sink);
        })
    };

    // Crea la barra dei pulsanti
    let buttons = LinearLayout::horizontal()
        .child(install_all_button)
        .child(DummyView.fixed_width(1))
        .child(install_button)
        .child(DummyView.fixed_width(1))
        //.child(uninstall_button)
        //.child(DummyView.fixed_width(1))
        //.child(reset_button)
        //.child(DummyView.fixed_width(1))
        //.child(remediate_button)
        //.child(DummyView.fixed_width(1))
        .child(clear_selection_button);

    // Layout principale
    let layout = LinearLayout::vertical()
        .child(Panel::new(select_view_with_events.scrollable())
            .title("Elementi disponibili")
            .fixed_width(PANEL_WIDTH)
            .fixed_height(PANEL_HEIGHT))
        .child(selection_info_view)
        .child(DummyView.fixed_height(1))
        .child(Panel::new(item_detail_view)
            .title("Dettagli")
            .fixed_width(PANEL_WIDTH)
            .fixed_height(10))
        .child(DummyView.fixed_height(1))
        .child(buttons);

    // Aggiungi la vista alla UI
    siv.add_layer(Dialog::around(layout)
        .title(view_title)
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