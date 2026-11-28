//! Reports of the electrical calculations: short circuit, selectivity, loads, power
//! supply budgets, safety functions and the rule set verification.

use super::Table;
use crate::model::*;
use crate::netlist::{Connection, Netlist};
use openecraft_i18n::{tr, tr_const, tr_fmt};

fn num(v: f32, digits: usize) -> String {
    if v.is_finite() {
        format!("{v:.digits$}")
    } else {
        "–".into()
    }
}

fn ok(good: bool) -> String {
    if good { tr("OK") } else { tr("Not OK") }.to_string()
}

pub(super) fn short_circuit(prj: &Project, nl: &Netlist, conns: &[Connection]) -> Table {
    let mut t = Table::with_widths(&[
        (tr_const!("Device"), 0.8),
        (tr_const!("Characteristic"), 0.7),
        (tr_const!("Rated current (A)"), 0.7),
        (tr_const!("Breaking capacity (kA)"), 0.8),
        (tr_const!("Ik max (kA)"), 0.7),
        (tr_const!("ip (kA)"), 0.6),
        (tr_const!("Cable"), 0.7),
        (tr_const!("Ik1 min (A)"), 0.7),
        (tr_const!("Disconnection (s)"), 0.8),
        (tr_const!("Permitted (s)"), 0.7),
        (tr_const!("Result"), 0.6),
    ]);
    for f in crate::shortcircuit::faults(prj, nl, conns) {
        let p = &f.protection;
        let disconnection = if f.residual.is_some() { tr_fmt!("RCD {} mA", f.residual.unwrap_or(0.0)) } else { num(f.time, 2) };
        let breaking = match (&f.backup, p.breaking) {
            (Some(up), _) => tr_fmt!("back-up {}", up),
            (None, Some(b)) => num(b, 1),
            _ => "–".into(),
        };
        let cable = if f.feeders.is_empty() { f.cable.clone() } else { format!("{}, {}", f.feeders.join(", "), f.cable) };
        t.push(
            vec![
                f.device.clone(),
                p.characteristic.title().into(),
                num(p.rated, 1),
                breaking,
                num(f.ik_max / 1000.0, 2),
                num(f.ip / 1000.0, 2),
                cable,
                num(f.ik_min_end, 0),
                disconnection,
                f.limit.map(|l| num(l, 2)).unwrap_or_else(|| "–".into()),
                ok(f.is_ok()),
            ],
            Some(f.target),
        );
    }
    t
}

pub(super) fn selectivity(prj: &Project, nl: &Netlist, conns: &[Connection]) -> Table {
    let mut t = Table::with_widths(&[
        (tr_const!("Upstream"), 0.8),
        (tr_const!("Downstream"), 0.8),
        (tr_const!("Selective up to (A)"), 0.9),
        (tr_const!("Ik max (A)"), 0.8),
        (tr_const!("Selectivity"), 0.8),
        (tr_const!("Basis"), 1.0),
    ]);
    for s in crate::shortcircuit::selectivity(prj, nl, conns) {
        let grade = if s.is_total() { tr("Total") } else { tr("Partial") };
        t.push(vec![s.upstream.clone(), s.downstream.clone(), num(s.limit, 0), num(s.ik, 0), grade.into(), s.basis.title().into()], Some(s.target));
    }
    t
}

pub(super) fn load_calculation(prj: &Project, nl: &Netlist) -> Table {
    let mut t = Table::with_widths(&[
        (tr_const!("Device"), 0.8),
        (tr_const!("Kind"), 1.0),
        (tr_const!("Protective device"), 0.8),
        (tr_const!("Phases"), 0.7),
        (tr_const!("Power (kW)"), 0.6),
        (tr_const!("Apparent power (kVA)"), 0.8),
        (tr_const!("Current (A)"), 0.6),
    ]);
    let c = crate::loads::calculate(prj, nl);
    for l in &c.loads {
        let phases = l.phases.iter().cloned().collect::<Vec<_>>().join(" ");
        t.push(vec![l.device.clone(), l.kind.title().into(), l.feeder.clone(), phases, num(l.power, 2), num(l.apparent, 2), num(l.current, 2)], Some(l.target));
    }
    for f in &c.feeders {
        t.push(vec![f.device.clone(), tr_fmt!("Loads behind it: {}", f.loads), String::new(), String::new(), String::new(), num(f.rated, 1), num(f.current, 2)], Some(f.target));
    }
    let mut row = |label: String, kw: String, kva: String, a: String| t.push(vec![label, String::new(), String::new(), String::new(), kw, kva, a], None);
    row(tr("Installed").into(), num(c.installed.0, 2), num(c.installed.1, 2), String::new());
    row(tr_fmt!("Demand (diversity factor {})", num(c.diversity, 2)), String::new(), num(c.demand, 2), num(c.demand_current, 1));
    for (p, i) in &c.phases {
        row(tr_fmt!("Phase {}", p), String::new(), String::new(), num(*i, 2));
    }
    if c.phases.len() >= 3 {
        row(tr_fmt!("Unbalance {} %", num(c.unbalance, 0)), String::new(), String::new(), String::new());
    }
    t
}

pub(super) fn power_supply_budget(prj: &Project, nl: &Netlist) -> Table {
    let mut t = Table::with_widths(&[(tr_const!("Power supply"), 0.8), (tr_const!("Device"), 0.8), (tr_const!("Current (A)"), 0.6), (tr_const!("Rated current (A)"), 0.7), (tr_const!("Utilisation (%)"), 0.7)]);
    for b in crate::loads::calculate(prj, nl).budgets {
        for (tag, i) in &b.loads {
            t.push(vec![b.device.clone(), tag.clone(), num(*i, 3), String::new(), String::new()], Some(b.target));
        }
        for tag in &b.without_data {
            t.push(vec![b.device.clone(), tag.clone(), "?".into(), String::new(), String::new()], Some(b.target));
        }
        t.push(vec![b.device.clone(), tr("Total").into(), num(b.current, 2), num(b.rated, 1), num(b.current / b.rated.max(0.01) * 100.0, 0)], Some(b.target));
    }
    t
}

pub(super) fn safety_functions(prj: &Project) -> Table {
    let mut t = Table::with_widths(&[
        (tr_const!("Safety function"), 1.0),
        (tr_const!("Subsystem"), 0.9),
        (tr_const!("Category"), 0.5),
        (tr_const!("DCavg (%)"), 0.5),
        (tr_const!("MTTFd (years)"), 0.6),
        (tr_const!("PFHd (1/h)"), 0.6),
        (tr_const!("PL"), 0.6),
        (tr_const!("Required"), 0.6),
        (tr_const!("Result"), 0.5),
    ]);
    let pfh = |p: Option<f32>| p.map(|p| format!("{p:.2e}")).unwrap_or_else(|| "–".into());
    for f in &prj.electrical.safety {
        let e = crate::safety::evaluate(prj, f);
        for (s, r) in f.subsystems.iter().zip(&e.subsystems) {
            t.push(
                vec![
                    f.name.clone(),
                    s.name.clone(),
                    s.category.title().into(),
                    num(r.dc, 0),
                    r.mttfd.map(|m| num(m, 1)).unwrap_or_else(|| "–".into()),
                    pfh(r.pfh),
                    crate::safety::pl_text(r.pl),
                    String::new(),
                    String::new(),
                ],
                None,
            );
        }
        t.push(
            vec![f.name.clone(), tr("Safety function").into(), String::new(), String::new(), String::new(), pfh(e.pfh), crate::safety::pl_text(e.achieved), crate::safety::pl_text(Some(f.required)), ok(e.is_met())],
            None,
        );
    }
    t
}

pub(super) fn verification(prj: &Project, conns: &[Connection]) -> Table {
    let mut t = Table::with_widths(&[(tr_const!("Rule set"), 0.8), (tr_const!("Rule"), 2.0), (tr_const!("Checked"), 0.5), (tr_const!("Violations"), 0.5), (tr_const!("Items"), 1.6)]);
    for r in crate::rules::verify(prj, conns) {
        let mut items: Vec<String> = r.failed.iter().take(8).map(|i| i.label.clone()).collect();
        if r.failed.len() > 8 {
            items.push("…".into());
        }
        let target = r.failed.iter().find_map(|i| i.target.filter(|t| t.1 != 0));
        t.push(vec![r.set.clone(), r.check.message.clone(), r.checked.to_string(), r.failed.len().to_string(), items.join(", ")], target);
    }
    t
}

/// The distribution board of protective device `tag`: the cabinet it is placed in, else
/// the location of its tag (empty when it has none).
fn board_name(prj: &Project, tag: &str) -> String {
    match crate::installation::board_of(prj, tag) {
        Some(ei) => prj.layout.enclosures[ei].name.clone(),
        None => {
            let loc = crate::structure::Dt::parse(tag).location;
            if loc.is_empty() { loc } else { format!("+{loc}") }
        }
    }
}

/// The cables leaving the load side (even designations) of the symbols of device `tag`,
/// with type or cross-section, in schematic order; those of its circuit that leave the
/// board (start at no device of the circuit's rooms, e.g. at its terminals) when it
/// has any, the cross-section of its wires otherwise.
pub(super) fn outgoing_cables(prj: &Project, conns: &[Connection], tag: &str, circuit: Option<&crate::installation::Circuit>) -> Vec<String> {
    let of_circuit: Vec<String> = circuit
        .map(|c| c.legs.iter().filter(|l| !l.cable.is_empty() && c.device(&l.from).is_none()).map(|l| format!("{} {}", l.cable, l.cable_type).trim().to_string()).collect())
        .unwrap_or_default();
    let mut out: Vec<String> = Vec::new();
    for c in of_circuit {
        if !out.contains(&c) {
            out.push(c);
        }
    }
    if !out.is_empty() {
        return out;
    }
    for c in conns {
        for (at, other) in [(c.a, c.b), (c.b, c.a)] {
            let s = &prj.pages[at.page].symbols[at.sym];
            if s.tag != tag || prj.pages[other.page].symbols[other.sym].tag == tag || !crate::sizing::designation_number(s, at.pin).is_some_and(|d| d % 2 == 0) {
                continue;
            }
            let text = match prj.cables.iter().find(|x| !c.cable.is_empty() && x.tag == c.cable) {
                Some(x) if !x.cable_type.is_empty() => format!("{} {}", x.tag, x.cable_type),
                Some(x) => format!("{} {}", x.tag, x.cross_section).trim().to_string(),
                None if !c.cable.is_empty() => c.cable.clone(),
                None => c.cross_section.clone(),
            };
            if !text.is_empty() && !out.contains(&text) {
                out.push(text);
            }
        }
    }
    out
}

/// The type of a protective device: characteristic and rated current ("B 16 A"), else
/// the rated current of its part.
fn protection_type(prj: &Project, tag: &str) -> String {
    match crate::protection::of_device(prj, tag) {
        Some(p) => format!("{} {} A", p.characteristic.title(), num(p.rated, if p.rated.fract() == 0.0 { 0 } else { 1 })),
        None => crate::sizing::device_parts(prj, tag).iter().map(|p| p.current.trim()).find(|c| !c.is_empty()).unwrap_or("").to_string(),
    }
}

/// `text` holds `word` as a whole word, ignoring case.
fn has_word(text: &str, word: &str) -> bool {
    let lower = text.to_lowercase();
    lower.split(|c: char| !c.is_alphanumeric()).any(|w| w == word)
}

/// Device `tag` detects arc faults: the property *Arc fault detection (AFDD)*, or a part
/// whose description or tech data names it (`AFDD`, `arc fault`).
fn is_afdd(prj: &Project, tag: &str) -> bool {
    let obj = crate::properties::Object::Device(tag.into());
    if let Some(v) = crate::properties::stored(prj, &obj, "device.afdd") {
        return v == "true";
    }
    crate::sizing::device_parts(prj, tag).iter().any(|p| {
        let text = format!("{} {}", p.description, p.tech_data);
        has_word(&text, "afdd") || text.to_lowercase().contains("arc fault")
    })
}

/// Whether the property *Board overview* of device `tag` shows (`Some(true)`) or hides
/// (`Some(false)`) it; `None`: as its kind says.
fn listed(prj: &Project, tag: &str) -> Option<bool> {
    match crate::properties::stored(prj, &crate::properties::Object::Device(tag.into()), "device.board_overview")? {
        "Show" => Some(true),
        "Hide" => Some(false),
        _ => None,
    }
}

/// What a device of the board overview is.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Role {
    /// A protective device fed from the mains.
    Protective,
    Rcd,
    Spd,
    /// A switch on the mains in a distribution board (main switch, switch disconnector).
    Switch,
    /// Any other device shown by its property.
    Other,
}

/// The kind of a device for the board overview: circuit breaker, fuse, RCBO, AFDD …
fn kind_text(prj: &Project, tag: &str, role: Role, sym: &str) -> String {
    use crate::parts::FnKind as F;
    let kinds: Vec<F> = crate::sizing::device_parts(prj, tag).iter().flat_map(|p| p.functions.iter().map(|f| f.kind)).collect();
    let afdd = is_afdd(prj, tag);
    let text = match role {
        Role::Protective if afdd => tr("Arc fault detection device"),
        Role::Protective if kinds.contains(&F::ResidualCurrent) => tr("RCBO"),
        Role::Protective => match crate::parts::kind_of_symbol(sym) {
            Some(F::Fuse | F::Fuse3) => tr("Fuse"),
            Some(F::MotorProtection) => tr("Motor protection switch"),
            _ => tr("Circuit breaker"),
        },
        Role::Rcd if afdd => tr("Arc fault detection device"),
        Role::Rcd if kinds.iter().any(|k| matches!(k, F::Breaker1 | F::Breaker2 | F::Breaker3 | F::Breaker4)) => tr("RCBO"),
        Role::Rcd => tr("Residual current device"),
        Role::Spd => tr("Surge protective device"),
        Role::Switch if crate::parts::kind_of_symbol(sym) == Some(F::Disconnector3) => tr("Main switch"),
        Role::Switch => tr("Switch"),
        Role::Other => match kinds.first() {
            Some(k) => tr(k.title()),
            None => prj.symbol(sym).map(|d| tr(d.name)).unwrap_or(""),
        },
    };
    text.to_string()
}

/// A device of the board overview.
struct Item {
    page: usize,
    pos: crate::geom::P,
    tag: String,
    sym: String,
    id: u64,
    role: Role,
    board: String,
    /// Nets it is fed from and nets it feeds (protected nets of a protective device).
    line: std::collections::BTreeSet<usize>,
    load: std::collections::BTreeSet<usize>,
}

/// The nets of the line side (odd designations; every pin without designations) and of
/// the load side (even designations) of the schematic symbols of `tag`.
fn sides_of(prj: &Project, nl: &Netlist, tag: &str) -> (std::collections::BTreeSet<usize>, std::collections::BTreeSet<usize>) {
    let (mut line, mut load) = (std::collections::BTreeSet::new(), std::collections::BTreeSet::new());
    for s in prj.pages.iter().filter(|p| p.kind == PageKind::Schematic).flat_map(|p| p.symbols.iter()).filter(|s| s.tag == tag) {
        for pin in 0..s.pins.len() {
            let Some(&n) = nl.pin_net.get(&(s.id, pin)) else { continue };
            match crate::sizing::designation_number(s, pin) {
                Some(d) if d % 2 == 0 => load.insert(n),
                _ => line.insert(n),
            };
        }
    }
    (line, load)
}

/// What a device feeds, as a text for one without a function text: the devices of its
/// circuit by kind (`Socket outlet -XD1, -XD2`), else the other devices on the nets it
/// feeds with their function texts (up to four).
fn fed_text(prj: &Project, nl: &Netlist, circuit: Option<&crate::installation::Circuit>, item: &Item, boards: &[Item]) -> String {
    if let Some(c) = circuit.filter(|c| !c.devices.is_empty()) {
        let mut by_kind: Vec<(crate::installation::Kind, Vec<&str>)> = Vec::new();
        for d in &c.devices {
            match by_kind.iter_mut().find(|(k, _)| *k == d.kind) {
                Some((_, tags)) => tags.push(&d.tag),
                None => by_kind.push((d.kind, vec![&d.tag])),
            }
        }
        return by_kind.iter().map(|(k, tags)| format!("{} {}", k.title(), tags.join(", "))).collect::<Vec<_>>().join("; ");
    }
    let mut tags: Vec<String> = Vec::new();
    for &n in &item.load {
        let Some(net) = nl.nets.get(n) else { continue };
        for pr in &net.pins {
            let Some(s) = prj.pages.get(pr.page).filter(|pg| pg.kind == PageKind::Schematic).and_then(|pg| pg.symbols.get(pr.sym)) else { continue };
            let terminal = prj.symbol(&s.sym).is_some_and(|d| matches!(d.func, crate::symbols::Func::Terminal | crate::symbols::Func::Potential | crate::symbols::Func::Interruption));
            if s.tag.is_empty() || s.tag == item.tag || terminal || boards.iter().any(|b| b.tag == s.tag) {
                continue;
            }
            let text = crate::installation::text_of(prj, &s.tag).replace('\n', " ");
            let entry = format!("{} {}", s.tag, text).trim().to_string();
            if !tags.contains(&entry) {
                tags.push(entry);
            }
        }
    }
    if tags.len() > 4 {
        tags.truncate(4);
        tags.push("…".into());
    }
    tags.join(", ")
}

/// Overview of the distribution boards: every protective device fed from the mains,
/// every residual current and surge protective device, the switches on the mains in the
/// boards and every device shown by its property *Board overview* (and none it hides),
/// board by board (the cabinet a device is placed in, or the location of its tag) as a
/// tree: each device below the one it is fed from in the same board (a circuit breaker
/// below its residual current device), in the order of the schematic. Kind, type,
/// phases, the residual current device protecting it, function text (else what it
/// feeds), the rooms of its circuit, the cables leaving the board and where it is drawn.
pub(super) fn board_overview(prj: &Project, an: &crate::analysis::Analysis) -> Table {
    use crate::parts::FnKind as F;
    let mut t = Table::with_widths(&[
        (tr_const!("Distribution board"), 0.7),
        (tr_const!("Device"), 0.9),
        (tr_const!("Kind"), 1.0),
        (tr_const!("Type"), 0.8),
        (tr_const!("Phases"), 0.6),
        (tr_const!("RCD"), 0.8),
        (tr_const!("Function text"), 1.6),
        (tr_const!("Rooms"), 1.0),
        (tr_const!("Cable"), 1.3),
        (tr_const!("Location"), 0.6),
    ]);
    t.tag_columns = vec![1, 5];
    t.tree_column = Some(1);
    let (nl, conns) = (&an.netlist, &an.connections);
    let feeds = crate::electrical::feeds(prj, nl);
    let spare: Vec<String> = crate::installation::breakers(prj, nl).into_iter().filter(|b| b.spare).map(|b| b.tag).collect();
    let circuits = crate::installation::circuits(prj, nl, conns, None);
    let mains = crate::electrical::mains(prj, nl);
    let on_mains = |s: &SymbolInst| (0..s.pins.len()).any(|pin| nl.pin_net.get(&(s.id, pin)).is_some_and(|n| mains.contains_key(n)));
    let mut items: Vec<Item> = Vec::new();
    // Switches count only in a board of the other devices (not those of the rooms).
    let mut switches: Vec<Item> = Vec::new();
    for (pi, pg) in prj.pages.iter().enumerate().filter(|(_, p)| p.kind == PageKind::Schematic) {
        for s in &pg.symbols {
            if s.tag.is_empty() || items.iter().chain(&switches).any(|i| i.tag == s.tag) || listed(prj, &s.tag) == Some(false) {
                continue;
            }
            let kind = crate::parts::kind_of_symbol(&s.sym);
            let feed = feeds.iter().find(|f| f.device == s.tag && f.symbol == s.id);
            let role = if feed.is_some() {
                Role::Protective
            } else if kind == Some(F::ResidualCurrent) {
                Role::Rcd
            } else if kind == Some(F::SurgeProtection) {
                Role::Spd
            } else if matches!(kind, Some(F::Disconnector3 | F::SwitchNo | F::SwitchNc)) && on_mains(s) {
                Role::Switch
            } else if listed(prj, &s.tag) == Some(true) {
                Role::Other
            } else {
                continue;
            };
            let (mut line, mut load) = sides_of(prj, nl, &s.tag);
            if let Some(f) = feed {
                line = f.line.clone();
                load = f.nets.clone();
            }
            if role == Role::Spd {
                line.append(&mut load);
            }
            let item = Item { page: pi, pos: s.pos, tag: s.tag.clone(), sym: s.sym.clone(), id: s.id, role, board: board_name(prj, &s.tag), line, load };
            if role == Role::Switch && listed(prj, &s.tag) != Some(true) { switches.push(item) } else { items.push(item) }
        }
    }
    let known: Vec<String> = items.iter().map(|i| i.board.clone()).collect();
    items.extend(switches.into_iter().filter(|i| crate::installation::board_of(prj, &i.tag).is_some() || known.contains(&i.board)));
    items.sort_by(|a, b| (a.page, a.pos.x, a.pos.y).partial_cmp(&(b.page, b.pos.x, b.pos.y)).unwrap_or(std::cmp::Ordering::Equal));
    // The device each one is fed from in its board: the nearest (fewest nets fed) whose
    // load side holds a net of its line side.
    let parent: Vec<Option<usize>> = (0..items.len())
        .map(|i| {
            (0..items.len())
                .filter(|&j| j != i && items[j].board == items[i].board && items[j].role != Role::Spd && !items[j].load.is_disjoint(&items[i].line))
                .min_by_key(|&j| (items[j].load.len(), j))
        })
        .collect();
    // Tree order: the roots of each board in schematic order, every device followed by
    // those it feeds.
    let mut order: Vec<(usize, u8)> = Vec::new();
    let mut seen = vec![false; items.len()];
    fn visit(i: usize, depth: u8, parent: &[Option<usize>], seen: &mut [bool], order: &mut Vec<(usize, u8)>) {
        if std::mem::replace(&mut seen[i], true) {
            return;
        }
        order.push((i, depth));
        for c in (0..parent.len()).filter(|&c| parent[c] == Some(i)) {
            visit(c, depth.saturating_add(1), parent, seen, order);
        }
    }
    let mut boards: Vec<String> = Vec::new();
    for i in &items {
        if !boards.contains(&i.board) {
            boards.push(i.board.clone());
        }
    }
    for board in &boards {
        for i in (0..items.len()).filter(|&i| &items[i].board == board) {
            // Roots, and devices in a loop of feeds.
            let root = parent[i].is_none_or(|p| {
                let mut at = p;
                let mut steps = 0;
                while let Some(q) = parent[at] {
                    if q == i || steps > items.len() {
                        return true;
                    }
                    at = q;
                    steps += 1;
                }
                false
            });
            if root {
                visit(i, 0, &parent, &mut seen, &mut order);
            }
        }
        for i in (0..items.len()).filter(|&i| &items[i].board == board) {
            visit(i, 0, &parent, &mut seen, &mut order);
        }
    }
    let phases_of = |tag: &str| -> String {
        let mut v: Vec<&String> = Vec::new();
        for s in prj.pages.iter().filter(|p| p.kind == PageKind::Schematic).flat_map(|p| p.symbols.iter()).filter(|s| s.tag == tag) {
            for pin in 0..s.pins.len() {
                if let Some(ph) = nl.pin_net.get(&(s.id, pin)).and_then(|n| mains.get(n)) {
                    if !v.contains(&ph) {
                        v.push(ph);
                    }
                }
            }
        }
        v.sort();
        v.into_iter().cloned().collect::<Vec<_>>().join(", ")
    };
    for (i, depth) in order {
        let item = &items[i];
        let tag = &item.tag;
        let obj = crate::properties::Object::Device(tag.clone());
        let kind = match item.role {
            Role::Rcd => {
                let ma = crate::electrical::stated_residual_current(prj, tag).unwrap_or(30.0);
                format!("{} {}", protection_type(prj, tag), tr_fmt!("RCD {} mA", num(ma, 0))).trim().to_string()
            }
            Role::Spd => crate::properties::stored(prj, &obj, "device.spd_type").map(String::from).unwrap_or_else(|| protection_type(prj, tag)),
            _ => protection_type(prj, tag),
        };
        let protecting = match item.role {
            Role::Rcd => String::new(),
            _ => feeds.iter().find(|f| &f.device == tag).and_then(|f| f.rcd.clone()).unwrap_or_default(),
        };
        let circuit = circuits.iter().find(|c| &c.breaker == tag);
        let mut text = crate::installation::text_of(prj, tag).replace('\n', " ");
        if text.is_empty() && spare.contains(tag) {
            text = tr("Spare").to_string();
        } else if text.is_empty() && matches!(item.role, Role::Protective | Role::Switch | Role::Other) {
            text = fed_text(prj, nl, circuit, item, &items);
        }
        let mut rooms: Vec<&str> = circuit.map(|c| c.devices.iter().filter_map(|d| d.placed).map(|(ri, _)| prj.layout.enclosures[ri].name.as_str()).collect()).unwrap_or_default();
        rooms.sort();
        rooms.dedup();
        let cables = if item.role == Role::Rcd { Vec::new() } else { outgoing_cables(prj, conns, tag, circuit) };
        t.push_at(
            vec![
                item.board.clone(),
                tag.clone(),
                kind_text(prj, tag, item.role, &item.sym),
                kind,
                phases_of(tag),
                protecting,
                text,
                rooms.join(", "),
                cables.join(", "),
                super::loc_str(prj, item.page, item.pos),
            ],
            Some((item.page, item.id)),
            depth,
        );
    }
    t
}
