//! Installation plan pages in the page editor: the page itself is generated, but its
//! installation devices are selected with a click, moved by dragging (onto another wall
//! or into another room of the level), taken out with Delete, and parts dragged onto
//! the page are put into the room they are dropped in. The functions are those of the
//! Buildings pane and the commands (`openecraft_core::installation`), so that the
//! schematic, the site plan, the 3D view and every other page follow.

use super::*;
use openecraft_core::installation;
use openecraft_core::render::installation_plan::{self as plan, PageMap};

/// What the editor holds while an installation plan page is worked on.
#[derive(Default)]
pub struct PlanEdit {
    /// The device selected (room and item id).
    pub selected: Option<(usize, u64)>,
    /// The device being dragged.
    dragging: Option<(usize, u64)>,
    /// The page (its id) the selection belongs to.
    page: u64,
}

impl PlanEdit {
    /// The page shown is `page` (its id): a selection of another page goes, so that
    /// Delete never takes out a device that is not on the page.
    pub fn on_page(&mut self, page: u64) {
        if self.page != page {
            *self = PlanEdit { page, ..Default::default() };
        }
    }
}

/// Where device `item` is on the page.
fn device_point(prj: &Project, map: &PageMap, item: u64) -> Option<P> {
    let poses = openecraft_core::panel::site_poses(prj);
    installation::items(prj, &poses).into_iter().find(|i| i.item == item).map(|i| map.page(i.at))
}

impl Editor {
    /// The page can be worked on as an installation plan: one, not released, not locked
    /// by another user.
    pub(super) fn plan_editable(&self, prj: &Project, page_i: usize) -> bool {
        let pg = &prj.pages[page_i];
        // A wall or a façade is looked at only; the devices are moved in the floor plan.
        matches!(pg.kind, PageKind::InstallationPlan(_)) && !pg.plan.projection.is_elevation() && !pg.locked && !self.read_only
    }

    /// Clicks, drags, drops and Delete on an installation plan page; true when they were
    /// for a device (the rest is left to the selection of the page).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn plan_mouse(&mut self, ui: &egui::Ui, resp: &egui::Response, prj: &mut Project, page_i: usize, (rect, world): (Rect, Option<P>), an: &Analysis, out: &mut CanvasOutput) -> bool {
        self.plan.on_page(prj.pages[page_i].id);
        let Some(map) = plan::page_map(prj, an, page_i) else { return false };
        // A device taken out in the meantime (undo, another view) is no longer selected.
        if self.plan.selected.is_some_and(|(ri, id)| prj.layout.enclosures.get(ri).and_then(|e| e.item(id)).is_none()) {
            self.plan.selected = None;
        }
        if let Some(pl) = crate::dropped::<PartDrag>(resp) {
            if let Some(w) = world {
                out.message = Some(self.plan_put_in(prj, &map, &pl.number, w));
            }
            return true;
        }
        if resp.drag_started_by(PointerButton::Primary) {
            // Where the button went down: the drag starts a little away from it.
            let pressed = ui.input(|i| i.pointer.press_origin()).map(|s| self.s2w(rect, s)).or(world);
            self.plan.dragging = pressed.and_then(|w| plan::device_at(prj, an, page_i, w));
            if self.plan.dragging.is_some() {
                self.plan.selected = self.plan.dragging;
            }
        }
        if let Some((ri, id)) = self.plan.dragging {
            if resp.drag_stopped() || !ui.input(|i| i.pointer.primary_down()) {
                self.plan.dragging = None;
                if let Some(w) = world.filter(|w| map.inside(*w)) {
                    out.message = Some(match installation::move_item(prj, ri, id, map.plant(w)) {
                        Ok(to) => {
                            self.plan.selected = Some((to, id));
                            tr_fmt!("Moved into {}", prj.layout.enclosures[to].name)
                        }
                        Err(e) => e,
                    });
                }
            }
            return true;
        }
        if resp.clicked_by(PointerButton::Primary) {
            self.plan.selected = world.and_then(|w| plan::device_at(prj, an, page_i, w));
            if self.plan.selected.is_some() {
                return true;
            }
        }
        let del = self.keys_active && !ui.ctx().egui_wants_keyboard_input() && ui.input(|i| self.shortcuts.cmd_pressed(i, Cmd::Delete));
        if let (true, Some((ri, id))) = (del, self.plan.selected) {
            self.plan.selected = None;
            out.message = Some(match installation::remove_item(prj, ri, id) {
                Ok(tag) if tag.is_empty() => tr("Taken out of the room").to_string(),
                Ok(tag) => tr_fmt!("{} taken out of the room; it stays in the schematic", tag),
                Err(e) => e,
            });
            return true;
        }
        false
    }

    /// Part `number` dropped at `at` (page mm) goes into the room of the level there.
    pub(super) fn plan_put_in(&mut self, prj: &mut Project, map: &PageMap, number: &str, at: P) -> String {
        if installation::find_part(prj, number).and_then(installation::kind_of).is_none() {
            return tr_fmt!("{} is no socket outlet, switch, light or other device of the installation", number);
        }
        if !map.inside(at) {
            return tr("Click in a room").to_string();
        }
        let rooms = openecraft_core::panel::buildings::rooms_on(&prj.layout, map.level);
        let q = map.plant(at);
        let Some(ri) = installation::room_at(prj, &rooms, q) else { return tr("Click in a room").to_string() };
        match installation::place(prj, ri, number, q) {
            Ok(id) => {
                self.plan.selected = Some((ri, id));
                tr_fmt!("{} put into {}", number, prj.layout.enclosures[ri].name)
            }
            Err(e) => e,
        }
    }

    /// The selected device ringed, the one dragged where it goes.
    pub(super) fn draw_plan_edit(&self, painter: &Painter, (rect, world): (Rect, Option<P>), pal: &Palette, prj: &Project, an: &Analysis, page_i: usize) {
        if self.plan.page != prj.pages[page_i].id || (self.plan.selected.is_none() && self.plan.dragging.is_none()) {
            return;
        }
        let Some(map) = plan::page_map(prj, an, page_i) else { return };
        let r = 3.6 * prj.pages[page_i].plan.symbol_scale.clamp(10.0, 1000.0) / 100.0 * self.zoom;
        if let Some(c) = self.plan.selected.and_then(|(_, id)| device_point(prj, &map, id)) {
            painter.circle_stroke(self.w2s(rect, c), r, Stroke::new(1.5, pal.selected));
        }
        if let (Some((_, id)), Some(w)) = (self.plan.dragging, world) {
            if let Some(c) = device_point(prj, &map, id) {
                painter.line_segment([self.w2s(rect, c), self.w2s(rect, w)], Stroke::new(1.0, pal.ghost));
            }
            painter.circle_stroke(self.w2s(rect, w), r, Stroke::new(1.5, pal.ghost));
        }
    }
}
