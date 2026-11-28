//! Device families: the variants of one kind of device of the installation (a socket
//! outlet with one or two outlets, with a hinged lid or a USB charger …) chosen by the
//! values of the family's options. A variant is a part, so it brings its 3D model, its
//! functions (and with them its schematic symbol) and its symbol on installation plans;
//! a variant may name other symbols. A family can take over the options, variants and
//! suggested texts of a base family and add its own, like a class those of its base
//! class. Built-in families cover the generic parts of the installation; projects add
//! their own (also through property library files).

use crate::model::families::{Choice, DeviceFamily, FamilyChoice, FamilyOption, Variant};
use crate::model::panel::PanelItem;
use crate::model::Project;
use openecraft_i18n::{tr, tr_const, tr_fmt};
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// A built-in option value: key, name, mark beside the symbol.
type C = (&'static str, &'static str, &'static str);

fn option(key: &str, name: &str, choices: &[C]) -> FamilyOption {
    FamilyOption {
        key: key.into(),
        name: name.into(),
        choices: choices.iter().map(|&(key, name, mark)| Choice { key: key.into(), name: name.into(), mark: mark.into() }).collect(),
        default: String::new(),
    }
}

fn variant(when: &[(&str, &str)], part: &str) -> Variant {
    Variant { when: when.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(), part: part.into(), ..Default::default() }
}

fn family(key: &str, name: &str, parent: &str) -> DeviceFamily {
    DeviceFamily { key: key.into(), name: name.into(), parent: parent.into(), builtin: true, ..Default::default() }
}

/// The built-in families.
pub fn builtin() -> &'static [DeviceFamily] {
    builtin_tables().0.as_slice()
}

/// The built-in families and the English texts they hold (names of families, options,
/// values and the suggested texts), for translating them.
fn builtin_tables() -> &'static (Vec<DeviceFamily>, Vec<&'static str>) {
    static FAMILIES: OnceLock<(Vec<DeviceFamily>, Vec<&'static str>)> = OnceLock::new();
    FAMILIES.get_or_init(|| {
        let v = builtin_families();
        let mut texts: Vec<&'static str> = Vec::new();
        for f in &v {
            let names = std::iter::once(&f.name).chain(&f.texts).chain(f.options.iter().flat_map(|o| std::iter::once(&o.name).chain(o.choices.iter().map(|c| &c.name))));
            for n in names {
                if !texts.iter().any(|t| t == n) {
                    texts.push(Box::leak(n.clone().into_boxed_str()));
                }
            }
        }
        (v, texts)
    })
}

/// The English text `en` of a built-in family translated (`en` itself if unknown).
fn translated(en: &str) -> String {
    match builtin_tables().1.iter().find(|t| **t == en) {
        Some(t) => tr(t).to_string(),
        None => en.to_string(),
    }
}

fn builtin_families() -> Vec<DeviceFamily> {
    {
        let mut v = Vec::new();
        let mut socket = family("socket", tr_const!("Socket outlet"), "");
        socket.texts = [
            tr_const!("Fridge"),
            tr_const!("Freezer"),
            tr_const!("Washing machine"),
            tr_const!("Tumble dryer"),
            tr_const!("Dishwasher"),
            tr_const!("Microwave oven"),
            tr_const!("Television"),
            tr_const!("Outdoors"),
        ]
        .map(String::from)
        .to_vec();
        v.push(socket);

        let mut schuko = family("socket.schuko", tr_const!("Socket outlet type F"), "socket");
        schuko.options = vec![
            option("outlets", tr_const!("Outlets"), &[("1", tr_const!("Single"), ""), ("2", tr_const!("Double"), "")]),
            option(
                "design",
                tr_const!("Design"),
                &[("standard", tr_const!("Standard"), ""), ("lid", tr_const!("Hinged lid, IP44"), "IP44"), ("usb", tr_const!("With USB charger"), "USB")],
            ),
        ];
        schuko.variants = vec![
            variant(&[("outlets", "1"), ("design", "standard")], "GEN.SOCKET-F-UP"),
            variant(&[("outlets", "2"), ("design", "standard")], "GEN.SOCKET-F-2X-UP"),
            variant(&[("outlets", "1"), ("design", "lid")], "GEN.SOCKET-F-LID-UP"),
            variant(&[("outlets", "1"), ("design", "usb")], "GEN.SOCKET-F-USB-UP"),
        ];
        v.push(schuko);

        let mut cee = family("socket.cee", tr_const!("CEE socket outlet, three-phase"), "socket");
        cee.options = vec![option("current", tr_const!("Rated current"), &[("16", "16 A", ""), ("32", "32 A", "")])];
        cee.variants = vec![variant(&[("current", "16")], "GEN.SOCKET-CEE-16-5P"), variant(&[("current", "32")], "GEN.SOCKET-CEE-32-5P")];
        v.push(cee);

        let mut switch = family("switch", tr_const!("Installation switch"), "");
        switch.options = vec![option(
            "function",
            tr_const!("Function"),
            &[
                ("on_off", tr_const!("On / off"), ""),
                ("two_way", tr_const!("Two-way"), ""),
                ("intermediate", tr_const!("Intermediate"), ""),
                ("series", tr_const!("Series"), ""),
                ("push", tr_const!("Push button"), ""),
                ("blind", tr_const!("Blind"), ""),
                ("dimmer", tr_const!("Dimmer"), ""),
            ],
        )];
        switch.variants = [
            ("on_off", "GEN.SW-ONOFF-UP"),
            ("two_way", "GEN.SW-2WAY-UP"),
            ("intermediate", "GEN.SW-INTER-UP"),
            ("series", "GEN.SW-SERIES-UP"),
            ("push", "GEN.PB-NO-UP"),
            ("blind", "GEN.SW-BLIND-UP"),
            ("dimmer", "GEN.DIM-LED-UP"),
        ]
        .iter()
        .map(|(f, part)| variant(&[("function", f)], part))
        .collect();
        v.push(switch);

        let mut light = family("light", tr_const!("Luminaire"), "");
        light.options = vec![option(
            "mounting",
            tr_const!("Mounting"),
            &[("ceiling", tr_const!("Ceiling"), ""), ("recessed", tr_const!("Recessed"), ""), ("wall", tr_const!("Wall"), "")],
        )];
        light.variants = vec![
            variant(&[("mounting", "ceiling")], "GEN.LAMP-CEILING-LED"),
            variant(&[("mounting", "recessed")], "GEN.LAMP-DOWNLIGHT-LED"),
            variant(&[("mounting", "wall")], "GEN.LAMP-WALL-LED"),
        ];
        v.push(light);

        let mut conn = family("connection_box", tr_const!("Device connection box"), "");
        conn.options = vec![option("supply", tr_const!("Supply"), &[("1", "230 V", ""), ("3", "400 V 3N", "")])];
        conn.variants = vec![variant(&[("supply", "1")], "GEN.CONN-BOX-UP"), variant(&[("supply", "3")], "GEN.CONN-BOX-COOKER-UP")];
        conn.texts = [
            tr_const!("Cooker"),
            tr_const!("Hob"),
            tr_const!("Oven"),
            tr_const!("Dishwasher"),
            tr_const!("Washing machine"),
            tr_const!("Tumble dryer"),
            tr_const!("Water heater"),
            tr_const!("Extractor hood"),
        ]
        .map(String::from)
        .to_vec();
        v.push(conn);
        v
    }
}

pub fn builtin_family(key: &str) -> Option<&'static DeviceFamily> {
    builtin().iter().find(|f| f.key == key)
}

/// A family by key: the project's own first, then the built-in ones.
pub fn get<'a>(prj: &'a Project, key: &str) -> Option<&'a DeviceFamily> {
    prj.property_db.families.iter().find(|f| f.key == key).or_else(|| builtin_family(key))
}

/// All families: the built-in ones (unless the project has its own under the key), then
/// the project's.
pub fn all(prj: &Project) -> Vec<&DeviceFamily> {
    let own = &prj.property_db.families;
    builtin().iter().filter(|b| !own.iter().any(|f| f.key == b.key)).chain(own.iter()).collect()
}

/// The name of a family in the language shown (built-in names are translated).
pub fn title(f: &DeviceFamily) -> String {
    if f.builtin { translated(&f.name) } else { f.name.clone() }
}

/// The name of an option or a value of family `f` in the language shown.
pub fn name_of(f: &DeviceFamily, name: &str) -> String {
    if f.builtin { translated(name) } else { name.to_string() }
}

/// Family `key` and its base families, the family first; stops before a family met
/// twice or a missing one.
pub fn lineage<'a>(prj: &'a Project, key: &str) -> Vec<&'a DeviceFamily> {
    let mut out: Vec<&DeviceFamily> = Vec::new();
    let mut next = get(prj, key);
    while let Some(f) = next {
        if out.iter().any(|x| x.key == f.key) {
            break;
        }
        out.push(f);
        next = if f.parent.is_empty() { None } else { get(prj, &f.parent) };
    }
    out
}

/// The options of family `key` with those of its base families (an option of the
/// family replaces one of the same key of a base), the base's first; with the family
/// whose names they carry.
pub fn options<'a>(prj: &'a Project, key: &str) -> Vec<(&'a DeviceFamily, &'a FamilyOption)> {
    let mut out: Vec<(&DeviceFamily, &FamilyOption)> = Vec::new();
    for f in lineage(prj, key).into_iter().rev() {
        for o in &f.options {
            match out.iter_mut().find(|(_, x)| x.key == o.key) {
                Some(x) => *x = (f, o),
                None => out.push((f, o)),
            }
        }
    }
    out
}

/// The suggested texts of family `key` and its base families (translated for the
/// built-in ones), without repeats.
pub fn texts(prj: &Project, key: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for f in lineage(prj, key) {
        for t in &f.texts {
            let t = name_of(f, t);
            if !out.contains(&t) {
                out.push(t);
            }
        }
    }
    out
}

/// The value of every option of family `key`: the one given, else the default (or the
/// first value). Unknown options and values are refused.
pub fn values(prj: &Project, key: &str, given: &BTreeMap<String, String>) -> Result<BTreeMap<String, String>, String> {
    if get(prj, key).is_none() {
        return Err(tr_fmt!("Unknown device family {}", key));
    }
    let opts = options(prj, key);
    for (k, v) in given {
        let Some((_, o)) = opts.iter().find(|(_, o)| &o.key == k) else {
            return Err(tr_fmt!("Device family {} has no option {}", key, k));
        };
        if !o.choices.iter().any(|c| &c.key == v) {
            return Err(tr_fmt!("Option {} has no value {}", k, v));
        }
    }
    Ok(opts
        .iter()
        .filter_map(|(_, o)| {
            let v = given.get(&o.key).cloned().or_else(|| (!o.default.is_empty()).then(|| o.default.clone())).or_else(|| o.choices.first().map(|c| c.key.clone()))?;
            Some((o.key.clone(), v))
        })
        .collect())
}

/// What the option values of a family come to.
#[derive(Clone, Debug, PartialEq)]
pub struct Resolved {
    pub part: String,
    /// Symbol on installation plans; empty: the one of the part.
    pub plan_symbol: String,
    /// Symbol in the schematic; empty: the one of the part's function.
    pub schematic_symbol: String,
    /// Marks of the values beside the plan symbol, in the order of the options.
    pub marks: Vec<String>,
    /// All option values (given or default).
    pub values: BTreeMap<String, String>,
}

/// The variant of family `key` the values `given` (the rest at their defaults) stand
/// for: of the variants of the family and its base families whose values all match, the
/// one naming the most (the family's own before a base's at a tie).
pub fn resolve(prj: &Project, key: &str, given: &BTreeMap<String, String>) -> Result<Resolved, String> {
    let values = values(prj, key, given)?;
    let v = best(prj, key, &values).ok_or_else(|| tr_fmt!("No variant of device family {} for these values", key))?;
    let marks = options(prj, key).iter().filter_map(|(_, o)| o.choices.iter().find(|c| values.get(&o.key) == Some(&c.key))).filter(|c| !c.mark.is_empty()).map(|c| c.mark.clone()).collect();
    Ok(Resolved { part: v.part.clone(), plan_symbol: v.plan_symbol.clone(), schematic_symbol: v.schematic_symbol.clone(), marks, values })
}

fn best<'a>(prj: &'a Project, key: &str, values: &BTreeMap<String, String>) -> Option<&'a Variant> {
    let mut found: Option<&Variant> = None;
    for f in lineage(prj, key) {
        for v in &f.variants {
            if v.when.iter().all(|(k, x)| values.get(k) == Some(x)) && found.is_none_or(|b| v.when.len() > b.when.len()) {
                found = Some(v);
            }
        }
    }
    found
}

/// Whether value `choice` of option `opt` leads to a variant with the other values of
/// `values` (for offering only the values that do).
pub fn reachable(prj: &Project, key: &str, values: &BTreeMap<String, String>, opt: &str, choice: &str) -> bool {
    let mut v = values.clone();
    v.insert(opt.to_string(), choice.to_string());
    if best(prj, key, &v).is_some() {
        return true;
    }
    // With the other options free: the value leads somewhere at all.
    lineage(prj, key).iter().any(|f| f.variants.iter().any(|x| x.when.get(opt).is_none_or(|c| c == choice)))
}

/// The family and the values a part is a variant of (the project's families first),
/// the text empty.
pub fn of_part(prj: &Project, part: &str) -> Option<FamilyChoice> {
    if part.is_empty() {
        return None;
    }
    let fams = all(prj);
    let mut order: Vec<&DeviceFamily> = fams.iter().copied().filter(|f| !f.builtin).collect();
    order.extend(fams.iter().copied().filter(|f| f.builtin));
    for f in order {
        if let Some(v) = f.variants.iter().find(|v| v.part == part) {
            let values = values(prj, &f.key, &v.when).ok()?;
            return Some(FamilyChoice { family: f.key.clone(), values, text: String::new() });
        }
    }
    None
}

/// The family, values and text of an item of a room: those chosen for it, else those
/// of its part.
pub fn of_item(prj: &Project, it: &PanelItem) -> FamilyChoice {
    if !it.family.family.is_empty() && get(prj, &it.family.family).is_some() {
        return it.family.clone();
    }
    let mut c = of_part(prj, &it.part).unwrap_or_default();
    c.text.clone_from(&it.family.text);
    c
}

/// Problems of a family definition: missing key or name, unknown base or a base that
/// leads back to it, options without values or with repeated ones, variants naming
/// unknown options or values or without a part, unknown symbols.
pub fn validate(prj: &Project, f: &DeviceFamily) -> Result<(), String> {
    if f.key.trim().is_empty() || f.name.trim().is_empty() {
        return Err(tr("The device family needs a key and a name").into());
    }
    if !f.parent.is_empty() {
        if f.parent == f.key {
            return Err(tr_fmt!("Device family {} cannot be its own base", f.key));
        }
        if get(prj, &f.parent).is_none() {
            return Err(tr_fmt!("Unknown device family {}", f.parent));
        }
        if lineage(prj, &f.parent).iter().any(|x| x.key == f.key) {
            return Err(tr_fmt!("Device family {} cannot be its own base", f.key));
        }
    }
    let mut keys: Vec<&str> = Vec::new();
    for o in &f.options {
        if o.key.trim().is_empty() || o.choices.is_empty() {
            return Err(tr_fmt!("Option {} needs a key and values", o.key));
        }
        if keys.contains(&o.key.as_str()) {
            return Err(tr_fmt!("Option {} is defined twice", o.key));
        }
        keys.push(&o.key);
        for (i, c) in o.choices.iter().enumerate() {
            if c.key.trim().is_empty() || o.choices[..i].iter().any(|x| x.key == c.key) {
                return Err(tr_fmt!("Option {} has an empty or repeated value", o.key));
            }
        }
        if !o.default.is_empty() && !o.choices.iter().any(|c| c.key == o.default) {
            return Err(tr_fmt!("Option {} has no value {}", o.key, o.default));
        }
    }
    // The options of the bases as they are with this definition.
    let mut base: Vec<&FamilyOption> = if f.parent.is_empty() { Vec::new() } else { options(prj, &f.parent).into_iter().map(|(_, o)| o).collect() };
    base.retain(|o| !f.options.iter().any(|x| x.key == o.key));
    let all_opts: Vec<&FamilyOption> = base.into_iter().chain(f.options.iter()).collect();
    for v in &f.variants {
        if v.part.trim().is_empty() {
            return Err(tr("Every variant needs a part").into());
        }
        for (k, x) in &v.when {
            let Some(o) = all_opts.iter().find(|o| &o.key == k) else {
                return Err(tr_fmt!("Device family {} has no option {}", f.key, k));
            };
            if !o.choices.iter().any(|c| &c.key == x) {
                return Err(tr_fmt!("Option {} has no value {}", k, x));
            }
        }
        for s in [&v.plan_symbol, &v.schematic_symbol] {
            if !s.is_empty() && prj.symbol(s).is_none() {
                return Err(tr_fmt!("Unknown symbol {}", s));
            }
        }
    }
    Ok(())
}

/// Adds or replaces a project family (a built-in key gives the project's own version).
pub fn define(prj: &mut Project, f: DeviceFamily) -> Result<(), String> {
    validate(prj, &f)?;
    let f = DeviceFamily { builtin: false, ..f };
    match prj.property_db.families.iter_mut().find(|x| x.key == f.key) {
        Some(x) => *x = f,
        None => prj.property_db.families.push(f),
    }
    Ok(())
}

/// Removes a project family; refused while another family names it as its base.
pub fn remove(prj: &mut Project, key: &str) -> Result<bool, String> {
    if let Some(child) = prj.property_db.families.iter().find(|f| f.parent == key && f.key != key) {
        if builtin_family(key).is_none() {
            return Err(tr_fmt!("Device family {} is the base of {}", key, child.key));
        }
    }
    let n = prj.property_db.families.len();
    prj.property_db.families.retain(|f| f.key != key);
    Ok(n != prj.property_db.families.len())
}

/// Family `f` as the project's own: the names of a built-in one in the language shown
/// (they are the project's texts from then on), under the same key, so that defining it
/// replaces the built-in family in the project.
pub fn editable(f: &DeviceFamily) -> DeviceFamily {
    let mut out = f.clone();
    if f.builtin {
        out.name = title(f);
        for o in &mut out.options {
            o.name = translated(&o.name);
            for c in &mut o.choices {
                c.name = translated(&c.name);
            }
        }
        out.texts = f.texts.iter().map(|t| translated(t)).collect();
    }
    out.builtin = false;
    out
}

/// Whether the project has its own version of built-in family `key`.
pub fn overrides_builtin(prj: &Project, key: &str) -> bool {
    builtin_family(key).is_some() && prj.property_db.families.iter().any(|f| f.key == key)
}

/// A copy of family `from` to change in the project, under a free key.
pub fn derive(prj: &Project, from: &DeviceFamily) -> DeviceFamily {
    let base = format!("user.{}", from.key.trim_start_matches("user."));
    let mut key = base.clone();
    let mut n = 1;
    while get(prj, &key).is_some() {
        n += 1;
        key = format!("{base}{n}");
    }
    DeviceFamily { key, ..editable(from) }
}

#[cfg(test)]
mod tests;
