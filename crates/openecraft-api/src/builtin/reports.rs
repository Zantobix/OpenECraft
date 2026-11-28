//! Report block and scheme commands: report pages generated per strip, plug, cable or
//! device, named report forms (filter, sort order, grouping) and the output of report
//! pages into other projects.

use crate::params::{optional, param, Args, Kind};
use crate::{Info, Plugin, Registry};
use super::{add_commands, info, Spec};
use openecraft_core::model::Project;
use openecraft_core::report_blocks::{self, ReportBlock};
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::{json, Value};

pub struct ReportBlocks;

impl Plugin for ReportBlocks {
    fn info(&self) -> Info {
        info("report_blocks", tr_const!("Report blocks"), tr_const!("Report blocks and report schemes"))
    }

    fn register(&self, reg: &mut Registry) -> Result<(), String> {
        add_commands(reg, COMMANDS)
    }
}

pub static COMMANDS: &[Spec] = &[
    Spec {
        name: "list_report_blocks",
        help: tr_const!("Lists the report blocks with their report, scheme, groups and generated pages"),
        params: &[],
        run: list_report_blocks,
    },
    Spec {
        name: "remove_report_block",
        help: tr_const!("Removes a report block; the pages it generated stay"),
        params: &[param("name", Kind::Str, tr_const!("Name of the block"))],
        run: remove_report_block,
    },
    Spec {
        name: "set_report_page",
        help: tr_const!("Sets how a report page shows its rows: the groups (boards, strips …), hidden columns, the aspects of the device tags, the tree and the graphical form"),
        params: &[
            super::common::PAGE,
            optional("groups", Kind::Str, tr_const!("Only these groups of the first column (e.g. +UV1 or -X*, several separated by commas); empty: all")),
            optional("hidden_columns", Kind::Str, tr_const!("Columns not shown, comma separated (their headers); empty: all shown")),
            optional("tag_parts", Kind::Str, tr_const!("Aspects of the device tags shown, comma separated (function, installation, location, product or =, ++, +, -); all: the full tags")),
            optional("tree", Kind::Bool, tr_const!("The report's tree (e.g. the devices below the residual current device they are fed from) instead of a plain table")),
            optional("graphic", Kind::Str, tr_const!("graphic, table or auto (as the report form says)")),
        ],
        run: set_report_page,
    },
    Spec {
        name: "continue_report_page",
        help: tr_const!("Continues a report page on following pages added and removed with its rows (it becomes a report block that follows the project)"),
        params: &[
            super::common::PAGE,
            optional("per_group", Kind::Bool, tr_const!("Every group (board, strip …) on pages of its own")),
        ],
        run: continue_report_page,
    },
    Spec {
        name: "list_report_schemes",
        help: tr_const!("Lists the report schemes (named report forms) with their settings"),
        params: &[],
        run: list_report_schemes,
    },
];

pub fn block_json(prj: &openecraft_core::model::Project, b: &ReportBlock) -> Value {
    let pages: Vec<String> = report_blocks::report_pages(prj, Some(&b.name)).into_iter().map(|i| prj.pages[i].name.clone()).collect();
    json!({
        "name": b.name,
        "kind": b.kind.as_str(),
        "scheme": b.scheme,
        "per_group": b.per_group,
        "groups": b.groups,
        "function": b.function,
        "location": b.location,
        "auto": b.auto,
        "pages": pages,
    })
}

fn list_report_blocks(prj: &mut Project, _: &Args) -> Result<Value, String> {
    Ok(Value::Array(prj.report_blocks.iter().map(|b| block_json(prj, b)).collect()))
}

fn remove_report_block(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let name = a.req("name");
    let i = prj.report_blocks.iter().position(|b| b.name == name).ok_or_else(|| tr_fmt!("No report block named {}", name))?;
    prj.report_blocks.remove(i);
    Ok(json!({ "removed": name }))
}

fn list_report_schemes(prj: &mut Project, _: &Args) -> Result<Value, String> {
    Ok(Value::Array(prj.forms.schemes.iter().map(|s| json!({ "name": s.name, "kind": s.kind.as_str(), "form": s.form })).collect()))
}

fn report_page(prj: &Project, a: &Args) -> Result<(usize, openecraft_core::model::ReportId), String> {
    let i = super::common::page_index(prj, a.req("page"))?;
    match prj.pages[i].kind {
        openecraft_core::model::PageKind::Report(k) => Ok((i, k)),
        _ => Err(tr_fmt!("Page {} is no report page", prj.pages[i].name)),
    }
}

fn view_json(prj: &Project, i: usize) -> Value {
    let part = prj.pages[i].report.clone().unwrap_or_default();
    let v = &part.view;
    let parts: Option<Vec<&str>> = v.tag_parts.map(|t| openecraft_core::structure::Aspect::ALL.into_iter().filter(|a| t.get(*a)).map(|a| a.key()).collect());
    json!({
        "page": prj.pages[i].name,
        "groups": part.groups,
        "group": part.group,
        "block": part.block,
        "hidden_columns": v.hidden,
        "tag_parts": parts,
        "tree": v.tree,
        "graphic": v.graphic,
    })
}

fn set_report_page(prj: &mut Project, a: &Args) -> Result<Value, String> {
    use openecraft_core::model::TagParts;
    use openecraft_core::structure::Aspect;
    let (i, kind) = report_page(prj, a)?;
    let mut part = prj.pages[i].report.clone().unwrap_or_default();
    if part.frozen.is_some() {
        return Err(tr_fmt!("Page {} is a fixed page output from another project", prj.pages[i].name));
    }
    // Every value checked before the page changes.
    if let Some(g) = a.str("groups") {
        part.groups = g.trim().to_string();
    }
    if let Some(h) = a.str("hidden_columns") {
        let an = openecraft_core::reports::Analysis::with_builtin(prj);
        let table = an.report(prj, kind).map(|t| (*t).clone()).unwrap_or_default();
        let mut hidden = Vec::new();
        for name in h.split(',').map(str::trim).filter(|n| !n.is_empty()) {
            let c = table.column(name).ok_or_else(|| tr_fmt!("The report has no column {}", name))?;
            hidden.push(table.header_keys()[c].to_string());
        }
        part.view.hidden = hidden;
    }
    if let Some(t) = a.str("tag_parts") {
        let t = t.trim();
        part.view.tag_parts = if t.is_empty() || t.eq_ignore_ascii_case("all") {
            None
        } else {
            let mut parts = TagParts { function: false, installation: false, location: false, product: false };
            for k in t.split(',').map(str::trim).filter(|k| !k.is_empty()) {
                *parts.get_mut(Aspect::from_key(k).ok_or_else(|| tr_fmt!("Unknown aspect: {}", k))?) = true;
            }
            Some(parts)
        };
    }
    if let Some(v) = a.value("tree").and_then(Value::as_bool) {
        part.view.tree = v;
    }
    if let Some(g) = a.str("graphic") {
        part.view.graphic = match g.trim().to_ascii_lowercase().as_str() {
            "graphic" | "true" => {
                if !report_blocks::has_graphic(kind) {
                    return Err(tr_fmt!("The report {} has no graphical form", kind.title()));
                }
                Some(true)
            }
            "table" | "false" => Some(false),
            "auto" | "" => None,
            other => return Err(tr_fmt!("Unknown form: {} (graphic, table or auto)", other)),
        };
    }
    // A page of a report block: the block keeps the view for its pages.
    if let Some(b) = prj.report_blocks.iter_mut().find(|b| !part.block.is_empty() && b.name == part.block) {
        b.view = part.view.clone();
        for r in prj.pages.iter_mut().filter_map(|p| p.report.as_mut()).filter(|r| r.frozen.is_none() && r.block == part.block) {
            r.view = part.view.clone();
        }
    }
    prj.pages[i].report = Some(part);
    Ok(view_json(prj, i))
}

fn continue_report_page(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let (i, _) = report_page(prj, a)?;
    let per_group = a.value("per_group").and_then(Value::as_bool).unwrap_or(false);
    let an = openecraft_core::reports::Analysis::with_builtin(prj);
    let name = report_blocks::continue_page(prj, &an, i, per_group)?;
    let b = prj.report_blocks.iter().find(|b| b.name == name).cloned().ok_or_else(|| tr_fmt!("No report block {}", name))?;
    Ok(block_json(prj, &b))
}
