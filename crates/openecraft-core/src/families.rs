//! Device families: the variants of one kind of device of the installation (a socket
//! outlet with one, two, three or four outlets) and the options they can be fitted with
//! (a hinged lid, a USB charger). Exactly one variant is chosen; it is a part, so it
//! brings its 3D model, its functions (and with them its schematic symbol) and its
//! symbol on installation plans, or names other symbols. Options are added to it: each
//! puts a mark beside the plan symbol and may add an accessory part; a variant with
//! options can be a part of its own (a combination). A family can take over the
//! variants, options and suggested texts of a base family and add its own, like a class
//! those of its base class. Built-in families cover the generic parts of the
//! installation; projects add their own or change the built-in ones (also through
//! property library files).

use crate::model::families::{Combination, DeviceFamily, FamilyChoice, FamilyOption, Variant};
use crate::model::panel::PanelItem;
use crate::model::Project;
use openecraft_i18n::{tr, tr_const, tr_fmt};
use std::sync::OnceLock;

fn variant(key: &str, name: &str, part: &str, plan_symbol: &str) -> Variant {
    Variant { key: key.into(), name: name.into(), part: part.into(), plan_symbol: plan_symbol.into(), schematic_symbol: String::new() }
}

fn option(key: &str, name: &str, mark: &str, variants: &[&str]) -> FamilyOption {
    FamilyOption { key: key.into(), name: name.into(), mark: mark.into(), accessory: String::new(), variants: variants.iter().map(|v| v.to_string()).collect() }
}

fn combination(variant: &str, options: &[&str], part: &str) -> Combination {
    Combination { variant: variant.into(), options: options.iter().map(|o| o.to_string()).collect(), part: part.into() }
}

fn family(key: &str, name: &str, parent: &str) -> DeviceFamily {
    DeviceFamily { key: key.into(), name: name.into(), parent: parent.into(), builtin: true, ..Default::default() }
}

/// The built-in families.
pub fn builtin() -> &'static [DeviceFamily] {
    builtin_tables().0.as_slice()
}

/// The built-in families and the English texts they hold (names of families, variants,
/// options and the suggested texts), for translating them.
fn builtin_tables() -> &'static (Vec<DeviceFamily>, Vec<&'static str>) {
    static FAMILIES: OnceLock<(Vec<DeviceFamily>, Vec<&'static str>)> = OnceLock::new();
    FAMILIES.get_or_init(|| {
        let v = builtin_families();
        let mut texts: Vec<&'static str> = Vec::new();
        for f in &v {
            let names = std::iter::once(&f.name).chain(&f.texts).chain(f.variants.iter().map(|x| &x.name)).chain(f.options.iter().map(|o| &o.name));
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
    schuko.variants = vec![
        variant("1", tr_const!("Single"), "GEN.SOCKET-F-UP", "inst_socket"),
        variant("2", tr_const!("Double"), "GEN.SOCKET-F-2X-UP", "inst_socket2"),
        variant("3", tr_const!("Triple"), "GEN.SOCKET-F-3X-UP", "inst_socket3"),
        variant("4", tr_const!("Quadruple"), "GEN.SOCKET-F-4X-UP", "inst_socket4"),
    ];
    schuko.options = vec![option("lid", tr_const!("Hinged lid, IP44"), "IP44", &["1"]), option("usb", tr_const!("USB charger"), "USB", &["1"])];
    schuko.combinations = vec![combination("1", &["lid"], "GEN.SOCKET-F-LID-UP"), combination("1", &["usb"], "GEN.SOCKET-F-USB-UP")];
    v.push(schuko);

    let mut cee = family("socket.cee", tr_const!("CEE socket outlet, three-phase"), "socket");
    cee.variants = vec![variant("16", "16 A", "GEN.SOCKET-CEE-16-5P", ""), variant("32", "32 A", "GEN.SOCKET-CEE-32-5P", "")];
    v.push(cee);

    let mut switch = family("switch", tr_const!("Installation switch"), "");
    switch.variants = vec![
        variant("on_off", tr_const!("On / off"), "GEN.SW-ONOFF-UP", ""),
        variant("two_way", tr_const!("Two-way"), "GEN.SW-2WAY-UP", ""),
        variant("intermediate", tr_const!("Intermediate"), "GEN.SW-INTER-UP", ""),
        variant("series", tr_const!("Series"), "GEN.SW-SERIES-UP", ""),
        variant("push", tr_const!("Push button"), "GEN.PB-NO-UP", ""),
        variant("blind", tr_const!("Blind"), "GEN.SW-BLIND-UP", ""),
        variant("dimmer", tr_const!("Dimmer"), "GEN.DIM-LED-UP", ""),
    ];
    v.push(switch);

    let mut light = family("light", tr_const!("Luminaire"), "");
    light.variants = vec![
        variant("ceiling", tr_const!("Ceiling"), "GEN.LAMP-CEILING-LED", ""),
        variant("recessed", tr_const!("Recessed"), "GEN.LAMP-DOWNLIGHT-LED", ""),
        variant("wall", tr_const!("Wall"), "GEN.LAMP-WALL-LED", ""),
    ];
    v.push(light);

    let mut conn = family("connection_box", tr_const!("Device connection box"), "");
    conn.variants = vec![variant("1", "230 V", "GEN.CONN-BOX-UP", ""), variant("3", "400 V 3N", "GEN.CONN-BOX-COOKER-UP", "")];
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

/// The name of a variant or an option of family `f` in the language shown.
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

/// Items of family `key` and its base families taken by `items` (one of a key of the
/// family replaces one of a base), the base's first; with the family holding each.
fn inherited<'a, T>(prj: &'a Project, key: &str, items: impl Fn(&'a DeviceFamily) -> &'a [T], key_of: impl Fn(&T) -> &str) -> Vec<(&'a DeviceFamily, &'a T)> {
    let mut out: Vec<(&DeviceFamily, &T)> = Vec::new();
    for f in lineage(prj, key).into_iter().rev() {
        for x in items(f) {
            match out.iter_mut().find(|(_, y)| key_of(y) == key_of(x)) {
                Some(y) => *y = (f, x),
                None => out.push((f, x)),
            }
        }
    }
    out
}

/// The variants of family `key` (those of its base families too).
pub fn variants<'a>(prj: &'a Project, key: &str) -> Vec<(&'a DeviceFamily, &'a Variant)> {
    inherited(prj, key, |f| &f.variants, |v| &v.key)
}

/// The options of family `key` (those of its base families too).
pub fn options<'a>(prj: &'a Project, key: &str) -> Vec<(&'a DeviceFamily, &'a FamilyOption)> {
    inherited(prj, key, |f| &f.options, |o| &o.key)
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

/// Option `o` can be fitted to variant `variant`.
pub fn fits(o: &FamilyOption, variant: &str) -> bool {
    o.variants.is_empty() || o.variants.iter().any(|v| v == variant)
}

/// What a variant with options comes to.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Resolved {
    pub variant: String,
    /// The options chosen, in the order of the family.
    pub options: Vec<String>,
    pub part: String,
    /// Parts added for the options (accessories).
    pub accessories: Vec<String>,
    /// Symbol on installation plans; empty: the one of the part.
    pub plan_symbol: String,
    /// Symbol in the schematic; empty: the one of the part's function.
    pub schematic_symbol: String,
    /// Marks of the options beside the plan symbol, in the order of the family.
    pub marks: Vec<String>,
}

/// The part, symbols and marks of variant `variant` (empty: the first) of family `key`
/// with options `chosen`: a combination of the variant and exactly these options, else
/// the variant's part with the accessories of the options. Unknown variants and
/// options, and options that do not fit the variant, are refused.
pub fn resolve(prj: &Project, key: &str, variant: &str, chosen: &[String]) -> Result<Resolved, String> {
    if get(prj, key).is_none() {
        return Err(tr_fmt!("Unknown device family {}", key));
    }
    let vs = variants(prj, key);
    let v = match variant {
        "" => vs.first().map(|(_, v)| *v).ok_or_else(|| tr_fmt!("Device family {} has no variants", key))?,
        k => vs.iter().find(|(_, v)| v.key == k).map(|(_, v)| *v).ok_or_else(|| tr_fmt!("Device family {} has no variant {}", key, k))?,
    };
    let opts = options(prj, key);
    for c in chosen {
        let Some((_, o)) = opts.iter().find(|(_, o)| &o.key == c) else {
            return Err(tr_fmt!("Device family {} has no option {}", key, c));
        };
        if !fits(o, &v.key) {
            return Err(tr_fmt!("Option {} does not fit variant {}", c, v.key));
        }
    }
    let on: Vec<&FamilyOption> = opts.iter().map(|(_, o)| *o).filter(|o| chosen.contains(&o.key)).collect();
    let keys: Vec<String> = on.iter().map(|o| o.key.clone()).collect();
    let combo = lineage(prj, key).into_iter().flat_map(|f| f.combinations.iter()).find(|c| {
        c.variant == v.key && c.options.len() == keys.len() && keys.iter().all(|k| c.options.contains(k))
    });
    let (part, accessories) = match combo {
        Some(c) => (c.part.clone(), Vec::new()),
        None => (v.part.clone(), on.iter().filter(|o| !o.accessory.is_empty()).map(|o| o.accessory.clone()).collect()),
    };
    Ok(Resolved {
        variant: v.key.clone(),
        options: keys,
        part,
        accessories,
        plan_symbol: v.plan_symbol.clone(),
        schematic_symbol: v.schematic_symbol.clone(),
        marks: on.iter().filter(|o| !o.mark.is_empty()).map(|o| o.mark.clone()).collect(),
    })
}

/// The family, variant and options a part is (the project's families first; a
/// combination before a variant), the text empty.
pub fn of_part(prj: &Project, part: &str) -> Option<FamilyChoice> {
    if part.is_empty() {
        return None;
    }
    let fams = all(prj);
    let mut order: Vec<&DeviceFamily> = fams.iter().copied().filter(|f| !f.builtin).collect();
    order.extend(fams.iter().copied().filter(|f| f.builtin));
    for f in &order {
        if let Some(c) = f.combinations.iter().find(|c| c.part == part) {
            return Some(FamilyChoice { family: f.key.clone(), variant: c.variant.clone(), options: c.options.clone(), text: String::new() });
        }
    }
    for f in &order {
        if let Some(v) = f.variants.iter().find(|v| v.part == part) {
            return Some(FamilyChoice { family: f.key.clone(), variant: v.key.clone(), options: Vec::new(), text: String::new() });
        }
    }
    None
}

/// The family, variant, options and text of an item of a room: those chosen for it,
/// else those of its part.
pub fn of_item(prj: &Project, it: &PanelItem) -> FamilyChoice {
    if !it.family.family.is_empty() && get(prj, &it.family.family).is_some() {
        return it.family.clone();
    }
    let mut c = of_part(prj, &it.part).unwrap_or_default();
    c.text.clone_from(&it.family.text);
    c
}

/// Problems of a family definition: missing key or name, unknown base or a base that
/// leads back to it, variants or options without a key or with a repeated one,
/// variants without a part, unknown symbols, options for unknown variants, combinations
/// of unknown or unfitting variants and options or without a part.
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
    for (i, v) in f.variants.iter().enumerate() {
        if v.key.trim().is_empty() || f.variants[..i].iter().any(|x| x.key == v.key) {
            return Err(tr("Every variant needs a key of its own").into());
        }
        if v.part.trim().is_empty() {
            return Err(tr("Every variant needs a part").into());
        }
        for s in [&v.plan_symbol, &v.schematic_symbol] {
            if !s.is_empty() && prj.symbol(s).is_none() {
                return Err(tr_fmt!("Unknown symbol {}", s));
            }
        }
    }
    for (i, o) in f.options.iter().enumerate() {
        if o.key.trim().is_empty() || f.options[..i].iter().any(|x| x.key == o.key) {
            return Err(tr("Every option needs a key of its own").into());
        }
    }
    // The variants and options of the bases as they are with this definition.
    let base_variants: Vec<&Variant> = if f.parent.is_empty() { Vec::new() } else { variants(prj, &f.parent).into_iter().map(|(_, v)| v).collect() };
    let base_options: Vec<&FamilyOption> = if f.parent.is_empty() { Vec::new() } else { options(prj, &f.parent).into_iter().map(|(_, o)| o).collect() };
    let has_variant = |k: &str| f.variants.iter().any(|v| v.key == k) || base_variants.iter().any(|v| v.key == k);
    let option = |k: &str| f.options.iter().find(|o| o.key == k).or_else(|| base_options.iter().copied().find(|o| o.key == k));
    for o in &f.options {
        if let Some(v) = o.variants.iter().find(|v| !has_variant(v)) {
            return Err(tr_fmt!("Device family {} has no variant {}", f.key, v));
        }
    }
    for c in &f.combinations {
        if !has_variant(&c.variant) {
            return Err(tr_fmt!("Device family {} has no variant {}", f.key, c.variant));
        }
        for k in &c.options {
            let Some(o) = option(k) else {
                return Err(tr_fmt!("Device family {} has no option {}", f.key, k));
            };
            if !fits(o, &c.variant) {
                return Err(tr_fmt!("Option {} does not fit variant {}", k, c.variant));
            }
        }
        if c.part.trim().is_empty() {
            return Err(tr("Every variant needs a part").into());
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

/// Removes a project family (the built-in one of its key comes back); refused while
/// another family names it as its base.
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
        for v in &mut out.variants {
            v.name = translated(&v.name);
        }
        for o in &mut out.options {
            o.name = translated(&o.name);
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
