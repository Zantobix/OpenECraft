//! Data of `crate::families` that the project file holds (see `docs/architecture.md`).

use serde::{Deserialize, Serialize};

/// A family of devices: the variants of one thing (a socket outlet with one, two, three
/// or four outlets; a CEE socket outlet of 16 or 32 A) and the options they can be
/// fitted with (a hinged lid, a USB charger). A family may name a base family whose
/// variants, options and suggested texts it takes over, like a class its base class.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DeviceFamily {
    pub key: String,
    /// English name of a built-in family, the name of a project one.
    pub name: String,
    /// Key of the base family; empty: none.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub parent: String,
    /// The forms of the device: exactly one is chosen.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<Variant>,
    /// What a device can be fitted with: any of them, or none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<FamilyOption>,
    /// Parts of a variant with options that are a part of their own (a socket outlet
    /// with a USB charger built in).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub combinations: Vec<Combination>,
    /// Suggested texts shown at a device of the family (what is connected to it).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub texts: Vec<String>,
    #[serde(skip)]
    pub builtin: bool,
}

/// A variant: the part it is (with its 3D model, its functions and so its schematic
/// symbol, its plan symbol), or other symbols named here.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Variant {
    pub key: String,
    pub name: String,
    /// Part number.
    pub part: String,
    /// Symbol on installation plans; empty: the one of the part.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub plan_symbol: String,
    /// Symbol in the schematic; empty: the one of the part's function.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub schematic_symbol: String,
}

/// An option a device of the family can be fitted with.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct FamilyOption {
    pub key: String,
    pub name: String,
    /// Short text the option adds beside the symbol on installation plans (e.g. `IP44`);
    /// empty: none.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub mark: String,
    /// Part added to the device for the option (an accessory); empty: none.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub accessory: String,
    /// Variants it fits (keys); empty: all.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<String>,
}

/// A variant with options that is a part of its own.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Combination {
    pub variant: String,
    pub options: Vec<String>,
    pub part: String,
}

/// What was chosen for a device placed in a room: its family, variant and options (the
/// part they come to is the item's part) and the text shown at its symbol.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct FamilyChoice {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub family: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub variant: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
    /// Text shown at the symbol (e.g. what is connected: cooker, washing machine).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub text: String,
}

impl FamilyChoice {
    pub fn is_empty(&self) -> bool {
        self.family.is_empty() && self.variant.is_empty() && self.options.is_empty() && self.text.is_empty()
    }
}
