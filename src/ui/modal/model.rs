
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum ModalRow {

    Toggle {
        key: String,
        label: String,
        enabled: bool,
    },

    Choice {
        key: String,
        label: String,
        options: Vec<(String, String, String)>,
        current: usize,
        searchable: bool,

        color: String,
    },

    TextInput {
        key: String,
        label: String,
        value: String,
    },

    Info(String),

    InfoColored { text: String, color: String },

    Separator(String),

    Progress {
        label: String,
        current: usize,
        total: usize,
    },

    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,

        color: String,
    },

    Section {
        title: String,

        color: String,
    },

    Stepper {
        key: String,
        label: String,
        value: i64,
        min: i64,
        max: i64,
        step: i64,
        unit: String,
    },
}

impl ModalRow {

    pub fn is_selectable(&self) -> bool {
        matches!(
            self,
            ModalRow::Toggle { .. }
                | ModalRow::Choice { .. }
                | ModalRow::Stepper { .. }
                | ModalRow::TextInput { .. }
        )
    }
}

#[derive(Debug, Clone)]
pub struct ModalStep {
    pub title: String,
    pub rows: Vec<ModalRow>,
}

#[derive(Debug, Clone)]
pub struct Modal {

    pub id: String,
    pub title: String,
    pub steps: Vec<ModalStep>,
    pub current_step: usize,
    pub rows: Vec<ModalRow>,

    pub commands: Vec<(String, String)>,
    pub selected: usize,

    pub persist: Option<PathBuf>,

    pub persist_config: Option<PathBuf>,
    pub dirty: bool,

    pub page_size: usize,

    pub page: usize,

    pub hints: Vec<(String, String)>,

    pub sticky_footer: Vec<ModalRow>,

    pub run_files: Vec<String>,

    pub pickup_command: String,

    pub mirror: bool,

    pub editing_text: bool,
}

impl Modal {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            steps: Vec::new(),
            current_step: 0,
            rows: Vec::new(),
            commands: Vec::new(),
            selected: 0,
            persist: None,
            persist_config: None,
            dirty: false,
            page_size: 0,
            page: 0,
            hints: Vec::new(),
            sticky_footer: Vec::new(),
            run_files: Vec::new(),
            pickup_command: String::new(),
            mirror: true,
            editing_text: false,
        }
    }

    pub fn set_page_size(&mut self, size: usize) {
        self.page_size = size;
        self.page = 0;
        self.selected = 0;
    }

    pub fn page_count(&self) -> usize {
        if self.page_size == 0 {
            return 1;
        }
        self.rows.len().div_ceil(self.page_size).max(1)
    }

    pub fn page_start(&self) -> usize {
        if self.page_size == 0 {
            return 0;
        }
        self.page
            .saturating_mul(self.page_size)
            .min(self.rows.len())
    }

    pub fn visible_rows(&self) -> &[ModalRow] {
        if self.page_size == 0 {
            return &self.rows;
        }
        let start = self.page_start();
        let end = (start + self.page_size).min(self.rows.len());
        &self.rows[start..end]
    }

    pub fn page_move(&mut self, delta: isize) -> bool {
        if self.page_size == 0 {
            return false;
        }
        let count = self.page_count();
        let target = (self.page as isize + delta).clamp(0, count as isize - 1) as usize;
        if target == self.page {
            return false;
        }
        self.page = target;
        self.selected = self.page_start();
        self.select_first_selectable_in_page();
        true
    }

    fn select_first_selectable_in_page(&mut self) {
        let start = self.page_start();
        let end = (start + self.page_size).min(self.rows.len());
        for i in start..end {
            if self.row_is_selectable(i) {
                self.selected = i;
                return;
            }
        }
        self.selected = start;
    }

    pub fn add_step(&mut self, title: impl Into<String>, rows: Vec<ModalRow>) {
        let step = ModalStep {
            title: title.into(),
            rows,
        };
        if self.steps.is_empty() {
            self.rows = step.rows.clone();
        }
        self.steps.push(step);
    }

    pub fn all_rows(&self) -> Vec<ModalRow> {
        let mut out = Vec::new();
        for step in &self.steps {
            if std::ptr::eq(step as *const _, &self.steps[self.current_step] as *const _) {

                out.extend(self.rows.iter().cloned());
            } else {
                out.extend(step.rows.iter().cloned());
            }
        }
        if self.steps.is_empty() {
            out.extend(self.rows.iter().cloned());
        }
        out
    }

    pub fn next_step(&mut self) -> bool {
        if self.steps.is_empty() || self.current_step + 1 >= self.steps.len() {
            return false;
        }
        self.steps[self.current_step].rows = self.rows.clone();
        self.current_step += 1;
        self.rows = self.steps[self.current_step].rows.clone();
        self.select_first_selectable();
        true
    }

    pub fn prev_step(&mut self) -> bool {
        if self.steps.is_empty() || self.current_step == 0 {
            return false;
        }
        self.steps[self.current_step].rows = self.rows.clone();
        self.current_step -= 1;
        self.rows = self.steps[self.current_step].rows.clone();
        self.select_first_selectable();
        true
    }

    pub fn set_step(&mut self, idx: usize) -> bool {
        if self.steps.is_empty() || idx >= self.steps.len() {
            return false;
        }
        if idx == self.current_step {
            return true;
        }
        self.steps[self.current_step].rows = self.rows.clone();
        self.current_step = idx;
        self.rows = self.steps[self.current_step].rows.clone();
        self.select_first_selectable();
        true
    }

    pub fn selection_len(&self) -> usize {
        self.rows.len() + self.commands.len()
    }

    pub fn move_selection(&mut self, delta: isize) {
        if self.selection_len() == 0 {
            return;
        }
        if self.page_size > 0 {

            let start = self.page_start();
            let end = (start + self.page_size).min(self.rows.len());
            if end == 0 || end <= start {
                return;
            }
            let new_idx =
                (self.selected as isize + delta).clamp(start as isize, (end - 1) as isize) as usize;

            let mut probe = new_idx;
            loop {
                if probe >= start && probe < end && self.row_is_selectable(probe) {
                    self.selected = probe;
                    break;
                }
                if delta > 0 {
                    if probe + 1 >= end {
                        break;
                    }
                    probe += 1;
                } else {
                    if probe == start {
                        break;
                    }
                    probe -= 1;
                }
            }
            return;
        }

        let mut steps = self.selection_len();
        loop {
            let len = self.selection_len() as isize;
            self.selected = ((self.selected as isize + delta).rem_euclid(len)) as usize;
            if self.selected_is_command() || self.row_is_selectable(self.selected) {
                break;
            }
            steps -= 1;
            if steps == 0 {
                break;
            }
        }
    }

    pub fn row_is_selectable(&self, idx: usize) -> bool {
        self.rows.get(idx).map(ModalRow::is_selectable).unwrap_or(false)
    }

    pub fn select_first_selectable(&mut self) {
        if self.page_size > 0 {
            self.select_first_selectable_in_page();
            return;
        }
        for i in 0..self.rows.len() {
            if self.row_is_selectable(i) {
                self.selected = i;
                return;
            }
        }
        self.selected = 0;
    }

    pub fn selected_is_command(&self) -> bool {
        self.selected >= self.rows.len() && self.selected < self.selection_len()
    }

    #[allow(dead_code)]
    pub fn selected_row(&self) -> Option<&ModalRow> {
        let idx = self.selected.min(self.rows.len().saturating_sub(1));
        self.rows.get(idx)
    }

    pub fn cycle_selected(&mut self) -> bool {
        let idx = self.selected.min(self.rows.len().saturating_sub(1));
        let Some(row) = self.rows.get_mut(idx) else {
            return false;
        };
        match row {
            ModalRow::Toggle { enabled, .. } => {
                *enabled = !*enabled;
                self.dirty = true;
            }
            ModalRow::Choice {
                options, current, ..
            } => {
                if !options.is_empty() {
                    *current = (*current + 1) % options.len();
                    self.dirty = true;
                }
            }
            ModalRow::Stepper {
                value,
                min,
                max,
                step,
                ..
            } => {
                let next = if *value + *step > *max {
                    *min
                } else {
                    *value + *step
                };
                *value = next;
                self.dirty = true;
            }
            ModalRow::TextInput { .. }
            | ModalRow::Info(_)
            | ModalRow::InfoColored { .. }
            | ModalRow::Separator(_)
            | ModalRow::Progress { .. }
            | ModalRow::Table { .. }
            | ModalRow::Section { .. } => return false,
        }
        if !self.steps.is_empty() && self.current_step < self.steps.len() {
            self.steps[self.current_step].rows = self.rows.clone();
        }
        true
    }

    pub fn adjust_stepper(&mut self, delta: i64) -> bool {
        let idx = self.selected.min(self.rows.len().saturating_sub(1));
        if let Some(ModalRow::Stepper {
            value,
            min,
            max,
            step,
            ..
        }) = self.rows.get_mut(idx)
        {
            let next = (*value + delta * *step).clamp(*min, *max);
            if next != *value {
                *value = next;
                self.dirty = true;
                if !self.steps.is_empty() && self.current_step < self.steps.len() {
                    self.steps[self.current_step].rows = self.rows.clone();
                }
                return true;
            }
        }
        false
    }

    pub fn select_option(&mut self, option_idx: usize) -> bool {
        let idx = self.selected.min(self.rows.len().saturating_sub(1));
        let mut model_update: Option<(String, String)> = None;
        {
            let Some(row) = self.rows.get_mut(idx) else {
                return false;
            };
            match row {
                ModalRow::Choice {
                    key,
                    options,
                    current,
                    ..
                } => {
                    if option_idx < options.len() {
                        *current = option_idx;
                        self.dirty = true;
                        if key.ends_with(".model") {
                            let selected_id = options[option_idx].1.clone();
                            model_update = Some((key.clone(), selected_id));
                        }
                    } else {
                        return false;
                    }
                }
                _ => return false,
            }
        }
        if let Some((model_key, selected_id)) = model_update {
            update_paired_effort_rows(self, &model_key, &selected_id);
        }
        if !self.steps.is_empty() && self.current_step < self.steps.len() {
            self.steps[self.current_step].rows = self.rows.clone();
        }
        true
    }

    pub fn selected_is_searchable_choice(&self) -> bool {
        let idx = self.selected.min(self.rows.len().saturating_sub(1));
        matches!(
            self.rows.get(idx),
            Some(ModalRow::Choice {
                searchable: true,
                ..
            })
        )
    }

    #[allow(dead_code)]
    pub fn selected_row_label(&self) -> Option<String> {
        let idx = self.selected.min(self.rows.len().saturating_sub(1));
        match self.rows.get(idx) {
            Some(ModalRow::Toggle { label, .. }) | Some(ModalRow::Choice { label, .. }) => {
                Some(label.clone())
            }
            _ => None,
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        let all_rows: Vec<&ModalRow> = if !self.steps.is_empty() {
            self.steps.iter().flat_map(|s| s.rows.iter()).collect()
        } else {
            self.rows.iter().collect()
        };
        for row in all_rows {
            match row {
                ModalRow::Toggle { key, enabled, .. } if key != "enabled" => {
                    map.insert(key.clone(), serde_json::json!(enabled));
                }
                ModalRow::Choice {
                    key,
                    options,
                    current,
                    ..
                } => {
                    let value = options
                        .get(*current)
                        .map(|(_, v, _)| v.clone())
                        .unwrap_or_default();
                    map.insert(key.clone(), serde_json::json!(value));
                }
                ModalRow::TextInput { key, value, .. } => {
                    map.insert(key.clone(), serde_json::json!(value));
                }
                ModalRow::Stepper { key, value, .. } => {
                    map.insert(key.clone(), serde_json::json!(value));
                }
                _ => {}
            }
        }
        serde_json::Value::Object(map)
    }
}

pub fn is_reasoning_model(model_id: &str) -> bool {
    let id = model_id.to_lowercase();
    if id.is_empty() {
        return false;
    }
    id.contains("gpt-5.6")
        || id.contains("gpt-5.5")
        || id.contains("opus-5")
        || id.contains("fable-5")
        || id.contains("opus-4")
        || id.contains("qwen3.8")
        || id.contains("qwen3.7")
        || id.contains("qwen3.6")
        || id.contains("deepseek-v4")
        || id.contains("glm-5")
        || id.contains("r1")
        || id.contains("o3")
        || id.contains("o1")
        || id.contains("inkling")
        || id.contains("thinking")
}

pub fn update_paired_effort_rows(modal: &mut Modal, model_key: &str, selected_model_id: &str) {
    let effort_key = model_key.replace(".model", ".effort");
    let reasoning_supported = is_reasoning_model(selected_model_id);

    for row in &mut modal.rows {
        if let ModalRow::Choice {
            key,
            options,
            current,
            ..
        } = row
        {
            if key == &effort_key {
                if reasoning_supported {
                    *options = vec![
                        ("Low".into(), "low".into(), String::new()),
                        ("Medium".into(), "medium".into(), String::new()),
                        ("High".into(), "high".into(), String::new()),
                        (
                            "X-High (Deep Reasoning)".into(),
                            "xhigh".into(),
                            String::new(),
                        ),
                        ("Max (Extended)".into(), "max".into(), String::new()),
                    ];
                    if *current >= options.len() {
                        *current = 2;
                    }
                } else {
                    *options = vec![(
                        "N/A (Standard Model - No Reasoning Control)".into(),
                        "none".into(),
                        String::new(),
                    )];
                    *current = 0;
                }
            }
        }
    }
}

