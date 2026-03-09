/// GRIB2 Section parsing (Sections 0 through 8).

use byteorder::{BigEndian, ReadBytesExt};
use std::io::{self, Cursor, Read};

use super::templates::LambertConformal;

/// Section 0: Indicator Section (always 16 bytes)
#[derive(Debug, Clone)]
pub struct IndicatorSection {
    pub discipline: u8,
    pub edition: u8,
    pub total_length: u64,
}

impl IndicatorSection {
    pub fn parse(data: &[u8]) -> io::Result<Self> {
        if data.len() < 16 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Section 0 too short"));
        }
        // Bytes 0-3: "GRIB"
        if &data[0..4] != b"GRIB" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Not a GRIB file (magic: {:?})", &data[0..4]),
            ));
        }
        // Bytes 4-5: reserved
        // Byte 6: discipline
        let discipline = data[6];
        // Byte 7: edition number
        let edition = data[7];
        if edition != 2 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Expected GRIB2 (edition 2), got edition {}", edition),
            ));
        }
        // Bytes 8-15: total length (8 bytes)
        let mut cur = Cursor::new(&data[8..16]);
        let total_length = cur.read_u64::<BigEndian>()?;

        Ok(IndicatorSection { discipline, edition, total_length })
    }
}

/// Section 1: Identification Section
#[derive(Debug, Clone)]
pub struct IdentificationSection {
    pub section_length: u32,
    pub center: u16,
    pub subcenter: u16,
    pub master_table_version: u8,
    pub local_table_version: u8,
    pub significance_of_ref_time: u8,
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub production_status: u8,
    pub data_type: u8,
}

impl IdentificationSection {
    pub fn parse(data: &[u8]) -> io::Result<Self> {
        let mut cur = Cursor::new(data);
        let section_length = cur.read_u32::<BigEndian>()?;
        let section_num = cur.read_u8()?;
        if section_num != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Expected section 1, got {}", section_num),
            ));
        }
        let center = cur.read_u16::<BigEndian>()?;
        let subcenter = cur.read_u16::<BigEndian>()?;
        let master_table_version = cur.read_u8()?;
        let local_table_version = cur.read_u8()?;
        let significance_of_ref_time = cur.read_u8()?;
        let year = cur.read_u16::<BigEndian>()?;
        let month = cur.read_u8()?;
        let day = cur.read_u8()?;
        let hour = cur.read_u8()?;
        let minute = cur.read_u8()?;
        let second = cur.read_u8()?;
        let production_status = cur.read_u8()?;
        let data_type = cur.read_u8()?;

        Ok(IdentificationSection {
            section_length, center, subcenter, master_table_version,
            local_table_version, significance_of_ref_time,
            year, month, day, hour, minute, second,
            production_status, data_type,
        })
    }
}

/// Section 3: Grid Definition Section
#[derive(Debug, Clone)]
pub struct GridDefinitionSection {
    pub section_length: u32,
    pub source: u8,
    pub num_data_points: u32,
    pub num_optional_octets: u8,
    pub interpretation: u8,
    pub template_number: u16,
    /// Template-specific data (raw bytes after the template number)
    pub template_data: Vec<u8>,
}

impl GridDefinitionSection {
    pub fn parse(data: &[u8]) -> io::Result<Self> {
        let mut cur = Cursor::new(data);
        let section_length = cur.read_u32::<BigEndian>()?;
        let section_num = cur.read_u8()?;
        if section_num != 3 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Expected section 3, got {}", section_num),
            ));
        }
        let source = cur.read_u8()?;
        let num_data_points = cur.read_u32::<BigEndian>()?;
        let num_optional_octets = cur.read_u8()?;
        let interpretation = cur.read_u8()?;
        let template_number = cur.read_u16::<BigEndian>()?;

        // Rest is the template data
        let pos = cur.position() as usize;
        let template_data = data[pos..section_length as usize].to_vec();

        Ok(GridDefinitionSection {
            section_length, source, num_data_points, num_optional_octets,
            interpretation, template_number, template_data,
        })
    }

    /// Try to parse as Lambert Conformal Conic (template 30)
    pub fn as_lambert_conformal(&self) -> io::Result<LambertConformal> {
        if self.template_number != 30 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Expected template 30, got {}", self.template_number),
            ));
        }
        LambertConformal::parse(&self.template_data)
    }
}

/// Section 4: Product Definition Section
#[derive(Debug, Clone)]
pub struct ProductDefinitionSection {
    pub section_length: u32,
    pub num_coordinate_values: u16,
    pub template_number: u16,
    pub parameter_category: u8,
    pub parameter_number: u8,
    pub generating_process: u8,
    pub forecast_time: u32,
    pub first_surface_type: u8,
    pub first_surface_scale: u8,
    pub first_surface_value: u32,
    pub template_data: Vec<u8>,
}

impl ProductDefinitionSection {
    pub fn parse(data: &[u8]) -> io::Result<Self> {
        let mut cur = Cursor::new(data);
        let section_length = cur.read_u32::<BigEndian>()?;
        let section_num = cur.read_u8()?;
        if section_num != 4 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Expected section 4, got {}", section_num),
            ));
        }
        let num_coordinate_values = cur.read_u16::<BigEndian>()?;
        let template_number = cur.read_u16::<BigEndian>()?;

        // Template 4.0 and 4.8 share the first fields:
        let parameter_category = cur.read_u8()?;
        let parameter_number = cur.read_u8()?;
        let generating_process = cur.read_u8()?;
        let _background_gen = cur.read_u8()?;
        let _analysis_gen = cur.read_u8()?;
        let _hours_cutoff = cur.read_u16::<BigEndian>()?;
        let _minutes_cutoff = cur.read_u8()?;
        let _time_range_unit = cur.read_u8()?;
        let forecast_time = cur.read_u32::<BigEndian>()?;
        let first_surface_type = cur.read_u8()?;
        let first_surface_scale = cur.read_u8()?;
        let first_surface_value = cur.read_u32::<BigEndian>()?;

        let pos = cur.position() as usize;
        let template_data = if pos < section_length as usize {
            data[pos..section_length as usize].to_vec()
        } else {
            vec![]
        };

        Ok(ProductDefinitionSection {
            section_length, num_coordinate_values, template_number,
            parameter_category, parameter_number, generating_process,
            forecast_time, first_surface_type, first_surface_scale,
            first_surface_value, template_data,
        })
    }
}

/// Section 5: Data Representation Section
#[derive(Debug, Clone)]
pub struct DataRepresentationSection {
    pub section_length: u32,
    pub num_data_points: u32,
    pub template_number: u16,
    pub template_data: Vec<u8>,
}

impl DataRepresentationSection {
    pub fn parse(data: &[u8]) -> io::Result<Self> {
        let mut cur = Cursor::new(data);
        let section_length = cur.read_u32::<BigEndian>()?;
        let section_num = cur.read_u8()?;
        if section_num != 5 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Expected section 5, got {}", section_num),
            ));
        }
        let num_data_points = cur.read_u32::<BigEndian>()?;
        let template_number = cur.read_u16::<BigEndian>()?;

        let pos = cur.position() as usize;
        let template_data = data[pos..section_length as usize].to_vec();

        Ok(DataRepresentationSection {
            section_length, num_data_points, template_number, template_data,
        })
    }
}

/// Section 6: Bitmap Section
#[derive(Debug, Clone)]
pub struct BitmapSection {
    pub section_length: u32,
    pub indicator: u8,
    pub bitmap: Option<Vec<u8>>,
}

impl BitmapSection {
    pub fn parse(data: &[u8]) -> io::Result<Self> {
        let mut cur = Cursor::new(data);
        let section_length = cur.read_u32::<BigEndian>()?;
        let section_num = cur.read_u8()?;
        if section_num != 6 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Expected section 6, got {}", section_num),
            ));
        }
        let indicator = cur.read_u8()?;

        let bitmap = if indicator == 0 {
            // Bitmap present
            let pos = cur.position() as usize;
            Some(data[pos..section_length as usize].to_vec())
        } else {
            // 255 = no bitmap, all data present
            None
        };

        Ok(BitmapSection { section_length, indicator, bitmap })
    }
}

/// Section 7: Data Section (raw bytes)
#[derive(Debug, Clone)]
pub struct DataSection {
    pub section_length: u32,
    pub raw_data: Vec<u8>,
}

impl DataSection {
    pub fn parse(data: &[u8]) -> io::Result<Self> {
        let mut cur = Cursor::new(data);
        let section_length = cur.read_u32::<BigEndian>()?;
        let section_num = cur.read_u8()?;
        if section_num != 7 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Expected section 7, got {}", section_num),
            ));
        }

        let pos = cur.position() as usize;
        let raw_data = data[pos..section_length as usize].to_vec();

        Ok(DataSection { section_length, raw_data })
    }
}
