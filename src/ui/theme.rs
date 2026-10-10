//! Colors, fonts and egui style of both themes.

use super::*;

/// Colors of one theme. Text pairs meet WCAG 2.2 AA contrast (see the tests).
pub(super) struct Palette {
    /// Window background, behind cards.
    pub(super) bg: Color32,
    /// Cards, top bar, inputs and dialogs.
    pub(super) surface: Color32,
    pub(super) ink: Color32,
    pub(super) ink2: Color32,
    pub(super) muted: Color32,
    pub(super) border: Color32,
    /// Outline-button stroke. Decorative: the label identifies the button.
    pub(super) button_border: Color32,
    /// Boundary of text fields, combo boxes and date buttons (3:1 against `surface`).
    pub(super) field_border: Color32,
    pub(super) hover_border: Color32,
    pub(super) divider: Color32,
    pub(super) chip: Color32,
    pub(super) accent: Color32,
    /// Accent text on `accent_soft`, and hovered primary buttons.
    pub(super) accent_strong: Color32,
    pub(super) accent_soft: Color32,
    pub(super) accent_tint: Color32,
    /// Text and icons on `accent` and `ok` fills.
    pub(super) on_accent: Color32,
    pub(super) selection: Color32,
    pub(super) danger: Color32,
    pub(super) danger_soft: Color32,
    pub(super) ok: Color32,
    pub(super) ok_fg: Color32,
    pub(super) ok_soft: Color32,
    pub(super) ok_border: Color32,
    pub(super) ok_tint: Color32,
    pub(super) warn: Color32,
    pub(super) warn_soft: Color32,
    pub(super) warn_border: Color32,
    pub(super) disabled: Color32,
    /// Swatches and bars, in [`Kind::ALL`] order.
    pub(super) kinds: [Color32; 3],
}

pub(super) const fn pdf_kind(kind: Kind) -> Color32 {
    let (r, g, b) = pdf::kind_rgb(kind);
    Color32::from_rgb(r, g, b)
}

pub(super) const fn hex(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// Palette from the mockup; task-type colors are shared with the PDFs.
pub(super) const LIGHT: Palette = Palette {
    bg: hex(0xF4F5F7),
    surface: Color32::WHITE,
    ink: hex(0x15171C),
    ink2: hex(0x3F4452),
    muted: hex(0x5B6170),
    border: hex(0xE2E4E9),
    button_border: hex(0xD5D8DF),
    field_border: hex(0x8C919E),
    hover_border: hex(0xB9BEC9),
    divider: hex(0xEEF0F3),
    chip: hex(0xF0F1F4),
    accent: hex(0x2446D8),
    accent_strong: hex(0x16309E),
    accent_soft: hex(0xE8ECFC),
    accent_tint: hex(0xF5F7FF),
    on_accent: Color32::WHITE,
    selection: hex(0xC9D3FA),
    danger: hex(0xB42318),
    danger_soft: hex(0xFDECEA),
    ok: hex(0x0F7B6C),
    ok_fg: hex(0x0A5248),
    ok_soft: hex(0xDDF3EF),
    ok_border: hex(0xBFE3DC),
    ok_tint: hex(0xF1FAF8),
    warn: hex(0x8A3C08),
    warn_soft: hex(0xFFF1E6),
    warn_border: hex(0xE9B48C),
    disabled: hex(0x8A8F9C),
    kinds: [
        pdf_kind(Kind::Frontend),
        pdf_kind(Kind::Backend),
        pdf_kind(Kind::Support),
    ],
};

/// Same hues as [`LIGHT`], lightened to read on dark surfaces; fills carry dark text.
pub(super) const DARK: Palette = Palette {
    bg: hex(0x111318),
    surface: hex(0x1A1D24),
    ink: hex(0xE8EAEF),
    ink2: hex(0xC3C7D1),
    muted: hex(0x9BA1AF),
    border: hex(0x2E333D),
    button_border: hex(0x3B414D),
    field_border: hex(0x6B7280),
    hover_border: hex(0x5A6170),
    divider: hex(0x262A33),
    chip: hex(0x272B35),
    accent: hex(0x8AA4FF),
    accent_strong: hex(0xB4C5FF),
    accent_soft: hex(0x243055),
    accent_tint: hex(0x1D2337),
    on_accent: hex(0x0E1220),
    selection: hex(0x2B3766),
    danger: hex(0xFF8F84),
    danger_soft: hex(0x3D1D1B),
    ok: hex(0x45C7B2),
    ok_fg: hex(0x7EE0CE),
    ok_soft: hex(0x15362F),
    ok_border: hex(0x245049),
    ok_tint: hex(0x14272A),
    warn: hex(0xF4AA6E),
    warn_soft: hex(0x3A2816),
    warn_border: hex(0x8C5B30),
    disabled: hex(0x6B7180),
    kinds: [hex(0x8AA4FF), hex(0xF0955E), hex(0x45C7B2)],
};

impl Palette {
    pub(super) fn of(visuals: &egui::Visuals) -> &'static Palette {
        if visuals.dark_mode { &DARK } else { &LIGHT }
    }

    pub(super) fn kind(&self, kind: Kind) -> Color32 {
        self.kinds[kind as usize]
    }
}

pub(super) fn theme_preference(appearance: Appearance) -> egui::ThemePreference {
    match appearance {
        Appearance::System => egui::ThemePreference::System,
        Appearance::Light => egui::ThemePreference::Light,
        Appearance::Dark => egui::ThemePreference::Dark,
    }
}

/// Palette of the theme `ui` is drawn with.
pub(super) fn colors(ui: &Ui) -> &'static Palette {
    Palette::of(ui.visuals())
}

#[derive(Clone, Copy)]
pub(super) enum W {
    Regular,
    Medium,
    SemiBold,
    Bold,
}

pub(super) fn sans(size: f32, weight: W) -> FontId {
    let family = match weight {
        W::Regular => FontFamily::Proportional,
        W::Medium => FontFamily::Name("medium".into()),
        W::SemiBold => FontFamily::Name("semibold".into()),
        W::Bold => FontFamily::Name("bold".into()),
    };
    FontId::new(size, family)
}

pub(super) fn mono(size: f32, semibold: bool) -> FontId {
    if semibold {
        FontId::new(size, FontFamily::Name("mono-semibold".into()))
    } else {
        FontId::monospace(size)
    }
}

pub(super) fn setup_fonts(ctx: &egui::Context) {
    let mut defs = FontDefinitions::default();
    let faces = [
        ("sans-regular", fonts::SANS_REGULAR),
        ("sans-medium", fonts::SANS_MEDIUM),
        ("sans-semibold", fonts::SANS_SEMIBOLD),
        ("sans-bold", fonts::SANS_BOLD),
        ("mono-regular", fonts::MONO_REGULAR),
        ("mono-semibold", fonts::MONO_SEMIBOLD),
    ];
    for (name, bytes) in faces {
        defs.font_data
            .insert(name.into(), Arc::new(FontData::from_static(bytes)));
    }
    let fallback = defs
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    defs.families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "sans-regular".into());
    defs.families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "mono-regular".into());
    for (family, face) in [
        ("medium", "sans-medium"),
        ("semibold", "sans-semibold"),
        ("bold", "sans-bold"),
        ("mono-semibold", "mono-semibold"),
    ] {
        let mut list = vec![face.to_owned()];
        list.extend(fallback.iter().cloned());
        defs.families.insert(FontFamily::Name(family.into()), list);
    }
    ctx.set_fonts(defs);
}

/// Styles both themes; [`App::ui`] picks one from [`Settings::appearance`].
pub(super) fn setup_style(ctx: &egui::Context) {
    for (theme, c) in [(egui::Theme::Light, &LIGHT), (egui::Theme::Dark, &DARK)] {
        ctx.style_mut_of(theme, |style| {
            style
                .text_styles
                .insert(egui::TextStyle::Body, sans(15.0, W::Regular));
            style
                .text_styles
                .insert(egui::TextStyle::Button, sans(15.0, W::Regular));
            style
                .text_styles
                .insert(egui::TextStyle::Heading, sans(30.0, W::Bold));
            style
                .text_styles
                .insert(egui::TextStyle::Small, sans(13.0, W::Regular));
            style.spacing.item_spacing = vec2(8.0, 8.0);
            let v = &mut style.visuals;
            v.panel_fill = c.bg;
            v.window_fill = c.surface;
            v.extreme_bg_color = c.surface;
            v.override_text_color = None;
            v.weak_text_color = Some(c.muted);
            v.selection.bg_fill = c.selection;
            v.selection.stroke = Stroke::new(2.0, c.accent);
            v.text_cursor.stroke = Stroke::new(2.0, c.accent);
            v.hyperlink_color = c.accent;
            let radius = CornerRadius::same(10);
            for w in [
                &mut v.widgets.noninteractive,
                &mut v.widgets.inactive,
                &mut v.widgets.hovered,
                &mut v.widgets.active,
                &mut v.widgets.open,
            ] {
                w.corner_radius = radius;
                w.expansion = 0.0;
                w.fg_stroke.color = c.ink;
            }
            v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, c.border);
            v.window_stroke = Stroke::new(1.0, c.hover_border);
            v.widgets.inactive.weak_bg_fill = c.chip;
            v.widgets.hovered.weak_bg_fill = c.border;
            v.widgets.active.weak_bg_fill = c.border;
            v.widgets.inactive.bg_stroke = Stroke::new(1.0, c.field_border);
            v.widgets.hovered.bg_stroke = Stroke::new(1.0, c.muted);
            v.widgets.active.bg_stroke = Stroke::new(1.0, c.accent);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WCAG 2.2 contrast ratio between two opaque colors.
    fn contrast(a: Color32, b: Color32) -> f32 {
        let luminance = |c: Color32| {
            let channel = |v: u8| {
                let v = v as f32 / 255.0;
                if v <= 0.04045 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * channel(c.r()) + 0.7152 * channel(c.g()) + 0.0722 * channel(c.b())
        };
        let (la, lb) = (luminance(a), luminance(b));
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }

    fn palettes() -> [(&'static str, &'static Palette); 2] {
        [("light", &LIGHT), ("dark", &DARK)]
    }

    #[test]
    fn contrast_matches_known_values() {
        assert!((contrast(Color32::BLACK, Color32::WHITE) - 21.0).abs() < 0.01);
        // #767676 is the lightest gray with 4.5:1 on white.
        let gray = Color32::from_gray(0x76);
        assert!((contrast(gray, Color32::WHITE) - 4.54).abs() < 0.01);
    }

    #[test]
    fn text_meets_wcag_aa_contrast() {
        for (theme, c) in palettes() {
            let mut pairs = vec![
                ("accent", c.accent, "accent_tint", c.accent_tint),
                ("accent", c.accent, "selection", c.selection),
                (
                    "accent_strong",
                    c.accent_strong,
                    "accent_soft",
                    c.accent_soft,
                ),
                ("danger", c.danger, "danger_soft", c.danger_soft),
                ("ink", c.ink, "selection", c.selection),
                ("ink2", c.ink2, "border", c.border),
                ("muted", c.muted, "accent_tint", c.accent_tint),
                ("ok_fg", c.ok_fg, "ok_soft", c.ok_soft),
                ("ok_fg", c.ok_fg, "ok_tint", c.ok_tint),
                ("warn", c.warn, "warn_soft", c.warn_soft),
                ("on_accent", c.on_accent, "accent", c.accent),
                ("on_accent", c.on_accent, "accent_strong", c.accent_strong),
                ("surface", c.surface, "ink", c.ink),
                ("surface", c.surface, "ink2", c.ink2),
            ];
            for (bg_name, bg) in [("surface", c.surface), ("bg", c.bg), ("chip", c.chip)] {
                for (fg_name, fg) in [
                    ("ink", c.ink),
                    ("ink2", c.ink2),
                    ("muted", c.muted),
                    ("accent", c.accent),
                    ("danger", c.danger),
                    ("ok_fg", c.ok_fg),
                    ("warn", c.warn),
                ] {
                    pairs.push((fg_name, fg, bg_name, bg));
                }
            }
            for (fg_name, fg, bg_name, bg) in pairs {
                let ratio = contrast(fg, bg);
                assert!(
                    ratio >= 4.5,
                    "{theme}: {fg_name} on {bg_name} is {ratio:.2}:1"
                );
            }
        }
    }

    #[test]
    fn controls_meet_wcag_aa_contrast() {
        for (theme, c) in palettes() {
            let mut pairs = vec![
                ("field_border", c.field_border, "surface", c.surface),
                ("accent", c.accent, "surface", c.surface),
                ("ok", c.ok, "surface", c.surface),
                ("muted", c.muted, "surface", c.surface),
                ("danger", c.danger, "surface", c.surface),
                ("on_accent", c.on_accent, "ok", c.ok),
            ];
            for kind in Kind::ALL {
                pairs.push((kind.label(), c.kind(kind), "surface", c.surface));
                pairs.push((kind.label(), c.kind(kind), "divider", c.divider));
            }
            for (fg_name, fg, bg_name, bg) in pairs {
                let ratio = contrast(fg, bg);
                assert!(
                    ratio >= 3.0,
                    "{theme}: {fg_name} on {bg_name} is {ratio:.2}:1"
                );
            }
        }
    }

    #[test]
    fn task_type_colors_have_distinct_hues() {
        for (theme, c) in palettes() {
            let hues: Vec<f32> = Kind::ALL
                .iter()
                .map(|&k| egui::ecolor::Hsva::from(c.kind(k)).h * 360.0)
                .collect();
            for (i, a) in hues.iter().enumerate() {
                for b in &hues[i + 1..] {
                    let gap = (a - b).abs().min(360.0 - (a - b).abs());
                    assert!(gap >= 60.0, "{theme}: task-type hues {a:.0}° and {b:.0}°");
                }
            }
        }
    }
}
