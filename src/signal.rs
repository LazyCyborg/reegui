use std::iter::Sum;
use rayon::prelude::*;
use std::sync::Arc;
use ndarray::{s, Array2, Array3, ArrayBase, ViewRepr};
use sci_rs::signal::filter::{design::*, sosfiltfilt_dyn};
use sci_rs::na::RealField;
use num_traits::{Float, Zero};
use cubic_spline::{Points, Point, SplineOpts,TryInto};
use nalgebra::Complex;
use rustfft::FftNum;

use crate::Markers;
use crate::EEGInfo;

// Helper functions
pub fn vec_to_ndarray<T: Clone>(v: Vec<Vec<T>>) -> Array2<T> {
    if v.is_empty() {
        return Array2::from_shape_vec((0, 0), Vec::new()).unwrap();
    }
    let nrows = v.len();
    let ncols = v[0].len();
    let mut data = Vec::with_capacity(nrows * ncols);
    for row in &v {
        assert_eq!(row.len(), ncols);
        data.extend_from_slice(&row);
    }
    Array2::from_shape_vec((nrows, ncols), data).unwrap()
}

pub fn vec_to_ndarray3<T: Clone>(v: Vec<Vec<Vec<T>>>) -> Array3<T> {
    if v.is_empty() {
        return Array3::from_shape_vec((0, 0, 0), Vec::new()).unwrap();
    }
    let d1 = v.len();
    let d2 = v[0].len();
    let d3 = v[0][0].len();

    let mut flat_data = Vec::with_capacity(d1 * d2 * d3);

    for d1_vec in &v {
        // Enforce the cuboid shape, just like in your 2D function
        assert_eq!(d1_vec.len(), d2);
        for d2_vec in d1_vec {
            assert_eq!(d2_vec.len(), d3);
            flat_data.extend_from_slice(d2_vec);
        }
    }

    Array3::from_shape_vec((d1, d2, d3), flat_data).unwrap()
}

pub fn get_one_channel(ch_idx: usize, eeg_data: &Array2<i16>) -> Result<Vec<i16>, Box<dyn std::error::Error>>{
    if eeg_data.is_empty(){
        return Err("Data is empty..".into());
    }
    let ch_data = eeg_data.row(ch_idx).clone();
    let ch_vec = ch_data.to_vec();
    Ok(ch_vec)
}


// Replace interval around of the TMS pulse with 0
pub fn remove_tms_pulse(
    tmin_cut: f64,
    tmax_cut: f64,
    markers: &Markers,
    eeg_info: &EEGInfo,
    eeg_data: &Array2<i16>,
) -> Result<Array2<i16>, Box<dyn std::error::Error>> {
    if eeg_data.is_empty() {
        return Ok(Array2::from_shape_vec((0, 0), Vec::new()).unwrap());
    }

    let mut data_copy = eeg_data.clone();
    let n_samples = data_copy.ncols();

    let min_samples = (tmin_cut * eeg_info.sfreq as f64).round() as usize;
    let max_samples = (tmax_cut * eeg_info.sfreq as f64).round() as usize;

    for &marker_pos in &markers.markers {
        let marker_idx = marker_pos.round() as usize;

        let start_cut = marker_idx.saturating_sub(min_samples);
        let end_cut = (marker_idx + max_samples).min(n_samples);
        if start_cut >= end_cut {
            continue;
        }
        for ch_idx in 0..data_copy.nrows() {
            let mut slice = data_copy.slice_mut(s![ch_idx, start_cut..end_cut]);

            slice.fill(0);
        }
    }

    Ok(data_copy)
}


// r
pub fn rm_interp_tms_pulse(
    tmin_cut: f64,
    tmax_cut: f64,
    markers: &Markers,
    eeg_info: &EEGInfo,
    eeg_data: &Array2<i16>,
) -> Result<Array2<i16>, Box<dyn std::error::Error>> {
    if eeg_data.is_empty() {
        return Ok(Array2::from_shape_vec((0, 0), Vec::new()).unwrap());
    }

    let mut data_copy = eeg_data.clone();
    let n_samples = data_copy.ncols();

    let min_samples = (tmin_cut * eeg_info.sfreq as f64).round() as usize;
    let max_samples = (tmax_cut * eeg_info.sfreq as f64).round() as usize;

    for &marker_pos in &markers.markers {
        let marker_idx = marker_pos.round() as usize;

        let start_cut = marker_idx.saturating_sub(min_samples);
        let end_cut = (marker_idx + max_samples).min(n_samples);
        if start_cut >= end_cut {
            continue;
        }
        for ch_idx in 0..data_copy.nrows() {
                    if start_cut == 0 || end_cut >= n_samples {
                        data_copy.slice_mut(s![ch_idx, start_cut..end_cut]).fill(0);
                        continue;
                    }
                    let p1_x = (start_cut - 1) as f64;
                    let p1_y = data_copy[[ch_idx, start_cut - 1]] as f64;
                    let p2_x = end_cut as f64;
                    let p2_y = data_copy[[ch_idx, end_cut]] as f64;

                    let source_points = vec![(p1_x, p1_y), (p2_x, p2_y)];
                    let gap_len = end_cut - start_cut;
                    if gap_len == 0 { continue; }
                    let opts = SplineOpts::new().num_of_segments(gap_len as u32);

                    let points = <cubic_spline::Points as cubic_spline::TryFrom<_>>::try_from(&source_points)?;
                    let calculated_points = points.calc_spline(&opts)?;

                    for i in 0..gap_len {
                        let new_y = calculated_points.get_ref()[i + 1].y;
                        data_copy[[ch_idx, start_cut + i]] = new_y.round() as i16;
                    }
                }
    }
    Ok(data_copy)
}


pub fn design_butter_lp<F>(order: usize, lowcut: F, fs: F) -> Vec<Sos<F>>
where
    F: Float + RealField + Sum,
{
    //print!("Building Butterworth filter of order {:?} with lowcut {:?}", order, lowcut);
    // Design Second Order Section (SOS) filter
    let filter = butter_dyn(
        order,
        [lowcut].to_vec(),
        Some(FilterBandType::Lowpass),
        Some(false),
        Some(FilterOutputType::Sos),
        Some(fs),
    );
    let DigitalFilter::Sos(SosFormatFilter {sos}) = filter else {
        panic!("Failed to design filter");
    };
    sos
}

pub fn design_butter_hp<F>(order: usize, highcut: F, fs: F) -> Vec<Sos<F>>
where
    F: Float + RealField + Sum,
{
    //print!("Building Butterworth filter of order {:?} with highcut {:?}", order, highcut);
    // Design Second Order Section (SOS) filter
    let filter = butter_dyn(
        order,
        [highcut].to_vec(),
        Some(FilterBandType::Highpass),
        Some(false),
        Some(FilterOutputType::Sos),
        Some(fs),
    );
    let DigitalFilter::Sos(SosFormatFilter {sos}) = filter else {
        panic!("Failed to design filter");
    };
    sos
}


pub fn hp_filter(
    lfreq: f64,
    eeg_info: &EEGInfo,
    eeg_data: &Array2<i16>,
) -> Result<Array2<i16>, Box<dyn std::error::Error>> {
    if eeg_data.is_empty() {
        return Ok(Array2::from_shape_vec((0, 0), Vec::new()).unwrap());
    }

    let sos = design_butter_hp(2, lfreq, eeg_info.sfreq as f64);
    let n_channels = eeg_data.nrows();

    let data_vec_vec: Vec<Vec<i16>> = (0..n_channels)
        .into_par_iter()
        .map(|ch_idx| {
            let channel = eeg_data.row(ch_idx);
            let filtered: Vec<f64> = sosfiltfilt_dyn(
                channel.into_iter().map(|sample| *sample as f64),
                &sos
            );
            filtered.into_iter()
                .map(|sample| sample.round() as i16)
                .collect()
        })
        .collect();

    let data = vec_to_ndarray(data_vec_vec);
    Ok(data)
}

pub fn lp_filter(
    hfreq: f64,
    eeg_info: &EEGInfo,
    eeg_data: &Array2<i16>,
) -> Result<Array2<i16>, Box<dyn std::error::Error>> {
    if eeg_data.is_empty() {
        return Ok(Array2::from_shape_vec((0, 0), Vec::new()).unwrap());
    }

    let sos = design_butter_lp(2, hfreq, eeg_info.sfreq as f64);
    let n_channels = eeg_data.nrows();

    let data_vec_vec: Vec<Vec<i16>> = (0..n_channels)
        .into_par_iter()
        .map(|ch_idx| {
            let channel = eeg_data.row(ch_idx);
            let filtered: Vec<f64> = sosfiltfilt_dyn(
                channel.into_iter().map(|sample| *sample as f64),
                &sos
            );
            filtered.into_iter()
                .map(|sample| sample.round() as i16)
                .collect()
        })
        .collect();

    let data = vec_to_ndarray(data_vec_vec);
    Ok(data)
}



fn resample_channel_opt(
    x: &[i16],
    target_length: usize,
    fft: &Arc<dyn rustfft::Fft<f64>>,
    ifft: &Arc<dyn rustfft::Fft<f64>>,
    scratch: &mut [nalgebra::Complex<f64>]
) -> Vec<f64> {
    use nalgebra::Complex;

    let input_length = x.len();

    // Convert to complex and apply window to reduce artifacts
    let mut x_complex: Vec<Complex<f64>> = x
        .iter()
        .enumerate()
        .map(|(i, &sample)| {

            let window = 0.5 * (1.0 - ((2.0 * std::f64::consts::PI * i as f64) / (input_length - 1) as f64).cos());
            Complex::new(sample as f64 * window, 0.0)
            //Complex::new(sample as f64, 0.0)
        })
        .collect();

    fft.process_with_scratch(&mut x_complex, scratch);


    let mut y_spectrum = vec![Complex::zero(); target_length];

    // Determine how many frequency bins to copy
    let bins_to_copy = std::cmp::min(input_length, target_length);
    let half_bins = bins_to_copy / 2;

    // Copy DC and positive frequencies
    y_spectrum[..=half_bins].copy_from_slice(&x_complex[..=half_bins]);

    if bins_to_copy > 1 {
        let neg_start_src = input_length - half_bins;
        let neg_start_dst = target_length - half_bins;
        y_spectrum[neg_start_dst..].copy_from_slice(&x_complex[neg_start_src..]);
    }

    // Handle Nyquist frequency for even lengths
    if input_length % 2 == 0 && target_length % 2 == 0 && input_length != target_length {
        let nyquist_src = input_length / 2;
        let nyquist_dst = target_length / 2;

        if target_length > input_length {
            y_spectrum[nyquist_dst] = x_complex[nyquist_src] * 0.5;
            y_spectrum[target_length - nyquist_dst] = x_complex[nyquist_src] * 0.5;
        } else if nyquist_dst < nyquist_src {
            // Downsampling: keep Nyquist
            y_spectrum[nyquist_dst] = x_complex[nyquist_dst];
        }
    }

    ifft.process_with_scratch(&mut y_spectrum, scratch);

    let scale_factor = target_length as f64 / input_length as f64;
    y_spectrum.iter()
        .map(|complex| complex.re * scale_factor)
        .collect()
}

pub fn resample_eeg(
    target_sfreq: usize,
    eeg_info: &EEGInfo,
    eeg_data: &Array2<i16>,
) -> Result<Array2<i16>, Box<dyn std::error::Error>> {
    if eeg_data.is_empty() {
        return Ok(Array2::from_shape_vec((0, 0), Vec::new()).unwrap());
    }

    let n_channels = eeg_data.nrows();
    let original_length = eeg_data.ncols();
    let original_sfreq = eeg_info.sfreq as f64;

    let duration_seconds = original_length as f64 / original_sfreq;
    let target_length = (duration_seconds * target_sfreq as f64).round() as usize;

    println!("Resampling {} channels from {} Hz ({} samples) to {} Hz ({} samples)...",
             n_channels, original_sfreq, original_length, target_sfreq, target_length);
    println!("Duration: {:.2} seconds", duration_seconds);

    let fft_planner = Arc::new(std::sync::Mutex::new(rustfft::FftPlanner::<f64>::new()));
    let fft = {
        let mut planner = fft_planner.lock().unwrap();
        planner.plan_fft_forward(original_length)
    };
    let ifft = {
        let mut planner = fft_planner.lock().unwrap();
        planner.plan_fft_inverse(target_length)
    };

    let scratch_length = std::cmp::max(
        fft.get_inplace_scratch_len(),
        ifft.get_inplace_scratch_len(),
    );

    let data_vec_vec: Vec<Vec<i16>> = (0..n_channels)
        .into_par_iter()
        .map(|ch_idx| {
            let mut scratch = vec![nalgebra::Complex::zero(); scratch_length];

            let channel = eeg_data.row(ch_idx);
            let resampled_vec = resample_channel_opt(
                channel.as_slice().unwrap(),
                target_length,
                &fft,
                &ifft,
                &mut scratch
            );

            resampled_vec.iter().map(|&i| i as i16).collect()
        })
        .collect();

    println!("Resampling completed! New shape should be [{}, {}]", n_channels, target_length);
    let data = vec_to_ndarray(data_vec_vec);
    Ok(data)
}

pub fn resample_eeg_linear(
    target_sfreq: usize,
    eeg_info: &EEGInfo,
    eeg_data: &Array2<i16>,
) -> Result<Array2<i16>, Box<dyn std::error::Error>> {
    if eeg_data.is_empty() {
        return Ok(Array2::from_shape_vec((0, 0), Vec::new()).unwrap());
    }

    let n_channels = eeg_data.nrows();
    let original_length = eeg_data.ncols();
    let original_sfreq = eeg_info.sfreq as f64;

    // Calculate the correct number of output samples
    let duration_seconds = original_length as f64 / original_sfreq;
    let target_length = (duration_seconds * target_sfreq as f64).round() as usize;

    println!("Linear resampling {} channels from {} Hz ({} samples) to {} Hz ({} samples)...",
             n_channels, original_sfreq, original_length, target_sfreq, target_length);

    let data_vec_vec: Vec<Vec<i16>> = (0..n_channels)
        .into_par_iter()
        .map(|ch_idx| {
            let channel = eeg_data.row(ch_idx);
            let channel_slice = channel.as_slice().unwrap();

            // Linear interpolation resampling
            (0..target_length)  // Use target_length, not target_sfreq
                .map(|i| {
                    let original_index = (i as f64 * original_length as f64) / target_length as f64;
                    let idx_floor = original_index.floor() as usize;
                    let idx_ceil = (idx_floor + 1).min(original_length - 1);
                    let fraction = original_index - idx_floor as f64;

                    if idx_floor == idx_ceil {
                        channel_slice[idx_floor]
                    } else {
                        let val_floor = channel_slice[idx_floor] as f64;
                        let val_ceil = channel_slice[idx_ceil] as f64;
                        (val_floor + fraction * (val_ceil - val_floor)) as i16
                    }
                })
                .collect()
        })
        .collect();

    let data = vec_to_ndarray(data_vec_vec);
    Ok(data)
}
