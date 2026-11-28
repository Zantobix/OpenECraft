use super::*;
use crate::installation;

fn opts(o: &[&str]) -> Vec<String> {
    o.iter().map(|s| s.to_string()).collect()
}

#[test]
fn built_in_variants_are_installation_parts_with_symbols() {
    let prj = Project::default();
    for f in builtin() {
        validate(&prj, f).unwrap_or_else(|e| panic!("{}: {e}", f.key));
        let parts = f.variants.iter().map(|v| (&v.part, &v.plan_symbol)).chain(f.combinations.iter().map(|c| (&c.part, &c.part)));
        for (part, sym) in parts {
            let p = installation::find_part(&prj, part).unwrap_or_else(|| panic!("{part} of {} missing", f.key));
            assert!(installation::kind_of(p).is_some(), "{part} is no installation part");
            if sym != part && !sym.is_empty() {
                assert!(prj.symbol(sym).is_some(), "{sym} of {}", f.key);
            }
        }
        // Every variant is reached by itself, every combination by its options.
        for v in &f.variants {
            assert_eq!(resolve(&prj, &f.key, &v.key, &[]).map(|r| r.part), Ok(v.part.clone()), "{}", f.key);
        }
        for c in &f.combinations {
            assert_eq!(resolve(&prj, &f.key, &c.variant, &c.options).map(|r| r.part), Ok(c.part.clone()), "{}", f.key);
        }
    }
}

#[test]
fn a_family_takes_over_the_texts_variants_and_options_of_its_base() {
    let prj = Project::default();
    assert_eq!(lineage(&prj, "socket.schuko").iter().map(|f| f.key.as_str()).collect::<Vec<_>>(), ["socket.schuko", "socket"]);
    assert!(texts(&prj, "socket.cee").contains(&"Fridge".to_string()));
    let mut prj = Project::default();
    let base = DeviceFamily {
        key: "user.base".into(),
        name: "Base".into(),
        variants: vec![variant("a", "A", "GEN.SOCKET-F-UP", "")],
        options: vec![option("spd", "Surge protection", "SPD", &[])],
        ..Default::default()
    };
    define(&mut prj, base).unwrap();
    // The child adds a variant and replaces the base's one of the same key.
    let child = DeviceFamily { key: "user.child".into(), name: "Child".into(), parent: "user.base".into(), variants: vec![variant("a", "A2", "GEN.SOCKET-F-2X-UP", ""), variant("b", "B", "GEN.SOCKET-F-USB-UP", "")], ..Default::default() };
    define(&mut prj, child).unwrap();
    assert_eq!(variants(&prj, "user.child").len(), 2);
    assert_eq!(resolve(&prj, "user.child", "a", &[]).unwrap().part, "GEN.SOCKET-F-2X-UP");
    let r = resolve(&prj, "user.child", "b", &opts(&["spd"])).unwrap();
    assert_eq!((r.part.as_str(), r.marks.clone()), ("GEN.SOCKET-F-USB-UP", vec!["SPD".to_string()]));
    // A base cannot go while a family builds on it.
    assert!(remove(&mut prj, "user.base").is_err());
}

#[test]
fn a_variant_with_options_is_its_combination_or_its_part_with_accessories() {
    let prj = Project::default();
    let r = resolve(&prj, "socket.schuko", "", &[]).unwrap();
    assert_eq!((r.variant.as_str(), r.part.as_str(), r.marks.len()), ("1", "GEN.SOCKET-F-UP", 0));
    assert_eq!(resolve(&prj, "socket.schuko", "4", &[]).unwrap().part, "GEN.SOCKET-F-4X-UP");
    let lid = resolve(&prj, "socket.schuko", "1", &opts(&["lid"])).unwrap();
    assert_eq!((lid.part.as_str(), lid.marks.clone()), ("GEN.SOCKET-F-LID-UP", vec!["IP44".to_string()]));
    // The lid does not fit a double socket outlet.
    assert!(resolve(&prj, "socket.schuko", "2", &opts(&["lid"])).is_err());
    assert!(resolve(&prj, "socket.schuko", "1", &opts(&["nope"])).is_err());
    assert!(resolve(&prj, "socket.schuko", "9", &[]).is_err());
    assert!(resolve(&prj, "nope", "", &[]).is_err());
    // An option without a combination: the variant's part and the option's accessory.
    let mut prj = Project::default();
    let mut f = derive(&prj, builtin_family("socket.schuko").unwrap());
    f.options.push(FamilyOption { key: "spd".into(), name: "SPD".into(), mark: "SPD".into(), accessory: "GEN.JBOX-UP".into(), variants: Vec::new() });
    let key = f.key.clone();
    define(&mut prj, f).unwrap();
    let r = resolve(&prj, &key, "2", &opts(&["spd"])).unwrap();
    assert_eq!((r.part.as_str(), r.accessories.clone()), ("GEN.SOCKET-F-2X-UP", vec!["GEN.JBOX-UP".to_string()]));
}

#[test]
fn a_part_knows_its_family() {
    let prj = Project::default();
    let c = of_part(&prj, "GEN.SW-2WAY-UP").unwrap();
    assert_eq!((c.family.as_str(), c.variant.as_str()), ("switch", "two_way"));
    let c = of_part(&prj, "GEN.SOCKET-F-USB-UP").unwrap();
    assert_eq!((c.variant.as_str(), c.options.clone()), ("1", vec!["usb".to_string()]));
    assert!(of_part(&prj, "GEN.JBOX-UP").is_none());
    assert!(of_part(&prj, "").is_none());
}

#[test]
fn wrong_definitions_are_refused() {
    let mut prj = Project::default();
    let ok = DeviceFamily { key: "user.a".into(), name: "A".into(), variants: vec![variant("1", "1", "GEN.SOCKET-F-UP", "")], options: vec![option("x", "X", "", &["1"])], ..Default::default() };
    define(&mut prj, ok.clone()).unwrap();
    for bad in [
        DeviceFamily { key: String::new(), ..ok.clone() },
        DeviceFamily { parent: "user.a".into(), ..ok.clone() },
        DeviceFamily { parent: "nope".into(), ..ok.clone() },
        DeviceFamily { variants: vec![variant("1", "1", "GEN.SOCKET-F-UP", ""), variant("1", "2", "GEN.SOCKET-F-UP", "")], ..ok.clone() },
        DeviceFamily { variants: vec![variant("1", "1", "", "")], ..ok.clone() },
        DeviceFamily { variants: vec![variant("1", "1", "GEN.SOCKET-F-UP", "nope")], ..ok.clone() },
        DeviceFamily { options: vec![option("x", "X", "", &["9"])], ..ok.clone() },
        DeviceFamily { options: vec![option("x", "X", "", &[]), option("x", "Y", "", &[])], ..ok.clone() },
        DeviceFamily { combinations: vec![combination("9", &[], "GEN.SOCKET-F-UP")], ..ok.clone() },
        DeviceFamily { combinations: vec![combination("1", &["y"], "GEN.SOCKET-F-UP")], ..ok.clone() },
        DeviceFamily { combinations: vec![combination("1", &["x"], "")], ..ok.clone() },
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

#[test]
fn a_derived_or_changed_built_in_family_takes_the_names_in_the_language_shown() {
    let mut prj = Project::default();
    let f = openecraft_i18n::with_lang(openecraft_i18n::Lang::De, || derive(&prj, builtin_family("socket.schuko").unwrap()));
    assert_eq!(f.variants[1].name, "Zweifach");
    assert!(!f.builtin);
    // Its names are its own now: shown as they are in any language.
    assert_eq!(openecraft_i18n::with_lang(openecraft_i18n::Lang::Fr, || name_of(&f, &f.variants[1].name)), "Zweifach");
    // A built-in family changed in the project keeps its key and can be reset.
    let mut own = editable(builtin_family("light").unwrap());
    own.variants.pop();
    define(&mut prj, own).unwrap();
    assert!(overrides_builtin(&prj, "light"));
    assert_eq!(variants(&prj, "light").len(), 2);
    assert_eq!(remove(&mut prj, "light"), Ok(true));
    assert_eq!(variants(&prj, "light").len(), 3);
}
