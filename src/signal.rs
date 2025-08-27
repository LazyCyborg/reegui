use std::iter::Sum;
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
    
    let mut data_vec_vec: Vec<Vec<i16>> = Vec::new();
    let sos = design_butter_hp(2, lfreq, eeg_info.sfreq as f64);
    
    //print!("Highpass filtering individual channels...");
    for channel in eeg_data.rows(){
        let filtered: Vec<f64> = sosfiltfilt_dyn(channel.into_iter().map(|sample| *sample as f64), &sos);
        let filtered_vec: Vec<i16> = filtered.into_iter().map(|sample| sample.round() as i16).collect(); 
        data_vec_vec.push(filtered_vec); 
    }
    
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
    
    let mut data_vec_vec: Vec<Vec<i16>> = Vec::new();
    let sos = design_butter_lp(2, hfreq, eeg_info.sfreq as f64);
    
    //print!("Lowpass filtering individual channels...");
    for channel in eeg_data.rows(){
        let filtered: Vec<f64> = sosfiltfilt_dyn(channel.into_iter().map(|sample| *sample as f64), &sos);
        let filtered_vec: Vec<i16> = filtered.into_iter().map(|sample| sample.round() as i16).collect(); 
        data_vec_vec.push(filtered_vec); 
    }
    
    let data = vec_to_ndarray(data_vec_vec);
    Ok(data)
    }


pub fn resample<F: Float + FftNum>(x: &[F], n: usize) -> Vec<F> {
    // SciPy style 'Fourier' resampling
    // 1. Compute FFT of x
    // 2. Fill vec of zeros with the desired length, y.
    // 3. Set the from beginning of y to the first half of x
    // 4. Set the from end of y to the second half of x
    // 5. Compute IFFT of y
    // 6. Multiply y by (n / x.len())
    // 7. Take the real part of y
    // Compute FFT of x
    let mut fft_planner = rustfft::FftPlanner::<F>::new();
    let fft = fft_planner.plan_fft_forward(x.len());
    let ifft = fft_planner.plan_fft_inverse(n);

    let scratch_length = std::cmp::max(
        fft.get_inplace_scratch_len(),
        ifft.get_inplace_scratch_len(),
    );
    let mut scratch = vec![Complex::zero(); scratch_length];
    let mut x = x
        .into_iter()
        .map(|x| Complex::new(*x, F::zero()))
        .collect::<Vec<_>>();
    fft.process_with_scratch(&mut x, &mut scratch);
    // Fill y with halfs of x
    let mut y = vec![Complex::zero(); n];
    let bins = std::cmp::min(x.len(), n);
    let half_spectrum = bins / 2;
    y[..half_spectrum].copy_from_slice(&x[..half_spectrum]);
    y[n - half_spectrum..].copy_from_slice(&x[x.len() - half_spectrum..]);
    // Compute iFFT of y
    ifft.process_with_scratch(&mut y, &mut scratch);
    // Take the scaled real domain as the resampled result
    let scale_factor = F::from(1.0 / x.len() as f64).unwrap();
    let y = y.iter().map(|x| x.re * scale_factor).collect::<Vec<_>>();
    y
}

pub fn resample_eeg(
    n_sfreq: usize,
    eeg_info: &EEGInfo,
    eeg_data: &Array2<i16>,
) -> Result<Array2<i16>, Box<dyn std::error::Error>> {
    if eeg_data.is_empty() {
        return Ok(Array2::from_shape_vec((0, 0), Vec::new()).unwrap());
    }
    
    let mut data_vec_vec: Vec<Vec<i16>> = Vec::new();
    
    //print!("Highpass filtering individual channels...");
    for channel in eeg_data.rows(){
        let channel_f64: Vec<f64> = channel.iter().map(|&i|i as f64).collect();
        let resampled_vec = resample(&channel_f64, n_sfreq); 
        let channel_i16: Vec<i16> = resampled_vec.iter().map(|&i| i as i16).collect();
        data_vec_vec.push(channel_i16); 
    }
    
    let data = vec_to_ndarray(data_vec_vec);
    Ok(data)
    }
    
    
