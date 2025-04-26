//! Visualizzazione e gestione degli stack nell'interfaccia utente
//!
//! Questo modulo fornisce la visualizzazione e l'interazione con gli stack.

use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow};

use cursive::Cursive;
use cursive::views::{Dialog, SelectView, TextView, LinearLayout, DummyView, Panel, TextContent, Button};
use cursive::view::Scrollable;
use cursive::traits::*;
use cursive::align::HAlign;

use crate::config::Config;
use crate::task::Task;
use crate::stack::Stack;

/// Crea la vista per la gestione degli stack
pub fn create_stack_view(siv: &mut Cursive, config: Arc<Mutex<Config>>, stacks: Arc<Mutex<Vec<Stack>>>, tasks: Arc<Mutex<Vec<Task>>>) -> Result<()> {
    let stacks_clone = Arc::clone(&stacks);

    // Ottiene gli stack dal mutex
    let stacks_guard = stacks.lock().map_err(|_| anyhow!("Failed to lock stacks mutex"))?;

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
        let select_view_cb = siv.cb_sink().clone();

        move || {
            if let Ok(stacks_guard) = stacks.lock() {
                // Crea una copia dei dati necessari
                let stack_data: Vec<(bool, bool, String, String)> = stacks_guard.iter()
                    .map(|s| (s.fully_installed, s.partially_installed, s.name.clone(), s.description.clone()))
                    .collect();

                // Invia i dati al callback
                select_view_cb.send(Box::new(move |s: &mut Cursive| {
                    s.call_on_name("stack_list", |view: &mut SelectView<usize>| {
                        view.clear();

                        for (idx, (fully_installed, partially_installed, name, description)) in stack_data.iter().enumerate() {
                            let status = if *fully_installed {
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
                })).unwrap();
            }
        }
    };

    // Crea i bottoni per le azioni
    let config_clone = Arc::clone(&config);
    let stacks_clone = Arc::clone(&stacks);
    let tasks_clone = Arc::clone(&tasks);
    let update_clone = update_stack_view.clone();

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
            if let Ok(mut stacks_guard) = stacks.lock() {
                let stack = match stacks_guard.get_mut(idx) {
                    Some(stack) => stack,
                    None => return,
                };

                // Verifica che lo stack non sia già installato
                if stack.fully_installed {
                    s.add_layer(Dialog::info("Lo stack è già completamente installato"));
                    return;
                }

                // Installa lo stack
                if let Ok(config_guard) = config.lock() {
                    if let Ok(mut tasks_guard) = tasks.lock() {
                        match stack.install(&config_guard, &mut tasks_guard) {
                            Ok(_) => {
                                s.add_layer(Dialog::info(format!("Stack {} installato con successo", stack.name)));
                                update_fn();
                            },
                            Err(e) => {
                                s.add_layer(Dialog::info(format!("Errore durante l'installazione dello stack: {}", e)));
                            }
                        }
                    }
                }
            }
        })
    };

    let uninstall_button = {
        let stacks = Arc::clone(&stacks_clone);
        let tasks = Arc::clone(&tasks_clone);
        let config = Arc::clone(&config_clone);
        let update_fn = update_clone.clone();

        Button::new("Uninstall", move |s| {
            let idx = match s.call_on_name("stack_list", |view: &mut SelectView<usize>| view.selected_id()) {
                Some(Some(idx)) => idx,
                _ => return,
            };

            // Ottieni lo stack selezionato
            if let Ok(mut stacks_guard) = stacks.lock() {
                let stack = match stacks_guard.get_mut(idx) {
                    Some(stack) => stack,
                    None => return,
                };

                // Verifica che lo stack sia almeno parzialmente installato
                if !stack.fully_installed && !stack.partially_installed {
                    s.add_layer(Dialog::info("Lo stack non è installato"));
                    return;
                }

                // Disinstalla lo stack
                if let Ok(config_guard) = config.lock() {
                    if let Ok(mut tasks_guard) = tasks.lock() {
                        match stack.uninstall(&config_guard, &mut tasks_guard) {
                            Ok(_) => {
                                s.add_layer(Dialog::info(format!("Stack {} disinstallato con successo", stack.name)));
                                update_fn();
                            },
                            Err(e) => {
                                s.add_layer(Dialog::info(format!("Errore durante la disinstallazione dello stack: {}", e)));
                            }
                        }
                    }
                }
            }
        })
    };

    let reset_button = {
        let stacks = Arc::clone(&stacks_clone);
        let tasks = Arc::clone(&tasks_clone);
        let config = Arc::clone(&config_clone);

        Button::new("Reset", move |s| {
            let idx = match s.call_on_name("stack_list", |view: &mut SelectView<usize>| view.selected_id()) {
                Some(Some(idx)) => idx,
                _ => return,
            };

            // Ottieni lo stack selezionato
            if let Ok(mut stacks_guard) = stacks.lock() {
                let stack = match stacks_guard.get_mut(idx) {
                    Some(stack) => stack,
                    None => return,
                };

                // Verifica che lo stack sia almeno parzialmente installato
                if !stack.fully_installed && !stack.partially_installed {
                    s.add_layer(Dialog::info("Lo stack non è installato"));
                    return;
                }

                // Reset dello stack
                if let Ok(config_guard) = config.lock() {
                    if let Ok(mut tasks_guard) = tasks.lock() {
                        match stack.reset(&config_guard, &mut tasks_guard) {
                            Ok(_) => {
                                s.add_layer(Dialog::info(format!("Reset dello stack {} completato con successo", stack.name)));
                            },
                            Err(e) => {
                                s.add_layer(Dialog::info(format!("Errore durante il reset dello stack: {}", e)));
                            }
                        }
                    }
                }
            }
        })
    };

    let remediate_button = {
        let stacks = Arc::clone(&stacks_clone);
        let tasks = Arc::clone(&tasks_clone);
        let config = Arc::clone(&config_clone);

        Button::new("Remediate", move |s| {
            let idx = match s.call_on_name("stack_list", |view: &mut SelectView<usize>| view.selected_id()) {
                Some(Some(idx)) => idx,
                _ => return,
            };

            // Ottieni lo stack selezionato
            if let Ok(mut stacks_guard) = stacks.lock() {
                let stack = match stacks_guard.get_mut(idx) {
                    Some(stack) => stack,
                    None => return,
                };

                // Verifica che lo stack sia almeno parzialmente installato
                if !stack.fully_installed && !stack.partially_installed {
                    s.add_layer(Dialog::info("Lo stack non è installato"));
                    return;
                }

                // Riavvia i servizi dello stack
                if let Ok(config_guard) = config.lock() {
                    if let Ok(mut tasks_guard) = tasks.lock() {
                        match stack.remediate(&config_guard, &mut tasks_guard) {
                            Ok(_) => {
                                s.add_layer(Dialog::info(format!("Remediation dello stack {} completata con successo", stack.name)));
                            },
                            Err(e) => {
                                s.add_layer(Dialog::info(format!("Errore durante la remediation dello stack: {}", e)));
                            }
                        }
                    }
                }
            }
        })
    };

    // Crea la barra dei pulsanti
    let buttons = LinearLayout::horizontal()
        .child(install_button)
        .child(DummyView.fixed_width(1))
        .child(uninstall_button)
        .child(DummyView.fixed_width(1))
        .child(reset_button)
        .child(DummyView.fixed_width(1))
        .child(remediate_button);

    // Layout principale
    let layout = LinearLayout::vertical()
        .child(Panel::new(select_view.with_name("stack_list").scrollable())
            .title("Stack disponibili"))
        .child(DummyView.fixed_height(1))
        .child(Panel::new(stack_detail_view)
            .title("Dettagli dello stack"))
        .child(DummyView.fixed_height(1))
        .child(buttons);

    // Aggiungi la vista alla UI
    siv.add_layer(Dialog::around(layout)
        .title("Gestione Stack")
        .button("Back", |s| {
            s.pop_layer();
        }));

    Ok(())
}