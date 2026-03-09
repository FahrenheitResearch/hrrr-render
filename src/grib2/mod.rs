/// GRIB2 parser for HRRR weather model data.
///
/// Parses the GRIB2 container format (Sections 0-8) and extracts
/// grid-point data values for rendering.

pub mod packing;
pub mod sections;
pub mod tables;
pub mod templates;

use sections::*;
use std::io;

/// A fully parsed GRIB2 message containing one data field.
#[derive(Debug, Clone)]
pub struct Grib2Message {
    pub indicator: IndicatorSection,
    pub identification: IdentificationSection,
    pub grid_definition: GridDefinitionSection,
    pub product_definition: ProductDefinitionSection,
    pub data_representation: DataRepresentationSection,
    pub bitmap: BitmapSection,
    pub data: DataSection,
}

impl Grib2Message {
    /// Parse a single GRIB2 message from raw bytes.
    /// The bytes should start with "GRIB" and end with "7777".
    pub fn parse(data: &[u8]) -> io::Result<Self> {
        if data.len() < 16 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Data too short for GRIB2"));
        }

        // Section 0: Indicator (always 16 bytes)
        let indicator = IndicatorSection::parse(&data[0..16])?;

        let mut offset = 16usize;

        // Section 1: Identification
        let identification = IdentificationSection::parse(&data[offset..])?;
        offset += identification.section_length as usize;

        // Now we may have Section 2 (local use), Section 3, 4, 5, 6, 7, 8
        // Section 2 is optional; we detect by section number byte at offset+4

        // Skip Section 2 (Local Use) if present
        if offset + 5 <= data.len() && data[offset + 4] == 2 {
            let sec2_len = u32::from_be_bytes([
                data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
            ]) as usize;
            offset += sec2_len;
        }

        // Section 3: Grid Definition
        let grid_definition = GridDefinitionSection::parse(&data[offset..])?;
        offset += grid_definition.section_length as usize;

        // Section 4: Product Definition
        let product_definition = ProductDefinitionSection::parse(&data[offset..])?;
        offset += product_definition.section_length as usize;

        // Section 5: Data Representation
        let data_representation = DataRepresentationSection::parse(&data[offset..])?;
        offset += data_representation.section_length as usize;

        // Section 6: Bitmap
        let bitmap = BitmapSection::parse(&data[offset..])?;
        offset += bitmap.section_length as usize;

        // Section 7: Data
        let data_section = DataSection::parse(&data[offset..])?;
        // offset += data_section.section_length as usize;

        // Section 8: End section "7777" - we don't need to parse it

        Ok(Grib2Message {
            indicator,
            identification,
            grid_definition,
            product_definition,
            data_representation,
            bitmap,
            data: data_section,
        })
    }

    /// Unpack the data values from this message.
    pub fn unpack_values(&self) -> io::Result<Vec<f64>> {
        let num_points = self.grid_definition.num_data_points as usize;

        packing::unpack_data(
            self.data_representation.template_number,
            &self.data_representation.template_data,
            &self.data.raw_data,
            num_points,
            self.bitmap.bitmap.as_deref(),
        )
    }

    /// Get the Lambert Conformal grid parameters (HRRR's native grid).
    pub fn lambert_grid(&self) -> io::Result<templates::LambertConformal> {
        self.grid_definition.as_lambert_conformal()
    }

    /// Get discipline, category, parameter number.
    pub fn parameter_id(&self) -> (u8, u8, u8) {
        (
            self.indicator.discipline,
            self.product_definition.parameter_category,
            self.product_definition.parameter_number,
        )
    }
}

/// Parse all GRIB2 messages from a byte buffer (which may contain multiple messages).
pub fn parse_messages(data: &[u8]) -> io::Result<Vec<Grib2Message>> {
    let mut messages = Vec::new();
    let mut offset = 0;

    while offset + 16 <= data.len() {
        // Look for "GRIB" magic
        if &data[offset..offset + 4] != b"GRIB" {
            offset += 1;
            continue;
        }

        let msg = Grib2Message::parse(&data[offset..])?;
        let msg_len = msg.indicator.total_length as usize;
        messages.push(msg);
        offset += msg_len;
    }

    Ok(messages)
}
