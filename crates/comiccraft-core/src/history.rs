use crate::document::Document;

#[derive(Clone, Debug)]
pub struct HistoryState {
    pub description: String,
    pub doc_snapshot: Document,
}

pub struct HistoryManager {
    undo_stack: Vec<HistoryState>,
    redo_stack: Vec<HistoryState>,
    max_history: usize,
}

impl HistoryManager {
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history: max_history.max(5),
        }
    }

    pub fn push_state(&mut self, description: impl Into<String>, doc: &Document) {
        if self.undo_stack.len() >= self.max_history {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(HistoryState {
            description: description.into(),
            doc_snapshot: Self::snapshot(doc),
        });
        self.redo_stack.clear();
    }

    /// Clones the document without the (large, rebuildable) composite cache.
    fn snapshot(doc: &Document) -> Document {
        let mut snap = doc.clone();
        snap.composite_cache = None;
        snap
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo(&mut self, current_doc: &mut Document) -> bool {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(HistoryState {
                description: prev.description.clone(),
                doc_snapshot: Self::snapshot(current_doc),
            });
            *current_doc = prev.doc_snapshot;
            current_doc.update_composite();
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self, current_doc: &mut Document) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(HistoryState {
                description: next.description.clone(),
                doc_snapshot: Self::snapshot(current_doc),
            });
            *current_doc = next.doc_snapshot;
            current_doc.update_composite();
            true
        } else {
            false
        }
    }

    pub fn undo_entries(&self) -> Vec<&str> {
        self.undo_stack.iter().map(|s| s.description.as_str()).collect()
    }

    pub fn redo_entries(&self) -> Vec<&str> {
        self.redo_stack.iter().map(|s| s.description.as_str()).collect()
    }
}
