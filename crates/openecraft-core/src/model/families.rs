//! Data of `crate::families` that the project file holds (see `docs/architecture.md`).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A family of devices: variants of one thing (a socket outlet with one or two outlets,
/// with a hinged lid, a CEE socket outlet …), chosen by the values of its options. A
/// family may name a base family whose options and suggested texts it takes over, like
/// a class its base class.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DeviceFamily {
    pub key: String,
    /// English name of a built-in family, the name of a project one.
    pub name: String,
    /// Key of the base family; empty: none.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub parent: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<FamilyOption>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<Variant>,
    /// Suggested texts shown at a device of the family (what is connected to it).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub texts: Vec<String>,
    #[serde(skip)]
    pub builtin: bool,
}

/// An option of a family with the values it can take.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct FamilyOption {
    pub key: String,
    pub name: String,
    pub choices: Vec<Choice>,
    /// Key of the value taken while none is chosen; empty: the first one.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub default: String,
}

/// A value of an option.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Choice {
    pub key: String,
    pub name: String,
    /// Short text the value adds beside the symbol on installation plans (e.g. `IP44`);
    /// empty: none.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub mark: String,
}

/// A variant of a family: the part it is (with its 3D model, its functions and so its
/// schematic symbol, its plan symbol) for the option values it stands for.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Variant {
    /// Option values the variant stands for (option key to value key); options it does
    /// not name may take any value.
    pub when: BTreeMap<String, String>,
    /// Part number.
    pub part: String,
    /// Symbol on installation plans; empty: the one of the part.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub plan_symbol: String,
    /// Symbol in the schematic; empty: the one of the part's function.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub schematic_symbol: String,
}

/// What was chosen for a device placed in a room: its family and option values (the
/// part of the variant is the item's part) and the text shown at its symbol.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct FamilyChoice {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub family: String,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, String>,
    /// Text shown at the symbol (e.g. what is connected: cooker, washing machine).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub text: String,
}

impl FamilyChoice {
    pub fn is_empty(&self) -> bool {
        self.family.is_empty() && self.values.is_empty() && self.text.is_empty()
    }
}
