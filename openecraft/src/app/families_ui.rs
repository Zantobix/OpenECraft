//! Tab *Device families* of the property database window: the built-in families and
//! those of the project as a tree (base families above the ones built on them); a
//! family's variants with their part, 3D model, plan and schematic symbol, its options
//! (mark, accessory, the variants they fit), combinations and suggested texts. A family
//! is edited as a draft taken over when it is valid; a built-in one becomes the
//! project's own version ([`openecraft_core::families`]).

use super::properties_ui::PropertiesUi;
use egui::RichText;
use openecraft_core::families;
use openecraft_core::model::families::{DeviceFamily, FamilyOption, Variant};
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

/// The fields of a family: name, base, variants (part, plan and schematic symbol with
/// their previews), options (mark, accessory, the variants they fit), combinations and
/// suggested texts.
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
    // Variants: one row each with its three representations.
    ui.separator();
    ui.label(RichText::new(tr("Variants")).strong()).on_hover_text(tr("The forms of the device; exactly one is chosen. Each is a part with its 3D model and functions; its plan and schematic symbol come from the part unless chosen here"));
    let mut remove = None;
    egui::Grid::new("family_variants").striped(true).num_columns(7).show(ui, |ui| {
        for h in [tr("Key"), tr("Name"), tr("Part"), tr("3D"), tr("Plan symbol"), tr("Schematic symbol"), ""] {
            ui.label(RichText::new(h).weak());
        }
        ui.end_row();
        for (i, v) in d.variants.iter_mut().enumerate() {
            ui.add_sized([60.0, 20.0], egui::TextEdit::singleline(&mut v.key));
            ui.add_sized([110.0, 20.0], egui::TextEdit::singleline(&mut v.name));
            let part = openecraft_core::installation::find_part(prj, &v.part).cloned();
            let r = ui.add_sized([170.0, 20.0], egui::TextEdit::singleline(&mut v.part));
            if part.is_none() {
                r.on_hover_text(tr("Unknown part"));
            }
            let size = egui::vec2(56.0, 44.0);
            match part.as_ref().and_then(|p| p.model_3d.as_ref()) {
                Some(mesh) => openecraft_ui::part_editor::mesh_preview(ui, mesh, size),
                None => openecraft_ui::part_editor::model_2d_preview(ui, part.as_ref().and_then(|p| p.model_2d.as_deref()), size),
            }
            ui.horizontal(|ui| {
                let shown = if v.plan_symbol.is_empty() { part.as_ref().and_then(|p| openecraft_core::installation::kind_of(p).map(|k| openecraft_core::installation::plan_symbol(prj, p, k))).unwrap_or_default() } else { v.plan_symbol.clone() };
                openecraft_canvas::editor::symbol_icon(ui, prj, &shown, egui::vec2(44.0, 44.0));
                symbol_combo(ui, prj, ("variant_plan", i), &mut v.plan_symbol, true);
            });
            ui.horizontal(|ui| {
                let shown = if v.schematic_symbol.is_empty() { part.as_ref().and_then(|p| openecraft_core::installation::schematic_symbol(prj, p)).unwrap_or_default() } else { v.schematic_symbol.clone() };
                openecraft_canvas::editor::symbol_icon(ui, prj, &shown, egui::vec2(44.0, 44.0));
                symbol_combo(ui, prj, ("variant_schematic", i), &mut v.schematic_symbol, false);
            });
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
        let n = d.variants.len() + 1;
        d.variants.push(Variant { key: n.to_string(), name: n.to_string(), part: d.variants.last().map(|v| v.part.clone()).unwrap_or_else(|| "GEN.SOCKET-F-UP".into()), ..Default::default() });
    }
    // Options: what a device can be fitted with.
    let variant_keys: Vec<(String, String)> = families::variants(prj, &d.parent)
        .into_iter()
        .map(|(of, v)| (v.key.clone(), families::name_of(of, &v.name)))
        .filter(|(k, _)| !d.variants.iter().any(|v| &v.key == k))
        .chain(d.variants.iter().map(|v| (v.key.clone(), v.name.clone())))
        .collect();
    ui.separator();
    ui.label(RichText::new(tr("Options")).strong()).on_hover_text(tr("What a device can be fitted with, any of them or none: a mark beside the plan symbol, an accessory part, the variants it fits"));
    let mut remove = None;
    egui::Grid::new("family_options").striped(true).num_columns(6).show(ui, |ui| {
        for h in [tr("Key"), tr("Name"), tr("Mark"), tr("Accessory"), tr("Fits"), ""] {
            ui.label(RichText::new(h).weak());
        }
        ui.end_row();
        for (i, o) in d.options.iter_mut().enumerate() {
            ui.add_sized([60.0, 20.0], egui::TextEdit::singleline(&mut o.key));
            ui.add_sized([130.0, 20.0], egui::TextEdit::singleline(&mut o.name));
            ui.add_sized([60.0, 20.0], egui::TextEdit::singleline(&mut o.mark)).on_hover_text(tr("Short text beside the plan symbol, e.g. IP44"));
            ui.add_sized([150.0, 20.0], egui::TextEdit::singleline(&mut o.accessory).hint_text(tr("(none)")));
            // The variants it fits, as a summary that opens the check boxes.
            let summary = if o.variants.is_empty() {
                tr("all").to_string()
            } else {
                variant_keys.iter().filter(|(k, _)| o.variants.contains(k)).map(|(_, n)| n.as_str()).collect::<Vec<_>>().join(", ")
            };
            ui.menu_button(summary, |ui| {
                let mut all = o.variants.is_empty();
                if ui.checkbox(&mut all, tr("all")).changed() && all {
                    o.variants.clear();
                }
                for (k, name) in &variant_keys {
                    let mut on = o.variants.is_empty() || o.variants.contains(k);
                    if ui.checkbox(&mut on, name).changed() {
                        if o.variants.is_empty() {
                            o.variants = variant_keys.iter().map(|(k, _)| k.clone()).collect();
                        }
                        if on { o.variants.push(k.clone()) } else { o.variants.retain(|x| x != k) }
                        o.variants.dedup();
                    }
                }
            });
            if icons::button(ui, Icon::Delete, tr("Remove")).clicked() {
                remove = Some(i);
            }
            ui.end_row();
        }
    });
    if let Some(i) = remove {
        let k = d.options.remove(i).key;
        for c in &mut d.combinations {
            c.options.retain(|x| *x != k);
        }
    }
    if icons::text_button(ui, Icon::Plus, tr("Option")).clicked() {
        let n = d.options.len() + 1;
        d.options.push(FamilyOption { key: format!("option{n}"), name: format!("{} {n}", tr("Option")), ..Default::default() });
    }
    // Combinations: a variant with options that is a part of its own.
    let option_keys: Vec<(String, String)> = families::options(prj, &d.parent)
        .into_iter()
        .map(|(of, o)| (o.key.clone(), families::name_of(of, &o.name)))
        .filter(|(k, _)| !d.options.iter().any(|o| &o.key == k))
        .chain(d.options.iter().map(|o| (o.key.clone(), o.name.clone())))
        .collect();
    ui.separator();
    ui.label(RichText::new(tr("Combinations")).strong()).on_hover_text(tr("A variant with options that is a part of its own, e.g. a socket outlet with a USB charger built in"));
    let mut remove = None;
    egui::Grid::new("family_combinations").striped(true).num_columns(4).show(ui, |ui| {
        for (i, c) in d.combinations.iter_mut().enumerate() {
            let shown = variant_keys.iter().find(|(k, _)| *k == c.variant).map(|(_, n)| n.clone()).unwrap_or_else(|| c.variant.clone());
            egui::ComboBox::from_id_salt(("combination_variant", i)).selected_text(shown).show_ui(ui, |ui| {
                for (k, name) in &variant_keys {
                    ui.selectable_value(&mut c.variant, k.clone(), name);
                }
            });
            ui.horizontal_wrapped(|ui| {
                for (k, name) in &option_keys {
                    let mut on = c.options.contains(k);
                    if ui.checkbox(&mut on, name).changed() {
                        if on { c.options.push(k.clone()) } else { c.options.retain(|x| x != k) }
                    }
                }
            });
            let known = openecraft_core::installation::find_part(prj, &c.part).is_some();
            let r = ui.add_sized([170.0, 20.0], egui::TextEdit::singleline(&mut c.part));
            if !known {
                r.on_hover_text(tr("Unknown part"));
            }
            if icons::button(ui, Icon::Delete, tr("Remove")).clicked() {
                remove = Some(i);
            }
            ui.end_row();
        }
    });
    if let Some(i) = remove {
        d.combinations.remove(i);
    }
    if icons::text_button(ui, Icon::Plus, tr("Combination")).clicked() {
        let variant = variant_keys.first().map(|(k, _)| k.clone()).unwrap_or_default();
        d.combinations.push(openecraft_core::model::families::Combination { variant, options: Vec::new(), part: String::new() });
    }
    ui.separator();
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
