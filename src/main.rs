
use std::fs;
use clap::Parser;
use ndarray::prelude::*;
//use plotly::{Plot, Scatter};
use eframe::NativeOptions;


// My stuff
mod io;
mod gui;
mod app;

use reegui::EEGInfo;
use reegui::EEGData;

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

    /// File path of the .eeg if BV or .edf if EDF (str)
    #[arg(long)]
    dfpath: String,

    /// Read and display the data as a table
    #[arg(short, long)]
    read_data: bool,

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
            //println!("CHANNELS {:?}", &channels.unwrap().len());
            let data = io::vec_to_ndarray(channels.unwrap());
            println!("SHAPE OF DATA {:?}", data.shape());
            println!("Row {:?}", &data.row(0).len());
            println!("Column {:?}", &data.column(0).len());

        }

        _ => {}
    };

   let _ = match cli.view {
        true => {
            
            match cli.format.as_str() {
            
            "brainvision" => {
                    
                println!("Reading from fpath {:?}", cli.hfpath);
                let header = io::get_header(&cli.hfpath)?;
                //println!("Header: {:?}", header);
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

