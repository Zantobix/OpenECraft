//! Report blocks and report pages: a report block generates the pages of one report
//! with a scheme (filter, sort order, grouping) – one page set per strip, plug,
//! cable, device … if wanted – and replaces them when it is generated again. Report
//! pages can also be output into other projects as fixed pages.

use openecraft_i18n::{tr, tr_fmt};
use crate::forms::{self, ReportForm};
use crate::model::*;
use crate::reports::Table;
use crate::analysis::Analysis;
use std::collections::VecDeque;
pub use crate::model::report_blocks::{LineupBox, PageTable, ReportBlock, ReportPart};

/// Rows per report page if the form sets none.
pub const ROWS_PER_PAGE: usize = 40;
/// Rows per page of the terminal line-up (the drawing of the strip takes space).
pub const LINEUP_ROWS_PER_PAGE: usize = 24;

impl Default for ReportBlock {
    fn default() -> Self {
        ReportBlock {
            name: String::new(),
            kind: ReportKind::TerminalDiagram.into(),
            scheme: String::new(),
            per_group: true,
            groups: String::new(),
            function: String::new(),
            location: String::new(),
            view: Default::default(),
            auto: false,
        }
    }
}

fn column(table: &Table, name: &str) -> Option<usize> {
    table.column(name)
}

/// The rows of a report filtered and sorted by a form (a sorted table loses its tree).
pub fn scheme_table(table: &Table, form: Option<&ReportForm>) -> Table {
    let Some(form) = form else { return table.clone() };
    let filters: Vec<(usize, &forms::RowFilter)> = form.filters.iter().filter_map(|f| Some((column(table, &f.column)?, f))).collect();
    // (cells, jump target, depth)
    type Row = (Vec<String>, Option<(usize, u64)>, u8);
    let mut rows: Vec<Row> = table
        .rows
        .iter()
        .zip(&table.targets)
        .enumerate()
        .filter(|(_, (r, _))| filters.iter().all(|(c, f)| f.passes(r.get(*c).map(String::as_str).unwrap_or(""))))
        .map(|(i, (r, t))| (r.clone(), *t, table.depth.get(i).copied().unwrap_or(0)))
        .collect();
    let keys: Vec<(usize, bool)> = form.sort.iter().filter_map(|k| Some((column(table, &k.column)?, k.descending))).collect();
    let mut out = table.clone_layout();
    out.headers = table.headers.clone();
    out.widths = table.widths.clone();
    if !keys.is_empty() {
        rows.sort_by(|a, b| {
            keys.iter().fold(std::cmp::Ordering::Equal, |o, (c, desc)| {
                o.then_with(|| {
                    let ord = natural_cmp(a.0.get(*c).map(String::as_str).unwrap_or(""), b.0.get(*c).map(String::as_str).unwrap_or(""));
                    if *desc { ord.reverse() } else { ord }
                })
            })
        });
        out.tree_column = None;
    }
    for (r, t, d) in rows {
        out.rows.push(r);
        out.targets.push(t);
        if out.tree_column.is_some() {
            out.depth.push(d);
        }
    }
    out
}

/// Column that groups the rows: the form's group column, otherwise the first one.
pub fn group_column(table: &Table, form: Option<&ReportForm>) -> usize {
    form.and_then(|f| column(table, &f.group)).unwrap_or(0)
}

pub fn rows_per_page(kind: impl Into<ReportId>, form: Option<&ReportForm>) -> usize {
    let kind = kind.into();
    page_rows(kind, form, is_graphic(kind, form))
}

/// Rows per page of a report page drawn graphically or as a table.
pub fn page_rows(kind: ReportId, form: Option<&ReportForm>, graphic: bool) -> usize {
    let default = if graphic {
        GRAPHIC_ROWS_PER_PAGE
    } else if kind == ReportId::from(ReportKind::TerminalLineup) {
        LINEUP_ROWS_PER_PAGE
    } else {
        ROWS_PER_PAGE
    };
    form.map(|f| f.rows).filter(|r| *r > 0).unwrap_or(default)
}

/// Rows per page of graphical report pages.
pub const GRAPHIC_ROWS_PER_PAGE: usize = 26;

/// The report has a graphical form (terminal diagrams, cable and plug diagrams, PLC
/// assignment).
pub fn has_graphic(kind: impl Into<ReportId>) -> bool {
    let kind = kind.into();
    [ReportKind::TerminalDiagram, ReportKind::TerminalWiring, ReportKind::CableDiagram, ReportKind::PlugDiagram, ReportKind::PlcAssignment].into_iter().any(|k| kind == ReportId::from(k))
}

/// The report is drawn in its graphical form: as its form asks; without a form the
/// terminal wiring diagram is.
pub fn is_graphic(kind: impl Into<ReportId>, form: Option<&ReportForm>) -> bool {
    let kind = kind.into();
    form.map_or(kind == ReportId::from(ReportKind::TerminalWiring), |f| f.graphic) && has_graphic(kind)
}

/// A page with `view` is drawn graphically: as the view says, else as [`is_graphic`].
pub fn view_graphic(kind: ReportId, form: Option<&ReportForm>, view: &crate::model::report_blocks::ReportView) -> bool {
    has_graphic(kind) && view.graphic.unwrap_or_else(|| is_graphic(kind, form))
}

/// Values of the group column in order of appearance.
pub fn groups(table: &Table, col: usize) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for r in &table.rows {
        let g = r.get(col).cloned().unwrap_or_default();
        if v.last() != Some(&g) && !v.contains(&g) {
            v.push(g);
        }
    }
    v
}

/// `pattern` matches `text`: wildcards `*` and `?`, several patterns separated by `,`
/// or `;`, case-insensitive; an empty pattern matches everything.
pub fn matches(pattern: &str, text: &str) -> bool {
    let text = text.to_lowercase();
    let mut any = false;
    for p in pattern.split([',', ';']).map(str::trim).filter(|p| !p.is_empty()) {
        any = true;
        if openecraft_parts::wildcard(&p.to_lowercase(), &text) {
            return true;
        }
    }
    !any
}

/// The rows of a report as a page shows them: the device tags of its tag columns with
/// the aspects of `view` only, and its tree column as a tree (`|-> ` below the row a row
/// depends on, `|   ` for each level further down).
pub fn view_table(table: &Table, view: &crate::model::report_blocks::ReportView) -> Table {
    let mut out = table.clone();
    if let Some(parts) = view.tag_parts.filter(|t| !t.is_all()) {
        for row in &mut out.rows {
            for &c in &table.tag_columns {
                if let Some(cell) = row.get_mut(c) {
                    *cell = shorten_tags(cell, &parts);
                }
            }
        }
    }
    if let (true, Some(c)) = (view.tree, table.tree_column) {
        for (row, d) in out.rows.iter_mut().zip(&table.depth) {
            if let (Some(cell), 1..) = (row.get_mut(c), *d) {
                *cell = format!("{}|-> {cell}", "|   ".repeat(*d as usize - 1));
            }
        }
    }
    out
}

/// Every device tag in `cell` (words starting with `=`, `+` or `-`, separated by blanks
/// or commas) with the aspects `parts` only.
fn shorten_tags(cell: &str, parts: &crate::model::TagParts) -> String {
    let mut out = String::new();
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| {
        if word.len() > 1 && word.starts_with(['=', '+', '-']) && word[1..].starts_with(|c: char| c.is_alphanumeric()) {
            out.push_str(&parts.apply(word));
        } else {
            out.push_str(word);
        }
        word.clear();
    };
    for ch in cell.chars() {
        if ch == ' ' || ch == ',' {
            flush(&mut word, &mut out);
            out.push(ch);
        } else {
            word.push(ch);
        }
    }
    flush(&mut word, &mut out);
    out
}

/// Content of a report page; `None` for other pages.
pub fn page_table(prj: &Project, an: &Analysis, page_i: usize) -> Option<PageTable> {
    let page = prj.pages.get(page_i)?;
    let PageKind::Report(kind) = page.kind else { return None };
    let part = page.report.clone().unwrap_or_default();
    if let Some(frozen) = part.frozen {
        return Some(frozen);
    }
    let form = forms::report_form(prj, kind, &part.scheme);
    let table = an.report(prj, kind)?;
    let table = view_table(&scheme_table(&table, form), &part.view);
    let gc = group_column(&table, form);
    let in_group = |i: usize| {
        let g = table.rows[i].get(gc).map(String::as_str).unwrap_or("");
        part.group.as_deref().is_none_or(|own| own == g) && matches(&part.groups, g)
    };
    let rows: Vec<usize> = (0..table.rows.len()).filter(|&i| in_group(i)).collect();
    let layout = form.and_then(|f| f.layout.clone());
    let graphic = layout.is_none() && view_graphic(kind, form, &part.view);
    let per_page = page_rows(kind, form, graphic);
    let shown: Vec<usize> = rows.iter().skip(part.first).take(per_page).copied().collect();
    let more = rows.len().saturating_sub(part.first + shown.len());
    // Graphical and drawn forms read the columns in the report's own order.
    let cols: Vec<usize> = if graphic || layout.is_some() {
        (0..table.headers.len()).collect()
    } else {
        forms::report_columns(form, &table.headers).into_iter().filter(|&c| !part.view.hidden.iter().any(|h| crate::table::header_matches(&table.headers[c], h))).collect()
    };
    let width = |c: usize| form.and_then(|f| forms::header_value(&f.widths, &table.headers[c])).copied().filter(|w| *w > 0.0).unwrap_or(table.widths[c]);
    let mut out_rows: Vec<Vec<String>> = shown.iter().map(|&i| cols.iter().map(|&c| table.rows[i].get(c).cloned().unwrap_or_default()).collect()).collect();
    if form.is_some_and(|f| f.blank_repeats) && !graphic && layout.is_none() {
        for i in (1..out_rows.len()).rev() {
            if !out_rows[i].is_empty() && out_rows[i][0] == out_rows[i - 1][0] {
                out_rows[i][0].clear();
            }
        }
    }
    let kind_title = kind.title();
    let base = form.map(|f| f.title.as_str()).filter(|t| !t.is_empty()).unwrap_or(&kind_title);
    let title = match part.group.as_deref() {
        Some(g) if !g.is_empty() => format!("{base} {g}"),
        _ => base.to_string(),
    };
    let lineup = if kind == ReportId::from(ReportKind::TerminalLineup) { lineup_boxes(&table, &shown) } else { Vec::new() };
    let table_form = form.filter(|_| !graphic && layout.is_none());
    let totals = match table_form {
        Some(f) if !f.totals.is_empty() && more == 0 => total_row(&table, &rows, &cols, &f.totals),
        _ => Vec::new(),
    };
    let align = table_form
        .filter(|f| !f.align.is_empty())
        .map(|f| cols.iter().map(|&c| forms::header_value(&f.align, &table.headers[c]).copied().unwrap_or(crate::symbols::Anchor::Left)).collect())
        .unwrap_or_default();
    Some(PageTable {
        title,
        headers: cols.iter().map(|&c| table.header_text(c).to_string()).collect(),
        widths: cols.iter().map(|&c| width(c)).collect(),
        rows: out_rows,
        targets: shown.iter().map(|&i| table.targets[i]).collect(),
        per_page,
        text_size: form.map(|f| f.text_size).unwrap_or(0.0),
        more,
        continued: more > 0 && !part.block.is_empty(),
        lineup,
        source: String::new(),
        graphic,
        keys: if layout.is_some() { table.header_keys().into_iter().map(String::from).collect() } else { Vec::new() },
        layout,
        totals,
        shade_rows: table_form.is_some_and(|f| f.shade_rows),
        align,
    })
}

/// The leading number of a cell (`1.5 m`, `2,5`) and the text after it.
pub fn cell_number(cell: &str) -> Option<(f64, &str)> {
    let cell = cell.trim();
    let end = cell.char_indices().find(|(i, c)| !(c.is_ascii_digit() || *c == '.' || *c == ',' || (*i == 0 && (*c == '-' || *c == '+')))).map(|(i, _)| i).unwrap_or(cell.len());
    let n: f64 = cell[..end].replace(',', ".").parse().ok()?;
    Some((n, cell[end..].trim()))
}

/// Total row of the rows `rows` of `table` for the shown columns `cols`: the sums
/// of the columns named in `summed` (with the unit of their first value), "Total"
/// in the first column when it is not summed.
pub fn total_row(table: &Table, rows: &[usize], cols: &[usize], summed: &[String]) -> Vec<String> {
    let mut out: Vec<String> = cols
        .iter()
        .map(|&c| {
            if !summed.iter().any(|s| crate::table::header_matches(&table.headers[c], s)) {
                return String::new();
            }
            let values: Vec<(f64, &str)> = rows.iter().filter_map(|&i| cell_number(table.rows[i].get(c)?)).collect();
            let sum: f64 = values.iter().map(|v| v.0).sum();
            let unit = values.first().map(|v| v.1).unwrap_or("");
            let mut s = format!("{:.3}", sum).trim_end_matches('0').trim_end_matches('.').to_string();
            if s == "-0" {
                s = "0".into();
            }
            if unit.is_empty() { s } else { format!("{s} {unit}") }
        })
        .collect();
    if out.iter().any(|c| !c.is_empty()) && out.first().is_some_and(String::is_empty) {
        out[0] = tr("Total").into();
    }
    out
}

/// Drawing elements of the terminal line-up rows (all columns of the report).
fn lineup_boxes(table: &Table, rows: &[usize]) -> Vec<LineupBox> {
    let (Some(element), Some(label), Some(jumper), Some(width)) =
        (column(table, tr("Element")), column(table, tr("Designation")), column(table, tr("Jumper")), column(table, tr("Width (mm)")))
    else {
        return Vec::new();
    };
    rows.iter()
        .map(|&i| {
            let r = &table.rows[i];
            let kind = r[element].as_str();
            LineupBox {
                label: r[label].clone(),
                width: r[width].parse().unwrap_or(crate::strips::TERMINAL_WIDTH),
                jumper: r[jumper].parse().unwrap_or(0),
                accessory: r[label].is_empty(),
                spare: kind == tr("Spare terminal"),
            }
        })
        .collect()
}

/// The pages of a block: (group, first row) per page.
pub fn block_pages(prj: &Project, an: &Analysis, block: &ReportBlock) -> Vec<(Option<String>, usize)> {
    let form = forms::report_form(prj, block.kind, &block.scheme);
    let Some(table) = an.report(prj, block.kind) else { return vec![(None, 0)] };
    let table = scheme_table(&table, form);
    let per_page = page_rows(block.kind, form, form.is_none_or(|f| f.layout.is_none()) && view_graphic(block.kind, form, &block.view));
    let chunks = |n: usize| (0..n.div_ceil(per_page).max(1)).map(move |k| k * per_page);
    let gc = group_column(&table, form);
    if !block.per_group {
        let n = table.rows.iter().filter(|r| matches(&block.groups, r.get(gc).map(String::as_str).unwrap_or(""))).count();
        return chunks(n).map(|f| (None, f)).collect();
    }
    let mut out = Vec::new();
    for g in groups(&table, gc).into_iter().filter(|g| matches(&block.groups, g)) {
        let n = table.rows.iter().filter(|r| r.get(gc) == Some(&g)).count();
        out.extend(chunks(n).map(|f| (Some(g.clone()), f)));
    }
    out
}

/// Removes the pages that `mine` selects and returns where they were and their names.
fn take_pages(prj: &mut Project, mine: impl Fn(&Page) -> bool) -> Result<(usize, VecDeque<String>), String> {
    if let Some(pg) = prj.pages.iter().find(|p| mine(p) && p.locked) {
        return Err(tr_fmt!("Page {} is locked", pg.name));
    }
    let at = prj.pages.iter().position(&mine).unwrap_or(prj.pages.len());
    let names = prj.pages.iter().filter(|p| mine(p)).map(|p| p.name.clone()).collect();
    prj.pages.retain(|p| !mine(p));
    Ok((at, names))
}

/// Generates the pages of a report block, replacing the ones it generated before (at
/// their place, with their page names). Returns the number of pages.
pub fn generate(prj: &mut Project, an: &Analysis, block: usize) -> Result<usize, String> {
    let b = prj.report_blocks.get(block).cloned().ok_or_else(|| tr_fmt!("No report block {}", block + 1))?;
    let mine = |p: &Page| p.report.as_ref().is_some_and(|r| r.frozen.is_none() && r.block == b.name);
    let (at, mut names) = take_pages(prj, mine)?;
    let pages = block_pages(prj, an, &b);
    let title = forms::report_form(prj, b.kind, &b.scheme).map(|f| f.title.clone()).filter(|t| !t.is_empty()).unwrap_or_else(|| b.kind.title());
    for (i, (group, first)) in pages.iter().enumerate() {
        let name = names.pop_front().unwrap_or_else(|| crate::ops::next_page_name(prj, at + i));
        let description = match group {
            Some(g) if !g.is_empty() => format!("{title} {g}"),
            _ => title.clone(),
        };
        let id = prj.new_id();
        let mut page = Page::new(id, &name, &description, PageKind::Report(b.kind));
        page.function = b.function.clone();
        page.location = b.location.clone();
        page.report = Some(ReportPart { block: b.name.clone(), scheme: b.scheme.clone(), group: group.clone(), first: *first, view: b.view.clone(), groups: if b.per_group { String::new() } else { b.groups.clone() }, ..Default::default() });
        crate::ops::insert_page(prj, at + i, page);
    }
    Ok(pages.len())
}

/// Called after every edit in the editor: generates again the report blocks that follow
/// the project ([`ReportBlock::auto`]) whose pages no longer fit their rows (with the
/// built-in reports). Returns the number of blocks generated.
pub fn follow(prj: &mut Project, before: &Project) -> usize {
    if !prj.report_blocks.iter().any(|b| b.auto) || prj == before {
        return 0;
    }
    let an = Analysis::with_builtin(prj);
    let mut n = 0;
    for i in 0..prj.report_blocks.len() {
        let b = prj.report_blocks[i].clone();
        if !b.auto {
            continue;
        }
        let want = block_pages(prj, &an, &b);
        let have: Vec<(Option<String>, usize)> = prj.pages.iter().filter_map(|p| p.report.as_ref()).filter(|r| r.frozen.is_none() && r.block == b.name).map(|r| (r.group.clone(), r.first)).collect();
        if want != have && generate(prj, &an, i).is_ok() {
            n += 1;
        }
    }
    n
}

/// Makes report page `page_i` the first page of a report block that follows the project
/// ([`ReportBlock::auto`]): its rows continue on pages added after it (and the pages go
/// again when the rows shrink); with `per_group` every group (strip, board …) starts on
/// a page of its own. The block takes the page's name, report, scheme, groups and view.
/// Returns the name of the block.
pub fn continue_page(prj: &mut Project, an: &Analysis, page_i: usize, per_group: bool) -> Result<String, String> {
    let pg = prj.pages.get(page_i).ok_or_else(|| tr_fmt!("No page {}", page_i + 1))?;
    let PageKind::Report(kind) = pg.kind else { return Err(tr_fmt!("Page {} is no report page", pg.name)) };
    let part = pg.report.clone().unwrap_or_default();
    if part.frozen.is_some() {
        return Err(tr_fmt!("Page {} is a fixed page output from another project", pg.name));
    }
    if !part.block.is_empty() {
        if let Some(b) = prj.report_blocks.iter_mut().find(|b| b.name == part.block) {
            b.auto = true;
            b.per_group = per_group;
            let name = b.name.clone();
            let i = prj.report_blocks.iter().position(|b| b.name == name).unwrap_or(0);
            generate(prj, an, i)?;
            return Ok(name);
        }
    }
    let base = if pg.name.trim().is_empty() { kind.title() } else { pg.name.clone() };
    let name = std::iter::once(base.clone()).chain((2..).map(|i| format!("{base} ({i})"))).find(|n| !prj.report_blocks.iter().any(|b| &b.name == n)).unwrap_or(base);
    let groups = match (&part.group, part.groups.trim()) {
        (_, g) if !g.is_empty() => g.to_string(),
        (Some(g), _) => g.clone(),
        _ => String::new(),
    };
    let block = ReportBlock {
        name: name.clone(),
        kind,
        scheme: part.scheme.clone(),
        per_group,
        groups,
        function: pg.function.clone(),
        location: pg.location.clone(),
        view: part.view.clone(),
        auto: true,
    };
    prj.report_blocks.push(block);
    if let Some(r) = prj.pages[page_i].report.as_mut() {
        r.block = name.clone();
    } else {
        prj.pages[page_i].report = Some(ReportPart { block: name.clone(), ..Default::default() });
    }
    generate(prj, an, prj.report_blocks.len() - 1)?;
    Ok(name)
}

/// Renames a report block and its pages. Fails for empty names and names in use.
pub fn rename_block(prj: &mut Project, old: &str, new: &str) -> bool {
    let new = new.trim();
    if new.is_empty() || prj.report_blocks.iter().any(|b| b.name == new) {
        return false;
    }
    let Some(b) = prj.report_blocks.iter_mut().find(|b| b.name == old) else { return false };
    b.name = new.to_string();
    for r in prj.pages.iter_mut().filter_map(|p| p.report.as_mut()).filter(|r| r.frozen.is_none() && r.block == old) {
        r.block = new.to_string();
    }
    true
}

/// Generates all report blocks with the built-in reports. Returns the number of pages.
pub fn generate_all(prj: &mut Project) -> Result<usize, String> {
    generate_all_with(prj, crate::reports::builtin())
}

/// Generates all report blocks with the reports of a registry (built-in and of plugins).
pub fn generate_all_with(prj: &mut Project, reports: Vec<crate::ext::Report>) -> Result<usize, String> {
    let mut an = Analysis::build(prj);
    an.set_reports(reports);
    let mut n = 0;
    for i in 0..prj.report_blocks.len() {
        n += generate(prj, &an, i)?;
    }
    Ok(n)
}

/// Report pages of a project (all, or those of one report block).
pub fn report_pages(prj: &Project, block: Option<&str>) -> Vec<usize> {
    (0..prj.pages.len())
        .filter(|&i| matches!(prj.pages[i].kind, PageKind::Report(_)))
        .filter(|&i| block.is_none_or(|b| prj.pages[i].report.as_ref().is_some_and(|r| r.block == b)))
        .collect()
}

/// Outputs report pages of `src` into `target` as fixed pages (they keep their
/// content when the target is changed). Pages output from the same project before
/// are replaced. Returns the number of pages.
pub fn output_to_project(src: &Project, an: &Analysis, pages: &[usize], target: &mut Project) -> Result<usize, String> {
    let source = src.info.name.clone();
    let from_here = |p: &Page| p.report.as_ref().and_then(|r| r.frozen.as_ref()).is_some_and(|f| f.source == source);
    let (at, mut names) = take_pages(target, from_here)?;
    target.fix_next_id();
    let mut n = 0;
    for &pi in pages {
        let Some(mut table) = page_table(src, an, pi) else { continue };
        table.source = source.clone();
        table.targets.clear();
        table.continued = table.continued || table.more > 0;
        let sp = &src.pages[pi];
        let name = names.pop_front().unwrap_or_else(|| crate::ops::next_page_name(target, at + n));
        let id = target.new_id();
        let mut page = Page::new(id, &name, &sp.description, sp.kind);
        page.function = sp.function.clone();
        page.location = sp.location.clone();
        let block = sp.report.as_ref().map(|r| r.block.clone()).unwrap_or_default();
        page.report = Some(ReportPart { block, frozen: Some(table), ..Default::default() });
        crate::ops::insert_page(target, at + n, page);
        n += 1;
    }
    Ok(n)
}

/// [`output_to_project`] on a project file: returns the changed file content.
pub fn output_to_file(src: &Project, an: &Analysis, pages: &[usize], target: &str) -> Result<(String, usize), String> {
    // Not `Project::from_json`: that registers the symbols of the other project.
    let mut prj: Project = serde_json::from_str(target).map_err(|e| tr_fmt!("Not a valid project file: {}", e))?;
    let n = output_to_project(src, an, pages, &mut prj)?;
    Ok((prj.to_json(), n))
}

#[cfg(test)]
mod tests;
