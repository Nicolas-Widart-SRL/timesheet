//! Settings dialog, also shown on first run.

use super::*;

impl App {
    /// Returns true when settings were saved.
    pub(super) fn settings_modal(&mut self, ctx: &egui::Context) -> bool {
        let Some(draft) = self.settings_draft.as_mut() else {
            return false;
        };
        let mut save = false;
        let mut cancel = false;
        let mut export = false;
        let mut import = false;
        let c = Palette::of(&ctx.global_style().visuals);
        let frame = Frame::new()
            .fill(c.surface)
            .corner_radius(14)
            .inner_margin(Margin::same(24))
            .stroke(Stroke::new(1.0, c.border));
        let modal = egui::Modal::new(Id::new("settings"))
            .frame(frame)
            .show(ctx, |ui| {
                ui.set_width(440.0);
                ui.spacing_mut().item_spacing.y = 6.0;
                text(ui, "Settings", sans(20.0, W::Bold), c.ink);
                text(
                    ui,
                    "Shown in the app header and on both PDF exports.",
                    sans(14.0, W::Regular),
                    c.muted,
                );
                ui.add_space(10.0);
                for (label, value, hint) in [
                    ("Consultant name", &mut draft.consultant, "e.g. Jane Doe"),
                    ("Client name", &mut draft.client, "e.g. Acme Corp"),
                    ("Project name", &mut draft.project, "e.g. Website redesign"),
                ] {
                    text(ui, label, sans(14.0, W::SemiBold), c.ink2);
                    ui.add(
                        TextEdit::singleline(value)
                            .hint_text(hint)
                            .font(sans(15.0, W::Regular))
                            .text_color(c.ink)
                            .margin(Margin::symmetric(12, 0))
                            .vertical_align(Align::Center)
                            .min_size(vec2(0.0, 44.0))
                            .desired_width(f32::INFINITY),
                    );
                    ui.add_space(8.0);
                }
                text(ui, "Public holidays", sans(14.0, W::SemiBold), c.ink2);
                let country_label = |c: Option<Country>| c.map_or("None", Country::label);
                egui::ComboBox::from_id_salt("country")
                    .selected_text(
                        egui::RichText::new(country_label(draft.country))
                            .font(sans(15.0, W::Regular))
                            .color(c.ink),
                    )
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for c in std::iter::once(None).chain(Country::ALL.map(Some)) {
                            ui.selectable_value(&mut draft.country, c, country_label(c));
                        }
                    })
                    .response
                    .widget_info(|| {
                        WidgetInfo::labeled(WidgetType::ComboBox, true, "Public holidays")
                    });
                text(
                    ui,
                    "Marked as days off unless you mark them as workdays.",
                    sans(13.0, W::Regular),
                    c.muted,
                );
                ui.add_space(8.0);
                text(ui, "Appearance", sans(14.0, W::SemiBold), c.ink2);
                egui::ComboBox::from_id_salt("appearance")
                    .selected_text(
                        egui::RichText::new(draft.appearance.label())
                            .font(sans(15.0, W::Regular))
                            .color(c.ink),
                    )
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for a in Appearance::ALL {
                            ui.selectable_value(&mut draft.appearance, a, a.label());
                        }
                    })
                    .response
                    .widget_info(|| WidgetInfo::labeled(WidgetType::ComboBox, true, "Appearance"));
                text(
                    ui,
                    "System follows your computer's light or dark mode. PDFs stay light.",
                    sans(13.0, W::Regular),
                    c.muted,
                );
                ui.add_space(8.0);
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(format!(
                            "Data is saved automatically in {}",
                            self.data_path.display()
                        ))
                        .font(sans(13.0, W::Regular))
                        .color(c.muted),
                    )
                    .wrap(),
                );
                ui.horizontal(|ui| {
                    export = Btn::outline(c, "Export data…").show(ui).clicked();
                    import = Btn::outline(c, "Import data…").show(ui).clicked();
                });
                if let Some(note) = &self.backup_note {
                    note_ui(ui, note);
                }
                ui.add_space(10.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if Btn::primary(c, "Save").min(96.0, 44.0).show(ui).clicked() {
                        save = true;
                    }
                    if Btn::outline(c, "Cancel").show(ui).clicked() {
                        cancel = true;
                    }
                });
            });
        if export {
            self.export_data();
        }
        if import {
            self.import_data();
            return false;
        }
        if save {
            let draft = self.settings_draft.take().unwrap_or_default();
            self.store.data.settings = Settings {
                consultant: draft.consultant.trim().to_owned(),
                client: draft.client.trim().to_owned(),
                project: draft.project.trim().to_owned(),
                country: draft.country,
                appearance: draft.appearance,
            };
            return true;
        }
        if cancel || modal.should_close() {
            self.settings_draft = None;
        }
        false
    }
}
