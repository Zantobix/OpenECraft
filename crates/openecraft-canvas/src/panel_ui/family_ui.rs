//! Device families in the building window: the variant and options of the part picked
//! in the installation menu, and the variant, options and text at the symbol of a
//! socket outlet, switch, light or box selected in a room
//! ([`openecraft_core::families`]).

use super::PanelUi;
use openecraft_core::panel;
use egui::RichText;
use openecraft_core::families;
use openecraft_core::installation as inst;
use openecraft_core::model::families::FamilyChoice;
use openecraft_core::model::Project;
use openecraft_i18n::{tr, tr_fmt};

/// The variants of family choice `c` as buttons and its options as check boxes
/// (options that do not fit the variant disabled); the choice clicked, its options
/// kept where they fit the new variant.
pub(super) fn choice_buttons(ui: &mut egui::Ui, prj: &Project, c: &FamilyChoice) -> Option<FamilyChoice> {
    let mut out = None;
    let variants = families::variants(prj, &c.family);
    let current = if c.variant.is_empty() { variants.first().map(|(_, v)| v.key.clone()).unwrap_or_default() } else { c.variant.clone() };
    if variants.len() > 1 {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(tr("Variant")).weak());
            for (of, v) in &variants {
                if ui.add(egui::Button::selectable(v.key == current, families::name_of(of, &v.name))).clicked() && v.key != current {
                    let options = c.options.iter().filter(|k| families::options(prj, &c.family).iter().any(|(_, o)| &o.key == *k && families::fits(o, &v.key))).cloned().collect();
                    out = Some(FamilyChoice { variant: v.key.clone(), options, ..c.clone() });
                }
            }
        });
    }
    let options = families::options(prj, &c.family);
    if !options.is_empty() {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(tr("Options")).weak());
            for (of, o) in &options {
                let mut on = c.options.contains(&o.key);
                let r = ui.add_enabled(families::fits(o, &current), egui::Checkbox::new(&mut on, families::name_of(of, &o.name)));
                if r.changed() {
                    let mut chosen: Vec<String> = c.options.iter().filter(|k| **k != o.key).cloned().collect();
                    if on {
                        chosen.push(o.key.clone());
                    }
                    out = Some(FamilyChoice { variant: current.clone(), options: chosen, ..c.clone() });
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
    let texts = if family.is_empty() { Vec::new() } else { families::texts(prj, family) };
    ui.horizontal(|ui| {
        ui.label(RichText::new(tr("Text at the symbol")).weak()).on_hover_text(tr("Shown beside the symbol on installation plans, e.g. what is connected"));
        let r = ui.add(egui::TextEdit::singleline(text).hint_text(tr("own text or a suggestion")).desired_width(150.0));
        take |= r.lost_focus();
        // The suggestions of the family in a list beside the field.
        egui::ComboBox::from_id_salt(("family_texts", family)).width(18.0).selected_text("").show_ui(ui, |ui| {
            if ui.selectable_label(text.is_empty(), tr("No text")).clicked() {
                text.clear();
                take = true;
            }
            for t in texts {
                if ui.selectable_label(*text == t, &t).clicked() {
                    *text = t;
                    take = true;
                }
            }
        });
    });
    take
}

/// The room item of device `tag` (its id), if it is a socket outlet, switch, light or
/// box placed in a room.
pub fn item_of_device(prj: &Project, tag: &str) -> Option<u64> {
    if tag.is_empty() {
        return None;
    }
    panel::rooms(&prj.layout).into_iter().find_map(|ri| prj.layout.enclosures[ri].items.iter().find(|i| i.tag == tag).map(|i| i.id))
}

/// Submenu *Device family* of a context menu for room item `id`: submenus with the
/// variants, the options (those that do not fit the variant disabled) and the suggested
/// texts. The variant, options or text picked are set (`installation::set_family`); the answer for the
/// status line, `None` when nothing was picked or the item is no device of a family.
pub fn family_menu(ui: &mut egui::Ui, prj: &mut Project, id: u64) -> Option<String> {
    let it = prj.layout.enclosures.iter().find_map(|e| e.item(id).cloned())?;
    inst::find_part(prj, &it.part).and_then(inst::kind_of)?;
    let current = families::of_item(prj, &it);
    let mut next: Option<FamilyChoice> = None;
    ui.menu_button(tr("Device family"), |ui| {
        if current.family.is_empty() {
            ui.weak(tr("None"));
        }
        let variants = families::variants(prj, &current.family);
        let cur = if current.variant.is_empty() { variants.first().map(|(_, v)| v.key.clone()).unwrap_or_default() } else { current.variant.clone() };
        if variants.len() > 1 {
            let shown = variants.iter().find(|(_, v)| v.key == cur).map(|(of, v)| families::name_of(of, &v.name)).unwrap_or_default();
            ui.menu_button(tr_fmt!("Variant: {}", shown), |ui| {
                for (of, v) in &variants {
                    if ui.add(egui::Button::selectable(v.key == cur, families::name_of(of, &v.name))).clicked() && v.key != cur {
                        let options = current.options.iter().filter(|k| families::options(prj, &current.family).iter().any(|(_, o)| &o.key == *k && families::fits(o, &v.key))).cloned().collect();
                        next = Some(FamilyChoice { variant: v.key.clone(), options, ..current.clone() });
                        ui.close();
                    }
                }
            });
        }
        let options = families::options(prj, &current.family);
        if !options.is_empty() {
            ui.menu_button(tr("Options"), |ui| {
                for (of, o) in &options {
                    let mut on = current.options.contains(&o.key);
                    if ui.add_enabled(families::fits(o, &cur), egui::Checkbox::new(&mut on, families::name_of(of, &o.name))).changed() {
                        let mut chosen: Vec<String> = current.options.iter().filter(|k| **k != o.key).cloned().collect();
                        if on {
                            chosen.push(o.key.clone());
                        }
                        next = Some(FamilyChoice { variant: cur.clone(), options: chosen, ..current.clone() });
                        ui.close();
                    }
                }
            });
        }
        let texts = if current.family.is_empty() { Vec::new() } else { families::texts(prj, &current.family) };
        ui.menu_button(tr("Text at the symbol"), |ui| {
            if ui.add_enabled(!current.text.is_empty(), egui::Button::new(tr("No text"))).clicked() {
                next = Some(FamilyChoice { text: String::new(), ..current.clone() });
                ui.close();
            }
            for t in texts {
                if ui.add(egui::Button::selectable(current.text == t, &t)).clicked() && current.text != t {
                    next = Some(FamilyChoice { text: t.clone(), ..current.clone() });
                    ui.close();
                }
            }
        });
    });
    let c = next?;
    Some(match inst::set_family(prj, id, c) {
        Ok(ch) if ch.part != ch.old_part => tr_fmt!("Now {}", ch.part),
        Ok(_) => tr("Text at the symbol changed").to_string(),
        Err(e) => e,
    })
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
        ui.label(RichText::new(tr("Device family")).strong()).on_hover_text(tr("The variants of a device, each a part with its schematic symbol, plan symbol and 3D model, and the options it can be fitted with"));
        let mut next: Option<FamilyChoice> = None;
        // Changed only within its family: the family is shown, not chosen.
        ui.label(families::get(prj, &current.family).map_or_else(|| tr("None").to_string(), families::title));
        if !current.family.is_empty() {
            if let Some(c) = choice_buttons(ui, prj, &current) {
                next = Some(c);
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
