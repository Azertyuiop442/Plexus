
use crate::ui::modal::{Modal, ModalRow};

pub struct InfoModalWidget {
    pub id: String,
    pub title: String,
    pub fields: Vec<(String, String)>,
}

impl InfoModalWidget {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            fields: Vec::new(),
        }
    }

    pub fn build(&self) -> Modal {
        let mut m = Modal::new(&self.id, &self.title);
        for (label, val) in &self.fields {
            m.rows.push(ModalRow::Info(format!("{}: {}", label, val)));
        }
        m.rows
            .push(ModalRow::Info("Press ESC or ENTER to close".to_string()));
        m
    }
}

