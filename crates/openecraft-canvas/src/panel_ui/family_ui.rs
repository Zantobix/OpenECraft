//! Device families in the building window: the option values of the part picked in
//! the installation menu, and the family, option values and text at the symbol of a
//! socket outlet, switch, light or box selected in a room
//! ([`openecraft_core::families`]).

use super::PanelUi;
use egui::RichText;
use openecraft_core::families;
use openecraft_core::installation as inst;
use openecraft_core::model::families::FamilyChoice;
use openecraft_core::model::Project;
use openecraft_i18n::{tr, tr_fmt};
use std::collections::BTreeMap;

/// The values of the options of family choice `c` as buttons (values that lead to no
/// variant disabled); the variant clicked: its part and values.
pub(super) fn option_buttons(ui: &mut egui::Ui, prj: &Project, c: &FamilyChoice) -> Option<(String, BTreeMap<String, String>)> {
    let mut out = None;
    for (of, o) in families::options(prj, &c.family) {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(families::name_of(of, &o.name)).weak());
            for ch in &o.choices {
                let on = c.values.get(&o.key) == Some(&ch.key);
                let ok = families::reachable(prj, &c.family, &c.values, &o.key, &ch.key);
                let r = ui.add_enabled(ok, egui::Button::selectable(on, families::name_of(of, &ch.name)));
                if r.clicked() && !on {
                    let mut v = c.values.clone();
                    v.insert(o.key.clone(), ch.key.clone());
                    // The other values as they are, else at their defaults.
                    let only = BTreeMap::from([(o.key.clone(), ch.key.clone())]);
                    if let Ok(r) = families::resolve(prj, &c.family, &v).or_else(|_| families::resolve(prj, &c.family, &only)) {
                        out = Some((r.part, r.values));
                    }
                }
            }
        });
    }
    out
}

/// A text field for the text at the symbol with the suggested texts of family `family`
/// beside it; whether `text` changed and is to be taken (Enter, leaving the field or a
/// suggestion clicked).
pub(super) fn text_field(ui: &mut egui::Ui, prj: &Project, family: &str, text: &mut String) -> bool {
    let mut take = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new(tr("Text at the symbol")).weak()).on_hover_text(tr("Shown beside the symbol on installation plans, e.g. what is connected"));
        let r = ui.add(egui::TextEdit::singleline(text).desired_width(160.0));
        take |= r.lost_focus();
    });
    let texts = if family.is_empty() { Vec::new() } else { families::texts(prj, family) };
    if !texts.is_empty() {
        ui.horizontal_wrapped(|ui| {
            for t in texts {
                if ui.small_button(&t).clicked() {
                    *text = t;
                    take = true;
                }
            }
        });
    }
    take
}

impl PanelUi {
    /// For a socket outlet, switch, light or box of a room: its device family, the
    /// values of its options and the text at its symbol.
    pub(super) fn family_tools(&mut self, ui: &mut egui::Ui, prj: &mut Project, ei: usize, id: u64) {
        let e = &prj.layout.enclosures[ei];
        if !e.is_room() {
            return;
        }
        let Some(it) = e.item(id).cloned() else { return };
        if inst::find_part(prj, &it.part).and_then(inst::kind_of).is_none() {
            return;
        }
        let current = families::of_item(prj, &it);
        ui.separator();
        ui.label(RichText::new(tr("Device family")).strong()).on_hover_text(tr("The variants of a device chosen by the values of its options: each a part with its schematic symbol, plan symbol and 3D model"));
        let mut next: Option<FamilyChoice> = None;
        let shown = families::get(prj, &current.family).map_or_else(|| tr("None").to_string(), families::title);
        egui::ComboBox::from_id_salt(("device_family", id)).selected_text(shown).show_ui(ui, |ui| {
            for f in families::all(prj).into_iter().filter(|f| !f.variants.is_empty()) {
                let label = match families::get(prj, &f.parent) {
                    Some(b) => format!("{} › {}", families::title(b), families::title(f)),
                    None => families::title(f),
                };
                if ui.selectable_label(f.key == current.family, label).clicked() && f.key != current.family {
                    next = Some(FamilyChoice { family: f.key.clone(), values: BTreeMap::new(), text: current.text.clone() });
                }
            }
        });
        if !current.family.is_empty() {
            if let Some((_, values)) = option_buttons(ui, prj, &current) {
                next = Some(FamilyChoice { values, ..current.clone() });
            }
        }
        let key = egui::Id::unique(("family_text", id));
        let mut text: String = ui.data(|d| d.get_temp(key)).unwrap_or_else(|| current.text.clone());
        if text_field(ui, prj, &current.family, &mut text) && text.trim() != current.text {
            next = Some(FamilyChoice { text: text.clone(), ..current.clone() });
        }
        ui.data_mut(|d| d.insert_temp(key, text));
        if let Some(c) = next {
            ui.data_mut(|d| d.remove::<String>(key));
            self.status = match inst::set_family(prj, id, c) {
                Ok(ch) if !ch.kept.is_empty() => tr_fmt!("Now {}; its schematic symbol stays: the variant has another number of connection points", ch.part),
                Ok(ch) if ch.part != ch.old_part => tr_fmt!("Now {}", ch.part),
                Ok(_) => String::new(),
                Err(e) => e,
            };
        }
    }

    /// After a part of the installation menu was placed as item `id`: the text typed in
    /// the menu goes to its symbol.
    pub(super) fn apply_pick_text(&mut self, prj: &mut Project, id: u64) {
        let text = self.installation.text.trim().to_string();
        if text.is_empty() {
            return;
        }
        let Some(it) = panel_item(prj, id) else { return };
        let mut c = families::of_part(prj, &it.part).unwrap_or_default();
        c.text = text;
        if let Err(e) = inst::set_family(prj, id, c) {
            self.status = e;
        }
    }
}

fn panel_item(prj: &Project, id: u64) -> Option<openecraft_core::panel::PanelItem> {
    prj.layout.enclosures.iter().find_map(|e| e.item(id).cloned())
}
