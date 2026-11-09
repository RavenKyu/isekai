use dear_imgui_rs::Ui;
use std::collections::BTreeMap;

#[derive(Default, Clone, Debug)]
pub struct PropsDraft {
    pub rows: Vec<(String, String)>,
    error: Option<String>,
    #[cfg(test)]
    pub key_input: Option<[f32; 2]>,
    #[cfg(test)]
    pub add_button: Option<[f32; 2]>,
}
impl PropsDraft {
    pub fn from_map(map: &BTreeMap<String, String>) -> Self {
        Self {
            rows: map.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            ..Self::default()
        }
    }
    pub fn validate(&self) -> Result<BTreeMap<String, String>, String> {
        let mut map = BTreeMap::new();
        for (i, (key, value)) in self.rows.iter().enumerate() {
            if key.is_empty() && value.is_empty() {
                continue;
            }
            let key = key.trim();
            if key.is_empty() || map.contains_key(key) {
                return Err(format!(
                    "property row {} has an empty or duplicate key",
                    i + 1
                ));
            }
            map.insert(key.to_owned(), value.clone());
        }
        Ok(map)
    }
    pub fn draw(&mut self, ui: &Ui, id: &str) -> Option<BTreeMap<String, String>> {
        let _id = ui.push_id(id);
        ui.text("Properties");
        if self.rows.is_empty() {
            ui.text_disabled("No properties");
        }
        let mut changed = false;
        let mut remove = None;
        for (i, (key, value)) in self.rows.iter_mut().enumerate() {
            let _row = ui.push_id(i as i32);
            ui.set_next_item_width(110.0);
            changed |= ui.input_text("##key", key).build();
            #[cfg(test)]
            {
                if i == 0 {
                    let min = ui.item_rect_min();
                    let size = ui.item_rect_size();
                    self.key_input = Some([min[0] + 10.0, min[1] + size[1] * 0.5]);
                }
            }
            ui.same_line();
            ui.set_next_item_width(110.0);
            changed |= ui.input_text("##value", value).build();
            ui.same_line();
            if ui.small_button("x") {
                remove = Some(i);
            }
        }
        if let Some(i) = remove {
            self.rows.remove(i);
            changed = true;
        }
        let add = ui.button("+ property");
        #[cfg(test)]
        {
            let min = ui.item_rect_min();
            let size = ui.item_rect_size();
            self.add_button = Some([min[0] + size[0] * 0.5, min[1] + size[1] * 0.5]);
        }
        if add {
            self.rows.push((String::new(), String::new()));
            changed = true;
        }
        let result = self.validate();
        self.error = result.as_ref().err().cloned();
        if let Some(error) = &self.error {
            ui.text_wrapped(error);
        }
        if changed { result.ok() } else { None }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rows_ignore_blank_trim_keys_and_reject_duplicates() {
        let mut draft = PropsDraft::default();
        draft.rows.push((String::new(), String::new()));
        assert!(draft.validate().unwrap().is_empty());
        draft.rows.push((" a ".into(), " = value ".into()));
        assert_eq!(draft.validate().unwrap()["a"], " = value ");
        draft.rows.push(("a".into(), String::new()));
        assert!(draft.validate().unwrap_err().contains("row 3"));
        draft.rows.pop();
        draft.rows.push(("  ".into(), "x".into()));
        assert!(draft.validate().is_err());
        draft.rows.pop();
        draft.rows.remove(1);
        assert!(draft.validate().unwrap().is_empty());
    }
}
