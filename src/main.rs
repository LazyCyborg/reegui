
use std::{fs, hash::DefaultHasher};
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
mod vis;

use reegui::{EEGInfo, EEGData, Markers, EpochsData, EvokedData};


//use std::any::type_name;
//fn type_of<T>(_: T) -> &'static str {
  //  type_name::<T>()
//}

// CLI code
// underscores will be converted to "-" when clap parses the arguments
#[derive(Parser)]
#[command(name = "reeg")]
#[command(version = "0.1.0")]
#[command(about = "Reads and manipulates EEG data from the brainvision system", long_about = None)]
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
    readdata: bool,

    /// Remove and interpolate the TMS-pulse using cubic spline interpolation
    #[arg(long)]
    rmtms: bool,

    /// N seconds before the tms pulse to cut
    #[arg(long, required_if_eq("rmtms", "true"))]
    tmincut: Option<f64>,

    /// N seconds after the tms pulse to cut
    #[arg(long, required_if_eq("rmtms", "true"))]
    tmaxcut: Option<f64>,

    /// Bandpass filter data (Butterworth)
    #[arg(long)]
    filter: bool,

    /// lfreq for highpass filter
    #[arg(long, required_if_eq("filter", "true"))]
    lfreq: Option<f64>,

    /// hfreq for lowpass filter
    #[arg(long, required_if_eq("filter", "true"))]
    hfreq: Option<f64>,

    /// Split data in epochs (tmin and tmax a)
    #[arg(long)]
    epoch: bool,

    /// tmin (time before stimulus to include in s)
    #[arg(long, required_if_eq("epoch", "true"))]
    tmin: Option<f64>,

    /// tmax (time after stimulus to include in s)
    #[arg(long, required_if_eq("epoch", "true"))]
    tmax: Option<f64>,

    /// Average across epochs
    #[arg(long)]
    evoked: bool,

    /// Save plot as html file (fname must be provided)
    #[arg(long)]
    plotevoked: bool,

    /// Name of the html file for the evoked plot
    #[arg(long, required_if_eq("plotevoked", "true"))]
    plotfname: Option<String>,

    /// Select data format (Brainvision or EDF)
    #[arg(short, long)]
    format: String,

    /// View data in reader (bool)
    #[arg(short, long)]
    view: bool,

    /// Use fast memory-mapped I/O for large files (requires unsafe code)
    #[arg(long)]
    fastio: bool,
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

    match cli.readdata {
        true => {
            println!("Reading from fpath {:?} \n", cli.hfpath);
            let header = io::get_header(&cli.hfpath)?;
            //println!("Header: {:?}", header);
            let eeg_info = io::parse_header(&header)?;
            println!("Reading data from fpath {:?} \n", cli.dfpath);
            //let samples = io::parse_bytes(&cli.dfpath, &eeg_info)?;
           //let times = io::convert_to_seconds(samples, &eeg_info)?;
            //println!("Recording duration: {:?} s \n", times.len());
            //let channels = io::demultiplex(times, &eeg_info);
            let channels = if cli.fastio {
                println!("Using fast memory-mapped I/O...");
                io::parse_bytes_opt(&cli.dfpath, &eeg_info)?
            } else {
                println!("Using standard I/O...");
                let samples = io::parse_bytes(&cli.dfpath, &eeg_info)?;
                let times = io::convert_to_seconds(samples, &eeg_info)?;
                println!("Recording duration: {:?} s \n", times.len());
                io::demultiplex(times, &eeg_info)?
            };
            println!("DATA READ");
            println!("Metadata {:?} \n", eeg_info);
            println!("Metadata read \n");
            let data = io::vec_to_ndarray(channels);

            println!("Data converted to ndarray: ");
            println!("Shape of loaded data {:?}", data.shape());
            println!("Row {:?}", &data.row(0).len());
            println!("Column {:?}", &data.column(0).len());

            let vmrk_file = io::get_vmrk(&cli.mfpath)?;
            let markers = io::parse_vmrk(&vmrk_file)?;
            println!("\n Reading events from .vmrk file {:?}", &cli.mfpath);
            println!("Number of events found {:?}", markers.markers.len());

            match (cli.rmtms, cli.filter, cli.epoch, cli.evoked) {

            (true, false, false, false) => {
                let default_tmincut = 0.002;
                let default_tmaxcut = 0.005;
                let tmincut = cli.tmincut.unwrap_or(default_tmincut);
                let tmaxcut = cli.tmaxcut.unwrap_or(default_tmaxcut);
                print!("\n Attempting to remove and interpolate the TMS pulse between {:?}-{:?} ms \n",tmincut * 1000.0, tmaxcut *1000.0);


                let rm_tms_data = signal::rm_interp_tms_pulse(tmincut, tmaxcut, &markers, &eeg_info, &data);

            },
            (true, true, false, false) => {
                let default_tmincut = 0.002;
                let default_tmaxcut = 0.005;
                let tmincut = cli.tmincut.unwrap_or(default_tmincut);
                let tmaxcut = cli.tmaxcut.unwrap_or(default_tmaxcut);
                print!("\n Attempting to remove and interpolate the TMS pulse between {:?}-{:?} ms \n",tmincut * 1000.0, tmaxcut *1000.0);


                let rm_tms_data = signal::rm_interp_tms_pulse(tmincut, tmaxcut, &markers, &eeg_info, &data);

                let hfreq_default = 40.0;
                let lfreq_default = 0.1;
                let hfreq = cli.hfreq.unwrap_or(hfreq_default);
                let lfreq = cli.lfreq.unwrap_or(lfreq_default);
                println!("\n Attempting to bandpass filter data between {:?}-{:?} Hz", lfreq, hfreq);


                let lp_filtered_data = signal::lp_filter(hfreq, &eeg_info, &rm_tms_data?);
                let hp_filtered_data = signal::hp_filter(lfreq, &eeg_info, &lp_filtered_data?);

            },
            (false, true, false, false) => {

                let hfreq_default = 40.0;
                let lfreq_default = 0.1;
                let hfreq = cli.hfreq.unwrap_or(hfreq_default);
                let lfreq = cli.lfreq.unwrap_or(lfreq_default);
                println!("\n Attempting to bandpass filter data between {:?}-{:?} Hz", lfreq, hfreq);


                let lp_filtered_data = signal::lp_filter(hfreq, &eeg_info, &data);
                let hp_filtered_data = signal::hp_filter(lfreq, &eeg_info, &lp_filtered_data.unwrap());

            },
            (true, true, true, false) => {
                let default_tmincut = 0.002;
                let default_tmaxcut = 0.005;
                let tmincut = cli.tmincut.unwrap_or(default_tmincut);
                let tmaxcut = cli.tmaxcut.unwrap_or(default_tmaxcut);
                print!("\n Attempting to remove and interpolate the TMS pulse between {:?}-{:?} ms \n",tmincut * 1000.0, tmaxcut *1000.0);


                let rm_tms_data = signal::rm_interp_tms_pulse(tmincut, tmaxcut, &markers, &eeg_info, &data);

                let hfreq_default = 40.0;
                let lfreq_default = 0.1;
                println!("\n Attempting to bandpass filter data between {:?}-{:?} Hz", cli.lfreq.unwrap_or(lfreq_default), cli.hfreq.unwrap_or(hfreq_default));
                let hfreq = cli.hfreq.unwrap_or(hfreq_default);
                let lfreq = cli.lfreq.unwrap_or(lfreq_default);

                let lp_filtered_data = signal::lp_filter(hfreq, &eeg_info, &rm_tms_data?);
                let hp_filtered_data = signal::hp_filter(lfreq, &eeg_info, &lp_filtered_data?);

                let default_tmin = 1.0;
                let default_tmax = 1.0;
                let tmin = cli.tmin.unwrap_or(default_tmin);
                let tmax = cli.tmax.unwrap_or(default_tmax);
                print!("\n Attempting epoch EEG data between tmin {:?} and tmax {:?} s \n", tmin, tmax);

                let epochs = epochs::epoch_eeg(tmin, tmax, &eeg_info, &data, &markers);

            },
            (true, true, true, true) => {
                let default_tmincut = 0.002;
                let default_tmaxcut = 0.005;
                let tmincut = cli.tmincut.unwrap_or(default_tmincut);
                let tmaxcut = cli.tmaxcut.unwrap_or(default_tmaxcut);
                print!("\n Attempting to remove and interpolate the TMS pulse between {:?}-{:?} ms \n",tmincut * 1000.0, tmaxcut *1000.0);


                let rm_tms_data = signal::rm_interp_tms_pulse(tmincut, tmaxcut, &markers, &eeg_info, &data)?;

                let hfreq_default = 40.0;
                let lfreq_default = 0.1;
                let hfreq = cli.hfreq.unwrap_or(hfreq_default);
                let lfreq = cli.lfreq.unwrap_or(lfreq_default);
                println!("\n Attempting to bandpass filter data between {:?}-{:?} Hz", lfreq, hfreq);

                let lp_filtered_data = signal::lp_filter(hfreq, &eeg_info, &rm_tms_data)?;
                let hp_filtered_data = signal::hp_filter(lfreq, &eeg_info, &lp_filtered_data)?;

                let default_tmin = 1.0;
                let default_tmax = 1.0;
                let tmin = cli.tmin.unwrap_or(default_tmin);
                let tmax = cli.tmax.unwrap_or(default_tmax);
                print!("\n Attempting epoch EEG data between tmin {:?} and tmax{:?} s \n",tmin,tmax);

                let epochs = epochs::epoch_eeg(tmin, tmax, &eeg_info, &hp_filtered_data, &markers)?;
                let epochs = epochs::epoch_eeg(tmin, tmax, &eeg_info, &hp_filtered_data, &markers)?;
                println!("Shape of epochs (epochs, channels, samples): {:?}", epochs.dim());
                let ch_names = eeg_info.ch_names.clone();
                let epochs_data = EpochsData { epochs, tmin, tmax, ch_names: ch_names.clone() };

                print!("\n Averageing across epochs..");
                let evoked = epochs::evoked_eeg(&epochs_data, &eeg_info)?;

                let evoked = epochs::evoked_eeg(&epochs_data, &eeg_info)?;
                println!("\nShape of evoked data (channels, samples): {:?}", evoked.dim());

                let evoked_data = EvokedData {evoked, tmin, tmax, ch_names: ch_names.clone()};
                let default_plotfname = String::from("Evoked");
                let file_name = cli.plotfname.unwrap_or(default_plotfname);

                print!("\n Saving plot!");
                vis::plot_evoked(evoked_data, &file_name);


            },

            (true, false, true, true) => {
                let default_tmincut = 0.002;
                let default_tmaxcut = 0.005;
                let tmincut = cli.tmincut.unwrap_or(default_tmincut);
                let tmaxcut = cli.tmaxcut.unwrap_or(default_tmaxcut);
                print!("\n Attempting to remove and interpolate the TMS pulse between {:?}-{:?} ms \n",tmincut * 1000.0, tmaxcut *1000.0);

                let rm_tms_data = signal::rm_interp_tms_pulse(tmincut, tmaxcut, &markers, &eeg_info, &data)?;

                let default_tmin = 1.0;
                let default_tmax = 1.0;
                let tmin = cli.tmin.unwrap_or(default_tmin);
                let tmax = cli.tmax.unwrap_or(default_tmax);
                print!("\n Attempting epoch EEG data between tmin {:?} and tmax {:?} s \n",tmin,tmax);
                let epochs = epochs::epoch_eeg(tmin, tmax, &eeg_info, &rm_tms_data, &markers)?;

                let ch_names = eeg_info.ch_names.clone();
                let epochs_data = EpochsData { epochs,tmin, tmax, ch_names: ch_names.clone()};
                let evoked = epochs::evoked_eeg(&epochs_data, &eeg_info)?;
                let evoked = epochs::evoked_eeg(&epochs_data, &eeg_info)?;
                let evoked_data = EvokedData {evoked, tmin, tmax, ch_names: ch_names.clone()};
                let default_plotfname = String::from("Evoked");
                let file_name = cli.plotfname.unwrap_or(default_plotfname);
                vis::plot_evoked(evoked_data, &file_name);
            },
            (false, false, true, true) => {
 let default_tmin = 1.0;
                let default_tmax = 1.0;
                let tmin = cli.tmin.unwrap_or(default_tmin);
                let tmax = cli.tmax.unwrap_or(default_tmax);
                print!("\n Attempting epoch EEG data between tmin {:?} and tmax {:?} ms \n",tmin,tmax);
                let epochs = epochs::epoch_eeg(tmin, tmax, &eeg_info, &data, &markers)?;
                let ch_names = eeg_info.ch_names.clone();
                let epochs_data = EpochsData { epochs,tmin, tmax, ch_names: ch_names.clone()};

                let evoked = epochs::evoked_eeg(&epochs_data, &eeg_info)?;

                let evoked = epochs::evoked_eeg(&epochs_data, &eeg_info)?;

                let evoked_data = EvokedData {evoked, tmin, tmax, ch_names};
                let default_plotfname = String::from("Evoked");
                let file_name = cli.plotfname.unwrap_or(default_plotfname);
                vis::plot_evoked(evoked_data, &file_name);
            },
            (true, true, false, true) => {
                eprintln!("Error: Epochs must be constructed before averaging");
                std::process::exit(1);
            },

            (false, false, false, true) => {
                eprintln!("Error: Epochs must be constructed before averaging");
                std::process::exit(1);
            },
            _ => {
                eprintln!("Whatever that combination was is not implemented yet");
                std::process::exit(1);
            }

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
                let channels = if cli.fastio {
                    println!("Using fast memory-mapped I/O...");
                    io::parse_bytes_opt(&cli.dfpath, &eeg_info)?
                } else {
                    println!("Using standard I/O...");
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
                    io::demultiplex(times, &eeg_info)?
                };

                println!("DATA READ");
                //println!("CHANNELS {:?}", &channels.unwrap().len());
                let data = io::vec_to_ndarray(channels);
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
