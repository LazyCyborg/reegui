
use std::fs;
use clap::Parser;
use ndarray::prelude::*;
//use plotly::{Plot, Scatter};
use eframe::NativeOptions;


// My stuff
mod io;
mod gui;
mod app;
mod signal;
mod epochs;

use reegui::{EEGInfo, EEGData, Markers, EpochsData};


//use std::any::type_name;
//fn type_of<T>(_: T) -> &'static str {
  //  type_name::<T>()
//}

// CLI code
// underscores will be converted to "-" when clap parses the arguments
#[derive(Parser)]
#[command(name = "reeg")]
#[command(version = "0.1.0")]
#[command(about = "Does awesome EEG things", long_about = None)]
pub struct Cli {
    
    /// File path of the .vhdr if BV (str)
    #[arg(long, required_if_eq("format", "brainvision"))]
    hfpath: Option<String>,

    /// File path of the .vmrk BV (str)
    #[arg(long, required_if_eq("format", "brainvision"))]
    mfpath: Option<String>,

    /// File path of the .eeg if BV or .edf if EDF (str)
    #[arg(long)]
    dfpath: String,

    /// Read and display the metadata
    #[arg(short, long)]
    read_data: bool,
    
    /// Remove and interpolate the TMS-pulse using cubic spline interpolation
    #[arg(long)]
    rmtms: bool,
    
    /// N seconds before the tms pulse to cut
    #[arg(long, required_if_eq("rmtms", "true"))]
    tmin: Option<f64>,
    
    /// N seconds after the tms pulse to cut
    #[arg(long, required_if_eq("rmtms", "true"))]
    tmax: Option<f64>,
    
    /// Bandpass filter data (Butterworth)
    #[arg(long)]
    filter: bool,
    
    /// lfreq for highpass filter
    #[arg(long, required_if_eq("filter", "true"))]
    lfreq: Option<f64>,
    
    /// hfreq for lowpass filter
    #[arg(long, required_if_eq("filter", "true"))]
    hfreq: Option<f64>,

    /// Select data format (Brainvision or EDF)
    #[arg(short, long)]
    format: String,

    /// View data in reader (bool)
    #[arg(short, long)]
    view: bool,
}

 

fn check(data: &Array2<f32>, eeg_info: &EEGInfo) {
    let view = data.view();
    println!("VIEW {:?}", view);
    println!("DATA {:?}", data.dim());
    println!("ROW 1 {:?}", data.row(1).shape());
    for c in data.outer_iter() {
        println!("C {:?} {:?}", c.first(), c.last())
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.read_data {
        true => {
            println!("Reading from fpath {:?}", cli.hfpath);
            let header = io::get_header(&cli.hfpath)?;
            //println!("Header: {:?}", header);
            let eeg_info = io::parse_header(&header)?;
            println!("Reading from fpath {:?}", cli.dfpath);
            let samples = io::parse_bytes(&cli.dfpath, &eeg_info)?;
            let times = io::convert_to_seconds(samples, &eeg_info)?;
            println!("TIMES: {:?}", times.len());
            let channels = io::demultiplex(times, &eeg_info);
            println!("DATA READ");
            println!("INFO {:?}", eeg_info);
            println!("METADATA READ");
            //println!("CHANNELS {:?}", &channels.unwrap().len());
            let data = io::vec_to_ndarray(channels.unwrap());
            println!("Shape of loaded data {:?}", data.shape());
            println!("Row {:?}", &data.row(0).len());
            println!("Column {:?}", &data.column(0).len());

            let vmrk_file = io::get_vmrk(&cli.mfpath)?;
            let markers = io::parse_vmrk(&vmrk_file)?;
            println!("Number of events found {:?}", markers.markers.len());
            
            if cli.rmtms{
                let default_tmin = 0.002;
                let default_tmax = 0.005;
                                print!("\n Attempting to remove and interpolate the TMS pulse between {:?}-{:?} ms \n",(cli.tmin.unwrap_or(default_tmin) * 1000.0), (cli.tmax.unwrap_or(default_tmax) *1000.0));
                let tmin = cli.tmin.unwrap_or(default_tmin);
                let tmax = cli.tmax.unwrap_or(default_tmax);
                
            let rm_tms_data = signal::rm_interp_tms_pulse(tmin, tmax, &markers, &eeg_info, &data);
                
            }
            if cli.filter{
                
                let hfreq_default = 40.0;
                let lfreq_default = 0.1;
                println!("\n Attempting to bandpass filter data between {:?}-{:?} Hz", cli.lfreq.unwrap_or(lfreq_default), cli.hfreq.unwrap_or(hfreq_default));
                let hfreq = cli.hfreq.unwrap_or(hfreq_default);
                let lfreq = cli.lfreq.unwrap_or(lfreq_default);
                
            let lp_filtered_data = signal::lp_filter(hfreq, &eeg_info, &data);
            let hp_filtered_data = signal::hp_filter(lfreq, &eeg_info, &lp_filtered_data.unwrap());             
            }

        }

        _ => {}
    };

   let _ = match cli.view {
        true => {
            
            match cli.format.as_str() {
            
            "brainvision" => {
                    
                println!("Reading header from fpath {:?}", cli.hfpath);
                let header = io::get_header(&cli.hfpath)?;
                //println!("Header: {:?}", header);
                let eeg_info = io::parse_header(&header)?;
                println!("Reading markers from {:?}", cli.mfpath);
                let vmrk_file = io::get_vmrk(&cli.mfpath)?;
                let markers = io::parse_vmrk(&vmrk_file)?;
                println!("Reading data from fpath {:?}", cli.dfpath);
                let samples = io::parse_bytes(&cli.dfpath, &eeg_info)?;
    
                let metadata = fs::metadata(&cli.dfpath)?;
                let file_size_bytes = metadata.len();
                let expected_size_from_samples = samples.len() * 2; // 2 bytes per i16 sample
                
                println!("[Verification] EEG file size on disk: {} bytes", file_size_bytes);
                println!("[Verification] Size calculated from parsed samples: {} bytes", expected_size_from_samples);
                if file_size_bytes == expected_size_from_samples as u64 {
                    println!("Total sample count matches file size.");
                } else {
                    println!("ERROR: Mismatch between file size and parsed samples!");
                }
                let times = io::convert_to_seconds(samples, &eeg_info)?;
                println!("TIMES: {:?} seconds", times.len());
                let channels = io::demultiplex(times, &eeg_info);
                println!("DATA READ");
                //println!("CHANNELS {:?}", &channels.unwrap().len());
                let data = io::vec_to_ndarray(channels.unwrap());
                println!("SHAPE OF DATA {:?}", data.shape());
                let eeg_data = EEGData { data };
    
                let native_options = eframe::NativeOptions {
                    viewport: egui::ViewportBuilder::default()
                        .with_inner_size([400.0, 300.0])
                        .with_min_inner_size([300.0, 220.0])
                        .with_icon(
                            // NOTE: Adding an icon is optional
                            eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
                                .expect("Failed to load icon"),
                        ),
                    ..Default::default()
                };
    
                eframe::run_native(
                    "reegui",
                    native_options,
                    Box::new(|cc| Ok(Box::new(reegui::TemplateApp::new(cc, eeg_info, eeg_data, markers)))),
                )
                    
                }
                
            "edf" => {
                    
                println!("Reading from fpath {:?}", cli.dfpath);
                let edf = io::parse_edf(&cli.dfpath.as_str())?;
                //println!("Header: {:?}", header);
                /*
                let eeg_info = io::parse_header(&header)?;
                println!("Reading from fpath {:?}", cli.dfpath);
                let samples = io::parse_bytes(&cli.dfpath, &eeg_info)?;
    
                let metadata = fs::metadata(&cli.dfpath)?;
                let file_size_bytes = metadata.len();
                let expected_size_from_samples = samples.len() * 2; // 2 bytes per i16 sample
    
                println!("[Verification] EEG file size on disk: {} bytes", file_size_bytes);
                println!("[Verification] Size calculated from parsed samples: {} bytes", expected_size_from_samples);
    
                if file_size_bytes == expected_size_from_samples as u64 {
                    println!("Total sample count matches file size.");
                } else {
                    println!("ERROR: Mismatch between file size and parsed samples!");
                }
                let times = io::convert_to_seconds(samples, &eeg_info)?;
                println!("TIMES: {:?} seconds", times.len());
                let channels = io::demultiplex(times, &eeg_info);
                println!("DATA READ");
                //println!("CHANNELS {:?}", &channels.unwrap().len());
                let data = io::vec_to_ndarray(channels.unwrap());
                println!("SHAPE OF DATA {:?}", data.shape());
                let eeg_data = EEGData { data };
    
                let native_options = eframe::NativeOptions {
                    viewport: egui::ViewportBuilder::default()
                        .with_inner_size([400.0, 300.0])
                        .with_min_inner_size([300.0, 220.0])
                        .with_icon(
                            // NOTE: Adding an icon is optional
                            eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
                                .expect("Failed to load icon"),
                        ),
                    ..Default::default()
                };
    
                eframe::run_native(
                    "reegui",
                    native_options,
                    Box::new(|cc| Ok(Box::new(reegui::TemplateApp::new(cc, eeg_info, eeg_data)))),
                )
                */
                Ok({})
                    
                }
            _ => {
                    println!("Error: Unknown format specified: {}", cli.format);
                    Ok(())
                }
             }

        }
        _ => Ok({})

    };

    Ok(())
}

