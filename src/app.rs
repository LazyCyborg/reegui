use std::f64;

use crate::{EEGData, EEGInfo, Markers};
use crate::signal;
use egui_plot::{Line, Plot, PlotPoint, PlotPoints, Text, VLine};
use egui::{Key, Vec2};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
//#[derive(serde::Deserialize, serde::Serialize)]
//#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    // Example stuff:

    //#[serde(skip)] // This how you opt-out of serialization of a field
    //#[serde(skip)]
    info: EEGInfo,
    //#[serde(skip)]
    data: EEGData,
    //#[serde(skip)]
    markers: Markers,
    selected_channel: usize,
    unselected_channels: Vec<usize>,
    x_view: f64,
    y_view_min: f64,
    y_view_max: f64,
    plot_zoom_factor: Vec2,
    gain: f64,
    view_all: bool,
    show_data: bool,
    decimation_factor: usize,
    tmin_cut: f64,
    tmax_cut: f64,
    lfreq: f64,
    hfreq: f64,
    n_sfreq: usize
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>, info: EEGInfo, data: EEGData, markers: Markers) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.
        Self{
            info: info,
            data: data,
            markers: markers,
            selected_channel: 0,
            y_view_min: 0.0,
            y_view_max: 600.0,
            x_view: 0.0,
            gain: 1.0,
            plot_zoom_factor: Vec2::new(1.0, 1.0),
            unselected_channels: Vec::new(),
            view_all: false,
            show_data: false,
            decimation_factor: 100,
            tmin_cut: 0.005,
            tmax_cut: 0.005,
            lfreq: 1.0,
            hfreq: 45.0,
            n_sfreq: 725,
        }
        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        //if let Some(storage) = cc.storage {
          //  eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
       // } else {
         //   Default::default()
       // }
    }
}

impl TemplateApp {
    fn get_adaptive_decimation(&self) -> usize {
        let zoom_x = self.plot_zoom_factor[0];

        if zoom_x > 4.0 {
            1  // No decimation when heavily zoomed in
        } else if zoom_x > 2.0 {
            2  // Light decimation
        } else if zoom_x > 1.0 {
            5  // Medium decimation
        } else {
            self.decimation_factor
        }
    }
}


impl TemplateApp {
   fn min_max_decimate(&self, data: &[i16], start_sample: usize, decimation: usize, offset: f64) -> Vec<[f64; 2]>{
        if decimation <= 1 {
            return data.into_iter().enumerate().map(|(i, &sample)| {
                let x = (start_sample + i) as f64 / self.info.sfreq as f64;
                let y = (sample as f64 / 100.0) * self.gain + offset;

                [x, y]
            }).collect();
        }
        let mut points = Vec::new();
        for chunk in data.chunks(decimation) {
            let chunk_start = (points.len() / 2) * decimation;
            let time_base = (start_sample + chunk_start) as f64 / self.info.sfreq as f64;

            if let (Some(&min_val), Some(&max_val)) = (chunk.iter().min(), chunk.iter().max()) {
                points.push([time_base, (min_val as f64 / 100.0) + offset]);
                points.push([time_base + (decimation as f64 * 0.5) / self.info.sfreq as f64,
                            (max_val as f64 / 100.0) * self.gain + offset]);

            }
        }

        points
    }
}

impl eframe::App for TemplateApp {
    /// Called by the framework to save state before shutdown.
    //fn save(&mut self, storage: &mut dyn eframe::Storage) {
      //  eframe::set_value(storage, eframe::APP_KEY, self);
    //}


    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::MenuBar::new().ui(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }
                egui::widgets::global_theme_preference_buttons(ui);
            });
        });

        if ctx.input(|i|i.key_pressed(Key::K)){
            self.y_view_max += 10.0;
            self.y_view_min += 10.0
        }
        if ctx.input(|i|i.key_pressed(Key::J)){
            self.y_view_max -= 10.0;
            self.y_view_min -= 10.0
        }
        if ctx.input(|i|i.key_pressed(Key::ArrowRight)){
            self.x_view += 10.0
        }
        if ctx.input(|i|i.key_pressed(Key::ArrowLeft)){
            self.x_view -= 10.0
        }

        if ctx.input(|i|i.key_pressed(Key::L)){
            self.x_view += 10.0
        }
        if ctx.input(|i|i.key_pressed(Key::H)){
            self.x_view -= 10.0
        }
        if ctx.input(|i|i.key_pressed(Key::ArrowUp)){
            self.gain *= 1.1;
        }
        if ctx.input(|i|i.key_pressed(Key::ArrowDown)){
            self.gain /= 1.1;
        }


        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("reegui");
            ui.horizontal(|ui| {
                ui.label("EEG channels: ");
                            });
            let alternatives = &self.info.ch_names;
            egui::ComboBox::from_label("Select channels to remove").show_index(
                ui,
                &mut self.selected_channel,
                alternatives.len(),
                |i| &alternatives[i]
            );
            if !self.unselected_channels.contains(&(self.selected_channel)){
            self.unselected_channels.push(self.selected_channel);
            }

            //let before = self.y_view_max;


            ui.checkbox(&mut self.view_all, "Show all channels");

            if ui.button("Show EEG data").clicked(){
                self.show_data = true
            }

            if self.view_all == true {
                self.unselected_channels.clear();
            }

            if self.show_data {
                let channel_offset = 10.0;
                let mut offset = 0.0;

                let plt = Plot::new("my_plot")
                    .show_x(true)
                    .show_y(false)
                    .show(ui, |plot_ui| {

                    let start_time = self.x_view;
                    let end_time = self.x_view + 10.0;
                    let start_sample = ((start_time * self.info.sfreq as f64) as usize).max(0);
                    let end_sample = (end_time * self.info.sfreq as f64) as usize;
                    let visible_channels = self.data.data.nrows() - self.unselected_channels.len();
                    let total_height = visible_channels as f64 * channel_offset;
                    plot_ui.set_plot_bounds_y(-channel_offset..=(total_height + channel_offset));

                    for ch in 0..self.data.data.nrows(){
                        if !self.unselected_channels.contains(&ch){
                            let one_channel = self.data.data.row(ch);
                            let channel_slice = one_channel.as_slice().unwrap();

                            if start_sample < channel_slice.len() {
                                let actual_end = end_sample.min(channel_slice.len());
                                let visible_data = &channel_slice[start_sample..actual_end];
                                let adaptive_decimation = self.get_adaptive_decimation();
                                let points = self.min_max_decimate(visible_data, start_sample, adaptive_decimation, offset);
                                let line = Line::new("EEG", points);
                                plot_ui.line(line);
                                let text_x = self.x_view + 0.1;
                                let text_y = offset;
                                let text_point = PlotPoint::new(text_x, text_y);
                                let text = Text::new(self.info.ch_names[ch].clone(), text_point, self.info.ch_names[ch].clone());
                                plot_ui.text(text);

                                offset += channel_offset;
                            }
                        }
                    }


                    let center_x = self.x_view + 5.0;
                    let center_y = self.y_view_min + 10.0 + (self.y_view_max - self.y_view_min) / 2.0;
                    let center_point = PlotPoint::new(center_x, center_y);

                    plot_ui.set_plot_bounds_x(self.x_view..=(self.x_view + 10.0));
                    plot_ui.zoom_bounds(Vec2::new(self.plot_zoom_factor[0], 1.0), center_point);

                    let marker_points = &self.markers.markers;
                    for x in 0..marker_points.len(){
                            let marker_pos = self.markers.markers[x] / self.info.sfreq as f64;
                            plot_ui.vline(VLine::new("TMS", marker_pos));
                        }

                    });

            };

            egui::widgets::global_theme_preference_buttons(ui);
            ui.separator();

            ui.add(egui::github_link_file!(
                "https://github.com/emilk/eframe_template/blob/main/",
                "Source code."
            ));

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });
        });

        egui::SidePanel::right("controls_panel").show(ctx, |ui| {
                ui.heading("Controls");
                            egui::ComboBox::from_label("Y scale")
                .selected_text(format!("{:?}", self.y_view_max))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.y_view_max, 200.0, "200");
                    ui.selectable_value(&mut self.y_view_max, 300.0, "300");
                    ui.selectable_value(&mut self.y_view_max, 400.0, "400");
                    ui.selectable_value(&mut self.y_view_max, 500.0, "500");
                    ui.selectable_value(&mut self.y_view_max, 600.0, "600");
                    ui.selectable_value(&mut self.y_view_max, 700.0, "700");
                }
            );

            egui::ComboBox::from_label("Decmation factor")
                .selected_text(format!("{:?}", self.decimation_factor))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.decimation_factor, 0, "0");
                    ui.selectable_value(&mut self.decimation_factor, 10, "10");
                    ui.selectable_value(&mut self.decimation_factor, 20, "20");
                    ui.selectable_value(&mut self.decimation_factor, 50, "50");
                    ui.selectable_value(&mut self.decimation_factor, 100, "100");

                }
            );

            if ui.button("Remove TMS pulse").clicked(){
                self.data.data = signal::remove_tms_pulse(self.tmin_cut, self.tmax_cut, &self.markers, &self.info, &self.data.data).expect("Removal failed")
            }

            if ui.button("Remove and interpolate TMS pulse").clicked(){
                self.data.data = signal::rm_interp_tms_pulse(self.tmin_cut, self.tmax_cut, &self.markers, &self.info, &self.data.data).expect("Removal failed")
            }

            ui.heading("Filter settings");
            egui::ComboBox::from_label("Highpass filter lfreq")
                .selected_text(format!("{:?}", self.lfreq))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.lfreq, 0.1, "0.1");
                    ui.selectable_value(&mut self.lfreq, 0.2, "0.2");
                    ui.selectable_value(&mut self.lfreq, 0.3, "0.3");
                    ui.selectable_value(&mut self.lfreq, 0.5, "0.5");
                    ui.selectable_value(&mut self.lfreq, 1.0, "1.0");
                    ui.selectable_value(&mut self.lfreq, 2.0, "2.0");
                }
            );

            egui::ComboBox::from_label("Lowpass filter hfreq")
                .selected_text(format!("{:?}", self.hfreq))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.hfreq, 30.0, "30");
                    ui.selectable_value(&mut self.hfreq, 40.0, "40");
                    ui.selectable_value(&mut self.hfreq, 45.0, "45");
                    ui.selectable_value(&mut self.hfreq, 50.0, "50");
                    ui.selectable_value(&mut self.hfreq, 70.0, "70");
                    ui.selectable_value(&mut self.hfreq, 100.0, "100");
                }
            );

            if ui.button("Filter data").clicked(){
                self.data.data = signal::hp_filter(self.lfreq, &self.info, &self.data.data).expect("Highpass filtering failed");
                self.data.data = signal::lp_filter(self.hfreq, &self.info, &self.data.data).expect("Lowpass filtering failed");
            }

            ui.heading("Resample data");
            ui.collapsing("Warning!", |ui| { ui.label("Do not resample before removing TMS artefact and filtering data!"); });
            egui::ComboBox::from_label("New sfreq")
                .selected_text(format!("{:?}", self.n_sfreq))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.n_sfreq, 100, "100 Hz");
                    ui.selectable_value(&mut self.n_sfreq, 200, "200 Hz");
                    ui.selectable_value(&mut self.n_sfreq, 250 , "250 Hz");
                    ui.selectable_value(&mut self.n_sfreq, 725, "725 Hz");
                    ui.selectable_value(&mut self.n_sfreq, 1000, "1000 Hz");
                    ui.selectable_value(&mut self.n_sfreq, 2000, "2000 Hz");
                }
            );

            if ui.button("Apply resampling").clicked(){
                self.data.data = signal::resample_eeg(self.n_sfreq, &self.info, &self.data.data).expect("Resampling failed");
                self.info.sfreq = self.n_sfreq as i32;

            }
    });
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });
}
