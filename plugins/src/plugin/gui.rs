use egui::{CollapsingHeader, RichText, Style, TextFormat, WidgetText, text::LayoutJob};

use crate::{
    perm_tree::{filesystem::*, *},
    permission::{Permission, helpers::AllOr},
    plugin::Plugin,
};

/// The user's decision on a permission request. `Pending` until a button is
/// clicked; the host reads this after each frame to know what to do.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PermOutcome {
    #[default]
    Pending,
    Granted,
    Denied,
}

pub struct PluginPermRequest<'a> {
    pub plugin: &'a Plugin,
    pub show_none: bool,
    pub outcome: PermOutcome,
}

impl<'a> PluginPermRequest<'a> {
    fn perm_label(perm: &impl Permission, name: &str) -> impl Into<WidgetText> {
        let mut job = LayoutJob::default();
        job.append(name, 0.0, Default::default());
        if perm.is_none() {
            job.append(
                "none",
                12.0,
                TextFormat {
                    italics: true,
                    ..Default::default()
                },
            );
        }
        if perm.is_all() {
            job.append(
                "all",
                12.0,
                TextFormat {
                    color: Style::default().visuals.strong_text_color(),
                    italics: true,
                    ..Default::default()
                },
            );
        }

        job
    }
    fn perm_collapsing_header(perm: &impl Permission, name: &str) -> CollapsingHeader {
        CollapsingHeader::new(Self::perm_label(perm, name))
    }
    fn force_show_perm_collapsible<T: Permission>(
        &self,
        ui: &mut egui::Ui,
        perm: &T,
        label: &str,
        draw_content: impl FnOnce(&mut egui::Ui, &T),
    ) {
        Self::perm_collapsing_header(perm, label)
            .default_open(!perm.is_all() && !perm.is_none())
            .show(ui, |ui| draw_content(ui, perm));
    }
    fn show_perm_collapsible<T: Permission>(
        &self,
        ui: &mut egui::Ui,
        perm: &T,
        label: &str,
        draw_content: impl FnOnce(&mut egui::Ui, &T),
    ) {
        self.show_perm(ui, perm, |ui, perm| {
            Self::force_show_perm_collapsible(self, ui, perm, label, draw_content);
        });
    }
    fn show_perm<T: Permission>(
        &self,
        ui: &mut egui::Ui,
        perm: &T,
        add_content: impl FnOnce(&mut egui::Ui, &T),
    ) {
        if !perm.is_none() || self.show_none {
            add_content(ui, perm);
        }
    }
    fn show_perm_bool(&self, ui: &mut egui::Ui, perm: bool, label: &str) {
        self.show_perm(ui, &perm, |ui, perm| {
            ui.label(Self::perm_label(perm, label));
        });
    }

    pub fn show_gui(&mut self, ui: &mut egui::Ui) {
        ui.separator();

        ui.label(RichText::new("Requested permissions").heading());
        let RSnesPermissions { internal, external } = &self.plugin.table.perms;

        self.force_show_perm_collapsible(ui, internal, "Internal", |ui, internal| {
            // we intentionally destructure the struct listing out all fields (without `..`),
            // so that we get a compile error in case we forget to list a field in the
            // destructure (and a warning if we write it just below but don't use it).
            // This guarantees that we render all requested permissions in the GUI,
            // which guarantees some security to the user (they at least know what is
            // requested)
            let InternalPermissions {
                control,
                cpu,
                ppu,
                bus,
                input,
            } = internal;

            self.show_perm_collapsible(ui, cpu, "CPU", |ui, cpu| {
                let CpuPermissions { registers } = cpu;
                self.show_perm_bool(ui, *registers, "Registers");
            });
            self.show_perm_collapsible(ui, bus, "Bus", |ui, bus| {
                let BusPermissions { read, write } = bus;
                self.show_perm_bool(ui, *read, "Read");
                self.show_perm_bool(ui, *write, "Write");
            });
            self.show_perm_collapsible(ui, ppu, "PPU", |ui, ppu| {
                let PpuPermissions { display } = ppu;
                self.show_perm_bool(ui, *display, "Display");
            });
            self.show_perm_bool(ui, *input, "Input");
            self.show_perm_collapsible(ui, control, "Control", |ui, control| {
                let ControlPermissions { dialog, pause } = control;
                self.show_perm_bool(ui, *pause, "Pause");
                self.show_perm_bool(ui, *dialog, "Dialog");
            });
        });

        self.force_show_perm_collapsible(ui, external, "External", |ui, external| {
            let ExternalPermissions { filesystem, http } = external;

            self.show_perm_bool(ui, *http, "HTTP");
            self.show_perm_collapsible(ui, filesystem, "Filesystem", |ui, fs| {
                let FileSystemPermissions { read, write } = fs;
                self.show_perm_bool(ui, *read, "Read");
                self.show_perm_collapsible(ui, write, "Write", |ui, write| match write {
                    AllOr::All => {
                        ui.label(RichText::new("all").strong());
                    }
                    AllOr::Inner(files) => {
                        for (file, options) in files.files.iter() {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 0.;
                                ui.label(RichText::new(format!("{file:?}")).monospace());
                                let label = match options {
                                    FileWriteOptions::NewOnly => "NewOnly",
                                    FileWriteOptions::CanOverwrite { create, mode } => &format!(
                                        ": {}{mode:?}",
                                        if *create { "Create + " } else { "" }
                                    ),
                                };
                                ui.label(label);
                            });
                        }
                    }
                })
            });
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_none, "Show 'none' fields");
        });

        ui.separator();
        ui.add_space(16.0);

        ui.horizontal(|ui| {
            if ui.button("Grant requested permissions").clicked() {
                self.outcome = PermOutcome::Granted;
            }
            if ui.button("Cancel plugin execution").clicked() {
                self.outcome = PermOutcome::Denied;
            }
        });
    }
}
