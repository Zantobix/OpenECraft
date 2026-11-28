//! Data of `crate::structure` that the project file holds (see `docs/architecture.md`).


use serde::{Deserialize, Serialize};

/// Structure management of the project.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Structures {
    /// Aspects the pages use; empty = function and location.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aspects: Vec<Aspect>,
    /// Defined identifiers with their descriptions.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub entries: Vec<Entry>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub work_areas: Vec<WorkArea>,
}

/// Aspect of a structure identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Aspect {
    /// `=` what the system does.
    Function,
    /// `++` the plant, building or area.
    Installation,
    /// `+` where it is mounted (enclosure, field).
    Location,
    /// `-` the higher-level product (assembly) the device belongs to.
    Product,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub aspect: Aspect,
    pub id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
}

/// A defined part of the project: only the pages and devices whose structure
/// matches the filter are shown.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkArea {
    pub name: String,
    /// Pattern per aspect: empty = any, `*` = any characters; without `*` the
    /// identifier and its sub-levels (`A1.2`, `U1-A3`) match.
    pub filter: Dt,
}

/// A structure box drawn on a schematic page: the devices whose insertion point lies
/// in the rectangle `a`..`b` take the aspects set in `structure` instead of the
/// page's (empty aspects are inherited from the page or an enclosing box).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StructureBox {
    pub id: u64,
    /// Corners: `a` top left, `b` bottom right.
    pub a: crate::geom::P,
    pub b: crate::geom::P,
    /// Aspects the box sets (without prefix signs); `name` is not used.
    pub structure: Dt,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Corner of the box where its identifiers are written.
    #[serde(skip_serializing_if = "is_top_left")]
    pub label: BoxCorner,
    /// Shift of the identifiers from their corner.
    #[serde(skip_serializing_if = "is_origin")]
    pub label_offset: crate::geom::P,
    #[serde(skip_serializing_if = "is_dash_dot")]
    pub line: BoxLine,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<[u8; 3]>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoxCorner {
    #[default]
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Outline of a structure box.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoxLine {
    #[default]
    DashDot,
    Dashed,
    Solid,
}

fn is_top_left(c: &BoxCorner) -> bool {
    *c == BoxCorner::TopLeft
}

fn is_origin(q: &crate::geom::P) -> bool {
    q.x == 0.0 && q.y == 0.0
}

fn is_dash_dot(l: &BoxLine) -> bool {
    *l == BoxLine::DashDot
}

/// A device tag split into its aspects (stored without the prefix signs, except
/// the device name which keeps its `-`). Product levels are separated by `-`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Dt {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub function: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub installation: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub location: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub product: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
}

/// The aspects of a tag in the order they are written in, each with its prefix sign:
/// `+KIT=SD1-X1` → `+KIT`, `=SD1`, `-X1`; `-X1.6+F1` → `-X1.6`, `+F1`. A sign starts
/// a new aspect only after the first character of an identifier, so a tag may begin
/// with any of them; the last `-` part is the device name (see `Dt::parse`).
pub fn tag_segments(tag: &str) -> Vec<(Aspect, &str)> {
    let tag = tag.trim();
    let mut out: Vec<(Aspect, &str)> = Vec::new();
    let (mut start, mut aspect) = (0, None::<Aspect>);
    let b = tag.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let sign = match b[i] {
            b'=' => Some((Aspect::Function, 1)),
            b'+' if b.get(i + 1) == Some(&b'+') => Some((Aspect::Installation, 2)),
            b'+' => Some((Aspect::Location, 1)),
            b'-' => Some((Aspect::Product, 1)),
            _ => None,
        };
        // A sign right after another one belongs to the identifier (`--1`).
        match sign {
            Some((a, len)) if i == 0 || i > start + aspect.map_or(0, |a| if a == Aspect::Installation { 2 } else { 1 }) => {
                if i > start {
                    out.push((aspect.unwrap_or(Aspect::Product), &tag[start..i]));
                }
                (start, aspect) = (i, Some(a));
                i += len;
            }
            _ => i += 1,
        }
    }
    if start < tag.len() {
        out.push((aspect.unwrap_or(Aspect::Product), &tag[start..]));
    }
    out
}
