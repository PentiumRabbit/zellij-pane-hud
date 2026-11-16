use crate::model::NodeKey;
use crate::rows::Rows;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TextQuery {
    value: String,
}

impl TextQuery {
    pub fn as_str(&self) -> &str {
        &self.value
    }

    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    pub fn insert_char(&mut self, character: char) {
        self.value.push(character);
    }

    pub fn insert_str(&mut self, text: &str) {
        self.value.push_str(text);
    }

    pub fn backspace(&mut self) {
        self.value.pop();
    }

    pub fn delete_word(&mut self) {
        while self.value.ends_with(char::is_whitespace) {
            self.value.pop();
        }
        while self
            .value
            .chars()
            .last()
            .is_some_and(|character| !character.is_whitespace())
        {
            self.value.pop();
        }
    }

    pub fn clear(&mut self) {
        self.value.clear();
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Selection {
    index: usize,
    selected_key: Option<NodeKey>,
}

impl Selection {
    pub fn with_index(index: usize) -> Self {
        Self {
            index,
            selected_key: None,
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn selected_key(&self) -> Option<&NodeKey> {
        self.selected_key.as_ref()
    }

    pub fn select_key(&mut self, rows: &Rows, key: &NodeKey) {
        if let Some(index) = rows.rows.iter().position(|row| &row.key == key) {
            self.index = index;
        }
        self.sync_key(rows);
    }

    pub fn select_first(&mut self, rows: &Rows) {
        self.index = 0;
        self.sync_key(rows);
    }

    pub fn restore(&mut self, rows: &Rows, fallback: Option<&NodeKey>) {
        if !rows.rows.is_empty() {
            if let Some(selected) = self.selected_key.as_ref() {
                if let Some(index) = rows.rows.iter().position(|row| &row.key == selected) {
                    self.index = index;
                    return;
                }
            }
            if let Some(fallback) = fallback {
                if let Some(index) = rows.rows.iter().position(|row| &row.key == fallback) {
                    self.index = index;
                }
            }
        }
        self.clamp(rows);
    }

    pub fn up(&mut self, rows: &Rows) {
        if rows.rows.is_empty() {
            self.index = 0;
            self.selected_key = None;
            return;
        }
        self.index = if self.index == 0 {
            rows.rows.len() - 1
        } else {
            self.index - 1
        };
        self.sync_key(rows);
    }

    pub fn down(&mut self, rows: &Rows) {
        if rows.rows.is_empty() {
            self.index = 0;
            self.selected_key = None;
            return;
        }
        self.index = (self.index + 1) % rows.rows.len();
        self.sync_key(rows);
    }

    pub fn page_up(&mut self, rows: &Rows, amount: usize) {
        if rows.rows.is_empty() {
            return;
        }
        self.index = self.index.saturating_sub(amount.max(1));
        self.sync_key(rows);
    }

    pub fn page_down(&mut self, rows: &Rows, amount: usize) {
        if rows.rows.is_empty() {
            return;
        }
        self.index = (self.index + amount.max(1)).min(rows.rows.len() - 1);
        self.sync_key(rows);
    }

    fn clamp(&mut self, rows: &Rows) {
        if rows.rows.is_empty() {
            self.index = 0;
            self.selected_key = None;
        } else {
            self.index = self.index.min(rows.rows.len() - 1);
            self.sync_key(rows);
        }
    }

    fn sync_key(&mut self, rows: &Rows) {
        self.selected_key = rows.rows.get(self.index).map(|row| row.key.clone());
    }
}
