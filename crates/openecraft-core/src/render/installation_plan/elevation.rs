//! Installation plan pages seen from the side: one wall of a room seen from in front of
//! it (from inside the room, a side of an inner wall, or the outside of a wall) with its
//! doors, windows and the devices on it at their heights, or one side of a building
//! seen from outside, every level of it, with the outer walls facing that way, their
//! doors and windows and what is mounted on the façade (lights, sensors, cameras,
//! outdoor sockets).

use super::*;
use crate::model::{FacadeSide, PlanProjection};
use crate::parts::vec3::V3;

/// A device seen on an elevation: where (u to the right, v up, mm), its symbol, tag,
/// item and height above its floor.
pub(super) struct Seen {
    pub at: [f32; 2],
    pub symbol: String,
    pub tag: String,
    pub item: u64,
    pub height: f32,
}

/// What an elevation shows, in its own plane (u to the right of the viewer, v up, mm).
#[derive(Default)]
pub(super) struct Elevation {
    /// Walls: lower left and upper right corner.
    pub walls: Vec<([f32; 2], [f32; 2])>,
    pub openings: Vec<(Opening, [f32; 2], [f32; 2])>,
    pub devices: Vec<Seen>,
    /// Names of the levels (façade) at the height of their floors.
    pub levels: Vec<(String, f32)>,
    /// The ground line (façade): its height.
    pub ground: Option<f32>,
    /// What the page shows, for its caption.
    pub title: String,
}

/// Projects points of the plant onto the plane of an elevation: `origin` at u 0, `right`
/// the direction of u in the floor plan of the plant, v the height above `base`.
struct Plane {
    origin: [f32; 2],
    right: [f32; 2],
    base: f32,
}

impl Plane {
    fn of(&self, q: V3) -> [f32; 2] {
        [(q[0] - self.origin[0]) * self.right[0] + (q[2] - self.origin[1]) * self.right[1], q[1] - self.base]
    }
}

/// A point of face `face` of room `ri` (face x, face y down from its top) in the plant.
fn face_in_plant(prj: &Project, poses: &[panel::SitePose], ri: usize, face: Face, x: f32, y: f32) -> V3 {
    let e = &prj.layout.enclosures[ri];
    let q = panel::face_point(e, face, [x, y, 0.0]);
    poses[ri].apply([q[0], e.height - q[1], q[2]])
}

/// The corners of an item of face `face` seen on `plane`.
fn item_rect(prj: &Project, poses: &[panel::SitePose], ri: usize, face: Face, it: &PanelItem, plane: &Plane) -> ([f32; 2], [f32; 2]) {
    let a = plane.of(face_in_plant(prj, poses, ri, face, it.x, it.y));
    let b = plane.of(face_in_plant(prj, poses, ri, face, it.x + it.w, it.y + it.h));
    ([a[0].min(b[0]), a[1].min(b[1])], [a[0].max(b[0]), a[1].max(b[1])])
}

/// The installation devices on face `face` of room `ri`, seen on `plane`.
fn devices_on(prj: &Project, poses: &[panel::SitePose], ri: usize, face: Face, plane: &Plane, out: &mut Vec<Seen>) {
    let e = &prj.layout.enclosures[ri];
    for it in e.items.iter().filter(|it| it.face == face) {
        let Some(pl) = installation::placed(prj, poses, ri, it) else { continue };
        let c = plane.of(face_in_plant(prj, poses, ri, face, it.x + it.w / 2.0, it.y + it.h / 2.0));
        out.push(Seen { at: c, symbol: pl.symbol, tag: pl.tag, item: it.id, height: pl.height });
    }
}

/// The face the openings of a wall face are stored on: a wall's own, or the inside of
/// the wall whose outside it is.
fn openings_face(face: Face) -> Face {
    match face {
        Face::Outside(n) => Face::Wall(n),
        f => f,
    }
}

/// Wall `face` (its key) of room `room` on level `level`.
pub(super) fn wall_view(prj: &Project, poses: &[panel::SitePose], level: u64, room: &str, face: &str) -> Option<Elevation> {
    let layout = &prj.layout;
    let ri = panel::buildings::rooms_on(layout, level).into_iter().find(|&ri| layout.enclosures[ri].name == room)?;
    let e = &layout.enclosures[ri];
    let face = panel::room_face_from_key(e, face).filter(|f| matches!(f, Face::Wall(_) | Face::Inner(..) | Face::Outside(_)) && panel::room_faces(e).contains(f))?;
    let w = panel::side(e, face)?;
    let pose = &poses[ri];
    let flat = |v: V3| [v[0], v[2]];
    let start = flat(pose.apply([w.start()[0], 0.0, w.start()[1]]));
    let plane = Plane { origin: start, right: flat(pose.turn([w.right()[0], 0.0, w.right()[1]])), base: pose.at[1] };
    let mut el = Elevation { title: format!("{} {}", e.name, panel::room_face_title(e, face)), ..Default::default() };
    el.walls.push(([0.0, 0.0], [w.length(), e.height]));
    let through = |o: Opening| matches!(o, Opening::Door | Opening::Window | Opening::Penetration | Opening::Entry);
    let of = openings_face(face);
    for it in e.items.iter().filter(|it| it.face == of && through(it.opening)) {
        let (a, b) = item_rect(prj, poses, ri, of, it, &plane);
        el.openings.push((it.opening, a, b));
    }
    // The other side of an inner wall, the neighbour's side of a shared wall.
    if let Face::Inner(id, s) = face {
        for it in e.items.iter().filter(|it| it.face == Face::Inner(id, 1 - s.min(1)) && through(it.opening)) {
            let (a, b) = item_rect(prj, poses, ri, it.face, it, &plane);
            el.openings.push((it.opening, a, b));
        }
    }
    if let Face::Wall(n) | Face::Outside(n) = face {
        for sw in panel::shared_walls(prj, poses, ri).into_iter().filter(|sw| sw.wall == n) {
            for it in layout.enclosures[sw.other].items.iter().filter(|it| it.face == Face::Wall(sw.other_wall) && through(it.opening)) {
                let (a, b) = item_rect(prj, poses, sw.other, it.face, it, &plane);
                if !el.openings.iter().any(|o| (o.1[0] - a[0]).abs() < 1.0 && (o.2[0] - b[0]).abs() < 1.0) {
                    el.openings.push((it.opening, a, b));
                }
            }
        }
    }
    devices_on(prj, poses, ri, face, &plane, &mut el.devices);
    Some(el)
}

/// Side `side` of building `bi` seen from outside: the walls of its rooms on every level
/// that face that way with no room in front of them.
pub(super) fn facade_view(prj: &Project, poses: &[panel::SitePose], bi: usize, side: FacadeSide) -> Elevation {
    let layout = &prj.layout;
    let b = &layout.buildings[bi];
    let bp = panel::buildings::building_pose(b);
    let (towards, right) = side.axes();
    let turn = |v: [f32; 2]| {
        let t = bp.turn([v[0], 0.0, v[1]]);
        [t[0], t[2]]
    };
    let (towards, right) = (turn(towards), turn(right));
    let plane = Plane { origin: [bp.at[0], bp.at[2]], right, base: bp.at[1] };
    let mut el = Elevation { title: tr(side.title()).to_string(), ground: Some(0.0), ..Default::default() };
    for lv in &b.levels {
        let rooms = panel::buildings::rooms_on(layout, lv.id);
        if rooms.is_empty() {
            continue;
        }
        el.levels.push((lv.name.clone(), lv.elevation));
        for ri in rooms {
            let e = &layout.enclosures[ri];
            let t = e.wall();
            for (n, w) in panel::walls(e).iter().enumerate() {
                let out = poses[ri].turn([-w.normal[0], 0.0, -w.normal[1]]);
                if out[0] * towards[0] + out[2] * towards[1] < 0.99 {
                    continue;
                }
                let face = Face::Outside(n as u16);
                let probe = PanelItem { face, x: w.length() / 2.0 - 1.0, y: e.height / 2.0, w: 2.0, h: 2.0, ..Default::default() };
                if !panel::outside_building(prj, poses, ri, &probe) {
                    continue;
                }
                let o = |q: [f32; 2], y: f32| plane.of(poses[ri].apply([q[0] - w.normal[0] * t, y, q[1] - w.normal[1] * t]));
                let (a, c) = (o(w.a, 0.0), o(w.b, e.height));
                el.walls.push(([a[0].min(c[0]), a[1].min(c[1])], [a[0].max(c[0]), a[1].max(c[1])]));
                for it in e.items.iter().filter(|it| it.face == Face::Wall(n as u16) && it.opening != Opening::Cutout) {
                    let (a, b) = item_rect(prj, poses, ri, it.face, it, &plane);
                    el.openings.push((it.opening, a, b));
                }
                devices_on(prj, poses, ri, face, &plane, &mut el.devices);
            }
        }
    }
    close_gaps(&mut el.walls);
    el
}

/// Widest gap between the walls of two rooms of a façade closed as wall: an inner wall
/// beside (as [`panel::SHARED_WALL_MAX`]) or a ceiling slab below, mm.
const FACADE_GAP: (f32, f32) = (panel::SHARED_WALL_MAX, 1000.0);

/// Walls of a façade that end a little apart (an inner wall between two rooms, the
/// slab between two levels) meet half-way, so that the building reads as one façade.
pub(super) fn close_gaps(walls: &mut [([f32; 2], [f32; 2])]) {
    let before = walls.to_vec();
    for (i, w) in walls.iter_mut().enumerate() {
        for (_, o) in before.iter().enumerate().filter(|(j, _)| *j != i) {
            let a = before[i];
            for k in 0..2 {
                let across = 1 - k;
                // Side by side along k, overlapping across it.
                if a.1[across].min(o.1[across]) - a.0[across].max(o.0[across]) <= 1.0 {
                    continue;
                }
                let max = if k == 0 { FACADE_GAP.0 } else { FACADE_GAP.1 };
                let after = o.0[k] - a.1[k];
                if after > 0.0 && after <= max {
                    w.1[k] = w.1[k].max(a.1[k] + after / 2.0);
                }
                let ahead = a.0[k] - o.1[k];
                if ahead > 0.0 && ahead <= max {
                    w.0[k] = w.0[k].min(a.0[k] - ahead / 2.0);
                }
            }
        }
    }
}

/// Smallest and largest u and v of what an elevation shows.
fn extent(el: &Elevation) -> ([f32; 2], [f32; 2]) {
    let pts: Vec<[f32; 2]> = el.walls.iter().flat_map(|(a, b)| [*a, *b]).chain(el.openings.iter().flat_map(|(_, a, b)| [*a, *b])).chain(el.devices.iter().map(|s| s.at)).collect();
    panel::bounds(&pts)
}

pub(super) fn draw_page(d: &mut dyn Draw, prj: &Project, an: &Analysis, page: &Page, level: u64, frame: &crate::frame::PlotFrame) {
    let a = frame.area();
    let layout = &prj.layout;
    let Some((bi, building, lv)) = layout.buildings.iter().enumerate().find_map(|(bi, b)| b.levels.iter().find(|l| l.id == level).map(|l| (bi, b, l))) else {
        d.text(a.min + p(15.0, 25.0), tr("Level not found - choose another one in the page properties"), 4.0, Anchor::Left, VAnchor::Middle, Ink::Error);
        return;
    };
    let poses = panel::site_poses(prj);
    let view = &page.plan;
    let el = match &view.projection {
        PlanProjection::Wall { room, face } => wall_view(prj, &poses, level, room, face),
        PlanProjection::Facade(side) => Some(facade_view(prj, &poses, bi, *side)),
        _ => None,
    };
    let Some(el) = el.filter(|el| !el.walls.is_empty()) else {
        d.text(a.min + p(15.0, 25.0), tr("Nothing to show - choose a wall or a side of the building in the page properties"), 4.0, Anchor::Left, VAnchor::Middle, Ink::Text);
        return;
    };
    let circuits: Vec<installation::Circuit> = installation::circuits(prj, &an.netlist, &an.connections, None);
    let circuit_of = |item: u64| circuits.iter().position(|c| c.has_item(item));
    let shown: Vec<usize> = (0..circuits.len()).filter(|k| el.devices.iter().any(|s| circuits[*k].has_item(s.item))).collect();
    let (lbox, bottom, (x0, y0, x1, y1)) = legend_layout(view, &circuits, &shown, frame);
    // Up is up on the page: v goes into the second plan coordinate negated.
    let (lo, hi) = extent(&el);
    let (lo, hi) = ([lo[0] - 300.0, -hi[1] - 300.0], [hi[0] + 300.0, -lo[1] + 300.0]);
    let real = ((hi[0] - lo[0]).max(1.0), (hi[1] - lo[1]).max(1.0));
    let fixed = page.scale > 1.0;
    let n = if fixed { page.scale } else { fit_scale(real, (x1 - x0, y1 - y0)) };
    let origin = p(x0 + ((x1 - x0) - real.0 / n) / 2.0, y0 + ((y1 - y0) - real.1 / n) / 2.0);
    let map = Map { lo, origin, n };
    let at = |q: [f32; 2]| map.at([q[0], -q[1]]);
    let mut clip = Clip { d: &mut *d, lo: p(x0, y0), hi: p(x1, y1) };
    let d: &mut dyn Draw = &mut clip;
    let rect = |d: &mut dyn Draw, a: [f32; 2], b: [f32; 2], w: f32| d.poly(&[at(a), at([b[0], a[1]]), at(b), at([a[0], b[1]])], true, w, Ink::Line, false);
    if let Some(g) = el.ground {
        let (u0, u1) = (lo[0], hi[0]);
        d.line(at([u0, g]), at([u1, g]), 0.5, Ink::Line, false);
    }
    for (a, b) in &el.walls {
        rect(d, *a, *b, 0.35);
    }
    for (o, a, b) in &el.openings {
        rect(d, *a, *b, THIN);
        if *o == Opening::Window {
            // The frame of the window inside its opening.
            let inset = 40.0_f32.min((b[0] - a[0]) / 4.0).min((b[1] - a[1]) / 4.0);
            rect(d, [a[0] + inset, a[1] + inset], [b[0] - inset, b[1] - inset], THIN);
        }
    }
    let size = pt_to_mm(view.tag_size);
    for (name, v) in &el.levels {
        d.text(p(at([lo[0], *v]).x + 1.0, at([0.0, *v]).y - size), name, size, Anchor::Left, VAnchor::Middle, Ink::Text);
        d.line(at([lo[0], *v]), at([hi[0], *v]), THIN, Ink::Line, true);
    }
    let sym = SYMBOL * view.symbol_scale.clamp(10.0, 1000.0) / 100.0;
    let ink_of = |k: Option<usize>| match k {
        Some(k) if view.circuit_colors => Ink::Color(circuit_color(k)),
        _ => Ink::Line,
    };
    let mut taken: Vec<(P, P)> = el.devices.iter().map(|s| { let c = at(s.at); (p(c.x - sym, c.y - sym), p(c.x + sym, c.y + sym)) }).collect();
    for (idx, s) in el.devices.iter().enumerate() {
        let k = circuit_of(s.item);
        let c = at(s.at);
        symbol_sized(d, prj, &s.symbol, c, None, ink_of(k.filter(|k| shown.contains(k))), sym);
        // The tag and the height above the floor (mm) beside the symbol.
        let tag = if view.tags { view.tag_parts.apply(&s.tag) } else { String::new() };
        let text = if tag.is_empty() { format!("{:.0}", s.height) } else { format!("{tag}  {:.0}", s.height) };
        let (pos, h, moved) = tag_place(c, p(1.0, 0.0), sym + 1.2, (printed_width(&text, size), size), &taken, idx);
        if moved {
            let r = (pos.x - c.x).hypot(pos.y - c.y).max(1e-3);
            d.line(p(c.x + (pos.x - c.x) / r * sym, c.y + (pos.y - c.y) / r * sym), pos, THIN / 2.0, Ink::Line, false);
        }
        taken.push(tag_box(pos, h, (printed_width(&text, size), size)));
        d.text(pos, &text, size, h, VAnchor::Middle, Ink::Text);
        if let Some(page) = k.and_then(|k| circuits[k].page) {
            d.link(p(c.x - sym, c.y - sym), p(c.x + sym, c.y + sym), page);
        }
    }
    let d = clip.d;
    if let Some(lb) = lbox {
        draw_legend(d, view, &circuits, &shown, lb, bottom);
    }
    let n_text = crate::graphics::scale_text(n);
    let scale = if fixed { tr_fmt!("Scale {}", n_text) } else { tr_fmt!("Scale {} (fitted)", n_text) };
    let caption = format!("{} {}  -  {}  -  {}", building.name, lv.name, el.title, scale);
    d.text(p(a.min.x + 8.0, a.max.y - 7.0), &caption, 3.0, Anchor::Left, VAnchor::Middle, Ink::Text);
}
