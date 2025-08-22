#![warn(clippy::all, rust_2018_idioms)]
use ndarray::Array2;

pub mod app; 
pub mod io; 
pub mod signal;
pub use app::TemplateApp;

#[derive(Debug)] 
pub struct EEGInfo {
    pub num_ch: i32,
    pub ch_namesx: Vec<String>,
    pub ch_names: Vec<String>,
    pub sfreq: i32,
    pub data_orientation: String,
    pub binary_format: String,
    pub sampling_interval_in: String,
    pub sampling_interval: i32,
}

#[derive(Debug)] 
pub struct EEGData {
    pub data: Array2<i16>,
}

