#![allow(dead_code)]

use ndarray::{Array, Array2, Ix1, Ix2};
use plotly::common::Mode;
use plotly::ndarray::ArrayTraces;
use plotly::{Plot, Scatter, Layout};

use crate::EvokedData;



pub fn plot_single_channel(evoked: EvokedData, channel: usize, file_name: &str) {
    let (n_channels, n_samples) = evoked.evoked.dim();
    if n_samples == 0 {
        return;
    }
    if channel >= n_channels {
        eprintln!(
            "Error: Invalid channel index {}. Must be less than {}.",
            channel, n_channels
        );
        return;
    }

    let t: Array<f64, Ix1> = Array::linspace(-evoked.tmin, evoked.tmax, n_samples);
    let ys = evoked.evoked.row(channel).to_owned();

    let channel_name = &evoked.ch_names[channel];
    let trace = Scatter::from_array(t, ys)
        .mode(Mode::LinesMarkers)
        .name(channel_name);  // Set trace name

    let mut plot = Plot::new();
    plot.add_trace(trace);

    let title_text = format!("<b>Evoked</b> (Channel: {})", channel_name);
    let layout = Layout::new().title(title_text);
    plot.set_layout(layout);

    plot.write_html(file_name);
}




pub fn plot_evoked(evoked: EvokedData, file_name: &str) {
    let (n_channels, n_samples) = evoked.evoked.dim();
    if n_samples == 0 {
        return;
    }

    let t: Array<f64, Ix1> = Array::linspace(-evoked.tmin, evoked.tmax, n_samples);

    let mut plot = Plot::new();

    // Create one trace per channel with proper names
    for (ch_idx, channel_name) in evoked.ch_names.iter().enumerate() {
        let ys = evoked.evoked.row(ch_idx).to_owned();
        let trace = Scatter::from_array(t.clone(), ys)
            .mode(Mode::Lines)
            .name(channel_name);
        plot.add_trace(trace);
    }

    let title_text = format!("<b>Evoked</b> ({} Channels)", n_channels);
    let layout = Layout::new().title(title_text);
    plot.set_layout(layout);

    plot.write_html(file_name);
}
