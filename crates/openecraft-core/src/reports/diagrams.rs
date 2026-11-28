//! Connection diagrams (terminals, plug pins, cables, devices), the terminal line-up
//! and strip overview, and the legend of the panel layout.

use super::{cable_connections, loc_str, sorted_cables, Table};
use crate::index::Index;
use crate::model::*;
use crate::netlist::{pin_designation, Connection, Netlist, PinRef};
use crate::panel::ItemKind;
use crate::strips::ElementKind;
use crate::symbols::{Func, PinKind};
use openecraft_i18n::{tr, tr_const, tr_fmt};
use std::collections::BTreeMap;

/// Connection columns shared by the connection diagrams: target, wire number, cable,
/// core, colour, cross-section, page of the target.
const CONNECTION_COLS: [(&str, f32); 7] = [
    (tr_const!("Target"), 1.5),
    (tr_const!("Wire no."), 0.7),
    (tr_const!("Cable"), 0.7),
    (tr_const!("Core"), 0.5),
    (tr_const!("Colour"), 0.5),
    (tr_const!("Cross-section"), 0.8),
    (tr_const!("Page"), 0.7),
];

fn columns(first: &[(&'static str, f32)]) -> Table {
    let cols: Vec<(&'static str, f32)> = first.iter().copied().chain(CONNECTION_COLS.iter().copied()).collect();
    Table::with_widths(&cols)
}

/// The connections at one connection point, as (other end, cells of the connection).
fn at_pin(prj: &Project, conns: &[Connection], pin: PinRef) -> Vec<(PinRef, Vec<String>)> {
    conns
        .iter()
        .filter_map(|c| if c.a == pin { Some((c.b, c)) } else if c.b == pin { Some((c.a, c)) } else { None })
        .map(|(other, c)| {
            let s = &prj.pages[other.page].symbols[other.sym];
            let cells = vec![
                pin_designation(prj, other),
                c.number.clone(),
                c.cable.clone(),
                c.core.clone(),
                c.color.clone(),
                c.cross_section.clone(),
                loc_str(prj, other.page, s.pos),
            ];
            (other, cells)
        })
        .collect()
}

fn target_of(prj: &Project, pin: PinRef) -> Option<(usize, u64)> {
    Some((pin.page, prj.pages[pin.page].symbols[pin.sym].id))
}

/// Rows of one connection point: one per connection, or an empty one if nothing is
/// connected. Sides without a name get no column.
fn push_point(t: &mut Table, prj: &Project, conns: &[Connection], lead: Vec<String>, sides: &[(PinRef, &str)]) {
    let mut any = false;
    for (pin, side) in sides {
        for (_, cells) in at_pin(prj, conns, *pin) {
            let mut row = lead.clone();
            if !side.is_empty() {
                row.push(side.to_string());
            }
            row.extend(cells);
            t.push(row, target_of(prj, *pin));
            any = true;
        }
    }
    if !any {
        let mut row = lead;
        row.resize(t.headers.len(), String::new());
        t.push(row, sides.first().and_then(|(p, _)| target_of(prj, *p)));
    }
}

/// Every terminal of every strip with what is connected on its external and internal
/// side.
pub(super) fn terminal_connections(prj: &Project, conns: &[Connection]) -> Table {
    let mut t = columns(&[(tr_const!("Strip"), 0.7), (tr_const!("Terminal"), 0.6), (tr_const!("Side"), 0.7)]);
    for strip in crate::strips::strip_tags(prj) {
        for term in crate::strips::terminals(prj, &strip) {
            let pin = |pin| PinRef { page: term.page, sym: term.sym, pin };
            let sides = [(pin(0), tr("External")), (pin(1), tr("Internal"))];
            push_point(&mut t, prj, conns, vec![strip.clone(), term.label.clone()], &sides);
        }
    }
    t
}

/// The conductor a terminal carries: `PE` and `N` for PE and N terminals, otherwise by
/// the kind of its potential (`L`, `N`, `PE`, `+`, `-`; empty for other potentials).
fn conductor(prj: &Project, sym: &str, potential: &str) -> &'static str {
    use crate::model::potentials::PotentialKind as K;
    match sym {
        "term_pe" => "PE",
        "term_n" => "N",
        _ if potential.is_empty() => "",
        _ => match crate::electrical::net_kind(prj, potential) {
            K::L => "L",
            K::N => "N",
            K::Pe => "PE",
            K::DcPlus => "+",
            K::DcMinus => "-",
            _ => "",
        },
    }
}

/// Terminal wiring diagram: one row per terminal (in the order of the strip, every
/// level) with the function text, the device and connection point wired directly to
/// its external and internal side (not every device of the potential), their cables
/// and cores and where they are drawn, its level and potential, and the jumper or
/// busbar the terminal belongs to with the terminals it spans; the first terminal of a
/// busbar after another busbar of the strip on the same level names the busbar it is
/// separated from (an interrupted N busbar). A side with several wires continues in
/// the rows below.
pub(super) fn terminal_wiring(prj: &Project, nl: &Netlist, open: &Netlist, conns: &[Connection]) -> Table {
    let mut t = Table::with_widths(&[
        (tr_const!("Strip"), 0.7),
        (tr_const!("Function text"), 1.0),
        (tr_const!("External target"), 0.8),
        (tr_const!("External point"), 0.6),
        (tr_const!("External location"), 0.7),
        (tr_const!("External cable"), 0.8),
        (tr_const!("Terminal"), 0.5),
        (tr_const!("Terminal level"), 0.65),
        (tr_const!("Jumper"), 1.0),
        (tr_const!("Potential"), 0.6),
        (tr_const!("Internal target"), 0.8),
        (tr_const!("Internal point"), 0.6),
        (tr_const!("Internal location"), 0.7),
        (tr_const!("Internal cable"), 0.8),
        (tr_const!("Conductor kind"), 0.5),
    ]);
    t.tag_columns = vec![2, 10];
    // Device, connection point, location and cable of the other end of each wire.
    let side = |pin: PinRef| -> Vec<[String; 4]> {
        conns
            .iter()
            .filter_map(|c| if c.a == pin { Some((c.b, c)) } else if c.b == pin { Some((c.a, c)) } else { None })
            // Not the terminal's own other side (the same potential drawn above and below it).
            .filter(|(other, _)| (other.page, other.sym) != (pin.page, pin.sym))
            .map(|(other, c)| {
                let s = &prj.pages[other.page].symbols[other.sym];
                let full = pin_designation(prj, other);
                let point = full.strip_prefix(s.tag.as_str()).map(|p| p.trim_start_matches(':')).filter(|_| !s.tag.is_empty()).unwrap_or(&full).to_string();
                [s.tag.clone(), point, loc_str(prj, other.page, s.pos), format!("{} {}", c.cable, c.core).trim().to_string()]
            })
            .collect()
    };
    for strip in crate::strips::strip_tags(prj) {
        let mut rows = crate::strips::rows(prj, &strip, nl, open);
        let jumpers = crate::strips::strip_jumpers(prj, &strip);
        let jumpered = prj.strips.get(&strip).map(|d| d.jumpers.clone()).unwrap_or_default();
        // The busbar of the terminal before on each level.
        let mut last_bar: BTreeMap<u8, Option<u32>> = BTreeMap::new();
        for e in crate::strips::strip_layout(prj, &strip) {
            if !matches!(e.kind, ElementKind::Terminal(_) | ElementKind::Spare(_)) {
                continue;
            }
            for label in e.designations() {
                let Some(i) = rows.iter().position(|r| r.label == label) else { continue };
                let r = rows.remove(i);
                let jumper = jumpered.get(&r.label).copied().filter(|n| *n > 0).and_then(|n| jumpers.iter().find(|j| j.number == n));
                let bar = jumper.filter(|j| j.kind == JumperKind::Busbar).map(|j| j.number);
                let before = last_bar.insert(r.level, bar).flatten();
                let jumper_text = match jumper {
                    Some(j) => {
                        let (a, b) = (j.span.first().cloned().unwrap_or_default(), j.span.last().cloned().unwrap_or_default());
                        match (j.kind, before) {
                            (JumperKind::Busbar, Some(prev)) if prev != j.number && a == r.label => tr_fmt!("busbar {} ({} – {}), separated from {}", j.number, a, b, prev),
                            (JumperKind::Busbar, _) => tr_fmt!("busbar {} ({} – {})", j.number, a, b),
                            _ => format!("{} ({a} – {b})", j.number),
                        }
                    }
                    None => jumpered.get(&r.label).copied().filter(|n| *n > 0).map(|n| n.to_string()).unwrap_or_default(),
                };
                let (ext, int) = match &r.terminal {
                    Some(term) => (side(PinRef { page: term.page, sym: term.sym, pin: 0 }), side(PinRef { page: term.page, sym: term.sym, pin: 1 })),
                    None => (Vec::new(), Vec::new()),
                };
                let target = r.terminal.as_ref().map(|t| (t.page, t.id));
                let level = if e.levels.is_empty() { String::new() } else { r.level.to_string() };
                let conductor = conductor(prj, r.terminal.as_ref().map(|t| prj.pages[t.page].symbols[t.sym].sym.as_str()).unwrap_or(""), &r.potential);
                let lines = ext.len().max(int.len()).max(1);
                for n in 0..lines {
                    let [ed, ep, el, ec] = ext.get(n).cloned().unwrap_or_default();
                    let [id, ip, il, ic] = int.get(n).cloned().unwrap_or_default();
                    let first = n == 0;
                    let pick = |v: &String| if first { v.clone() } else { String::new() };
                    t.push(vec![strip.clone(), pick(&r.function_text), ed, ep, el, ec, pick(&r.label), pick(&level), pick(&jumper_text), pick(&r.potential), id, ip, il, ic, conductor.to_string()], target);
                }
            }
        }
    }
    t
}

/// Every pin of every plug with what is connected on its socket and pin side.
pub(super) fn pin_connections(prj: &Project, conns: &[Connection]) -> Table {
    let mut t = columns(&[(tr_const!("Plug"), 0.7), (tr_const!("Pin"), 0.5), (tr_const!("Side"), 0.8)]);
    for plug in crate::plugs::plug_tags(prj) {
        let pins = crate::plugs::pins(prj, &plug);
        for label in crate::plugs::designations(prj, &plug) {
            let mut sides = Vec::new();
            for p in pins.iter().filter(|p| p.label == label) {
                let pin = |pin| PinRef { page: p.page, sym: p.sym, pin };
                match p.kind {
                    PinKind::Pair => sides.extend([(pin(0), tr("Socket side")), (pin(1), tr("Pin side"))]),
                    PinKind::Female => sides.push((pin(0), tr("Socket side"))),
                    PinKind::Male => sides.push((pin(0), tr("Pin side"))),
                }
            }
            push_point(&mut t, prj, conns, vec![plug.clone(), label], &sides);
        }
    }
    t
}

/// Every connection point of the devices (main and auxiliary functions, PLC points;
/// no terminals and plugs) with its connections.
pub(super) fn device_connections(prj: &Project, idx: &Index, conns: &[Connection]) -> Table {
    let mut t = columns(&[(tr_const!("Device"), 0.8), (tr_const!("Connection point"), 0.8), (tr_const!("Function"), 1.2)]);
    for tag in idx.sorted_tags() {
        for (pi, pg) in prj.pages.iter().enumerate().filter(|(_, p)| p.kind == PageKind::Schematic) {
            for (si, s) in pg.symbols.iter().enumerate().filter(|(_, s)| &s.tag == tag) {
                let Some(def) = prj.symbol(&s.sym) else { continue };
                if matches!(def.func, Func::Terminal | Func::Plug(_) | Func::Overview | Func::Potential | Func::Interruption) {
                    continue;
                }
                for pin in 0..def.pins.len() {
                    let at = PinRef { page: pi, sym: si, pin };
                    let full = pin_designation(prj, at);
                    let point = full.strip_prefix(tag.as_str()).map(|p| p.trim_start_matches(':')).unwrap_or(&full).to_string();
                    push_point(&mut t, prj, conns, vec![tag.clone(), point, tr(def.name).to_string()], &[(at, "")]);
                }
            }
        }
    }
    t
}

/// Every core of every cable with both ends, the reserve cores included.
pub(super) fn cable_connections_report(prj: &Project, conns: &[Connection]) -> Table {
    let mut t = Table::with_widths(&[
        (tr_const!("Cable"), 0.8),
        (tr_const!("Type"), 1.1),
        (tr_const!("Length (m)"), 0.6),
        (tr_const!("Source"), 1.4),
        (tr_const!("Core"), 0.6),
        (tr_const!("Target"), 1.4),
        (tr_const!("Wire no."), 0.7),
        (tr_const!("Potential"), 0.9),
        (tr_const!("Cross-section"), 0.8),
    ]);
    for cable in sorted_cables(prj) {
        let used = cable_connections(prj, conns, &cable.tag);
        let length = if cable.length > 0.0 { format!("{}", cable.length) } else { String::new() };
        let mut cores = cable.core_list();
        for c in &used {
            if !cores.contains(&c.core) {
                cores.push(c.core.clone());
            }
        }
        for core in cores {
            let lead = vec![cable.tag.clone(), cable.cable_type.clone(), length.clone()];
            let on_core: Vec<&Connection> = used.iter().filter(|c| c.core == core).collect();
            if on_core.is_empty() {
                let row = [lead.clone(), vec![String::new(), core.clone(), String::new(), String::new(), tr("Reserve").into(), cable.cross_section.clone()]].concat();
                t.push(row, None);
            }
            for c in on_core {
                let cs = if c.cross_section.is_empty() { cable.cross_section.clone() } else { c.cross_section.clone() };
                let cells = vec![pin_designation(prj, c.a), core.clone(), pin_designation(prj, c.b), c.number.clone(), c.net.clone(), cs];
                t.push([lead.clone(), cells].concat(), target_of(prj, c.a));
            }
        }
    }
    t
}

/// The strips on the rail element by element: position, part, width and where the
/// element sits along the strip.
pub(super) fn terminal_lineup(prj: &Project) -> Table {
    let mut t = Table::with_widths(&[
        (tr_const!("Strip"), 0.7),
        (tr_const!("Pos"), 0.4),
        (tr_const!("Element"), 1.0),
        (tr_const!("Designation"), 0.8),
        (tr_const!("Jumper"), 0.5),
        (tr_const!("Part number"), 1.6),
        (tr_const!("Width (mm)"), 0.7),
        (tr_const!("Position (mm)"), 1.0),
    ]);
    for strip in crate::strips::strip_tags(prj) {
        let data = prj.strips.get(&strip).cloned().unwrap_or_default();
        let terms = crate::strips::terminals(prj, &strip);
        let mut x = 0.0;
        for (i, e) in crate::strips::strip_layout(prj, &strip).into_iter().enumerate() {
            let (title, label, target) = match &e.kind {
                ElementKind::Terminal(l) => {
                    let target = terms.iter().find(|t| &t.label == l).map(|t| (t.page, t.id));
                    (tr("Terminal"), l.clone(), target)
                }
                ElementKind::Spare(l) => (tr("Spare terminal"), l.clone(), None),
                ElementKind::EndBracket => (tr("End bracket"), String::new(), None),
                ElementKind::EndPlate => (tr("End plate"), String::new(), None),
            };
            let jumper = data.jumpers.get(&label).filter(|j| **j > 0).map(|j| j.to_string()).unwrap_or_default();
            let row = vec![
                strip.clone(),
                (i + 1).to_string(),
                title.into(),
                label,
                jumper,
                e.part.clone(),
                format!("{:.1}", e.width),
                format!("{:.1} – {:.1}", x, x + e.width),
            ];
            x += e.width;
            t.push(row, target);
        }
    }
    t
}

/// One row per strip: terminals, spares, connected terminals, jumpers, length on the
/// rail, parts and pages.
pub(super) fn strip_overview(prj: &Project, nl: &Netlist, open: &Netlist) -> Table {
    let mut t = Table::with_widths(&[
        (tr_const!("Strip"), 0.7),
        (tr_const!("Description"), 1.5),
        (tr_const!("Enclosure"), 0.7),
        (tr_const!("Terminals"), 0.6),
        (tr_const!("Spare"), 0.5),
        (tr_const!("Connected"), 0.6),
        (tr_const!("Jumpers"), 0.5),
        (tr_const!("Length (mm)"), 0.7),
        (tr_const!("Parts"), 1.6),
        (tr_const!("Pages"), 1.0),
    ]);
    for strip in crate::strips::strip_tags(prj) {
        let terms = crate::strips::terminals(prj, &strip);
        let rows = crate::strips::rows(prj, &strip, nl, open);
        let layout = crate::strips::strip_layout(prj, &strip);
        let spares = prj.strips.get(&strip).map(|d| d.spares).unwrap_or(0);
        let connected = rows.iter().filter(|r| r.terminal.is_some() && !(r.external.is_empty() && r.internal.is_empty())).count();
        let mut parts: BTreeMap<&str, usize> = BTreeMap::new();
        for e in layout.iter().filter(|e| !e.part.is_empty()) {
            *parts.entry(e.part.as_str()).or_default() += 1;
        }
        let parts: Vec<String> = parts.iter().map(|(p, n)| if *n > 1 { format!("{n}× {p}") } else { p.to_string() }).collect();
        let mut pages: Vec<String> = terms.iter().map(|t| prj.pages[t.page].name.clone()).collect();
        pages.dedup();
        let enclosure = prj.layout.find_device(&strip).map(|(e, _)| prj.layout.enclosures[e].name.clone()).unwrap_or_default();
        let length: f32 = layout.iter().map(|e| e.width).sum();
        t.push(
            vec![
                strip.clone(),
                prj.devices.get(&strip).map(|d| d.description.clone()).unwrap_or_default(),
                enclosure,
                terms.len().to_string(),
                spares.to_string(),
                connected.to_string(),
                crate::strips::strip_jumpers(prj, &strip).len().to_string(),
                format!("{length:.1}"),
                parts.join(", "),
                pages.join(", "),
            ],
            terms.first().map(|t| (t.page, t.id)),
        );
    }
    t
}

/// Legend of the panel layout: the devices of every enclosure numbered top to bottom,
/// left to right, with description, part and position.
pub(super) fn panel_legend(prj: &Project, idx: &Index) -> Table {
    let mut t = Table::with_widths(&[
        (tr_const!("Enclosure"), 0.7),
        (tr_const!("Pos"), 0.4),
        (tr_const!("Device tag"), 0.9),
        (tr_const!("Description"), 2.0),
        (tr_const!("Part number"), 1.4),
        (tr_const!("Mounting"), 0.8),
        (tr_const!("Position (mm)"), 0.9),
        (tr_const!("Size (mm)"), 0.9),
        (tr_const!("Length (mm)"), 0.7),
    ]);
    let order = |a: &&crate::panel::PanelItem, b: &&crate::panel::PanelItem| a.face.order().cmp(&b.face.order()).then(a.y.round().total_cmp(&b.y.round())).then(a.x.total_cmp(&b.x));
    for e in &prj.layout.enclosures {
        // The devices, then the rails, ducts and busbars to cut to length with their
        // cross-section (width × height) and cut length.
        let mut devices: Vec<_> = e.items.iter().filter(|i| i.kind == ItemKind::Device).collect();
        devices.sort_by(order);
        let mut profiles: Vec<_> = e.items.iter().filter(|i| i.kind.is_profile()).collect();
        profiles.sort_by(|a, b| a.kind.layer().cmp(&b.kind.layer()).then(order(a, b)));
        for (n, it) in devices.into_iter().chain(profiles).enumerate() {
            let part = prj.parts.iter().chain(crate::parts::catalog()).find(|p| p.number == it.part);
            let part_text = || part.map(|p| p.description.clone()).filter(|d| !d.is_empty());
            let (description, target, size, length) = if it.kind == ItemKind::Device {
                let data = prj.devices.get(&it.tag);
                let description = data.map(|d| d.description.clone()).filter(|d| !d.is_empty()).or_else(part_text).unwrap_or_default();
                let target = idx.devices.get(&it.tag).and_then(|d| d.mains.first().or(d.others.first())).map(|l| (l.page, l.sym_id));
                (description, target, format!("{:.0} × {:.0}", it.w, it.h), String::new())
            } else {
                (part_text().unwrap_or_else(|| it.kind.title().into()), None, it.profile_section(), crate::panel::mm_text(it.length()))
            };
            t.push(
                vec![
                    e.name.clone(),
                    (n + 1).to_string(),
                    it.tag.clone(),
                    description,
                    it.part.clone(),
                    it.face.title().into(),
                    format!("{:.0} / {:.0}", it.x, it.y),
                    size,
                    length,
                ],
                target,
            );
        }
    }
    t
}

#[cfg(test)]
mod tests;
