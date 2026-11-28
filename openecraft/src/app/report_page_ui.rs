//! The options of a report page in the page properties (groups, columns, aspects of the
//! device tags, tree, graphical form, continuation on following pages) and the switches
//! of a device for the distribution board overview in its properties.

use eframe::egui;
use egui::RichText;
use openecraft_core::model::report_blocks::ReportView;
use openecraft_core::model::{Page, PageKind, Project, TagParts};
use openecraft_core::reports::Analysis;
use openecraft_i18n::{tr, tr_fmt};

/// What the page properties show of the report of a page.
pub(super) struct ReportInfo {
    /// Column keys (as stored in forms) and their headers in the current language.
    cols: Vec<(String, String)>,
    /// Values of the group column (boards, strips …) in order.
    groups: Vec<String>,
    tree: bool,
    graphic: bool,
    /// A device tag of the report as the example of the tag aspects.
    example: Option<String>,
}

/// What the user asked for that changes more than the page.
pub(super) enum ReportAction {
    /// Continue on following pages; every group on pages of its own.
    Continue(bool),
}

/// The report of page `pi` as the page properties show it; `None` for other pages.
pub(super) fn report_info(prj: &Project, an: &Analysis, pi: usize) -> Option<ReportInfo> {
    let PageKind::Report(kind) = prj.pages.get(pi)?.kind else { return None };
    let table = an.report(prj, kind)?;
    let cols = table.header_keys().into_iter().map(String::from).zip(table.header_texts().into_iter().map(String::from)).collect();
    let groups = openecraft_core::report_blocks::groups(&table, 0);
    let example = table.tag_columns.iter().find_map(|&c| table.rows.iter().map(|r| r.get(c).map(String::as_str).unwrap_or("")).find(|t| t.starts_with(['=', '+', '-'])).map(String::from));
    Some(ReportInfo { cols, groups, tree: table.tree_column.is_some(), graphic: openecraft_core::report_blocks::has_graphic(kind), example })
}

/// The comma separated entries of a groups pattern.
fn entries(pattern: &str) -> Vec<String> {
    pattern.split([',', ';']).map(str::trim).filter(|g| !g.is_empty()).map(String::from).collect()
}

/// The options of the report page `pg`.
pub(super) fn report_page_options(ui: &mut egui::Ui, pg: &mut Page, info: &ReportInfo) -> Option<ReportAction> {
    let mut action = None;
    let part = pg.report.get_or_insert_with(Default::default);
    if part.frozen.is_some() {
        ui.weak(tr("Fixed page output from another project"));
        return None;
    }
    ui.separator();
    ui.label(RichText::new(tr("Report page")).strong());
    // Groups: the boards, strips … of the first column.
    if part.group.is_none() && !info.groups.is_empty() {
        ui.label(tr("Groups shown (none ticked: all)"));
        let mut chosen = entries(&part.groups);
        egui::ScrollArea::vertical().id_salt("rpgroups").max_height(120.0).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for g in &info.groups {
                    let mut on = chosen.contains(g);
                    if ui.checkbox(&mut on, if g.is_empty() { tr("(none)").to_string() } else { g.clone() }).changed() {
                        if on {
                            chosen.push(g.clone());
                        } else {
                            chosen.retain(|c| c != g);
                        }
                    }
                }
            });
        });
        let wanted = chosen.join(", ");
        if entries(&wanted) != entries(&part.groups) {
            part.groups = wanted;
        }
        ui.horizontal(|ui| {
            ui.label(tr("Groups"));
            ui.add(egui::TextEdit::singleline(&mut part.groups).hint_text(tr("all, or e.g. -X*, -W1")).desired_width(240.0));
        });
    }
    let view: &mut ReportView = &mut part.view;
    // Columns of the table.
    ui.collapsing(tr("Columns"), |ui| {
        ui.horizontal_wrapped(|ui| {
            for (key, text) in &info.cols {
                let mut on = !view.hidden.contains(key);
                if ui.checkbox(&mut on, text).changed() {
                    if on {
                        view.hidden.retain(|h| h != key);
                    } else {
                        view.hidden.push(key.clone());
                    }
                }
            }
        });
    });
    if let Some(example) = &info.example {
        ui.horizontal(|ui| {
            ui.label(tr("Tags shown as"));
            let current = view.tag_parts.unwrap_or_default();
            let preset = TagParts::PRESETS.iter().find(|(_, t)| *t == current).map(|(n, _)| tr(n).to_string());
            let shown = format!("{}  ({})", preset.unwrap_or_else(|| tr("Custom").to_string()), current.apply(example));
            egui::ComboBox::from_id_salt("rptags").selected_text(shown).show_ui(ui, |ui| {
                for (name, t) in TagParts::PRESETS {
                    if ui.selectable_label(current == t, format!("{}  ({})", tr(name), t.apply(example))).clicked() {
                        view.tag_parts = if t.is_all() { None } else { Some(t) };
                    }
                }
            });
        });
    }
    if info.tree {
        ui.checkbox(&mut view.tree, tr("As a tree: every device below the one it is fed from"));
    }
    if info.graphic {
        ui.horizontal(|ui| {
            ui.label(tr("Form"));
            let text = |g: Option<bool>| match g {
                None => tr("As the report form says"),
                Some(true) => tr("Graphic"),
                Some(false) => tr("Table"),
            };
            egui::ComboBox::from_id_salt("rpgraphic").selected_text(text(view.graphic)).show_ui(ui, |ui| {
                for g in [None, Some(true), Some(false)] {
                    ui.selectable_value(&mut view.graphic, g, text(g));
                }
            });
        });
    }
    if part.block.is_empty() {
        ui.horizontal_wrapped(|ui| {
            if ui.button(tr("Continue on following pages")).on_hover_text(tr("Pages are added after this one for the rows that do not fit, and removed again when the rows shrink")).clicked() {
                action = Some(ReportAction::Continue(false));
            }
            if !info.groups.is_empty() && ui.button(tr("Every group on pages of its own")).clicked() {
                action = Some(ReportAction::Continue(true));
            }
        });
    } else {
        ui.weak(tr_fmt!("Page of the report block {}", part.block));
    }
    action
}

/// The switches of device `tag` for the distribution board overview: shown as its kind
/// says, always or never; for a protective device whether it detects arc faults.
pub(super) fn board_overview_row(ui: &mut egui::Ui, prj: &mut Project, tag: &str) {
    use openecraft_core::properties::{self, Object};
    let obj = Object::Device(tag.to_string());
    let current = properties::stored(prj, &obj, "device.board_overview").unwrap_or("").to_string();
    let text = |v: &str| match v {
        "Show" => tr("Show"),
        "Hide" => tr("Hide"),
        _ => tr("Automatic"),
    };
    ui.horizontal(|ui| {
        ui.label(tr("Distribution board overview"));
        let mut chosen = current.clone();
        egui::ComboBox::from_id_salt("devboard").selected_text(text(&chosen)).show_ui(ui, |ui| {
            for v in ["", "Show", "Hide"] {
                ui.selectable_value(&mut chosen, v.to_string(), text(v));
            }
        })
        .response
        .on_hover_text(tr("Automatic: protective, residual current and surge protective devices and switches on the mains of a board"));
        if chosen != current {
            let _ = properties::set(prj, &obj, "device.board_overview", &chosen);
        }
        if openecraft_core::protection::of_device(prj, tag).is_some() {
            let mut afdd = properties::stored(prj, &obj, "device.afdd") == Some("true");
            if ui.checkbox(&mut afdd, tr("Arc fault detection (AFDD)")).changed() {
                let _ = properties::set(prj, &obj, "device.afdd", if afdd { "true" } else { "false" });
            }
        }
    });
}

/// After the page properties of report page `pi`: a page of a report block gives its
/// view and groups to the block and its other pages; `action` makes it continue on
/// following pages.
pub(super) fn follow_page(prj: &mut Project, an: &Analysis, pi: usize, action: Option<ReportAction>) {
    let Some(part) = prj.pages.get(pi).and_then(|p| p.report.clone()) else { return };
    if !part.block.is_empty() {
        if let Some(b) = prj.report_blocks.iter_mut().find(|b| b.name == part.block) {
            if b.view != part.view {
                b.view = part.view.clone();
                for r in prj.pages.iter_mut().filter_map(|p| p.report.as_mut()).filter(|r| r.frozen.is_none() && r.block == part.block) {
                    r.view = part.view.clone();
                }
            }
        }
    }
    if let Some(ReportAction::Continue(per_group)) = action {
        let _ = openecraft_core::report_blocks::continue_page(prj, an, pi, per_group);
    }
}
