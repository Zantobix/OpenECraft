use super::*;
use crate::installation;

fn vals(v: &[(&str, &str)]) -> BTreeMap<String, String> {
    v.iter().map(|(k, x)| (k.to_string(), x.to_string())).collect()
}

#[test]
fn built_in_variants_are_installation_parts_with_symbols() {
    let prj = Project::default();
    for f in builtin() {
        validate(&prj, f).unwrap_or_else(|e| panic!("{}: {e}", f.key));
        for v in &f.variants {
            let part = installation::find_part(&prj, &v.part).unwrap_or_else(|| panic!("{} of {} missing", v.part, f.key));
            assert!(installation::kind_of(part).is_some(), "{} is no installation part", v.part);
        }
        // Every variant is reached by its own values.
        for v in &f.variants {
            assert_eq!(resolve(&prj, &f.key, &v.when).map(|r| r.part), Ok(v.part.clone()), "{}", f.key);
        }
    }
}

#[test]
fn a_family_takes_over_the_texts_and_options_of_its_base() {
    let prj = Project::default();
    assert_eq!(lineage(&prj, "socket.schuko").iter().map(|f| f.key.as_str()).collect::<Vec<_>>(), ["socket.schuko", "socket"]);
    assert!(texts(&prj, "socket.cee").contains(&"Fridge".to_string()));
    let mut prj = Project::default();
    let mut base = derive(&prj, builtin_family("socket").unwrap());
    base.key = "user.base".into();
    base.options = vec![option("colour", "Colour", &[("white", "White", ""), ("black", "Black", "")])];
    define(&mut prj, base).unwrap();
    let child = DeviceFamily { key: "user.child".into(), name: "Child".into(), parent: "user.base".into(), variants: vec![variant(&[("colour", "black")], "GEN.SOCKET-F-UP")], ..Default::default() };
    define(&mut prj, child).unwrap();
    assert_eq!(options(&prj, "user.child").len(), 1);
    // The default (white) has no variant; black has.
    assert!(resolve(&prj, "user.child", &BTreeMap::new()).is_err());
    assert_eq!(resolve(&prj, "user.child", &vals(&[("colour", "black")])).unwrap().part, "GEN.SOCKET-F-UP");
    // A base cannot go while a family builds on it.
    assert!(remove(&mut prj, "user.base").is_err());
}

#[test]
fn values_choose_the_variant_and_add_their_marks() {
    let prj = Project::default();
    let r = resolve(&prj, "socket.schuko", &BTreeMap::new()).unwrap();
    assert_eq!((r.part.as_str(), r.marks.len()), ("GEN.SOCKET-F-UP", 0));
    assert_eq!(resolve(&prj, "socket.schuko", &vals(&[("outlets", "2")])).unwrap().part, "GEN.SOCKET-F-2X-UP");
    let lid = resolve(&prj, "socket.schuko", &vals(&[("design", "lid")])).unwrap();
    assert_eq!((lid.part.as_str(), lid.marks.clone()), ("GEN.SOCKET-F-LID-UP", vec!["IP44".to_string()]));
    // Two outlets with a lid: no such part.
    assert!(resolve(&prj, "socket.schuko", &vals(&[("outlets", "2"), ("design", "lid")])).is_err());
    assert!(!reachable(&prj, "socket.schuko", &vals(&[("outlets", "2"), ("design", "standard")]), "design", "nope"));
    assert!(reachable(&prj, "socket.schuko", &vals(&[("outlets", "2"), ("design", "standard")]), "design", "lid"));
    assert!(resolve(&prj, "socket.schuko", &vals(&[("colour", "red")])).is_err());
    assert!(resolve(&prj, "socket.schuko", &vals(&[("outlets", "9")])).is_err());
    assert!(resolve(&prj, "nope", &BTreeMap::new()).is_err());
}

#[test]
fn a_part_knows_its_family() {
    let prj = Project::default();
    let c = of_part(&prj, "GEN.SW-2WAY-UP").unwrap();
    assert_eq!((c.family.as_str(), c.values.get("function").map(String::as_str)), ("switch", Some("two_way")));
    assert!(of_part(&prj, "GEN.JBOX-UP").is_none());
    assert!(of_part(&prj, "").is_none());
}

#[test]
fn wrong_definitions_are_refused() {
    let mut prj = Project::default();
    let ok = DeviceFamily { key: "user.a".into(), name: "A".into(), options: vec![option("n", "N", &[("1", "1", "")])], variants: vec![variant(&[("n", "1")], "GEN.SOCKET-F-UP")], ..Default::default() };
    define(&mut prj, ok.clone()).unwrap();
    for bad in [
        DeviceFamily { key: String::new(), ..ok.clone() },
        DeviceFamily { parent: "user.a".into(), ..ok.clone() },
        DeviceFamily { parent: "nope".into(), ..ok.clone() },
        DeviceFamily { options: vec![option("n", "N", &[])], ..ok.clone() },
        DeviceFamily { options: vec![option("n", "N", &[("1", "1", ""), ("1", "2", "")])], ..ok.clone() },
        DeviceFamily { variants: vec![variant(&[("m", "1")], "GEN.SOCKET-F-UP")], ..ok.clone() },
        DeviceFamily { variants: vec![variant(&[("n", "2")], "GEN.SOCKET-F-UP")], ..ok.clone() },
        DeviceFamily { variants: vec![variant(&[], "")], ..ok.clone() },
        DeviceFamily { variants: vec![Variant { plan_symbol: "nope".into(), ..variant(&[], "GEN.SOCKET-F-UP") }], ..ok.clone() },
    ] {
        assert!(define(&mut prj, bad.clone()).is_err(), "{bad:?}");
    }
    // A base that leads back to the family.
    define(&mut prj, DeviceFamily { key: "user.b".into(), parent: "user.a".into(), ..ok.clone() }).unwrap();
    assert!(define(&mut prj, DeviceFamily { parent: "user.b".into(), ..ok.clone() }).is_err());
    assert_eq!(prj.property_db.families.len(), 2);
}

#[test]
fn project_families_go_into_property_libraries() {
    let mut prj = Project::default();
    let light = derive(&prj, builtin_family("light").unwrap());
    define(&mut prj, light).unwrap();
    let json = prj.property_db.to_json();
    let mut other = Project::default();
    assert_eq!(other.property_db.merge(crate::model::properties::Library::from_json(&json).unwrap()), 1);
    assert_eq!(other.property_db.families[0].key, "user.light");
    assert!(get(&other, "user.light").is_some_and(|f| !f.builtin));
    // An item without a choice writes nothing.
    let it = PanelItem::default();
    assert!(!serde_json::to_string(&it).unwrap().contains("family"));
}
