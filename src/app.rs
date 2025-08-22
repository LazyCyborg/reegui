use crate::{EEGData, EEGInfo};
use egui_plot::{Line, Plot, PlotPoint, PlotPoints, Text};
use egui::{Key, Vec2};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
//#[derive(serde::Deserialize, serde::Serialize)]
//#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    // Example stuff:

    //#[serde(skip)] // This how you opt-out of serialization of a field
    //value: f32,
 
    //#[serde(skip)]
    info: EEGInfo,
    //#[serde(skip)]
    data: EEGData,
    //#[serde(skip)]
    selected_channel: usize,
    y_view: f64,
    user_interacted: bool,
    x_view: f64,
    y_view_min: f64,
    y_view_max: f64,
    plot_zoom_factor: Vec2
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>, info: EEGInfo, data: EEGData) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.
        Self{
            info: info,
            data: data,
            selected_channel: 0,
            y_view_min: 0.0,
            y_view_max: 1000.0,
            user_interacted: false,
            x_view: 0.0,
            y_view: 1000.0,
            plot_zoom_factor: Vec2::new(1.0, 1.0)
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

impl eframe::App for TemplateApp {
    /// Called by the framework to save state before shutdown.
    //fn save(&mut self, storage: &mut dyn eframe::Storage) {
      //  eframe::set_value(storage, eframe::APP_KEY, self);
    //}
    

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

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
            self.plot_zoom_factor[1] += 1.0
        }

        if ctx.input(|i|i.key_pressed(Key::ArrowDown)){
            self.plot_zoom_factor[1] -= 1.0
        }




        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("reegui");

            ui.horizontal(|ui| {
                ui.label("EEG channels: ");
                            });
            let alternatives = &self.info.ch_names;
            egui::ComboBox::from_label("Select one!").show_index(
                ui,
                &mut self.selected_channel,
                alternatives.len(),
                |i| &alternatives[i]
            );
           

            //

            let channel_offset = 10.0;             
            let mut offset = 0.0; 

              Plot::new("my_plot")
                .show_axes(false)
                .show(ui, |plot_ui| {

                for ch in 0..self.data.data.nrows(){
                let one_channel = self.data.data.row(ch);
                let points: PlotPoints = one_channel.iter().enumerate().step_by(100).map(|(i, &sample)| {

                    let x = i as f64 / self.info.sfreq as f64;
                    let y = (sample as f64 / 100.0) + offset;
                    [x, y]
                }).collect();
                let text_x = self.x_view + 0.1;
                let text_y = offset;
                let text_point = PlotPoint::new(text_x, text_y);
                let line = Line::new("EEG", points);
                plot_ui.line(line);
                let text = Text::new(self.info.ch_names[ch].clone(), text_point, self.info.ch_names[ch].clone());

                plot_ui.text(text);

                offset += channel_offset;

                }
                let center_x = self.x_view + 5.0;
                let center_y = self.y_view_min + 10.0 + (self.y_view_max - self.y_view_min) / 2.0;
                let center_point = PlotPoint::new(center_x, center_y);

                    plot_ui.set_plot_bounds_x(self.x_view..=(self.x_view + 10.0));
                    plot_ui.set_plot_bounds_y((self.y_view_min+10.0)..=(self.y_view_max + 10.0));

                    plot_ui.zoom_bounds(self.plot_zoom_factor, center_point);
                });

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
