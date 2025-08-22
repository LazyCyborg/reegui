use ndarray::Array2;


pub fn vec_to_ndarray(data: Array2<i16>) -> Result<Array2<i16>, Box<dyn std::error::Error>>{
    if data.is_empty() {
        return Ok(Array2::from_shape_vec((0, 0), Vec::new()).unwrap());
    }

    Ok(data)


   
}
