use base64::{engine::general_purpose::STANDARD, Engine};
use core::fmt;
use std::{collections::HashMap, fs};

pub struct ImgData {
    id: i32,
    b64_data: String,
    x_offset: i32,
    y_offset: i32,
    format: i32,
    width: i32,
    height: i32,
}

impl ImgData {
    pub fn new(i: i32, data: String) -> Self {
        ImgData {
            id: i,
            b64_data: data,
            x_offset: 0,
            y_offset: 0,
            format: 100,
            width: 0,
            height: 0,
        }
    }
    pub fn new_from_file(i: i32, path: &str, width: i32, height: i32) -> Self {
        let data: String;
        match ImgData::base64_from_file(path) {
            Ok(b64) => data = b64,
            Err(_) => data = "".to_string(),
        }
        ImgData {
            id: i,
            b64_data: data,
            x_offset: 0,
            y_offset: 0,
            format: 100,
            width: width,
            height: height,
        }
    }

    fn base64_from_file(name: &str) -> Result<String, std::io::Error> {
        let data = fs::read(name)?;
        Ok(STANDARD.encode(&data))
    }

    pub fn load(&mut self) {
        print!(
            "\x1b_Gi={},a=t,f={},q=2,X={},Y={},s={},v={};{}\x1b\\",
            self.id,
            self.format,
            self.x_offset,
            self.y_offset,
            self.width,
            self.height,
            self.b64_data
        );
    }

    pub fn draw(&mut self, width: i32, height: i32) {
        print!("{},r={},c={}\x1b", self, width, height);
    }
}

impl From<ImgData> for String {
    fn from(img: ImgData) -> Self {
        format!("\x1b_Ga=p,i={},C=1,q=2\x1b", img.id,)
    }
}
impl fmt::Display for ImgData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\x1b_Ga=p,i={},C=1,q=2", self.id,)
    }
}

pub struct Imgs {
    loaded: HashMap<i32, ImgData>,
    count: i32,
}
