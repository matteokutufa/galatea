// File: src/ui/components/selection.rs

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::fmt::Display;

/// Componente generico per gestire la selezione multipla di elementi in una lista
pub struct MultiSelection<T> {
    /// Indici degli elementi selezionati
    selected_indices: HashSet<usize>,
    /// Tipo di marker per consentire la parametrizzazione
    _marker: std::marker::PhantomData<T>,
}

impl<T> MultiSelection<T> {
    /// Crea una nuova istanza del componente di selezione multipla
    pub fn new() -> Self {
        MultiSelection {
            selected_indices: HashSet::new(),
            _marker: std::marker::PhantomData,
        }
    }

    /// Attiva/disattiva la selezione di un elemento
    pub fn toggle(&mut self, idx: usize) {
        if self.selected_indices.contains(&idx) {
            self.selected_indices.remove(&idx);
        } else {
            self.selected_indices.insert(idx);
        }
    }

    /// Verifica se un elemento è selezionato
    pub fn is_selected(&self, idx: usize) -> bool {
        self.selected_indices.contains(&idx)
    }

    /// Cancella tutte le selezioni
    pub fn clear(&mut self) {
        self.selected_indices.clear();
    }

    /// Conta quanti elementi sono selezionati
    pub fn count(&self) -> usize {
        self.selected_indices.len()
    }

    /// Restituisce un vettore ordinato di indici selezionati
    pub fn get_selected_indices(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = self.selected_indices.iter().cloned().collect();
        indices.sort();
        indices
    }
}

/// Trait per elementi che possono essere visualizzati in una lista selezionabile
pub trait SelectableItem: Display {
    /// Determina lo stato dell'elemento per visualizzazione
    fn get_status_marker(&self) -> &'static str;
    
    /// Formatta l'elemento per la visualizzazione nella lista
    fn format_for_list(&self) -> String;
    
    /// Formatta l'elemento per la visualizzazione dettagliata
    fn format_details(&self) -> String;
    
    /// Verifica se l'elemento può essere installato
    fn can_install(&self) -> bool;
    
    /// Verifica se l'elemento può essere disinstallato
    fn can_uninstall(&self) -> bool;
    
    /// Verifica se l'elemento può essere resettato
    fn can_reset(&self) -> bool;
    
    /// Verifica se l'elemento può essere rimediato
    fn can_remediate(&self) -> bool;
}

/// Struttura contenitore condivisa per l'accesso thread-safe agli elementi
pub type SharedSelection<T> = Arc<Mutex<MultiSelection<T>>>;

/// Crea una nuova selezione condivisa
pub fn new_shared_selection<T>() -> SharedSelection<T> {
    Arc::new(Mutex::new(MultiSelection::new()))
}