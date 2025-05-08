// File: src/ui/components/selectable_view.rs

use std::sync::{Arc, Mutex};
use std::fmt::Display;
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
use crate::ui::components::selection::{MultiSelection, SelectableItem, SharedSelection};

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
    can_modify_items: bool, // Se gli elementi possono essere modificati (es: task installati)
) -> Result<()> 
where
    T: 'static,
    E: SelectableItem + Executable<E> + Clone + 'static,
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

    // Rilascia il lock prima di creare le closure
    drop(items_guard);

    // Dettagli dell'elemento selezionato
    let item_detail = TextContent::new("Seleziona un elemento per vedere i dettagli");
    let item_detail_view = TextView::new_with_content(item_detail.clone())
        .scrollable();

    // Aggiungi handler per la selezione multipla con Invio
    let selection_clone = Arc::clone(&selection);
    let select_view = select_view.with_name("item_list");
    
    // Avvolgi con OnEventView per gestire gli eventi
    let select_view_with_events = OnEventView::new(select_view)
    .on_event_inner(Event::Key(Key::Enter), move |view, _event| {
        if let Some(idx) = view.selected_id() {
            if let Ok(mut sel) = selection_clone.lock() {
                sel.toggle(idx);
                
                // Aggiorna l'interfaccia utente per mostrare la selezione
                let is_selected = sel.is_selected(idx);
                
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
            }
            Some(cursive::event::EventResult::Consumed(None))
        } else {
            None
        }
    }).with_name("item_list");

    // Informazioni sulla selezione
    let selection_info = TextContent::new("Premi 'Invio' per selezionare/deselezionare. Nessun elemento selezionato.");
    let selection_info_view = TextView::new_with_content(selection_info.clone())
        .h_align(HAlign::Center);

    // Gestisci la selezione degli elementi
    let items_clone = Arc::clone(&items);
    let selection_clone = Arc::clone(&selection);
    select_view.set_on_select(move |_siv, idx| {
        if let Ok(items_guard) = items_clone.lock() {
            if let Some(item) = items_guard.get(*idx) {
                // Aggiorna il testo dei dettagli
                item_detail.set_content(item.format_details());
            }
        }
    });

    // Funzione per aggiornare la vista
    let update_view = {
        let items = Arc::clone(&items);
        let selection = Arc::clone(&selection);
        let selection_info_content = selection_info.clone();
        let cb_sink = siv.cb_sink().clone();

        move || {
            if let Ok(items_guard) = items.lock() {
                // Raccogli i dati necessari per l'aggiornamento
                let items_data: Vec<(String, usize)> = items_guard.iter().enumerate()
                    .map(|(idx, item)| (item.format_for_list(), idx))
                    .collect();

                // Invia i dati al callback
                if let Err(e) = cb_sink.send(Box::new(move |s: &mut Cursive| {
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
                    eprintln!("Errore nell'aggiornamento della vista: {}", e);
                }
            }
        }
    };

    // Funzione per eseguire un'operazione sugli elementi selezionati
    let execute_on_selected = {
        let items = Arc::clone(&items);
        let config = Arc::clone(&config);
        let selection = Arc::clone(&selection);
        let update_fn = update_view.clone();
        
        move |s: &mut Cursive, 
              operation_name: &str, 
              operation_fn: fn(&mut E, &Config) -> Result<()>, 
              condition_fn: fn(&E) -> bool| {
            
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
            s.add_layer(Dialog::around(TextView::new(format!("Sei sicuro di voler {} {} elementi selezionati?", 
                                                           operation_name.to_lowercase(), 
                                                           selected_indices.len())))
                .title(format!("Conferma {}", operation_name))
                .button("No", |s| { s.pop_layer(); })
                .button("Sì", {
                    let items = Arc::clone(&items);
                    let config = Arc::clone(&config);
                    let update_fn = update_fn.clone();
                    let selected_indices = selected_indices.clone();
                    let operation_name = operation_name.to_string();
                    
                    move |s| {
                        s.pop_layer();
                        
                        // Mostra una finestra di progresso
                        let progress_text = TextContent::new(format!("Inizializzazione {}...", operation_name.to_lowercase()));
                        let progress_view = Dialog::around(TextView::new_with_content(progress_text.clone()))
                            .title(format!("{} in corso", operation_name))
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
                                if !condition_fn(item) {
                                    continue;
                                }
                                
                                // Aggiorna il messaggio di progresso
                                progress_text.set_content(format!("{} dell'elemento {} ({}/{})...", 
                                                                operation_name,
                                                                item, i+1, selected_indices.len()));
                                
                                // Esegui l'operazione
                                let config_guard = match config.lock() {
                                    Ok(guard) => guard,
                                    Err(e) => {
                                        error_messages.push(format!("Errore nel blocco della configurazione: {}", e));
                                        continue;
                                    }
                                };
                                
                                operation_fn(item, &config_guard)
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
                                .title(format!("Risultato {}", operation_name))
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
        }
    };

    // Funzione per eseguire un'operazione su un singolo elemento
    let execute_on_single = {
        let items = Arc::clone(&items);
        let config = Arc::clone(&config);
        let update_fn = update_view.clone();
        
        move |s: &mut Cursive, 
              operation_name: &str, 
              operation_fn: fn(&mut E, &Config) -> Result<()>, 
              condition_fn: fn(&E) -> bool| {
            
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
                if !condition_fn(item) {
                    s.add_layer(Dialog::info(format!("L'elemento non può essere {}", operation_name.to_lowercase()))
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

                operation_fn(item, &config_guard)
            };

            // Gestisci il risultato dell'operazione
            match item_result {
                Ok(_) => {
                    s.add_layer(Dialog::info(format!("Operazione {} completata con successo", operation_name.to_lowercase()))
                                 .fixed_width(50)
                                 .fixed_height(7));
                    update_fn();
                    
                    // Mostra i log recenti
                    log_view::show_recent_logs_popup(s);
                },
                Err(e) => {
                    s.add_layer(Dialog::info(format!("Errore durante l'operazione {}: {}", 
                                                   operation_name.to_lowercase(), e))
                                 .fixed_width(50)
                                 .fixed_height(7));
                }
            }
        }
    };

    // Crea i bottoni per le azioni
    let install_all_button = Button::new("Install Selezionati", {
        let execute_fn = execute_on_selected.clone();
        let items = Arc::clone(&items);
        
        move |s| {
            execute_fn(s, "Installazione", E::install, E::can_install);
        }
    });

    let install_button = Button::new("Install", {
        let execute_fn = execute_on_single.clone();
        
        move |s| {
            execute_fn(s, "Installazione", E::install, E::can_install);
        }
    });

    let uninstall_button = Button::new("Uninstall", {
        let execute_fn = execute_on_selected.clone();
        let execute_single_fn = execute_on_single.clone();
        
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
                execute_fn(s, "Disinstallazione", E::uninstall, E::can_uninstall);
            } else {
                execute_single_fn(s, "Disinstallazione", E::uninstall, E::can_uninstall);
            }
        }
    });

    let reset_button = Button::new("Reset", {
        let execute_fn = execute_on_selected.clone();
        let execute_single_fn = execute_on_single.clone();
        
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
                execute_fn(s, "Reset", E::reset, E::can_reset);
            } else {
                execute_single_fn(s, "Reset", E::reset, E::can_reset);
            }
        }
    });

    let remediate_button = Button::new("Remediate", {
        let execute_fn = execute_on_selected.clone();
        let execute_single_fn = execute_on_single.clone();
        
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
                execute_fn(s, "Remediation", E::remediate, E::can_remediate);
            } else {
                execute_single_fn(s, "Remediation", E::remediate, E::can_remediate);
            }
        }
    });

    // Bottone per pulire la selezione
    let clear_selection_button = {
        let selection = Arc::clone(&selection);
        let update_fn = update_view.clone();
        
        Button::new("Pulisci Selezione", move |_s| {
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