use crate::{EEGData, EEGInfo};
use egui_plot::{Line, Plot, PlotPoints};
use egui::{Color32, Key};

pub struct TemplateApp {
    // Core data
    info: EEGInfo,
    data: EEGData,

    // Display settings
    selected_channels: Vec<bool>,
    channel_spacing: f64,
    global_gain: f64,
    time_window: f64,
    time_offset: f64,
    total_duration: f64,

    // Channel colors
    channel_colors: Vec<Color32>,

    // Display options
    show_grid: bool,
    baseline_correction: bool,
    downsampling: usize,

    // UI state
    show_help: bool,
    keyboard_enabled: bool,
}

impl TemplateApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, info: EEGInfo, data: EEGData) -> Self {
        let num_channels = info.num_ch as usize;

        // Generate unique colors for each channel
        let mut channel_colors = Vec::new();
        for i in 0..num_channels {
            let hue = (i as f32 / num_channels as f32) * 360.0;
            let r = ((1.0 - 0.5 * (hue.to_radians()).cos()) * 255.0) as u8;
            let g = ((1.0 - 0.5 * ((hue - 120.0).to_radians()).cos()) * 255.0) as u8;
            let b = ((1.0 - 0.5 * ((hue - 240.0).to_radians()).cos()) * 255.0) as u8;
            channel_colors.push(Color32::from_rgb(r, g, b));
        }

        // Calculate total duration
        let total_samples = if data.data.ncols() > 0 { data.data.ncols() } else { 1 };
        let total_duration = total_samples as f64 / info.sfreq as f64;

        // Start with first 4 channels selected
        let mut selected_channels = vec![false; num_channels];
        for i in 0..std::cmp::min(4, num_channels) {
            selected_channels[i] = true;
        }

        Self {
            info,
            data,
            selected_channels,
            channel_spacing: 50.0,
            global_gain: 1000.0,
            time_window: 10.0,
            time_offset: 5.0,
            total_duration,
            channel_colors,
            show_grid: true,
            baseline_correction: true,
            downsampling: 10,
            show_help: false,
            keyboard_enabled: true,
        }
    }

    fn get_channel_data(&self, channel_idx: usize) -> Option<Vec<f64>> {
        if channel_idx >= self.data.data.nrows() {
            return None;
        }

        let channel_data = self.data.data.row(channel_idx);
        let baseline = if self.baseline_correction {
            channel_data.iter().map(|&x| x as f64).sum::<f64>() / channel_data.len() as f64
        } else {
            0.0
        };

        Some(channel_data.iter().map(|&sample| sample as f64 - baseline).collect())
    }

    fn get_sample_range(&self) -> (usize, usize) {
        let half_window = self.time_window / 2.0;
        let start_time = (self.time_offset - half_window).max(0.0);
        let end_time = (self.time_offset + half_window).min(self.total_duration);

        let start_sample = (start_time * self.info.sfreq as f64) as usize;
        let end_sample = (end_time * self.info.sfreq as f64) as usize;
        let max_samples = self.data.data.ncols();

        (start_sample.min(max_samples.saturating_sub(1)), end_sample.min(max_samples))
    }

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        if !self.keyboard_enabled {
            return;
        }

        ctx.input(|i| {
            if i.key_pressed(Key::ArrowLeft) {
                self.time_offset = (self.time_offset - 1.0).max(self.time_window / 2.0);
            }
            if i.key_pressed(Key::ArrowRight) {
                self.time_offset = (self.time_offset + 1.0).min(self.total_duration - self.time_window / 2.0);
            }
            if i.key_pressed(Key::Home) {
                self.time_offset = self.time_window / 2.0;
            }
            if i.key_pressed(Key::End) {
                self.time_offset = self.total_duration - self.time_window / 2.0;
            }
            if i.key_pressed(Key::F1) {
                self.show_help = !self.show_help;
            }
            if i.key_pressed(Key::Escape) {
                self.show_help = false;
            }

            // Channel toggles (1-9)
            for (idx, key) in [Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5,
                              Key::Num6, Key::Num7, Key::Num8, Key::Num9].iter().enumerate() {
                if i.key_pressed(*key) && idx < self.selected_channels.len() {
                    self.selected_channels[idx] = !self.selected_channels[idx];
                }
            }

            // Toggle all channels with 0
            if i.key_pressed(Key::Num0) {
                let all_selected = self.selected_channels.iter().all(|&x| x);
                self.selected_channels.fill(!all_selected);
            }
        });
    }
}

impl eframe::App for TemplateApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_keyboard(ctx);

        // Top panel with menu
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.show_grid, "Show Grid");
                    ui.checkbox(&mut self.baseline_correction, "Baseline Correction");
                    ui.checkbox(&mut self.keyboard_enabled, "Keyboard Navigation");
                });

                ui.menu_button("Help", |ui| {
                    if ui.button("Keyboard Shortcuts").clicked() {
                        self.show_help = true;
                    }
                });

                egui::widgets::global_theme_preference_buttons(ui);
            });
        });

        // Side panel with controls
        egui::SidePanel::left("controls").resizable(true).show(ctx, |ui| {
            ui.heading("EEG Controls");

            // Channel selection
            ui.group(|ui| {
                ui.label("Channel Selection");

                ui.horizontal(|ui| {
                    if ui.button("All").clicked() {
                        self.selected_channels.fill(true);
                    }
                    if ui.button("None").clicked() {
                        self.selected_channels.fill(false);
                    }
                    if ui.button("First 8").clicked() {
                        self.selected_channels.fill(false);
                        for i in 0..std::cmp::min(8, self.selected_channels.len()) {
                            self.selected_channels[i] = true;
                        }
                    }
                });

                ui.separator();

                // Scrollable channel list
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for (i, channel_name) in self.info.ch_namesx.iter().enumerate() {
                            if i < self.selected_channels.len() {
                                ui.horizontal(|ui| {
                                    // Color indicator
                                    let color_rect = ui.allocate_space(egui::Vec2::new(12.0, 12.0)).1;
                                    ui.painter().rect_filled(color_rect, 1.0, self.channel_colors[i]);

                                    ui.checkbox(&mut self.selected_channels[i], channel_name);
                                });
                            }
                        }
                    });
            });

            ui.separator();

            // Display controls
            ui.group(|ui| {
                ui.label("Display Settings");

                ui.horizontal(|ui| {
                    ui.label("Gain:");
                    ui.add(egui::Slider::new(&mut self.global_gain, 10.0..=10000.0)
                        .logarithmic(true)
                        .suffix("x"));
                });

                ui.horizontal(|ui| {
                    ui.label("Channel Spacing:");
                    ui.add(egui::Slider::new(&mut self.channel_spacing, 10.0..=200.0));
                });

                ui.horizontal(|ui| {
                    ui.label("Time Window:");
                    ui.add(egui::Slider::new(&mut self.time_window, 1.0..=60.0).suffix("s"));
                });

                ui.horizontal(|ui| {
                    ui.label("Downsampling:");
                    ui.add(egui::Slider::new(&mut self.downsampling, 1..=50));
                });
            });

            ui.separator();

            // Time navigation
            ui.group(|ui| {
                ui.label("Time Navigation");

                ui.horizontal(|ui| {
                    ui.label("Position:");
                    ui.add(egui::Slider::new(&mut self.time_offset,
                        self.time_window / 2.0..=self.total_duration - self.time_window / 2.0)
                        .suffix("s"));
                });

                ui.horizontal(|ui| {
                    if ui.button("Start").clicked() {
                        self.time_offset = self.time_window / 2.0;
                    }
                    if ui.button("Back").clicked() {
                        self.time_offset = (self.time_offset - self.time_window / 4.0)
                            .max(self.time_window / 2.0);
                    }
                    if ui.button("Forward").clicked() {
                        self.time_offset = (self.time_offset + self.time_window / 4.0)
                            .min(self.total_duration - self.time_window / 2.0);
                    }
                    if ui.button("End").clicked() {
                        self.time_offset = self.total_duration - self.time_window / 2.0;
                    }
                });
            });

            ui.separator();

            // Statistics
            ui.group(|ui| {
                ui.label("Recording Info");
                ui.label(format!("Duration: {:.1}s", self.total_duration));
                ui.label(format!("Sample Rate: {}Hz", self.info.sfreq));
                ui.label(format!("Channels: {}", self.info.num_ch));

                let selected_count = self.selected_channels.iter().filter(|&&x| x).count();
                ui.label(format!("Selected: {}", selected_count));
            });
        });

        // Main plot area
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("EEG Viewer");

            let (start_sample, end_sample) = self.get_sample_range();
            let selected_channels: Vec<usize> = self.selected_channels.iter()
                .enumerate()
                .filter_map(|(i, &selected)| if selected { Some(i) } else { None })
                .collect();

            let plot = Plot::new("eeg_plot")
                .height(ui.available_height() - 60.0)
                .show_grid(self.show_grid)
                .allow_boxed_zoom(true)
                .allow_drag(true);

            plot.show(ui, |plot_ui| {
                for (display_idx, &channel_idx) in selected_channels.iter().enumerate() {
                    let channel_offset = display_idx as f64 * self.channel_spacing;

                    if let Some(channel_data) = self.get_channel_data(channel_idx) {
                        let points: PlotPoints<'_> = channel_data
                            .iter()
                            .enumerate()
                            .skip(start_sample)
                            .take(end_sample - start_sample)
                            .step_by(self.downsampling)
                            .map(|(i, &sample)| {
                                let x = i as f64 / self.info.sfreq as f64;
                                let y = (sample / self.global_gain) + channel_offset;
                                [x, y]
                            })
                            .collect();

                        let line = Line::new(&self.info.ch_namesx[channel_idx], points)
                            .color(self.channel_colors[channel_idx]);

                        plot_ui.line(line);
                    }
                }

                // Set plot bounds
                if !selected_channels.is_empty() {
                    let num_selected = selected_channels.len() as f64;
                    let total_height = (num_selected - 1.0) * self.channel_spacing;
                    let margin = self.channel_spacing * 0.5;

                    let half_window = self.time_window / 2.0;
                    let time_start = self.time_offset - half_window;
                    let time_end = self.time_offset + half_window;

                    let bounds = egui_plot::PlotBounds::from_min_max(
                        [time_start, -margin],
                        [time_end, total_height + margin],
                    );

                    plot_ui.set_plot_bounds(bounds);
                }
            });

            // Status bar
            ui.separator();
            ui.horizontal(|ui| {
                let selected_count = self.selected_channels.iter().filter(|&&x| x).count();
                if selected_count > 0 {
                    ui.colored_label(Color32::GREEN, format!("Active: {} channels", selected_count));
                } else {
                    ui.colored_label(Color32::RED, "No channels selected");
                }

                let progress = self.time_offset / self.total_duration * 100.0;
                ui.label(format!("Position: {:.1}%", progress));
            });
        });

        // Help dialog
        if self.show_help {
            egui::Window::new("Keyboard Shortcuts")
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("Navigation:");
                    ui.label("  Left/Right Arrow: Navigate time");
                    ui.label("  Home/End: Go to start/end");
                    ui.label("  1-9: Toggle channels 1-9");
                    ui.label("  0: Toggle all channels");
                    ui.label("  F1: Show/hide this help");
                    ui.label("  Esc: Close dialogs");

                    if ui.button("Close").clicked() {
                        self.show_help = false;
                    }
                });
        }
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to("eframe", "https://github.com/emilk/egui/tree/master/crates/eframe");
    });
}
