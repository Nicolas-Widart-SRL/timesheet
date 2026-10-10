//! Line description field with ticket completion: typing "#508" lists the matching tickets below it.

use std::ops::Range;

use egui::text::{CCursor, CCursorRange, TextWrapping};
use egui::text_edit::TextEditState;
use egui::{EventFilter, Spinner};

use super::*;
use crate::tickets::{self, Search, Ticket, Tickets};

/// Ticket search and the open ticket list, shared by every description field.
pub(super) struct TicketPicker {
    tickets: Tickets,
    /// Line whose ticket list is open, and its highlighted row.
    open: Option<(u64, usize)>,
    /// Line whose field takes focus back on the next frame, after a click on the list.
    refocus: Option<u64>,
}

/// What the ticket list shows.
enum List {
    Loading,
    Tickets(Vec<Ticket>),
}

impl TicketPicker {
    pub(super) fn new(tickets: Tickets) -> Self {
        Self {
            tickets,
            open: None,
            refocus: None,
        }
    }

    /// The description field of line `line`. Typing a ticket number opens the list of matching
    /// tickets; Up/Down highlight one, Enter or a click puts its label in the description, Esc closes
    /// the list. Returns the field's response and whether a ticket was chosen.
    pub(super) fn desc_field(
        &mut self,
        ui: &mut Ui,
        line: u64,
        desc: &mut String,
        a11y: String,
    ) -> (Response, bool) {
        let c = colors(ui);
        let id = ui.make_persistent_id(("desc", line));
        let mut selected = self.open.filter(|&(l, _)| l == line).map(|(_, s)| s);
        let mut enter = None;
        let mut chosen = false;
        if self.refocus == Some(line) {
            self.refocus = None;
            ui.memory_mut(|m| m.request_focus(id));
        }

        // The list drawn last frame takes its keys before the field sees them.
        if let Some(sel) = &mut selected
            && ui.memory(|m| m.has_focus(id))
        {
            let cursor = TextEditState::load(ui.ctx(), id)
                .and_then(|s| s.cursor.char_range())
                .map(|r| r.primary.index.0);
            let mut visible = false;
            if let Some((range, list)) = self.list(desc, cursor) {
                visible = true;
                ui.input_mut(|i| {
                    if i.consume_key(Modifiers::NONE, Key::Escape) {
                        visible = false;
                        return;
                    }
                    let List::Tickets(hits) = &list else { return };
                    *sel = (*sel).min(hits.len() - 1);
                    if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                        *sel = (*sel + 1) % hits.len();
                    }
                    if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                        *sel = (*sel + hits.len() - 1) % hits.len();
                    }
                    if i.consume_key(Modifiers::NONE, Key::Enter) {
                        enter = Some((range, hits[*sel].clone()));
                    }
                });
            }
            if !visible {
                selected = None;
            }
        }
        if let Some((range, ticket)) = enter {
            choose(ui, id, desc, range, &ticket);
            chosen = true;
            selected = None;
        }

        let output = TextEdit::singleline(desc)
            .id(id)
            .hint_text("What did you do? e.g. ticket #1234")
            .font(sans(15.0, W::Regular))
            .text_color(c.ink)
            .vertical_align(Align::Center)
            .margin(Margin::symmetric(12, 0))
            .min_size(vec2(0.0, 44.0))
            .desired_width(ui.available_width())
            .show(ui);
        let response = output.response.response;
        response.widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, &a11y));
        if response.changed() {
            selected = Some(0);
        }

        // A click on a row takes focus from the field in the frame it lands, so the list still
        // shows in that frame.
        let focused = response.has_focus() || response.lost_focus();
        if let Some(sel) = &mut selected
            && focused
            && let Some((range, list)) =
                self.list(desc, output.cursor_range.map(|r| r.primary.index.0))
        {
            // While the list shows, the next Esc closes it instead of leaving the field.
            let filter = EventFilter {
                horizontal_arrows: true,
                vertical_arrows: true,
                escape: true,
                ..Default::default()
            };
            ui.memory_mut(|m| m.set_focus_lock_filter(id, filter));
            let width = response.rect.width();
            let mut clicked = None;
            egui::Popup::from_response(&response)
                .width(width)
                .gap(4.0)
                .show(|ui| {
                    ui.set_width(width - 12.0);
                    ui.spacing_mut().item_spacing.y = 2.0;
                    match &list {
                        List::Loading => {
                            ui.horizontal(|ui| {
                                ui.set_height(36.0);
                                ui.add_space(10.0);
                                ui.add(Spinner::new().size(16.0).color(c.muted));
                                text(ui, "Searching tickets…", sans(15.0, W::Regular), c.muted);
                            });
                        }
                        List::Tickets(hits) => {
                            *sel = (*sel).min(hits.len() - 1);
                            for (k, ticket) in hits.iter().enumerate() {
                                if ticket_row(ui, ticket, k == *sel) {
                                    clicked = Some(ticket.clone());
                                }
                            }
                        }
                    }
                });
            if let Some(ticket) = clicked {
                choose(ui, id, desc, range, &ticket);
                // Not at once: the click still counts as a click outside the field this frame.
                self.refocus = Some(line);
                chosen = true;
                selected = None;
            }
        } else if !focused {
            selected = None;
        }

        match selected {
            Some(sel) => self.open = Some((line, sel)),
            None if self.open.is_some_and(|(l, _)| l == line) => self.open = None,
            None => {}
        }
        (response, chosen)
    }

    /// The list for the ticket number typed before `cursor`, if there is one to show.
    fn list(&mut self, desc: &str, cursor: Option<usize>) -> Option<(Range<usize>, List)> {
        let (range, digits) = tickets::query_at(desc, cursor?)?;
        match self.tickets.search(&digits) {
            Search::Off | Search::Found([]) => None,
            Search::Loading => Some((range, List::Loading)),
            Search::Found(hits) => Some((range, List::Tickets(hits.to_vec()))),
        }
    }
}

/// Replaces the ticket number in `range` with the ticket's label and puts the cursor after it.
fn choose(ui: &Ui, id: Id, desc: &mut String, range: Range<usize>, ticket: &Ticket) {
    let (text, cursor) = tickets::complete(desc, range, ticket);
    *desc = text;
    let mut state = TextEditState::load(ui.ctx(), id).unwrap_or_default();
    state
        .cursor
        .set_char_range(Some(CCursorRange::one(CCursor::new(cursor))));
    state.store(ui.ctx(), id);
    ui.ctx().request_repaint();
}

/// One ticket in the list: its number, then its title cut to the row. Returns true when clicked.
fn ticket_row(ui: &mut Ui, ticket: &Ticket, selected: bool) -> bool {
    let c = colors(ui);
    let (rect, item) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::click());
    item.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, selected, ticket.label()));
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        if selected || item.hovered() {
            p.rect_filled(rect, 8, c.chip);
        }
        let number = p.layout_no_wrap(format!("#{}", ticket.id), mono(14.0, true), c.ink);
        let left = rect.left() + 10.0;
        p.galley(
            pos2(left, rect.center().y - number.size().y / 2.0),
            number.clone(),
            c.ink,
        );
        let title_left = left + number.size().x + 10.0;
        let mut job =
            LayoutJob::simple_singleline(ticket.title.clone(), sans(15.0, W::Regular), c.ink2);
        job.wrap = TextWrapping::truncate_at_width(rect.right() - 10.0 - title_left);
        let title = p.layout_job(job);
        p.galley(
            pos2(title_left, rect.center().y - title.size().y / 2.0),
            title,
            c.ink2,
        );
    }
    item.on_hover_cursor(CursorIcon::PointingHand).clicked()
}
