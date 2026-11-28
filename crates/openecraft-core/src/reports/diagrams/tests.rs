use super::*;
use crate::geom::p;
use crate::analysis::Analysis;
use crate::sample::demo_project;

fn rows<'a>(t: &'a Table, first: &str) -> Vec<&'a Vec<String>> {
    t.rows.iter().filter(|r| r[0] == first).collect()
}

#[test]
fn terminal_connections_list_both_sides() {
    let prj = demo_project();
    let an = Analysis::checked(&prj);
    let t = &an.report(&prj, ReportKind::TerminalConnections).unwrap();
    let x1 = rows(t, "-X1");
    // Terminal 1: -K1 inside the cabinet, the motor through cable -W1 outside.
    let one: Vec<(&str, &str, &str)> = x1.iter().filter(|r| r[1] == "1").map(|r| (r[2].as_str(), r[3].as_str(), r[5].as_str())).collect();
    assert!(one.contains(&("External", "-M1:U1", "-W1")) || one.contains(&("Internal", "-M1:U1", "-W1")), "{one:?}");
    assert!(one.iter().any(|r| r.1.starts_with("-K1")), "{one:?}");
    assert!(x1.iter().all(|r| r.len() == t.headers.len()));
    assert!(t.targets.iter().all(|t| t.is_some()), "every row jumps to its terminal");
}

#[test]
fn cable_connections_show_every_core() {
    let prj = demo_project();
    let an = Analysis::checked(&prj);
    let t = &an.report(&prj, ReportKind::CableConnections).unwrap();
    let w1 = rows(t, "-W1");
    let cores: Vec<(&str, &str, &str)> = w1.iter().map(|r| (r[3].as_str(), r[4].as_str(), r[5].as_str())).collect();
    assert_eq!(cores, vec![("-X1:1", "BN", "-M1:U1"), ("-X1:2", "BK", "-M1:V1"), ("-X1:3", "GY", "-M1:W1"), ("-X1:PE", "GNYE", "-M1:PE")]);
    assert!(w1.iter().all(|r| r[2] == "15"), "cable length");
}

#[test]
fn device_connections_list_every_connection_point() {
    let prj = demo_project();
    let an = Analysis::checked(&prj);
    let t = &an.report(&prj, ReportKind::DeviceConnections).unwrap();
    let k1 = rows(t, "-K1");
    for point in ["A1", "A2", "13", "14", "53", "54", "83", "84"] {
        assert!(k1.iter().any(|r| r[1] == point), "{point} missing: {k1:?}");
    }
    assert!(!t.rows.iter().any(|r| r[0] == "-X1"), "no terminals");
    // Every connection appears once at each of its device ends.
    let device_ends = an.connections.iter().flat_map(|c| [c.a, c.b]).filter(|p| {
        let s = &prj.pages[p.page].symbols[p.sym];
        !matches!(prj.symbol(&s.sym).map(|d| d.func), Some(Func::Terminal | Func::Plug(_)))
    });
    assert_eq!(t.rows.iter().filter(|r| !r[3].is_empty()).count(), device_ends.count());
    assert!(t.rows.iter().all(|r| r.len() == t.headers.len()));
}

#[test]
fn pin_connections_follow_the_plug_sides() {
    let mut prj = Project::default();
    for (key, x, y, tag, label) in [("pot_24", 100.0, 76.0, "", "+24V"), ("plug_pair", 100.0, 100.0, "-X10", "1"), ("coil", 100.0, 120.0, "-K1", "")] {
        let def = prj.symbol(key).unwrap();
        let id = prj.new_id();
        let pins = def.pins.iter().map(|x| x.label.to_string()).collect();
        prj.pages[0].symbols.push(SymbolInst { id, sym: key.into(), pos: p(x, y), tag: tag.into(), label: label.into(), pins, ..Default::default() });
    }
    for (y1, y2) in [(80.0, 96.0), (104.0, 116.0)] {
        let id = prj.new_id();
        prj.pages[0].wires.push(Wire { id, a: p(100.0, y1), b: p(100.0, y2), ..Default::default() });
    }
    let an = Analysis::checked(&prj);
    let t = &an.report(&prj, ReportKind::PinConnections).unwrap();
    let x10 = rows(t, "-X10");
    assert_eq!(x10.len(), 1, "{x10:?}");
    assert_eq!((x10[0][1].as_str(), x10[0][3].as_str()), ("1", "-K1:A1"));
    assert!(["Socket side", "Pin side"].contains(&x10[0][2].as_str()));
}

#[test]
fn terminal_lineup_and_strip_overview() {
    let prj = demo_project();
    let an = Analysis::checked(&prj);
    let lineup_table = an.report(&prj, ReportKind::TerminalLineup).unwrap();
    let lineup = rows(&lineup_table, "-X1");
    let labels: Vec<&str> = lineup.iter().filter(|r| r[2] == "Terminal").map(|r| r[3].as_str()).collect();
    assert_eq!(labels, vec!["1", "2", "3", "PE"]);
    // Positions follow one another along the rail.
    let width: f32 = lineup.iter().map(|r| r[6].parse::<f32>().unwrap()).sum();
    assert!(lineup.last().unwrap()[7].ends_with(&format!("{width:.1}")), "{lineup:?}");

    let overview_table = an.report(&prj, ReportKind::TerminalStripOverview).unwrap();
    let overview = rows(&overview_table, "-X1");
    assert_eq!(overview.len(), 1);
    assert_eq!((overview[0][3].as_str(), overview[0][5].as_str()), ("4", "4"));
    assert_eq!(overview[0][7], format!("{width:.1}"));
}

#[test]
fn panel_legend_numbers_the_devices() {
    let prj = demo_project();
    let an = Analysis::checked(&prj);
    let t = &an.report(&prj, ReportKind::PanelLegend).unwrap();
    let cab = rows(t, "+CAB1");
    let items = &prj.layout.enclosures[0].items;
    let placed = items.iter().filter(|i| i.kind == ItemKind::Device).count();
    let profiles = items.iter().filter(|i| i.kind.is_profile()).count();
    assert!(placed > 0 && profiles > 0);
    assert_eq!(cab.len(), placed + profiles);
    let numbers: Vec<String> = cab.iter().map(|r| r[1].clone()).collect();
    assert_eq!(numbers, (1..=placed + profiles).map(|n| n.to_string()).collect::<Vec<_>>());
    assert!(cab.iter().any(|r| r[2] == "-K1" && !r[4].is_empty() && r[8].is_empty()));
}

#[test]
fn panel_legend_lists_profiles_with_section_and_cut_length() {
    let prj = demo_project();
    let an = Analysis::checked(&prj);
    let t = &an.report(&prj, ReportKind::PanelLegend).unwrap();
    let cab = rows(t, "+CAB1");
    // After the devices: rails, then ducts, each with its part, cross-section and length.
    let first = cab.iter().position(|r| r[2].is_empty()).expect("profiles");
    assert!(cab[first..].iter().all(|r| r[2].is_empty() && !r[8].is_empty()));
    let duct = prj.layout.enclosures[0].items.iter().find(|i| i.kind == ItemKind::Duct).unwrap();
    let row = cab.iter().find(|r| r[4] == duct.part && r[8] == crate::panel::mm_text(duct.length())).expect("duct row");
    assert_eq!(row[7], format!("{} × {}", crate::panel::mm_text(duct.w.min(duct.h)), crate::panel::mm_text(duct.d)));
    let kinds: Vec<ItemKind> = cab[first..].iter().map(|r| prj.layout.enclosures[0].items.iter().find(|i| i.kind.is_profile() && i.part == r[4]).unwrap().kind).collect();
    assert!(kinds.windows(2).all(|w| w[0].layer() <= w[1].layer()), "{kinds:?}");
}

#[test]
fn terminal_wiring_names_the_wired_ends_of_both_sides() {
    let mut prj = demo_project();
    let data = prj.strips.entry("-X1".into()).or_default();
    data.jumpers.insert("1".into(), 1);
    data.jumpers.insert("3".into(), 1);
    let an = Analysis::checked(&prj);
    let t = &an.report(&prj, ReportKind::TerminalWiring).unwrap();
    assert_eq!(t.headers.len(), 15);
    assert!(t.rows.iter().all(|r| r.len() == t.headers.len()));
    let x1 = rows(t, "-X1");
    // One terminal after the other in the order of the strip, no accessories.
    let labels: Vec<&str> = x1.iter().map(|r| r[6].as_str()).filter(|l| !l.is_empty()).collect();
    assert_eq!(labels, ["1", "2", "3", "PE"]);
    // Terminal 1: the motor through cable -W1 on one side, -K1 on the other; only the
    // device wired to it, not every device of the potential.
    let one = x1.iter().find(|r| r[6] == "1").unwrap();
    let sides = [(&one[2], &one[3], &one[5]), (&one[10], &one[11], &one[13])];
    assert!(sides.iter().any(|s| s.0 == "-M1" && s.1 == "U1" && s.2.starts_with("-W1")), "{one:?}");
    assert!(sides.iter().any(|s| s.0.starts_with("-K1")), "{one:?}");
    assert!(!one[2].contains(',') && !one[10].contains(','), "{one:?}");
    assert!(!one[4].is_empty() || !one[12].is_empty(), "the location of a target: {one:?}");
    assert!(!one[9].is_empty(), "the potential: {one:?}");
    assert_eq!(x1.iter().find(|r| r[6] == "PE").unwrap()[14], "PE", "the conductor of the PE terminal");
    // The jumper with the terminals it spans, on each of its terminals.
    assert_eq!(one[8], "1 (1 – 3)");
    assert_eq!(x1.iter().find(|r| r[6] == "3").unwrap()[8], "1 (1 – 3)");
    assert_eq!(x1.iter().find(|r| r[6] == "2").unwrap()[8], "");
    assert!(t.targets.iter().all(|t| t.is_some()), "every row jumps to its terminal");
}

/// Two wires at one side of a terminal: the second continues in the next row.
#[test]
fn terminal_wiring_continues_a_side_with_several_wires() {
    use crate::testkit::Kit;
    let mut k = Kit::new();
    k.sym("term", 100.0, 100.0, "-X2", "1");
    for (tag, x, y) in [("-E1", 60.0, 140.0), ("-E2", 140.0, 140.0), ("-E3", 100.0, 40.0)] {
        k.sym("lamp", x, y, tag, "");
    }
    // Connection point `i` of the symbol of `tag`.
    let at = |prj: &Project, tag: &str, i: usize| {
        let s = prj.pages[0].symbols.iter().find(|s| s.tag == tag).unwrap();
        prj.symbol(&s.sym).unwrap().pin_world(i, s.pos, s.rot, s.mirror)
    };
    // The bottom (internal) side to -E1 and -E2, the top one to -E3.
    for (a, b) in [(at(&k.prj, "-X2", 1), at(&k.prj, "-E1", 0)), (at(&k.prj, "-X2", 1), at(&k.prj, "-E2", 0)), (at(&k.prj, "-X2", 0), at(&k.prj, "-E3", 1))] {
        let id = k.prj.new_id();
        k.prj.pages[0].wires.push(Wire { id, a, b, ..Default::default() });
    }
    let an = Analysis::checked(&k.prj);
    let t = &an.report(&k.prj, ReportKind::TerminalWiring).unwrap();
    let x2 = rows(t, "-X2");
    assert_eq!(x2.len(), 2, "{x2:?}");
    assert_eq!(x2[0][6], "1");
    assert_eq!(x2[1][6], "", "the continuation has no terminal of its own");
    assert!(x2[1][9].is_empty(), "nor potential: {x2:?}");
    let mut internal: Vec<&str> = x2.iter().map(|r| r[10].as_str()).collect();
    internal.sort();
    assert_eq!(internal, ["-E1", "-E2"], "{x2:?}");
    assert_eq!((x2[0][2].as_str(), x2[1][2].as_str()), ("-E3", ""), "{x2:?}");
}

/// An N busbar interrupted between N2 and N3 (two busbars), a PE busbar, and a
/// wire jumper between the levels of a double-level terminal.
#[test]
fn terminal_wiring_shows_busbars_their_interruption_and_levels() {
    use crate::testkit::Kit;
    let mut k = Kit::new();
    for (i, n) in ["N1", "N2", "N3", "N4"].iter().enumerate() {
        k.sym("term_n", 100.0 + 10.0 * i as f32, 100.0, "-XN", n);
    }
    for (i, n) in ["PE1", "PE2"].iter().enumerate() {
        k.sym("term_pe", 100.0 + 10.0 * i as f32, 150.0, "-XPE", n);
    }
    for (i, n) in ["1", "2"].iter().enumerate() {
        k.sym("term", 100.0 + 10.0 * i as f32, 200.0, "-X3", n);
    }
    let bar = |k: &mut Kit, strip: &str, terminals: &[&str], number: u32| {
        let data = k.prj.strips.entry(strip.into()).or_default();
        for t in terminals {
            data.jumpers.insert((*t).into(), number);
        }
        data.jumper_kinds.insert(number, JumperKind::Busbar);
    };
    bar(&mut k, "-XN", &["N1", "N2"], 1);
    bar(&mut k, "-XN", &["N3", "N4"], 2);
    bar(&mut k, "-XPE", &["PE1", "PE2"], 1);
    let data = k.prj.strips.entry("-X3".into()).or_default();
    data.levels.insert("2".into(), 2);
    data.jumpers.insert("1".into(), 1);
    data.jumpers.insert("2".into(), 1);
    data.jumper_kinds.insert(1, JumperKind::Wire);
    let an = Analysis::checked(&k.prj);
    let t = &an.report(&k.prj, ReportKind::TerminalWiring).unwrap();
    let cell = |strip: &str, terminal: &str, col: usize| t.rows.iter().find(|r| r[0] == strip && r[6] == terminal).map(|r| r[col].clone()).unwrap_or_else(|| panic!("{strip}:{terminal} {:?}", t.rows));
    assert_eq!(cell("-XN", "N1", 8), "busbar 1 (N1 – N2)");
    assert_eq!(cell("-XN", "N2", 8), "busbar 1 (N1 – N2)");
    assert_eq!(cell("-XN", "N3", 8), "busbar 2 (N3 – N4), separated from 1");
    assert_eq!(cell("-XN", "N4", 8), "busbar 2 (N3 – N4)");
    // PE terminals on a busbar are fine, a plug-in jumper there is not.
    assert_eq!(cell("-XPE", "PE1", 8), "busbar 1 (PE1 – PE2)");
    assert!(crate::strips::strip_jumpers(&k.prj, "-XPE").iter().all(|j| j.problem.is_none()));
    // Both levels of the block, bridged by a wire.
    assert_eq!((cell("-X3", "1", 7), cell("-X3", "2", 7)), ("1".to_string(), "2".to_string()));
    assert_eq!(cell("-X3", "2", 8), "1 (1 – 2)");
}

/// The same potential drawn above and below a terminal is no target of the terminal
/// itself.
#[test]
fn terminal_wiring_leaves_out_the_terminal_itself() {
    use crate::testkit::Kit;
    let mut k = Kit::new();
    k.sym("term_pe", 100.0, 100.0, "-X5", "PE");
    let above = k.sym("pot_pe", 100.0, 80.0, "", "PE");
    let below = k.sym("pot_pe", 100.0, 120.0, "", "PE");
    let at = |prj: &Project, id: u64, i: usize| {
        let s = prj.pages[0].symbols.iter().find(|s| s.id == id).unwrap();
        prj.symbol(&s.sym).unwrap().pin_world(i, s.pos, s.rot, s.mirror)
    };
    let x5 = k.prj.pages[0].symbols.iter().find(|s| s.tag == "-X5").unwrap().id;
    for (a, b) in [(at(&k.prj, x5, 0), at(&k.prj, above, 0)), (at(&k.prj, x5, 1), at(&k.prj, below, 0))] {
        let id = k.prj.new_id();
        k.prj.pages[0].wires.push(Wire { id, a, b, ..Default::default() });
    }
    let an = Analysis::checked(&k.prj);
    let t = &an.report(&k.prj, ReportKind::TerminalWiring).unwrap();
    let x5 = rows(t, "-X5");
    assert_eq!(x5.len(), 1, "{x5:?}");
    assert!(x5[0][2] != "-X5" && x5[0][10] != "-X5", "{x5:?}");
}
