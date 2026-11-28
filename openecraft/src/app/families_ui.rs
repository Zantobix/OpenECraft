//! Tab *Device families* of the property database window: the built-in families and
//! those of the project as a tree (base families above the ones built on them), a
//! family's options with their values and marks, its variants (option values, part,
//! plan and schematic symbol) and suggested texts; project families are edited as a
//! draft that is taken over when it is valid ([`openecraft_core::families`]).

use super::properties_ui::PropertiesUi;
use egui::RichText;
use openecraft_core::families;
use openecraft_core::model::families::{Choice, DeviceFamily, FamilyOption, Variant};
use openecraft_core::model::Project;
use openecraft_i18n::tr;
use openecraft_ui::icons::{self, Icon};

/// Edit state of the tab: the family selected and the draft of a project family.
#[derive(Default)]
pub struct FamiliesUi {
    selected: String,
    draft: Option<DeviceFamily>,
    /// Suggested texts of the draft, one per line.
    texts: String,
    error: Option<String>,
}

pub(super) fn families_tab(ui: &mut egui::Ui, prj: &mut Project, st: &mut PropertiesUi) {
    let fu = &mut st.families;
    ui.columns(2, |cols| {
        let ui = &mut cols[0];
        egui::ScrollArea::vertical().id_salt("family_list").auto_shrink([false, false]).show(ui, |ui| {
            tree(ui, prj, fu, "", 0);
        });
        let ui = &mut cols[1];
        egui::ScrollArea::vertical().id_salt("family_details").auto_shrink([false, false]).show(ui, |ui| details(ui, prj, fu));
    });
}

/// The families whose base is `parent`, indented by `depth`, each followed by those
/// built on it.
fn tree(ui: &mut egui::Ui, prj: &Project, fu: &mut FamiliesUi, parent: &str, depth: usize) {
    if depth > 8 {
        return;
    }
    let children: Vec<(String, String, bool)> = families::all(prj)
        .into_iter()
        .filter(|f| if parent.is_empty() { f.parent.is_empty() || families::get(prj, &f.parent).is_none() } else { f.parent == parent && f.key != parent })
        .map(|f| (f.key.clone(), families::title(f), f.builtin))
        .collect();
    for (key, title, builtin) in children {
        ui.horizontal(|ui| {
            ui.add_space(16.0 * depth as f32);
            let text = if builtin { RichText::new(title) } else { RichText::new(title).strong() };
            if ui.selectable_label(fu.selected == key, text).on_hover_text(&key).clicked() && fu.selected != key {
                select(prj, fu, &key);
            }
        });
        tree(ui, prj, fu, &key, depth + 1);
    }
}

fn select(prj: &Project, fu: &mut FamiliesUi, key: &str) {
    fu.selected = key.to_string();
    fu.error = None;
    fu.draft = families::get(prj, key).map(families::editable);
    fu.texts = fu.draft.as_ref().map(|f| f.texts.join("\n")).unwrap_or_default();
}

fn details(ui: &mut egui::Ui, prj: &mut Project, fu: &mut FamiliesUi) {
    let Some(f) = families::get(prj, &fu.selected).cloned() else {
        ui.weak(tr("Select a device family. Project families (bold) can be changed and are kept in property libraries."));
        return;
    };
    ui.label(RichText::new(families::title(&f)).strong());
    ui.label(RichText::new(&f.key).monospace().small());
    let builtin = families::builtin_family(&f.key).is_some();
    if builtin {
        let note = if families::overrides_builtin(prj, &f.key) { tr("Built-in family, changed in this project") } else { tr("Built-in family: changes are kept in the project") };
        ui.weak(note);
    }
    let Some(d) = fu.draft.as_mut() else {
        select(prj, fu, &f.key);
        return;
    };
    editor(ui, prj, d, &mut fu.texts);
    let mut candidate = d.clone();
    candidate.texts = fu.texts.lines().map(str::trim).filter(|t| !t.is_empty()).map(String::from).collect();
    // The family itself out of the way, so that its own key is no base of itself.
    let mut probe = prj.clone();
    probe.property_db.families.retain(|x| x.key != candidate.key);
    let problem = families::validate(&probe, &candidate).err();
    ui.separator();
    if let Some(e) = &problem {
        ui.colored_label(ui.visuals().error_fg_color, e);
    }
    ui.horizontal(|ui| {
        let changed = families::get(prj, &candidate.key).map(families::editable).as_ref() != Some(&candidate);
        if ui.add_enabled(problem.is_none() && changed, egui::Button::new(tr("Apply"))).clicked() {
            if let Err(e) = families::define(prj, candidate.clone()) {
                fu.error = Some(e);
            }
        }
        if ui.add_enabled(changed, egui::Button::new(tr("Discard changes"))).clicked() {
            let key = fu.selected.clone();
            select(prj, fu, &key);
        }
        if ui.button(tr("Duplicate")).clicked() {
            let new = families::derive(prj, &candidate);
            let key = new.key.clone();
            match families::define(prj, new) {
                Ok(()) => select(prj, fu, &key),
                Err(e) => fu.error = Some(e),
            }
        }
        if builtin {
            let own = families::overrides_builtin(prj, &fu.selected);
            if ui.add_enabled(own, egui::Button::new(tr("Reset to built-in"))).on_hover_text(tr("The project's changes of this built-in family go")).clicked() {
                let key = fu.selected.clone();
                match families::remove(prj, &key) {
                    Ok(_) => select(prj, fu, &key),
                    Err(e) => fu.error = Some(e),
                }
            }
        } else if icons::text_button(ui, Icon::Delete, tr("Delete")).on_hover_text(tr("Devices placed with it keep their part")).clicked() {
            match families::remove(prj, &fu.selected) {
                Ok(_) => {
                    let key = fu.selected.clone();
                    select(prj, fu, &key);
                }
                Err(e) => fu.error = Some(e),
            }
        }
    });
    if let Some(e) = &fu.error {
        ui.colored_label(ui.visuals().error_fg_color, e);
    }
}

/// The fields of a project family.
fn editor(ui: &mut egui::Ui, prj: &Project, d: &mut DeviceFamily, texts: &mut String) {
    egui::Grid::new("family_edit").num_columns(2).show(ui, |ui| {
        ui.label(tr("Name"));
        ui.text_edit_singleline(&mut d.name);
        ui.end_row();
        ui.label(tr("Base family"));
        let shown = families::get(prj, &d.parent).map_or_else(|| tr("(none)").to_string(), families::title);
        egui::ComboBox::from_id_salt("family_parent").selected_text(shown).show_ui(ui, |ui| {
            ui.selectable_value(&mut d.parent, String::new(), tr("(none)"));
            for f in families::all(prj).into_iter().filter(|f| f.key != d.key) {
                ui.selectable_value(&mut d.parent, f.key.clone(), families::title(f));
            }
        });
        ui.end_row();
    });
    ui.label(RichText::new(tr("Options")).strong());
    let mut remove = None;
    for (i, o) in d.options.iter_mut().enumerate() {
        ui.push_id(("family_option", i), |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut o.key).hint_text(tr("Key")).desired_width(90.0));
                ui.add(egui::TextEdit::singleline(&mut o.name).hint_text(tr("Name")).desired_width(140.0));
                if icons::button(ui, Icon::Delete, tr("Remove")).clicked() {
                    remove = Some(i);
                }
            });
            let mut gone = None;
            for (j, c) in o.choices.iter_mut().enumerate() {
                ui.push_id(j, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        ui.add(egui::TextEdit::singleline(&mut c.key).hint_text(tr("Key")).desired_width(74.0));
                        ui.add(egui::TextEdit::singleline(&mut c.name).hint_text(tr("Name")).desired_width(124.0));
                        ui.add(egui::TextEdit::singleline(&mut c.mark).hint_text(tr("Mark")).desired_width(60.0)).on_hover_text(tr("Short text beside the plan symbol, e.g. IP44"));
                        if o.default == c.key && !c.key.is_empty() {
                            ui.weak(tr("default"));
                        } else if ui.small_button(tr("Default")).clicked() {
                            o.default = c.key.clone();
                        }
                        if icons::button(ui, Icon::Delete, tr("Remove")).clicked() {
                            gone = Some(j);
                        }
                    });
                });
            }
            if let Some(j) = gone {
                let c = o.choices.remove(j);
                if o.default == c.key {
                    o.default.clear();
                }
            }
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                if icons::text_button(ui, Icon::Plus, tr("Value")).clicked() {
                    let n = o.choices.len() + 1;
                    o.choices.push(Choice { key: n.to_string(), name: n.to_string(), mark: String::new() });
                }
            });
        });
    }
    if let Some(i) = remove {
        d.options.remove(i);
    }
    if icons::text_button(ui, Icon::Plus, tr("Option")).clicked() {
        let n = d.options.len() + 1;
        d.options.push(FamilyOption { key: format!("option{n}"), name: format!("{} {n}", tr("Option")), choices: vec![Choice { key: "1".into(), name: "1".into(), mark: String::new() }], default: String::new() });
    }
    ui.label(RichText::new(tr("Variants")).strong()).on_hover_text(tr("Each variant is a part: its 3D model, its functions (and so its schematic symbol) and its plan symbol; another plan or schematic symbol can be named"));
    let opts: Vec<FamilyOption> = {
        let mut base: Vec<FamilyOption> = if d.parent.is_empty() { Vec::new() } else { families::options(prj, &d.parent).into_iter().map(|(_, o)| o.clone()).collect() };
        base.retain(|o| !d.options.iter().any(|x| x.key == o.key));
        base.into_iter().chain(d.options.iter().cloned()).collect()
    };
    let mut remove = None;
    egui::Grid::new("family_variants").striped(true).show(ui, |ui| {
        for o in &opts {
            ui.label(RichText::new(if o.name.is_empty() { &o.key } else { &o.name }).weak());
        }
        ui.label(RichText::new(tr("Part")).weak());
        ui.label(RichText::new(tr("Plan symbol")).weak());
        ui.label(RichText::new(tr("Schematic symbol")).weak());
        ui.end_row();
        for (i, v) in d.variants.iter_mut().enumerate() {
            for o in &opts {
                let shown = v.when.get(&o.key).and_then(|x| o.choices.iter().find(|c| &c.key == x)).map_or_else(|| tr("any").to_string(), |c| c.name.clone());
                egui::ComboBox::from_id_salt(("variant_when", i, &o.key)).selected_text(shown).show_ui(ui, |ui| {
                    if ui.selectable_label(!v.when.contains_key(&o.key), tr("any")).clicked() {
                        v.when.remove(&o.key);
                    }
                    for c in &o.choices {
                        if ui.selectable_label(v.when.get(&o.key) == Some(&c.key), &c.name).clicked() {
                            v.when.insert(o.key.clone(), c.key.clone());
                        }
                    }
                });
            }
            let known = openecraft_core::installation::find_part(prj, &v.part).is_some();
            let r = ui.add_sized([170.0, 20.0], egui::TextEdit::singleline(&mut v.part));
            if !known {
                r.on_hover_text(tr("Unknown part"));
            }
            symbol_combo(ui, prj, ("variant_plan", i), &mut v.plan_symbol, true);
            symbol_combo(ui, prj, ("variant_schematic", i), &mut v.schematic_symbol, false);
            if icons::button(ui, Icon::Delete, tr("Remove")).clicked() {
                remove = Some(i);
            }
            ui.end_row();
        }
    });
    if let Some(i) = remove {
        d.variants.remove(i);
    }
    if icons::text_button(ui, Icon::Plus, tr("Variant")).clicked() {
        d.variants.push(Variant { part: "GEN.SOCKET-F-UP".into(), ..Default::default() });
    }
    ui.label(RichText::new(tr("Suggested texts")).strong()).on_hover_text(tr("One per line; offered for the text at the symbol"));
    ui.add(egui::TextEdit::multiline(texts).desired_rows(3));
}

/// A choice of a symbol of the library (plan symbols only for `plan`); empty: the one
/// of the part.
fn symbol_combo(ui: &mut egui::Ui, prj: &Project, id: impl std::hash::Hash + std::fmt::Debug, key: &mut String, plan: bool) {
    let shown = if key.is_empty() { tr("from the part").to_string() } else { prj.symbol(key).map_or_else(|| key.clone(), |s| tr(s.name).to_string()) };
    egui::ComboBox::from_id_salt(id).selected_text(shown).height(320.0).show_ui(ui, |ui| {
        ui.selectable_value(key, String::new(), tr("from the part"));
        if plan {
            for (k, _) in openecraft_core::parts::PLAN_SYMBOLS {
                if let Some(s) = prj.symbol(k) {
                    ui.selectable_value(key, k.to_string(), tr(s.name));
                }
            }
        } else {
            for s in openecraft_core::symbols::library().iter().filter(|s| s.pins.len() > 1) {
                ui.selectable_value(key, s.key.to_string(), tr(s.name));
            }
        }
    });
}
