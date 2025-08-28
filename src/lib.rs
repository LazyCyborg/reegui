#![warn(clippy::all, rust_2018_idioms)]
use ndarray::{Array2, Array3};

pub mod app;
pub mod io;
pub mod signal;
pub mod epochs;
pub mod vis;
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
pub struct Markers {
    pub n_markers: usize,
    pub markers: Vec<f64>
}

#[derive(Debug)]
pub struct EEGData {
    pub data: Array2<i16>,
}

#[derive(Debug)]
pub struct EpochsData {
    pub epochs: Array3<i16>,
    pub ch_names: Vec<String>,
    pub tmin: f64,
    pub tmax: f64
}

#[derive(Debug)]
pub struct EvokedData {
    pub evoked: Array2<f64>,
    pub ch_names: Vec<String>,
    pub tmin: f64,
    pub tmax: f64
}
