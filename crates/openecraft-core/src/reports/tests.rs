use super::*;
use crate::sample::demo_project;

#[test]
fn metered_cable_unknown_lengths_keep_partial_material_totals_uncertain() {
    use crate::parts::Part;
    let mut prj=Project::default();
    prj.marking.cable_material.clear();
    prj.parts.push(Part {number:"BULK".into(),quantity_unit:"m".into(),price:2.,..Default::default()});
    prj.parts.push(Part {number:"UNKNOWN".into(),quantity_unit:"m".into(),price:4.,..Default::default()});
    prj.parts.push(Part {number:"PIECE".into(),quantity_unit:"pcs".into(),price:3.,..Default::default()});
    prj.cables=vec![Cable {id:1,tag:"-W1".into(),part:"BULK".into(),length:2.5,..Default::default()},
        Cable {id:2,tag:"-W2".into(),part:"BULK".into(),..Default::default()},
        Cable {id:3,tag:"-W3".into(),part:"UNKNOWN".into(),..Default::default()},
        Cable {id:4,tag:"-W4".into(),part:"PIECE".into(),..Default::default()}];
    let table=bom(&prj,&Index::build(&prj),&[],&[]);
    let bulk=table.rows.iter().find(|r|r[3]=="BULK").unwrap();
    assert_eq!((bulk[1].as_str(),bulk[2].as_str(),bulk[9].as_str()),("2.50 + ?","m","5.00 + ?"));
    assert!(bulk[7].contains("-W1") && bulk[7].contains("-W2"));
    let unknown=table.rows.iter().find(|r|r[3]=="UNKNOWN").unwrap();
    assert_eq!((unknown[1].as_str(),unknown[9].as_str()),("?","?"));
    let piece=table.rows.iter().find(|r|r[3]=="PIECE").unwrap();
    assert_eq!((piece[1].as_str(),piece[9].as_str()),("1","3.00"));
    assert_eq!(table.rows.last().unwrap()[9],"8.00 + ?");
    prj.cables[1].length=1.5;prj.cables[2].length=3.;
    let table=bom(&prj,&Index::build(&prj),&[],&[]);
    assert_eq!(table.rows.iter().find(|r|r[3]=="BULK").unwrap()[1],"4");
    assert_eq!(table.rows.last().unwrap()[9],"23.00");
}

#[test]
fn bill_of_materials_sums_each_currency_apart() {
    use crate::parts::Part;
    let mut prj=Project::default();
    prj.marking.cable_material.clear();
    prj.parts.push(Part {number:"EUR-A".into(),price:8.7,..Default::default()});
    prj.parts.push(Part {number:"EUR-B".into(),price:0.16,..Default::default()});
    prj.parts.push(Part {number:"USD-A".into(),price:10.,currency:"USD".into(),..Default::default()});
    prj.parts.push(Part {number:"FREE".into(),currency:"CHF".into(),..Default::default()});
    prj.cables=["EUR-A","EUR-B","USD-A","FREE"].iter().enumerate()
        .map(|(i,p)|Cable {id:i as u64+1,tag:format!("-W{}",i+1),part:p.to_string(),..Default::default()}).collect();
    let table=bom(&prj,&Index::build(&prj),&[],&[]);
    let sums:Vec<&str>=table.rows.iter().filter(|r|r[6]=="Sum").map(|r|r[9].as_str()).collect();
    assert_eq!(sums,["8.86 EUR","10.00 USD"]);
    prj.cables.retain(|c|c.part!="USD-A");
    let table=bom(&prj,&Index::build(&prj),&[],&[]);
    let sums:Vec<&str>=table.rows.iter().filter(|r|r[6]=="Sum").map(|r|r[9].as_str()).collect();
    assert_eq!(sums,["8.86"]);
}

#[test]
fn metered_cable_length_check_names_unknown_cables_and_accepts_configured_lengths() {
    let mut prj=Project::default();
    prj.cables.push(Cable {id:1,tag:"-W1".into(),part:"GEN.CBL-4G1.5".into(),..Default::default()});
    let check=crate::checks::builtin().into_iter().find(|c|c.name=="wires").unwrap();
    let issues=(check.run)(&prj,&Analysis::build(&prj)).unwrap();
    assert!(issues.iter().any(|i|i.message().contains("-W1: cable length is missing") && i.severity==Severity::Warning));
    let table=bom(&prj,&Index::build(&prj),&[],&[]);
    assert_eq!(table.rows.iter().find(|r|r[3]=="GEN.CBL-4G1.5").unwrap()[1],"?");
    prj.cables[0].length=25.;
    assert!(!(check.run)(&prj,&Analysis::build(&prj)).unwrap().iter().any(|i|i.message().contains("length is missing")));
    let reopened:Project=serde_json::from_str(&serde_json::to_string(&prj).unwrap()).unwrap();
    assert_eq!(material_quantity(&reopened,"GEN.CBL-4G1.5"),25.);
}

fn material_quantity(prj:&Project,number:&str)->f32 {
    bom(prj,&Index::build(prj),&[],&[]).rows.iter().find(|r|r[3]==number).map(|r|r[1].parse().unwrap()).unwrap_or(0.)
}

#[test]
fn cabinet_only_device_material_is_independent_of_additional_articles() {
    use crate::parts::{Part,PartRef};use crate::panel::{Enclosure,PanelItem,ItemKind};
    let mut prj=Project::default();prj.layout.hardware=false;
    prj.parts.push(Part {number:"GLAND".into(),..Default::default()});
    prj.devices.entry("-S1".into()).or_default().parts.push(PartRef {number:"GLAND".into(),count:1,total:false});
    prj.layout.enclosures.push(Enclosure {no_implicit_plate:true,items:vec![PanelItem {id:1,kind:ItemKind::Device,
        tag:"-S1".into(),part:"GLAND".into(),single_part:true,w:20.,h:30.,..Default::default()}],..Default::default()});
    assert_eq!(material_quantity(&prj,"GLAND"),1.);
    prj.devices.get_mut("-S1").unwrap().parts.push(PartRef {number:"GLAND".into(),count:0,total:true});
    assert_eq!(material_quantity(&prj,"GLAND"),1.);
    prj.devices.get_mut("-S1").unwrap().parts.pop();assert_eq!(material_quantity(&prj,"GLAND"),1.);
    let mut duplicate=prj.layout.enclosures[0].items[0].clone();duplicate.id=2;
    prj.layout.enclosures[0].items.push(duplicate);assert_eq!(material_quantity(&prj,"GLAND"),1.,"duplicate views are one logical device");
}

#[test]
fn cabinet_terminals_and_new_schematic_terminals_share_one_primary_quantity() {
    let mut prj=demo_project();prj.layout.hardware=false;
    let number=crate::strips::terminal_part(&prj,"-X1",false).unwrap().number.clone();
    let terminal=crate::strips::terminals(&prj,"-X1").into_iter().find(|t|t.label=="1").unwrap();
    let mut symbol=prj.pages[terminal.page].symbols[terminal.sym].clone();
    let before=material_quantity(&prj,&number);
    let mut item=prj.layout.find_device("-X1").unwrap().1.clone();item.single_part=true;item.terminal="1".into();
    prj.layout.enclosures[0].items.retain(|i|i.tag!="-X1");prj.layout.enclosures[0].items.push(item.clone());
    assert_eq!(material_quantity(&prj,&number),before,"schematic and cabinet terminal must collapse");
    item.id=prj.new_id();item.terminal="99".into();prj.layout.enclosures[0].items.push(item.clone());
    item.id=prj.new_id();prj.layout.enclosures[0].items.push(item);
    assert_eq!(material_quantity(&prj,&number),before+1.,"one undrawn terminal despite duplicate physical views");
    symbol.id=prj.new_id();symbol.label="99".into();prj.pages[terminal.page].symbols.push(symbol);
    assert_eq!(material_quantity(&prj,&number),before+1.,"drawing the cabinet terminal must not add a second piece");
}

#[test]
fn excluded_physical_duplicates_keep_geometry_and_material_owner_counts() {
    use crate::parts::{Part,PartRef};use crate::panel::{Enclosure,PanelItem,ItemKind};
    let mut prj=Project::default();prj.layout.hardware=false;
    prj.parts.push(Part {number:"END".into(),..Default::default()});
    prj.devices.entry("-X1".into()).or_default().parts.push(PartRef {number:"END".into(),count:2,total:true});
    let child=prj.devices.entry("-X1_1".into()).or_default();
    child.parts.push(PartRef {number:"END".into(),count:1,total:false});
    child.values.insert("device.material_owner".into(),"-X1".into());
    child.values.insert("device.exclude_from_bom".into(),"true".into());
    prj.layout.enclosures.push(Enclosure {no_implicit_plate:true,items:vec![PanelItem {id:1,kind:ItemKind::Accessory,
        tag:"-X1_1".into(),owner_device:"-X1".into(),bom_excluded:true,part:"END".into(),w:20.,h:30.,..Default::default()}],..Default::default()});
    assert_eq!(material_quantity(&prj,"END"),2.);
    prj.devices.get_mut("-X1_1").unwrap().values.remove("device.exclude_from_bom");
    assert_eq!(material_quantity(&prj,"END"),3.,"placement exclusion must not suppress the logical assignment");
    prj.layout.enclosures[0].items[0].bom_excluded=false;
    assert_eq!(material_quantity(&prj,"END"),3.,"one physical primary must replace its logical primary");
    prj.devices.get_mut("-X1").unwrap().values.insert("device.exclude_from_bom".into(),"true".into());
    assert_eq!(material_quantity(&prj,"END"),1.);
    let reopened=Project::from_json(&prj.to_json()).unwrap();
    assert_eq!(reopened.layout.enclosures[0].items[0].owner_device,"-X1");
    assert_eq!(reopened.layout.enclosures[0].items[0].w,20.);
    assert_eq!(material_quantity(&reopened,"END"),1.);
}

#[test]
fn literal_terminal_extras_do_not_multiply_with_the_strip_and_new_terminals_still_count() {
    let mut prj=demo_project();prj.layout.hardware=false;
    let number=crate::strips::terminal_part(&prj,"-X1",false).unwrap().number.clone();
    let before=material_quantity(&prj,&number);
    prj.devices.get_mut("-X1").unwrap().parts.push(crate::parts::PartRef {number:number.clone(),count:2,total:true});
    assert_eq!(material_quantity(&prj,&number),before+2.);
    let terminal=crate::strips::terminals(&prj,"-X1").into_iter().find(|t|t.label=="1").unwrap();
    let mut symbol=prj.pages[terminal.page].symbols[terminal.sym].clone();symbol.id=prj.new_id();symbol.label="99".into();
    prj.pages[terminal.page].symbols.push(symbol);
    assert_eq!(material_quantity(&prj,&number),before+3.);
    let reopened=Project::from_json(&prj.to_json()).unwrap();
    assert!(reopened.devices["-X1"].parts.last().unwrap().total);
    assert_eq!(material_quantity(&reopened,&number),before+3.);
}

#[test]
fn physical_primary_articles_count_once_and_literal_only_devices_and_zero_quantities_are_retained() {
    use crate::parts::{Part,PartRef};use crate::panel::{Enclosure,EnclosureKind,PanelItem,ItemKind};
    let mut prj=Project::default();prj.layout.hardware=false;
    prj.parts.extend([Part {number:"BODY".into(),..Default::default()},Part {number:"PLATE".into(),..Default::default()},Part {number:"EXTRA".into(),..Default::default()},Part {number:"ZERO".into(),..Default::default()}]);
    for (tag,part) in [("-U1","BODY"),("-U2","PLATE")] {
        prj.devices.entry(tag.into()).or_default().parts.extend([PartRef {number:part.into(),count:1,total:false},PartRef {number:part.into(),count:3,total:true}]);
    }
    prj.devices.entry("-OnlyMaterial".into()).or_default().parts.push(PartRef {number:"EXTRA".into(),count:4,total:true});
    prj.devices.entry("-OnlyMaterial".into()).or_default().parts.push(PartRef {number:"ZERO".into(),count:0,total:true});
    prj.layout.enclosures.push(Enclosure {device_tag:"-U1".into(),part:"BODY".into(),no_implicit_plate:true,
        items:vec![PanelItem {id:1,kind:ItemKind::MountingPlate,tag:"-U2".into(),part:"PLATE".into(),w:300.,h:400.,d:2.,..Default::default()}],..Default::default()});
    assert_eq!(material_quantity(&prj,"BODY"),4.);assert_eq!(material_quantity(&prj,"PLATE"),4.);
    assert_eq!(material_quantity(&prj,"EXTRA"),4.);assert_eq!(material_quantity(&prj,"ZERO"),0.);
    prj.layout.enclosures[0].kind=EnclosureKind::Assembly;prj.layout.enclosures[0].part.clear();
    assert_eq!(material_quantity(&prj,"BODY"),4.,"unplaced primary plus literal extras remain counted when no body owns the article");
}

fn row<'a>(t: &'a Table, first: &str) -> Vec<&'a Vec<String>> {
    t.rows.iter().filter(|r| r[0] == first).collect()
}

#[test]
fn cable_reports_of_the_demo() {
    let prj = demo_project();
    let an = Analysis::checked(&prj);
    let overview = &an.report(&prj, ReportKind::CableOverview).unwrap();
    let w1 = row(overview, "-W1");
    assert_eq!(w1.len(), 1);
    assert_eq!(w1[0][2], "4/4");
    assert_eq!((w1[0][4].as_str(), w1[0][5].as_str(), w1[0][6].as_str()), ("15", "-X1", "-M1"));

    let diagram = &an.report(&prj, ReportKind::CableDiagram).unwrap();
    let cores: Vec<(&str, &str, &str)> = row(diagram, "-W1").iter().map(|r| (r[1].as_str(), r[2].as_str(), r[3].as_str())).collect();
    assert_eq!(cores, vec![("BN", "-X1:1", "-M1:U1"), ("BK", "-X1:2", "-M1:V1"), ("GY", "-X1:3", "-M1:W1"), ("GNYE", "-X1:PE", "-M1:PE")]);
    assert!(row(diagram, "-W1").iter().all(|r| !r[5].is_empty()), "wire numbers");

    // Connection list: wire numbers and cable cores, terminal sides apart.
    let conns = &an.report(&prj, ReportKind::ConnectionList).unwrap();
    let motor = conns.rows.iter().find(|r| r[3] == "-M1:U1").unwrap();
    assert_eq!((motor[2].as_str(), motor[7].as_str(), motor[8].as_str()), ("-X1:1", "-W1", "BN"));
    assert!(conns.rows.iter().all(|r| !r[0].is_empty()), "every connection is numbered");
    assert!(conns.rows.iter().all(|r| r[2] != r[3]), "no connection from a terminal to itself");

    // The cable is in the bill of materials by length.
    let bom = &an.report(&prj, ReportKind::BillOfMaterials).unwrap();
    let cable = bom.rows.iter().find(|r| r[3] == "GEN.CBL-4G1.5").unwrap();
    assert_eq!((cable[1].as_str(), cable[2].as_str(), cable[7].as_str()), ("15", "m", "-W1"));
}

fn messages(prj: &Project) -> Vec<String> {
    Analysis::checked(prj).issues.into_iter().map(|i| i.message()).collect()
}

fn find<'a>(msgs: &'a [String], part: &str) -> Option<&'a String> {
    msgs.iter().find(|m| m.contains(part))
}

#[test]
fn cable_assignments_are_checked() {
    let mut prj = demo_project();
    let page = prj.pages.iter().position(|p| p.name == "2").unwrap();
    let cabled: Vec<usize> = prj.pages[page].wires.iter().enumerate().filter(|(_, w)| w.cable == "-W1").map(|(i, _)| i).collect();
    // Same core on two connections, a core the cable does not have, an unknown cable.
    prj.pages[page].wires[cabled[1]].core = "BN".into();
    prj.pages[page].wires[cabled[2]].core = "RD".into();
    prj.pages[page].wires[cabled[3]].cable = "-W7".into();
    let msgs = messages(&prj);
    assert!(find(&msgs, "-W1: core BN used for 2 connections").is_some(), "{msgs:#?}");
    assert!(find(&msgs, "core RD is not a core of the cable").is_some(), "{msgs:#?}");
    assert!(find(&msgs, "unknown cable -W7").is_some(), "{msgs:#?}");
    prj.cables.push(prj.cables[0].clone());
    assert!(find(&messages(&prj), "Cable -W1 defined 2 times").is_some());
}

#[test]
fn wire_numbers_and_tags_are_checked() {
    let mut prj = demo_project();
    let open = Netlist::build(&prj, false);
    let bridged = Netlist::build(&prj, true);
    let numbered: Vec<&crate::netlist::Net> = open.nets.iter().filter(|n| n.wires.len() > 1 && !n.pins.is_empty()).collect();
    // Two numbers in one connection
    let (pi, id) = numbered[0].wires[0];
    prj.pages[pi].wires.iter_mut().find(|w| w.id == id).unwrap().number = "X99".into();
    assert!(find(&messages(&prj), "Different wire numbers in one connection").is_some());
    // The same number on two potentials
    let pot = |n: &crate::netlist::Net| {
        let p = n.pins[0];
        bridged.pin_net[&(prj.pages[p.page].symbols[p.sym].id, p.pin)]
    };
    let other = open.nets.iter().find(|n| !n.wires.is_empty() && !n.pins.is_empty() && pot(n) != pot(numbered[1])).unwrap().clone();
    let number = crate::wirenum::number_of(&prj, numbered[1]);
    crate::wirenum::set(&mut prj, &other, &number, true);
    assert!(find(&messages(&prj), &format!("Wire number {number} is used for 2 different potentials")).is_some());
    // A tag without device name
    prj.pages[2].symbols[0].tag = "=A1".into();
    assert!(find(&messages(&prj), "=A1: device tag without device name").is_some());
}

#[test]
fn mounting_pages_without_enclosure_are_reported() {
    let mut prj = demo_project();
    assert!(find(&messages(&prj), "no longer exists").is_none());
    prj.layout.enclosures.clear();
    let msgs = messages(&prj);
    let page = prj.pages.iter().find(|p| matches!(p.kind, PageKind::Mounting(_))).unwrap();
    assert_eq!(find(&msgs, "no longer exists").unwrap(), &format!("Page {}: its enclosure no longer exists", page.name));
}


#[test]
fn assemblies_list_their_components_in_the_bom() {
    let mut prj = demo_project();
    let number = prj.devices.values().find_map(|d| d.parts.first().map(|r| r.number.clone())).unwrap();
    let comp = |n: &str, q: f32| crate::parts::Component { number: n.into(), quantity: q };
    let part = prj.parts.iter_mut().find(|p| p.number == number).unwrap();
    part.components = vec![comp("GEN.SCREW-M4", 4.0)];
    let an = crate::analysis::Analysis::checked(&prj);
    let bom = an.report(&prj, ReportKind::BillOfMaterials).unwrap();
    let main = bom.rows.iter().position(|r| r[3] == number).unwrap();
    let sub = &bom.rows[main + 1];
    assert_eq!(sub[0], format!("{}.1", bom.rows[main][0]));
    assert_eq!(sub[3], "GEN.SCREW-M4");
    let qty: f32 = bom.rows[main][1].parse().unwrap();
    assert_eq!(sub[1].parse::<f32>().unwrap(), qty * 4.0);
    assert_eq!(sub[7], format!("in {number}"));
}

#[test]
fn the_inspection_reports_are_found_by_their_standards() {
    let found = |q: &str| -> Vec<String> { builtin().into_iter().filter(|r| r.matches(crate::params::text(&r.title), q)).map(|r| r.name).collect() };
    let inspection = ["inspection", "inspection_summary", "inspection_sources"];
    for q in ["VDE 0100 600", "DIN VDE 0100-600", "vde 0100-600", "IEC 60364-6", "60364-6", "Prüfprotokoll", "BS 7671"] {
        assert_eq!(found(q), inspection, "{q:?}");
    }
    assert_eq!(found("VDE 0100-410"), ["short_circuit"]);
    assert!(found("ISO 13849").contains(&"safety_functions".to_string()));
    assert_eq!(found("").len(), ReportKind::ALL.len());
}

#[test]
fn the_drilling_pattern_lists_every_hole_and_cutout_from_the_bottom_left() {
    let mut prj = demo_project();
    let e = prj.layout.enclosures[0].clone();
    let at = {
        let n = prj.pages.len();
        crate::ops::add_page(&mut prj, PageKind::Drilling(e.id), n)
    };
    let an = Analysis::with_builtin(&prj);
    let t = table(&prj, &an, ReportKind::DrillingPattern);
    let faces = crate::production::drilling(&e, &prj.parts);
    let features: usize = faces.iter().map(|f| f.features.len()).sum();
    let kinds: usize = faces.iter().map(|f| f.kinds.len()).sum();
    assert_eq!(t.rows.len(), features + kinds, "a row per hole and cutout, one per kind with its count");
    // The rows jump to the drilling pattern page.
    assert!(t.targets.iter().flatten().all(|(pg, _)| *pg == at) && t.targets.iter().flatten().count() == features);
    // The dimensions as the NC data: from the bottom left corner, y up.
    let csv = crate::production::nc_csv(&prj, 0, crate::panel::Face::Plate);
    let plate = &faces[0];
    assert_eq!(plate.face, crate::panel::Face::Plate);
    let first = &plate.features[0];
    let row = t.rows.iter().find(|r| r[3] == crate::production::dim(first.x) && r[4] == crate::production::dim(first.y)).expect("the first hole");
    assert_eq!(row[2], "A");
    assert!(csv.contains(&format!(";{:.1};{:.1};", first.x, first.y)), "{csv}");
    // The rails screwed into M5 tapped holes.
    assert!(t.rows.iter().any(|r| r[5] == "Ø4.2" && r[6] == "M5" && r[7] == crate::panel::Fastening::Tapped.title()));
    assert!(crate::reports::builtin().iter().any(|r| r.name == "drilling_pattern"));
}

/// Board +UV1: main switch -Q0, surge protective device -F9 (type 2), residual
/// current device -Q1 (40 A, 30 mA) feeding -F1 (B16, a socket outlet through cable -W1),
/// the spare -F2 and the arc fault detection device -F4, and a staircase timer -K1
/// shown by its property; board +UV2 with -F3 (hidden by its property) and the RCBO -F5
/// straight from L1; a switch -Q9 of a room on the mains.
fn boards() -> crate::testkit::Kit {
    use crate::parts::FnKind;
    let mut k = crate::testkit::Kit::new();
    let l1 = k.sym("pot_l1", 100.0, 20.0, "", "L1");
    let q0 = k.sym("disc3", 40.0, 60.0, "+UV1-Q0", "");
    let f9 = k.sym("spd", 70.0, 60.0, "+UV1-F9", "");
    let q1 = k.sym("rcd4", 100.0, 40.0, "+UV1-Q1", "");
    let f1 = k.sym("cb1", 100.0, 80.0, "+UV1-F1", "");
    let f2 = k.sym("cb1", 140.0, 80.0, "+UV1-F2", "");
    let f4 = k.sym("cb1", 180.0, 80.0, "+UV1-F4", "");
    k.sym("coil", 220.0, 80.0, "+UV1-K1", "");
    let x1 = k.sym("socket", 100.0, 140.0, "+R1-XD1", "");
    let f3 = k.sym("cb1", 300.0, 40.0, "+UV2-F3", "");
    let f5 = k.sym("cb1", 340.0, 40.0, "+UV2-F5", "");
    let q9 = k.sym("disc3", 400.0, 60.0, "+R1-Q9", "");
    for (sym, pin) in [(q0, "1"), (f9, "L"), (q1, "1"), (f3, "1"), (f5, "1"), (q9, "1")] {
        k.wire((l1, ""), (sym, pin));
    }
    for f in [f1, f2, f4] {
        k.wire((q1, "2"), (f, "1"));
    }
    let socket = k.prj.symbol("socket").unwrap().pins[0].label.to_string();
    k.core((f1, "2"), (x1, &socket), "+UV1-W1", "1");
    k.cable("+UV1-W1", "1 2 3", "1.5 mm\u{b2}", 12.0);
    k.prj.cables.last_mut().unwrap().cable_type = "NYM-J 3x1.5".into();
    // A second socket outlet behind the first: its cable does not leave the board.
    let x2 = k.sym("socket", 60.0, 180.0, "+R1-XD2", "");
    k.core((x1, &socket), (x2, &socket), "+R1-W2", "1");
    k.cable("+R1-W2", "1 2 3", "1.5 mm\u{b2}", 4.0);
    k.part("+UV1-Q1", FnKind::ResidualCurrent, "1 3 5 N 2 4 6 N'", |p| {
        p.current = "40 A".into();
        p.tech_data = "Rated residual current: 30 mA".into();
    });
    for t in ["+UV1-F1", "+UV1-F2", "+UV2-F3", "+UV2-F5"] {
        k.part(t, FnKind::Breaker1, "1 2", |p| p.current = "16 A".into());
    }
    k.part("+UV1-F4", FnKind::Breaker1, "1 2", |p| {
        p.current = "16 A".into();
        p.description = "Arc fault detection device (AFDD) with circuit breaker".into();
    });
    k.part("+UV1-K1", FnKind::Coil, "A1 A2", |_| {});
    // The RCBO: a breaker with a residual current function.
    let rcbo = k.prj.parts.iter_mut().find(|p| p.number.contains("UV2-F5")).unwrap();
    rcbo.functions.push(crate::parts::FunctionTemplate::new(FnKind::ResidualCurrent, "1 2"));
    let device = |t: &str| crate::properties::Object::Device(t.into());
    crate::properties::set(&mut k.prj, &device("+UV1-F9"), "device.spd_type", "Type 2").unwrap();
    crate::properties::set(&mut k.prj, &device("+UV1-K1"), "device.fuse_overview", "Show").unwrap();
    crate::properties::set(&mut k.prj, &device("+UV2-F3"), "device.fuse_overview", "Hide").unwrap();
    let page = &mut k.prj.pages[0];
    page.symbols.iter_mut().find(|s| s.tag == "+UV1-F1").unwrap().function_text = "Kitchen sockets".into();
    k
}

#[test]
fn fuse_overview_lists_the_devices_of_each_board() {
    let k = boards();
    let an = Analysis::checked(&k.prj);
    let t = an.report(&k.prj, ReportKind::FuseOverview).unwrap();
    assert!(t.rows.iter().all(|r| r.len() == t.headers.len()));
    let devices: Vec<(&str, &str)> = t.rows.iter().map(|r| (r[0].as_str(), r[1].as_str())).collect();
    assert_eq!(
        devices,
        [("+UV1", "+UV1-Q0"), ("+UV1", "+UV1-F9"), ("+UV1", "+UV1-Q1"), ("+UV1", "+UV1-F1"), ("+UV1", "+UV1-F2"), ("+UV1", "+UV1-F4"), ("+UV1", "+UV1-K1"), ("+UV2", "+UV2-F5")]
    );
    let row = |tag: &str| t.rows.iter().find(|r| r[1] == tag).unwrap();
    let kind = |tag: &str| row(tag)[2].clone();
    assert_eq!(kind("+UV1-Q0"), "Main switch");
    assert_eq!(kind("+UV1-F9"), "Surge protective device");
    assert_eq!(kind("+UV1-Q1"), "Residual current device");
    assert_eq!(kind("+UV1-F1"), "Circuit breaker");
    assert_eq!(kind("+UV1-F4"), "Arc fault detection device");
    assert_eq!(kind("+UV1-K1"), "Coil");
    assert_eq!(kind("+UV2-F5"), "RCBO");
    assert_eq!(row("+UV1-Q1")[3], "40 A RCD 30 mA");
    assert_eq!(row("+UV1-F9")[3], "Type 2");
    let f1 = row("+UV1-F1");
    assert!(f1[3].ends_with("16 A"), "{f1:?}");
    assert_eq!((f1[4].as_str(), f1[5].as_str(), f1[6].as_str()), ("L1", "+UV1-Q1", "Kitchen sockets"));
    assert_eq!(f1[8], "+UV1-W1 NYM-J 3x1.5", "only the cable leaving the board");
    assert!(!f1[9].is_empty(), "where it is drawn");
    // The spare breaker and the one without residual current device.
    assert_eq!(row("+UV1-F2")[6], "Spare");
    assert_eq!(row("+UV2-F5")[5], "");
    assert!(t.targets.iter().all(|t| t.is_some()), "every row jumps to its device");
    // Found by the words of practice.
    assert!(keywords(ReportKind::FuseOverview).contains(&"Panel schedule"));
}

/// The property says it: `device.afdd` false overrides the description of the part.
#[test]
fn fuse_overview_follows_the_afdd_property() {
    let mut k = boards();
    crate::properties::set(&mut k.prj, &crate::properties::Object::Device("+UV1-F4".into()), "device.afdd", "false").unwrap();
    let an = Analysis::checked(&k.prj);
    let t = an.report(&k.prj, ReportKind::FuseOverview).unwrap();
    assert_eq!(t.rows.iter().find(|r| r[1] == "+UV1-F4").unwrap()[2], "Circuit breaker");
}

/// The board of a protective device is the cabinet it is placed in.
#[test]
fn fuse_overview_takes_the_board_from_the_layout() {
    let prj = crate::sample::example("electrical_room").unwrap();
    let an = Analysis::checked(&prj);
    let t = an.report(&prj, ReportKind::FuseOverview).unwrap();
    assert!(!t.rows.is_empty());
    for r in &t.rows {
        if let Some(ei) = crate::installation::board_of(&prj, &r[1]) {
            assert_eq!(r[0], prj.layout.enclosures[ei].name, "{r:?}");
        }
    }
    assert!(t.rows.iter().any(|r| crate::installation::board_of(&prj, &r[1]).is_some()), "{:?}", t.rows);
}

/// Only the cables leaving the board: not those between the devices of the rooms.
#[test]
fn fuse_overview_names_the_cables_leaving_the_board() {
    use crate::installation::{Circuit, Kind, Leg, SchematicDevice};
    let leg = |from: &str, to: &str, cable: &str| Leg { from: from.into(), to: to.into(), cable: cable.into(), cable_type: "NYM-J 3x1.5".into(), mm2: 1.5, length: None, routed: false };
    let device = |tag: &str| SchematicDevice { tag: tag.into(), kind: Kind::Socket, text: String::new(), part: None, placed: None };
    let c = Circuit {
        breaker: "-F1".into(),
        text: String::new(),
        rated: 16.0,
        characteristic: "B".into(),
        three_phase: false,
        board: None,
        devices: vec![device("-XD1"), device("-XD2")],
        legs: vec![leg("-X1", "-XD1", "-W1"), leg("-XD1", "-XD2", "-W2")],
        page: None,
    };
    let prj = Project::default();
    assert_eq!(electrical_reports::outgoing_cables(&prj, &[], "-F1", Some(&c)), ["-W1 NYM-J 3x1.5"]);
}
