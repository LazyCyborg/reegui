use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

use ndarray::prelude::*;

use crate::EEGInfo;
use crate::Markers;

//fn type_of<T>(_: T) -> &'static str {
//    type_name::<T>()
//}

pub fn get_header(fpath: &Option<String>) -> Result<Option<String>, Box<dyn std::error::Error>> {
    match  fpath {
        Some(path) => {
        let mut file = File::open(path)?;
        let mut header = String::new();
        file.read_to_string(&mut header)?;
        Ok(Some(header))
        }

    None => Ok(None),
    }

}

pub fn parse_header(header: &Option<String>) -> Result<EEGInfo, Box<dyn std::error::Error>> {

    let header_content = match header {
        Some(content) => content,
        // Return a clear error if the header is needed but not provided.
        None => return Err("Header content is missing and required for this operation.".into()),
    };

    let header_vec: Vec<String> = header_content.split("\n").map(|x| x.to_string()).collect();

    let mut eeg_info = EEGInfo {
        num_ch: 0,
        ch_namesx: Vec::new(),
        ch_names: Vec::new(),
        sfreq: 0,
        data_orientation: String::new(),
        binary_format: String::new(),
        sampling_interval_in: String::new(),
        sampling_interval: 0,
    };
    //Prints the whole header
    //header_vec.iter().for_each(|x| println!("Lines {:?}", x));

    //println!("Header {:?}", header_vec);
    let mut header_str = String::new();
    header_vec.iter().for_each(|x| {
        if x.contains("Sampling Rate [Hz]") {
            header_str.push_str(x);
            header_str.push_str("\n");
            //println!("SFREQ: {}", &x[20..].replace("\r", ""));
            eeg_info.sfreq = x[20..].replace("\r", "").parse::<i32>().unwrap();
            println!("Sampling rate {:?}", eeg_info.sfreq);
        }

        if x.contains("Number of channels:") {
            header_str.push_str(x);
            header_str.push_str("\n");
            //println!("N: {}", &x[20..]);
            let n_ch = x[20..].replace("\r", "").parse::<i32>().unwrap(); // Should make this more
            //solid
            //println!("NCH: {}", n_ch);
            eeg_info.num_ch = n_ch;

            println!("Number of channels {:?}", n_ch);

            for i in 0..n_ch {
                eeg_info.ch_namesx.push(format!("Ch{}", i + 1));
            }
            //println!("CHANNELSXX: {:?}", eeg_info.ch_namesx);
        }

        if x.contains("DataOrientation") {
            header_str.push_str(x);
            //header_str.push_str("\n");
            //println!("SFREQ: {}", &x[20..].replace("\r", ""));
            eeg_info.data_orientation = x.replace("\r", "");
            println!("Data orientation {:?}", eeg_info.data_orientation);
        }

        if x.contains("BinaryFormat") {
            header_str.push_str(x);
            eeg_info.binary_format = x.replace("BinaryFormat=", "").replace("\r", "");
            println!("Binary format {:?}", eeg_info.binary_format);
        }

        if x.contains("Sampling interval in") {
            header_str.push_str(x);
            eeg_info.sampling_interval_in = x.replace("Sampling interval in", "").replace("\r", "");
        }

        if x.contains("SamplingInterval") {
            header_str.push_str(x);
            eeg_info.sampling_interval = x
                .replace("SamplingInterval=", "")
                .replace("\r", "")
                .parse::<i32>()
                .unwrap();
        }
    });

    for x in header_vec.iter() {
         
        if x.contains("µV") {
            //println!("{:?}", x);
            eeg_info
                .ch_names
                .push(x.to_string().replace(",,0.1,µV", "").replace("Ch", ""));
        }
        if eeg_info.ch_names.len() == eeg_info.num_ch as usize{
            break;
        }
        };

    //println!("Header: {:?}", header_str);
    //println!("{:?}", eeg_info);
    Ok(eeg_info)
}

pub fn get_vmrk(fpath: &Option<String>) -> Result<Option<String>, Box<dyn std::error::Error>> {
    match  fpath {
        Some(path) => {
        let mut file = File::open(path)?;
        let mut vmrk = String::new();
        file.read_to_string(&mut vmrk)?;
        Ok(Some(vmrk))
        }

    None => Ok(None),
    }

}


pub fn parse_vmrk(vmrk: &Option<String>) -> Result<Markers, Box<dyn std::error::Error>> {

    let vmrk_content = match vmrk {
        Some(content) => content,
        None => return Err(".VMRK content is missing and required for this operation.".into()),
    };
    let vmrk_vec: Vec<String> = vmrk_content.split("\n").map(|x| x.to_string()).collect();
    let mut markers = Markers {
        n_markers: 0,
        markers: Vec::new()
    };
    let mut marker_vec: Vec<f64> = Vec::new();
    vmrk_vec.iter().for_each(|x| {
        if x.contains("R128") {
        let chars: Vec<&str> = x.split(",").collect();
        //println!("CHARS: {:?}", chars[2]);
        let default: f64 = 0.0;
        marker_vec.push(chars[2].parse::<f64>().unwrap_or(default));
        }
});
    //println!("MARKERS {:?}", marker_vec);
    markers.markers = marker_vec.clone();
    markers.n_markers = marker_vec.len();
    Ok(markers)
}


pub fn parse_bytes(path: &str, eeg_info: &EEGInfo) -> Result<Vec<i16>, Box<dyn std::error::Error>> {
    match eeg_info.binary_format.as_str() {
        "INT_16" => {
            let f = File::open(path).expect("could not open file");
            let mut reader: BufReader<File> = BufReader::new(f);
            let mut buffer = Vec::new();
            // Read file into vector.
            reader
                .read_to_end(&mut buffer)
                .expect("error while reading file");

            let bytes = buffer.to_vec();
            let mut samples: Vec<i16> = Vec::new();
            for pair in bytes.chunks_exact(2) {
                let p = pair.try_into().unwrap();
                samples.push(i16::from_le_bytes(p));
            }

            //print!("Samples {:?}", samples.len());
            Ok(samples)
        }
        _ => Err("Format not supported".into()),
    }
}

pub fn convert_to_seconds(
    samples: Vec<i16>,
    eeg_info: &EEGInfo,
) -> Result<Vec<Vec<i16>>, Box<dyn std::error::Error>> {
    let mut step: i32 = 0; // initialise a samples step counter
    let mut seconds: Vec<Vec<i16>> = Vec::new(); // create vector of vectors for seconds
    seconds.push(Vec::new()); // Initialise the first vector

    let mut n = 0; // initialise second counter

    for s in samples.iter() {
        seconds[n].push(*s);
        step += 1;
        if step % (eeg_info.sfreq * eeg_info.num_ch) == 0 {
            n += 1;
            seconds.push(Vec::new());
        }
    }
    Ok(seconds)
}


pub fn demultiplex(
    seconds: Vec<Vec<i16>>,
    eeg_info: &EEGInfo,
) -> Result<Vec<Vec<i16>>, Box<dyn std::error::Error>> {
    let mut channels: Vec<Vec<i16>> = vec![Vec::new(); eeg_info.num_ch as usize];
    if eeg_info.num_ch as usize == 0 {
        return Err("Number of channels cannot be zero".into());
    }
    for second in seconds {
        let mut counter = 0;
        for &val in &second {
            let channel_idx = counter % eeg_info.num_ch;
            channels[channel_idx as usize].push(val);
            counter += 1;
        }
    }
    Ok(channels)
}

pub fn vec_to_array<T, const N: usize>(v: Vec<T>) -> [T; N] {
    v.try_into()
        .unwrap_or_else(|v: Vec<T>| panic!("Expected a Vec of length {} but it was {}", N, v.len()))
}

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


// EDF


pub fn parse_edf(path: &str ) -> Result<Vec<i16>, Box<dyn std::error::Error>> {

            let mut file = File::open(path).expect("could not open file");

            let metadata = fs::metadata(path)?;
            println!("METADATA {:?}", metadata);

            let mut reader = BufReader::new(file);
            // The first 256 bytes of an EDF file contain the main header in ASCII.
            let mut header_bytes = vec![0u8; 256];
            // Read exactly 256 bytes into the buffer.
            reader.read_exact(&mut header_bytes)?;
            // Convert the ASCII bytes to a String.
            // Using from_utf8_lossy is safe in case of any non-ASCII characters.
            let header_str = String::from_utf8_lossy(&header_bytes).to_string();

            println!("\n HEADER {:?}", header_str);

            let f = File::open(path).expect("could not open file");
            let mut reader: BufReader<File> = BufReader::new(f);
            let mut buffer = Vec::new();
            // Read file into vector.
            reader
                .read_to_end(&mut buffer)
                .expect("error while reading file");

            println!("DATA {:?}", buffer.len());

            let bytes = buffer.to_vec();
            let mut samples: Vec<i16> = Vec::new();
            for pair in bytes.chunks_exact(2) {
                let p = pair.try_into().unwrap();
                samples.push(i16::from_le_bytes(p));
            }

            //print!("Samples {:?}", samples.len());
            Ok(samples)
        }
