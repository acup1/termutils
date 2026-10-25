use crate::{
    color::{Rgb, Rgba},
    terminal::Terminal,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use std::{
    error::Error,
    fmt, fs,
    io::{self, Write},
    path::Path,
    time::{Duration, Instant},
};

const MAX_ENCODED_CHUNK: usize = 4096;
const SUPPORT_QUERY_ID: u32 = u32::MAX - 1;
const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

#[derive(Debug)]
pub enum GraphicsError {
    Io(io::Error),
    EmptyImage,
    RaggedRows {
        row: usize,
        expected: usize,
        actual: usize,
    },
    InvalidPng(&'static str),
    InvalidImageId,
    ImageNotUploaded,
    DimensionMismatch {
        expected: (u32, u32),
        actual: (u32, u32),
    },
    InvalidFrame {
        requested: u32,
        available: u32,
    },
}

impl fmt::Display for GraphicsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "graphics I/O failed: {error}"),
            Self::EmptyImage => write!(formatter, "image dimensions must be non-zero"),
            Self::RaggedRows {
                row,
                expected,
                actual,
            } => write!(
                formatter,
                "image row {row} has {actual} pixels; expected {expected}"
            ),
            Self::InvalidPng(reason) => write!(formatter, "invalid PNG: {reason}"),
            Self::InvalidImageId => write!(formatter, "image ID must be non-zero"),
            Self::ImageNotUploaded => write!(formatter, "image must be uploaded first"),
            Self::DimensionMismatch { expected, actual } => write!(
                formatter,
                "frame dimensions are {}x{}; expected {}x{}",
                actual.0, actual.1, expected.0, expected.1
            ),
            Self::InvalidFrame {
                requested,
                available,
            } => write!(
                formatter,
                "frame {requested} does not exist; image has {available} frames"
            ),
        }
    }
}

impl Error for GraphicsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for GraphicsError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageFormat {
    Rgb,
    Rgba,
    Png,
}

impl ImageFormat {
    const fn protocol_value(self) -> u8 {
        match self {
            Self::Rgb => 24,
            Self::Rgba => 32,
            Self::Png => 100,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageData {
    width: u32,
    height: u32,
    format: ImageFormat,
    bytes: Vec<u8>,
}

impl ImageData {
    pub fn from_rgb<const WIDTH: usize, const HEIGHT: usize>(
        pixels: [[Rgb; WIDTH]; HEIGHT],
    ) -> Result<Self, GraphicsError> {
        let mut bytes = Vec::with_capacity(WIDTH.saturating_mul(HEIGHT).saturating_mul(3));
        for row in pixels {
            for pixel in row {
                bytes.extend_from_slice(&[pixel.r, pixel.g, pixel.b]);
            }
        }
        Self::raw(WIDTH, HEIGHT, ImageFormat::Rgb, bytes)
    }

    pub fn from_rgba<const WIDTH: usize, const HEIGHT: usize>(
        pixels: [[Rgba; WIDTH]; HEIGHT],
    ) -> Result<Self, GraphicsError> {
        let mut bytes = Vec::with_capacity(WIDTH.saturating_mul(HEIGHT).saturating_mul(4));
        for row in pixels {
            for pixel in row {
                bytes.extend_from_slice(&[pixel.r, pixel.g, pixel.b, pixel.a]);
            }
        }
        Self::raw(WIDTH, HEIGHT, ImageFormat::Rgba, bytes)
    }

    pub fn from_rgb_rows(rows: &[Vec<Rgb>]) -> Result<Self, GraphicsError> {
        let width = validate_rows(rows)?;
        let mut bytes = Vec::with_capacity(width.saturating_mul(rows.len()).saturating_mul(3));
        for row in rows {
            for pixel in row {
                bytes.extend_from_slice(&[pixel.r, pixel.g, pixel.b]);
            }
        }
        Self::raw(width, rows.len(), ImageFormat::Rgb, bytes)
    }

    pub fn from_rgba_rows(rows: &[Vec<Rgba>]) -> Result<Self, GraphicsError> {
        let width = validate_rows(rows)?;
        let mut bytes = Vec::with_capacity(width.saturating_mul(rows.len()).saturating_mul(4));
        for row in rows {
            for pixel in row {
                bytes.extend_from_slice(&[pixel.r, pixel.g, pixel.b, pixel.a]);
            }
        }
        Self::raw(width, rows.len(), ImageFormat::Rgba, bytes)
    }

    pub fn from_monochrome<const WIDTH: usize, const HEIGHT: usize>(
        mask: [[bool; WIDTH]; HEIGHT],
        color: Rgba,
    ) -> Result<Self, GraphicsError> {
        let mut bytes = Vec::with_capacity(WIDTH.saturating_mul(HEIGHT).saturating_mul(4));
        for row in mask {
            for visible in row {
                let pixel = if visible { color } else { Rgba::TRANSPARENT };
                bytes.extend_from_slice(&[pixel.r, pixel.g, pixel.b, pixel.a]);
            }
        }
        Self::raw(WIDTH, HEIGHT, ImageFormat::Rgba, bytes)
    }

    pub fn from_monochrome_rows(rows: &[Vec<bool>], color: Rgba) -> Result<Self, GraphicsError> {
        let width = validate_rows(rows)?;
        let mut bytes = Vec::with_capacity(width.saturating_mul(rows.len()).saturating_mul(4));
        for row in rows {
            for visible in row {
                let pixel = if *visible { color } else { Rgba::TRANSPARENT };
                bytes.extend_from_slice(&[pixel.r, pixel.g, pixel.b, pixel.a]);
            }
        }
        Self::raw(width, rows.len(), ImageFormat::Rgba, bytes)
    }

    pub fn solid(width: u32, height: u32, color: Rgba) -> Result<Self, GraphicsError> {
        if width == 0 || height == 0 {
            return Err(GraphicsError::EmptyImage);
        }
        let pixel_count = usize::try_from(width)
            .ok()
            .and_then(|width| {
                usize::try_from(height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .ok_or(GraphicsError::EmptyImage)?;
        let mut bytes = Vec::with_capacity(pixel_count.saturating_mul(4));
        for _ in 0..pixel_count {
            bytes.extend_from_slice(&[color.r, color.g, color.b, color.a]);
        }
        Ok(Self {
            width,
            height,
            format: ImageFormat::Rgba,
            bytes,
        })
    }

    pub fn from_png_file(path: impl AsRef<Path>) -> Result<Self, GraphicsError> {
        Self::from_png_bytes(fs::read(path)?)
    }

    pub fn from_png_bytes(bytes: impl Into<Vec<u8>>) -> Result<Self, GraphicsError> {
        let bytes = bytes.into();
        if bytes.len() < 24 || &bytes[..8] != PNG_SIGNATURE {
            return Err(GraphicsError::InvalidPng("missing PNG signature or IHDR"));
        }
        if u32::from_be_bytes(bytes[8..12].try_into().unwrap()) != 13 || &bytes[12..16] != b"IHDR" {
            return Err(GraphicsError::InvalidPng("first chunk is not IHDR"));
        }
        let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
        let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
        if width == 0 || height == 0 {
            return Err(GraphicsError::InvalidPng("dimensions must be non-zero"));
        }
        Ok(Self {
            width,
            height,
            format: ImageFormat::Png,
            bytes,
        })
    }

    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn height(&self) -> u32 {
        self.height
    }

    pub const fn format(&self) -> ImageFormat {
        self.format
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    fn raw(
        width: usize,
        height: usize,
        format: ImageFormat,
        bytes: Vec<u8>,
    ) -> Result<Self, GraphicsError> {
        if width == 0 || height == 0 {
            return Err(GraphicsError::EmptyImage);
        }
        let width = u32::try_from(width).map_err(|_| GraphicsError::EmptyImage)?;
        let height = u32::try_from(height).map_err(|_| GraphicsError::EmptyImage)?;
        Ok(Self {
            width,
            height,
            format,
            bytes,
        })
    }
}

fn validate_rows<T>(rows: &[Vec<T>]) -> Result<usize, GraphicsError> {
    let width = rows
        .first()
        .map(Vec::len)
        .ok_or(GraphicsError::EmptyImage)?;
    if width == 0 {
        return Err(GraphicsError::EmptyImage);
    }
    for (index, row) in rows.iter().enumerate() {
        if row.len() != width {
            return Err(GraphicsError::RaggedRows {
                row: index,
                expected: width,
                actual: row.len(),
            });
        }
    }
    Ok(width)
}

#[derive(Debug)]
pub struct TerminalImage {
    id: u32,
    data: ImageData,
    uploaded: bool,
    frame_count: u32,
}

impl TerminalImage {
    pub fn new(id: u32, data: ImageData) -> Result<Self, GraphicsError> {
        if id == 0 {
            return Err(GraphicsError::InvalidImageId);
        }
        Ok(Self {
            id,
            data,
            uploaded: false,
            frame_count: 0,
        })
    }

    pub const fn id(&self) -> u32 {
        self.id
    }

    pub const fn frame_count(&self) -> u32 {
        self.frame_count
    }

    pub fn upload(&mut self, writer: &mut impl Write) -> Result<(), GraphicsError> {
        transmit(writer, &format!("a=t,i={}", self.id), &self.data)?;
        self.uploaded = true;
        self.frame_count = 1;
        Ok(())
    }

    pub fn place(
        &self,
        writer: &mut impl Write,
        columns: u32,
        rows: u32,
    ) -> Result<(), GraphicsError> {
        self.require_uploaded()?;
        write_command(
            writer,
            &format!("a=p,i={},p=1,c={},r={},C=1,q=2", self.id, columns, rows),
        )?;
        Ok(())
    }

    pub fn upload_and_place(
        &mut self,
        writer: &mut impl Write,
        columns: u32,
        rows: u32,
    ) -> Result<(), GraphicsError> {
        self.upload(writer)?;
        self.place(writer, columns, rows)
    }

    /// Replaces the root frame in-place, preserving the image ID and placement.
    pub fn replace_frame(
        &mut self,
        writer: &mut impl Write,
        frame: ImageData,
    ) -> Result<(), GraphicsError> {
        self.require_dimensions(&frame)?;
        self.require_uploaded()?;
        transmit(writer, &format!("a=f,i={},r=1,X=1", self.id), &frame)?;
        write_command(writer, &format!("a=a,i={},c=1,q=2", self.id))?;
        self.data = frame;
        Ok(())
    }

    /// Uploads a terminal-managed animation frame and returns its 1-based index.
    pub fn push_frame(
        &mut self,
        writer: &mut impl Write,
        frame: ImageData,
        gap: Duration,
    ) -> Result<u32, GraphicsError> {
        self.require_dimensions(&frame)?;
        self.require_uploaded()?;
        let gap_ms = gap.as_millis().clamp(1, i32::MAX as u128);
        let first_animation_frame = self.frame_count == 1;
        transmit(writer, &format!("a=f,i={},X=1,z={gap_ms}", self.id), &frame)?;
        if first_animation_frame {
            write_command(writer, &format!("a=a,i={},r=1,z={gap_ms},q=2", self.id))?;
        }
        self.frame_count += 1;
        Ok(self.frame_count)
    }

    pub fn show_frame(&self, writer: &mut impl Write, frame: u32) -> Result<(), GraphicsError> {
        self.require_frame(frame)?;
        write_command(writer, &format!("a=a,i={},c={frame},q=2", self.id))?;
        Ok(())
    }

    pub fn start_animation(
        &self,
        writer: &mut impl Write,
        plays: Option<u32>,
    ) -> Result<(), GraphicsError> {
        self.require_uploaded()?;
        let protocol_loops = plays.map_or(1, |plays| plays.saturating_add(1).max(2));
        write_command(
            writer,
            &format!("a=a,i={},s=3,v={protocol_loops},q=2", self.id),
        )?;
        Ok(())
    }

    pub fn stop_animation(&self, writer: &mut impl Write) -> Result<(), GraphicsError> {
        self.require_uploaded()?;
        write_command(writer, &format!("a=a,i={},s=1,q=2", self.id))?;
        Ok(())
    }

    pub fn delete(&mut self, writer: &mut impl Write) -> Result<(), GraphicsError> {
        write_command(writer, &format!("a=d,d=I,i={},q=2", self.id))?;
        self.uploaded = false;
        self.frame_count = 0;
        Ok(())
    }

    fn require_uploaded(&self) -> Result<(), GraphicsError> {
        if self.uploaded {
            Ok(())
        } else {
            Err(GraphicsError::ImageNotUploaded)
        }
    }

    fn require_dimensions(&self, frame: &ImageData) -> Result<(), GraphicsError> {
        let expected = (self.data.width, self.data.height);
        let actual = (frame.width, frame.height);
        if actual == expected {
            Ok(())
        } else {
            Err(GraphicsError::DimensionMismatch { expected, actual })
        }
    }

    fn require_frame(&self, frame: u32) -> Result<(), GraphicsError> {
        self.require_uploaded()?;
        if frame > 0 && frame <= self.frame_count {
            Ok(())
        } else {
            Err(GraphicsError::InvalidFrame {
                requested: frame,
                available: self.frame_count,
            })
        }
    }
}

/// Actively queries the terminal for Kitty graphics protocol support.
pub fn query_support(terminal: &mut Terminal, timeout: Duration) -> io::Result<bool> {
    let restore_mode = !terminal.mode_state;
    if restore_mode {
        terminal.enable_raw_mode()?;
    }

    let result = (|| {
        write!(
            terminal.stdout,
            "\x1b_Gi={SUPPORT_QUERY_ID},s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\\x1b[c"
        )?;
        terminal.refresh()?;

        let deadline = Instant::now() + timeout;
        let mut response = Vec::with_capacity(128);
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Ok(false);
            }
            terminal.get_key(
                remaining.as_millis().clamp(1, u64::MAX as u128) as u64,
                false,
            )?;
            let count = *terminal.readed.lock().unwrap();
            let chunk = {
                let buffer = terminal.reading_buffer.lock().unwrap();
                buffer[..count].to_vec()
            };
            response.extend_from_slice(&chunk);
            if contains_graphics_response(&response, SUPPORT_QUERY_ID) {
                return Ok(true);
            }
            if contains_device_attributes_response(&response) {
                return Ok(false);
            }
            if response.len() > 4096 {
                response.drain(..response.len() - 4096);
            }
        }
    })();

    if restore_mode {
        let restore_result = terminal.disable_raw_mode();
        if result.is_ok() {
            restore_result?;
        }
    }
    result
}

fn transmit(writer: &mut impl Write, control: &str, data: &ImageData) -> io::Result<()> {
    let encoded = STANDARD.encode(&data.bytes);
    let chunks: Vec<_> = encoded.as_bytes().chunks(MAX_ENCODED_CHUNK).collect();
    for (index, chunk) in chunks.iter().enumerate() {
        let more = u8::from(index + 1 < chunks.len());
        if index == 0 {
            write!(
                writer,
                "\x1b_G{control},f={},s={},v={},q=2,m={more};",
                data.format.protocol_value(),
                data.width,
                data.height
            )?;
        } else if control.starts_with("a=f") {
            write!(writer, "\x1b_Ga=f,q=2,m={more};")?;
        } else {
            write!(writer, "\x1b_Gq=2,m={more};")?;
        }
        writer.write_all(chunk)?;
        writer.write_all(b"\x1b\\")?;
    }
    Ok(())
}

fn write_command(writer: &mut impl Write, control: &str) -> io::Result<()> {
    write!(writer, "\x1b_G{control};\x1b\\")
}

fn contains_graphics_response(response: &[u8], id: u32) -> bool {
    let prefix = format!("\x1b_Gi={id};");
    response
        .windows(prefix.len())
        .any(|window| window == prefix.as_bytes())
}

fn contains_device_attributes_response(response: &[u8]) -> bool {
    let mut offset = 0;
    while let Some(start) = response[offset..]
        .windows(2)
        .position(|window| window == b"\x1b[")
    {
        let body = &response[offset + start + 2..];
        if let Some(end) = body.iter().position(|byte| byte.is_ascii_alphabetic()) {
            if body[end] == b'c'
                && body[..end]
                    .iter()
                    .all(|byte| byte.is_ascii_digit() || matches!(byte, b'?' | b'>' | b';' | b':'))
            {
                return true;
            }
        }
        offset += start + 2;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_rgb_rgba_and_monochrome_images() {
        let rgb = ImageData::from_rgb([[Rgb::new(1, 2, 3), Rgb::new(4, 5, 6)]]).unwrap();
        assert_eq!((rgb.width(), rgb.height()), (2, 1));
        assert_eq!(rgb.format(), ImageFormat::Rgb);
        assert_eq!(rgb.bytes(), &[1, 2, 3, 4, 5, 6]);

        let rgba = ImageData::from_rgba([[Rgba::new(7, 8, 9, 10)]]).unwrap();
        assert_eq!(rgba.bytes(), &[7, 8, 9, 10]);

        let mono = ImageData::from_monochrome([[true, false]], Rgba::new(20, 30, 40, 255)).unwrap();
        assert_eq!(mono.bytes(), &[20, 30, 40, 255, 0, 0, 0, 0]);
    }

    #[test]
    fn rejects_empty_and_ragged_matrices() {
        assert!(matches!(
            ImageData::from_rgb_rows(&[]),
            Err(GraphicsError::EmptyImage)
        ));
        assert!(matches!(
            ImageData::from_rgb_rows(&[vec![Rgb::BLACK, Rgb::WHITE], vec![Rgb::BLACK]]),
            Err(GraphicsError::RaggedRows { row: 1, .. })
        ));
    }

    #[test]
    fn reads_png_dimensions_without_decoding_pixels() {
        let png =
            ImageData::from_png_bytes(include_bytes!("../tests/imgs/err.png").as_slice()).unwrap();
        assert_eq!(png.format(), ImageFormat::Png);
        assert!(png.width() > 0);
        assert!(png.height() > 0);
    }

    #[test]
    fn chunks_large_frames_and_keeps_animation_action() {
        let data = ImageData::solid(40, 40, Rgba::WHITE).unwrap();
        let mut image = TerminalImage::new(7, data.clone()).unwrap();
        let mut output = Vec::new();
        image.upload(&mut output).unwrap();
        image.replace_frame(&mut output, data).unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("a=t,i=7,f=32,s=40,v=40,q=2,m=1"));
        assert!(output.contains("a=f,i=7,r=1,X=1,f=32,s=40,v=40,q=2,m=1"));
        assert!(output.contains("\x1b_Ga=f,q=2,m=0;"));
        assert!(output.contains("a=a,i=7,c=1,q=2"));
    }

    #[test]
    fn validates_animation_dimensions_and_frame_numbers() {
        let data = ImageData::solid(2, 2, Rgba::WHITE).unwrap();
        let mut image = TerminalImage::new(1, data).unwrap();
        let mut output = Vec::new();
        image.upload(&mut output).unwrap();
        assert!(matches!(
            image.replace_frame(&mut output, ImageData::solid(3, 2, Rgba::WHITE).unwrap()),
            Err(GraphicsError::DimensionMismatch { .. })
        ));
        assert!(matches!(
            image.show_frame(&mut output, 2),
            Err(GraphicsError::InvalidFrame { .. })
        ));

        let second = image
            .push_frame(
                &mut output,
                ImageData::solid(2, 2, Rgba::BLACK).unwrap(),
                Duration::from_millis(40),
            )
            .unwrap();
        assert_eq!(second, 2);
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("a=a,i=1,r=1,z=40,q=2"));
    }

    #[test]
    fn recognizes_graphics_and_device_attribute_responses() {
        assert!(contains_graphics_response(
            b"\x1b_Gi=4294967294;OK\x1b\\",
            SUPPORT_QUERY_ID
        ));
        assert!(contains_device_attributes_response(b"\x1b[?1;2c"));
    }
}
