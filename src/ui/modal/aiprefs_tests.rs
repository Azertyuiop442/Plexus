#[cfg(test)]
mod ai_prefs_nav_tests {
    use crate::ui::modal::model::{Modal, ModalRow};

    fn rows_features() -> Vec<ModalRow> {
        vec![
            ModalRow::Separator("Model per background feature".into()),
            ModalRow::TextInput { key: "featureModels.compaction".into(), label: "Compaction".into(), value: String::new() },
        ]
    }

    fn rows_peak() -> Vec<ModalRow> {
        vec![
            ModalRow::InfoColored { text: "tz".into(), color: "blue".into() },

        ]
    }

    #[test]
    fn switching_to_a_page_with_no_selectable_row_terminates_and_recovers() {
        let mut m = Modal::new("ai_prefs", "AI Prefs");
        m.set_page_size(10);
        m.add_step("Feature Models", rows_features());
        m.add_step("Peak Hours", rows_peak());
        m.select_first_selectable();

        assert!(m.next_step(), "must reach the peak step");

        assert!(!m.row_is_selectable(m.selected));

        assert!(m.prev_step(), "must come back to the features step");

    }
}

