// Soluzione completa: Ristrutturazione del file src/ui/components/selectable_view.rs

use std::sync::{Arc, Mutex};
use anyhow::{Result, anyhow};

use cursive::Cursive;
use cursive::views::{Dialog, SelectView, TextView, LinearLayout, DummyView, Panel, TextContent, Button, OnEventView, ScrollView};
use cursive::view::Scrollable;
use cursive::traits::*;
use cursive::align::HAlign;
use cursive::event::{Event, Key};

use crate::config::Config;
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
    
    // Clone items for the on_event closure
    let items_for_event = Arc::clone(&items);
    
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
                        
                        // Aggiorna l'etichetta basata sulla selezione - CORREZIONE
                        let new_label = if is_selected {
                            if item_label.starts_with("[ ]") {
                                item_label.replacen("[ ]", "[*]", 1)
                            } else if item_label.starts_with("[✓]") {
                                item_label.replacen("[✓]", "[*]", 1)
                            } else if item_label.starts_with("[!]") {
                                item_label.replacen("[!]", "[*]", 1)
                            } else {
                                format!("[*]{}", &item_label[3..])
                            }
                        } else {
                            // Ripristina lo stato originale
                            if item_label.contains("[✓]") {
                                item_label.replacen("[*]", "[✓]", 1)
                            } else if item_label.contains("[!]") {
                                item_label.replacen("[*]", "[!]", 1)
                            } else {
                                item_label.replacen("[*]", "[ ]", 1)
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
                
                // Aggiorna l'area dei log
                s.call_on_name("log_scroll_view", |view: &mut ScrollView<TextView>| {
                    let current_text = view.get_inner().get_content().source().to_string();
                    let item_name = if let Ok(items_guard) = items_for_event.lock() { // Use items_for_event here
                        if let Some(item) = items_guard.get(idx) {
                            format!("{}", item)
                        } else {
                            "elemento sconosciuto".to_string()
                        }
                    } else {
                        "elemento sconosciuto".to_string()
                    };
                    
                    let msg = if is_selected {
                        format!("Elemento selezionato: {}", item_name)
                    } else {
                        format!("Elemento deselezionato: {}", item_name)
                    };
                    
                    view.get_inner_mut().set_content(format!("{}\n{}", current_text, msg));
                    view.scroll_to_bottom();
                });
            }
        }
    });

    // Informazioni sulla selezione
    let selection_info = TextContent::new("Premi 'Invio' per selezionare/deselezionare. Nessun elemento selezionato.");
    let selection_info_view = TextView::new_with_content(selection_info.clone())
        .h_align(HAlign::Center);

    // Funzione di aggiornamento UI
    fn update_ui<T: Send + Sync + 'static, E: SelectableItem + Clone + 'static>(
        items: &Arc<Mutex<Vec<E>>>,
        selection: &SharedSelection<T>,
        selection_info_content: &TextContent,
        cb_sink: &cursive::CbSink,
    ) {
        if let Ok(items_guard) = items.lock() {
            let items_data: Vec<(String, usize)> = items_guard.iter().enumerate()
                .map(|(idx, item)| (item.format_for_list(), idx))
                .collect();

            let items_data = items_data.clone();
            let selection = Arc::clone(selection);
            let selection_info_content = selection_info_content.clone();
            
            if let Err(_) = cb_sink.send(Box::new(move |s: &mut Cursive| {
                let selection_count = {
                    if let Ok(sel) = selection.lock() {
                        sel.count()
                    } else {
                        0
                    }
                };

                if selection_count > 0 {
                    selection_info_content.set_content(format!("Premi 'Invio' per selezionare/deselezionare. {} elementi selezionati.", selection_count));
                } else {
                    selection_info_content.set_content("Premi 'Invio' per selezionare/deselezionare. Nessun elemento selezionato.".to_string());
                }

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

                        // CORREZIONE: Preserva l'etichetta completa
                        let display_str = if is_selected {
                            if item_str.starts_with("[ ]") {
                                item_str.replacen("[ ]", "[*]", 1)
                            } else if item_str.starts_with("[✓]") {
                                item_str.replacen("[✓]", "[*]", 1)
                            } else if item_str.starts_with("[!]") {
                                item_str.replacen("[!]", "[*]", 1)
                            } else {
                                format!("[*]{}", &item_str[3..])
                            }
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

    // BOTTONI PER LE AZIONI
    
    // Install All Button
    let install_all_button = Button::new("Install Selezionati", {
        let items = Arc::clone(&items);
        let config = Arc::clone(&config);
        let selection = Arc::clone(&selection);
        let selection_info = selection_info.clone();
        let cb_sink = siv.cb_sink().clone();
        
        move |s| {
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
                    let outer_selection = Arc::clone(&selection);
                    let selection_clone = Arc::clone(&selection);
                    let selection_for_update = Arc::clone(&selection);
                    
                    move |s| {
                        s.pop_layer();
                        
                        let progress_text = TextContent::new("Inizializzazione installazione...");
                        let progress_view = Dialog::around(TextView::new_with_content(progress_text.clone()))
                            .title("Installazione in corso")
                            .fixed_width(60)
                            .fixed_height(10);
                        
                        s.add_layer(progress_view);
                        
                        // Aggiorna l'area dei log
                        s.call_on_name("log_scroll_view", |view: &mut ScrollView<TextView>| {
                            let current_text = view.get_inner().get_content().source().to_string();
                            view.get_inner_mut().set_content(format!("{}\nAvvio installazione elementi selezionati...", current_text));
                            view.scroll_to_bottom();
                        });
                        
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
                                
                                if !item.can_install() {
                                    continue;
                                }
                                
                                progress_text.set_content(format!("Installazione dell'elemento {} ({}/{})...", 
                                                                item, i+1, selected_indices.len()));
                                
                                // Aggiorna l'area dei log
                                s.call_on_name("log_scroll_view", |view: &mut ScrollView<TextView>| {
                                    let current_text = view.get_inner().get_content().source().to_string();
                                    let msg = format!("Installazione dell'elemento {} ({}/{})...", 
                                                    item, i+1, selected_indices.len());
                                    view.get_inner_mut().set_content(format!("{}\n{}", current_text, msg));
                                    view.scroll_to_bottom();
                                });
                                
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
                                Ok(_) => {
                                    success_count += 1;
                                    // Aggiorna l'area dei log
                                    s.call_on_name("log_scroll_view", |view: &mut ScrollView<TextView>| {
                                        let current_text = view.get_inner().get_content().source().to_string();
                                        view.get_inner_mut().set_content(format!("{}\nCompletato con successo", current_text));
                                        view.scroll_to_bottom();
                                    });
                                },
                                Err(e) => {
                                    error_messages.push(format!("Errore nell'operazione su {}: {}", idx, e));
                                    // Aggiorna l'area dei log
                                    s.call_on_name("log_scroll_view", |view: &mut ScrollView<TextView>| {
                                        let current_text = view.get_inner().get_content().source().to_string();
                                        view.get_inner_mut().set_content(format!("{}\nErrore: {}", current_text, e));
                                        view.scroll_to_bottom();
                                    });
                                }
                            }
                        }
                        
                        s.pop_layer();
                        
                        if error_messages.is_empty() {
                            s.add_layer(Dialog::info(format!("Tutti i {} elementi sono stati elaborati con successo", success_count))
                                         .fixed_width(60)
                                         .fixed_height(10));
                                         
                            // Aggiorna l'area dei log
                            s.call_on_name("log_scroll_view", |view: &mut ScrollView<TextView>| {
                                let current_text = view.get_inner().get_content().source().to_string();
                                view.get_inner_mut().set_content(format!("{}\nInstallazione completata con successo per tutti gli elementi", current_text));
                                view.scroll_to_bottom();
                            });
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
                                
                            // Aggiorna l'area dei log
                            s.call_on_name("log_scroll_view", |view: &mut ScrollView<TextView>| {
                                let current_text = view.get_inner().get_content().source().to_string();
                                view.get_inner_mut().set_content(format!("{}\nInstallazione completata con errori. Successi: {}/{}",
                                                     current_text, success_count, selected_indices.len()));
                                view.scroll_to_bottom();
                            });
                        }
                        
                        update_ui(&items, &selection_for_update, &selection_info, &cb_sink);
                    }
                })
                .fixed_width(60)
                .fixed_height(10));
        }
    });

    // Install Button
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

            // Ottieni il nome dell'elemento per il log
            let item_name = {
                if let Ok(items_guard) = items.lock() {
                    if let Some(item) = items_guard.get(idx) {
                        format!("{}", item)
                    } else {
                        "elemento sconosciuto".to_string()
                    }
                } else {
                    "elemento sconosciuto".to_string()
                }
            };
            
            // Aggiorna l'area dei log
            s.call_on_name("log_scroll_view", |view: &mut ScrollView<TextView>| {
                let current_text = view.get_inner().get_content().source().to_string();
                let msg = format!("Installazione di {}...", item_name);
                view.get_inner_mut().set_content(format!("{}\n{}", current_text, msg));
                view.scroll_to_bottom();
            });

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

                if !item.can_install() {
                    s.add_layer(Dialog::info("L'elemento non può essere installato")
                                 .fixed_width(50)
                                 .fixed_height(7));
                    return;
                }

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

            match item_result {
                Ok(_) => {
                    s.add_layer(Dialog::info("Operazione installazione completata con successo")
                                 .fixed_width(50)
                                 .fixed_height(7));
                    
                    // Aggiorna l'area dei log
                    s.call_on_name("log_scroll_view", |view: &mut ScrollView<TextView>| {
                        let current_text = view.get_inner().get_content().source().to_string();
                        let msg = format!("Operazione completata con successo per {}", item_name);
                        view.get_inner_mut().set_content(format!("{}\n{}", current_text, msg));
                        view.scroll_to_bottom();
                    });
                    
                    update_ui(&items, &selection, &selection_info, &cb_sink);
                    log_view::show_recent_logs_popup(s);
                },
                Err(e) => {
                    s.add_layer(Dialog::info(format!("Errore durante l'operazione installazione: {}", e))
                                 .fixed_width(50)
                                 .fixed_height(7));
                    
                    // Aggiorna l'area dei log
                    s.call_on_name("log_scroll_view", |view: &mut ScrollView<TextView>| {
                        let current_text = view.get_inner().get_content().source().to_string();
                        let msg = format!("Errore durante l'installazione di {}: {}", item_name, e);
                        view.get_inner_mut().set_content(format!("{}\n{}", current_text, msg));
                        view.scroll_to_bottom();
                    });
                }
            }
        }
    });

    // Clear Selection Button
    let clear_selection_button = {
        let selection = Arc::clone(&selection);
        let items = Arc::clone(&items);
        let selection_info = selection_info.clone();
        let cb_sink = siv.cb_sink().clone();
        
        Button::new("Pulisci Selezione", move |s| {
            {
                if let Ok(mut sel) = selection.lock() {
                    sel.clear();
                }
            }
            
            // Aggiorna l'area dei log
            s.call_on_name("log_scroll_view", |view: &mut ScrollView<TextView>| {
                let current_text = view.get_inner().get_content().source().to_string();
                let msg = "Selezione elementi pulita";
                view.get_inner_mut().set_content(format!("{}\n{}", current_text, msg));
                view.scroll_to_bottom();
            });
            
            update_ui(&items, &selection, &selection_info, &cb_sink);
        })
    };

    // Area di log nella parte inferiore - CORREZIONE: Aggiunto ScrollView con nome
    let log_text = TextView::new("Log operazioni:");
    let log_scroll_view = ScrollView::new(log_text)
        .with_name("log_scroll_view")
        .fixed_height(5);  // Altezza fissa di 5 righe

    // NUOVO LAYOUT RISTRUTTURATO
    
    // 1. Contenitore principale diviso in due parti: lista e dettagli
    let main_container = LinearLayout::horizontal()
        .child(Panel::new(select_view_with_events.scrollable().min_size((40, 15)))
            .title("Elementi")
            .full_width())
        .child(DummyView.fixed_width(1))
        .child(Panel::new(item_detail_view)
            .title("Dettagli")
            .full_width());
    
    // 2. Barra inferiore con info sulla selezione
    let selection_bar = LinearLayout::vertical()
        .child(selection_info_view);
    
    // 3. Barra dei pulsanti posizionata orizzontalmente
    let buttons_bar = LinearLayout::horizontal()
        .child(install_all_button)
        .child(DummyView.fixed_width(1))
        .child(install_button)
        .child(DummyView.fixed_width(1))
        .child(clear_selection_button);
    
    // 4. Layout principale con allineamento verticale - AGGIUNTO PANNELLO LOG
    let layout = LinearLayout::vertical()
        .child(main_container)
        .child(DummyView.fixed_height(1))
        .child(selection_bar)
        .child(DummyView.fixed_height(1))
        .child(Panel::new(buttons_bar)
            .title("Azioni"))
        .child(DummyView.fixed_height(1))
        .child(Panel::new(log_scroll_view)
            .title("Log operazioni"));

    // Dialog esterno con dimensioni fisse
    siv.add_layer(Dialog::around(layout)
        .title(view_title)
        .button("Log", |s| {
            log_view::show_recent_logs_popup(s);
        })
        .button("Back", |s| {
            s.pop_layer();
        })
        .full_screen());

    Ok(())
}