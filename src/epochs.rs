use ndarray::{s, Array2, Array3, Axis};

use crate::io::vec_to_ndarray;
use crate::{EEGData, EEGInfo, Markers, EpochsData};
use crate::signal;


pub fn epoch_eeg(
    tmin: f64,
    tmax: f64,
    eeg_info: &EEGInfo,
    eeg_data: &Array2<i16>,
    markers: &Markers
) -> Result<Array3<i16>, Box<dyn std::error::Error>> {
    if eeg_data.is_empty() {
        return Ok(Array3::from_shape_vec((0, 0, 0), Vec::new()).unwrap());
    }
    
    let mut epochs: Vec<Vec<Vec<i16>>> = Vec::new();
    
    let mut data_copy = eeg_data.clone();
    let n_samples = data_copy.ncols();

    let min_samples = (tmin * eeg_info.sfreq as f64).round() as usize;
    let max_samples = (tmax * eeg_info.sfreq as f64).round() as usize;
    
    for &marker_pos in &markers.markers {
        let mut current_epoch: Vec<Vec<i16>> = Vec::new();
        let marker_idx = marker_pos.round() as usize;

        let start_cut = marker_idx.saturating_sub(min_samples);
        let end_cut = (marker_idx + max_samples).min(n_samples);
        if start_cut >= end_cut {
            continue; 
        }
        for ch_idx in 0..data_copy.nrows() {
            let slice = data_copy.slice_mut(s![ch_idx, start_cut..end_cut]).to_vec();
            current_epoch.push(slice);
        }
        epochs.push(current_epoch);
    }
    let data = signal::vec_to_ndarray3(epochs);
    Ok(data)
}


pub fn evoked_eeg(
    epochs: &EpochsData,
    eeg_info: &EEGInfo,
) -> Result<Array2<f64>, Box<dyn std::error::Error>> {
    if epochs.epochs.is_empty() {
        return Ok(Array2::from_shape_vec((0, 0), Vec::new()).unwrap());
    }
    let default_evoked = Array2::from_shape_vec((0, 0), Vec::new()).unwrap();
    let epochs_f64 = epochs.epochs.mapv(|i| i as f64);
    
    let evoked = epochs_f64.mean_axis(Axis(0)).unwrap_or(default_evoked);
    Ok(evoked)   

}
    
