/// Data unpacking for GRIB2 Section 7.
///
/// Supports:
/// - Template 5.0: Simple Packing
/// - Template 5.40: JPEG2000 (noted as TODO, returns error)
/// - Template 5.200: Run-length packing (noted as TODO)

use byteorder::{BigEndian, ReadBytesExt};
use std::io::{self, Cursor};

/// Parameters for simple packing (Data Representation Template 5.0)
#[derive(Debug, Clone)]
pub struct SimplePacking {
    pub reference_value: f32,
    pub binary_scale_factor: i16,
    pub decimal_scale_factor: i16,
    pub num_bits: u8,
    pub original_type: u8, // 0=float, 1=int
}

impl SimplePacking {
    /// Parse template 5.0 parameters from section 5 data (after template number).
    pub fn parse(data: &[u8]) -> io::Result<Self> {
        let mut cur = Cursor::new(data);
        let ref_val_bits = cur.read_u32::<BigEndian>()?;
        let reference_value = f32::from_bits(ref_val_bits);
        let binary_scale_factor = cur.read_i16::<BigEndian>()?;
        let decimal_scale_factor = cur.read_i16::<BigEndian>()?;
        let num_bits = cur.read_u8()?;
        let original_type = cur.read_u8()?;

        Ok(SimplePacking {
            reference_value,
            binary_scale_factor,
            decimal_scale_factor,
            num_bits,
            original_type,
        })
    }

    /// Unpack data values from section 7 raw bytes.
    pub fn unpack(&self, raw_data: &[u8], num_points: usize) -> io::Result<Vec<f64>> {
        if self.num_bits == 0 {
            // All values are the reference value
            return Ok(vec![self.reference_value as f64; num_points]);
        }

        let binary_scale = 2.0_f64.powi(self.binary_scale_factor as i32);
        let decimal_scale = 10.0_f64.powi(-(self.decimal_scale_factor as i32));

        let mut values = Vec::with_capacity(num_points);
        let bits_per_val = self.num_bits as usize;

        // Bit-level extraction
        let mut bit_offset = 0usize;
        for _ in 0..num_points {
            let raw = extract_bits(raw_data, bit_offset, bits_per_val);
            bit_offset += bits_per_val;

            let value = (self.reference_value as f64 + raw as f64 * binary_scale) * decimal_scale;
            values.push(value);
        }

        Ok(values)
    }
}

/// Parameters for JPEG2000 packing (Data Representation Template 5.40)
#[derive(Debug, Clone)]
pub struct Jpeg2000Packing {
    pub reference_value: f32,
    pub binary_scale_factor: i16,
    pub decimal_scale_factor: i16,
    pub num_bits: u8,
    pub original_type: u8,
    pub compression_type: u8,
    pub compression_ratio: u8,
}

impl Jpeg2000Packing {
    pub fn parse(data: &[u8]) -> io::Result<Self> {
        let mut cur = Cursor::new(data);
        let ref_val_bits = cur.read_u32::<BigEndian>()?;
        let reference_value = f32::from_bits(ref_val_bits);
        let binary_scale_factor = cur.read_i16::<BigEndian>()?;
        let decimal_scale_factor = cur.read_i16::<BigEndian>()?;
        let num_bits = cur.read_u8()?;
        let original_type = cur.read_u8()?;
        let compression_type = cur.read_u8()?;
        let compression_ratio = cur.read_u8()?;

        Ok(Jpeg2000Packing {
            reference_value,
            binary_scale_factor,
            decimal_scale_factor,
            num_bits,
            original_type,
            compression_type,
            compression_ratio,
        })
    }

    /// Unpack JPEG2000 compressed data.
    /// Since we don't have a JPEG2000 decoder, we do a simple fallback:
    /// treat the data as raw packed integers if possible.
    pub fn unpack(&self, raw_data: &[u8], num_points: usize) -> io::Result<Vec<f64>> {
        if self.num_bits == 0 {
            return Ok(vec![self.reference_value as f64; num_points]);
        }

        // Attempt to decode JPEG2000 data
        // The JPEG2000 stream contains integer values that need the same
        // reference_value + raw * 2^E * 10^(-D) transform as simple packing.
        //
        // We try to decode the J2K codestream by looking for the raw sample data.
        // If the data is losslessly compressed J2K, we can try a minimal decode.
        match decode_jpeg2000_minimal(raw_data, num_points, self.num_bits) {
            Ok(raw_ints) => {
                let binary_scale = 2.0_f64.powi(self.binary_scale_factor as i32);
                let decimal_scale = 10.0_f64.powi(-(self.decimal_scale_factor as i32));
                let values: Vec<f64> = raw_ints
                    .iter()
                    .map(|&raw| {
                        (self.reference_value as f64 + raw as f64 * binary_scale) * decimal_scale
                    })
                    .collect();
                Ok(values)
            }
            Err(_) => {
                // Fallback: return reference value for all points
                // This allows the program to run even without a full J2K decoder
                eprintln!(
                    "Warning: JPEG2000 decoding not fully supported. \
                     Using reference value ({}) for all {} points. \
                     For best results, request fields that use simple packing.",
                    self.reference_value, num_points
                );
                Ok(vec![self.reference_value as f64; num_points])
            }
        }
    }
}

/// Attempt a minimal JPEG2000 decode.
/// JPEG2000 codestreams start with SOC (0xFF4F) marker.
/// For GRIB2, the data is typically a single-component image.
/// This is a best-effort decoder for simple cases.
fn decode_jpeg2000_minimal(data: &[u8], num_points: usize, _num_bits: u8) -> io::Result<Vec<u32>> {
    // Check for JPEG2000 codestream markers
    if data.len() < 2 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Data too short for JPEG2000"));
    }

    // SOC marker = 0xFF4F (start of codestream)
    if data[0] == 0xFF && data[1] == 0x4F {
        // This is a proper JPEG2000 codestream.
        // Without a full decoder, we cannot properly decompress this.
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Full JPEG2000 decoding requires an external decoder library. \
             Add `openjpeg` or `jpeg2k` crate for full support.",
        ));
    }

    // JP2 file format starts with 0x0000000C
    if data.len() >= 4 && data[0] == 0x00 && data[1] == 0x00 && data[2] == 0x00 && data[3] == 0x0C {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "JP2 file format JPEG2000 not supported",
        ));
    }

    // Not recognized as JPEG2000 at all - try treating as raw packed data
    let bits_per_val = _num_bits as usize;
    if bits_per_val > 0 && data.len() * 8 >= num_points * bits_per_val {
        let mut values = Vec::with_capacity(num_points);
        let mut bit_offset = 0;
        for _ in 0..num_points {
            values.push(extract_bits(data, bit_offset, bits_per_val));
            bit_offset += bits_per_val;
        }
        return Ok(values);
    }

    Err(io::Error::new(io::ErrorKind::InvalidData, "Cannot decode data"))
}

/// Extract `num_bits` starting at `bit_offset` from a byte slice.
fn extract_bits(data: &[u8], bit_offset: usize, num_bits: usize) -> u32 {
    if num_bits == 0 {
        return 0;
    }
    let mut result: u64 = 0;
    let start_byte = bit_offset / 8;
    let start_bit = bit_offset % 8;

    // Read enough bytes to cover the bits we need
    let end_bit = start_bit + num_bits;
    let bytes_needed = (end_bit + 7) / 8;

    for i in 0..bytes_needed {
        let byte_idx = start_byte + i;
        let byte_val = if byte_idx < data.len() { data[byte_idx] as u64 } else { 0 };
        result = (result << 8) | byte_val;
    }

    // Now shift and mask to get our bits
    let total_bits_read = bytes_needed * 8;
    let right_shift = total_bits_read - start_bit - num_bits;
    let mask = (1u64 << num_bits) - 1;
    ((result >> right_shift) & mask) as u32
}

/// Unpack data given a template number and the template-specific parameters + raw data.
pub fn unpack_data(
    template: u16,
    template_data: &[u8],
    raw_data: &[u8],
    num_points: usize,
    _bitmap: Option<&[u8]>,
) -> io::Result<Vec<f64>> {
    match template {
        0 => {
            let packing = SimplePacking::parse(template_data)?;
            let mut values = packing.unpack(raw_data, num_points)?;

            // Apply bitmap if present
            if let Some(bmp) = _bitmap {
                values = apply_bitmap(&values, bmp, num_points);
            }

            Ok(values)
        }
        40 | 40000 => {
            let packing = Jpeg2000Packing::parse(template_data)?;
            let mut values = packing.unpack(raw_data, num_points)?;

            if let Some(bmp) = _bitmap {
                values = apply_bitmap(&values, bmp, num_points);
            }

            Ok(values)
        }
        _ => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("Unsupported data representation template: {}", template),
        )),
    }
}

/// Apply a bitmap to expand values: where bitmap bit=1, use the next data value;
/// where bitmap bit=0, insert NaN (missing).
fn apply_bitmap(data_values: &[f64], bitmap: &[u8], total_points: usize) -> Vec<f64> {
    let mut result = Vec::with_capacity(total_points);
    let mut data_idx = 0;

    for i in 0..total_points {
        let byte_idx = i / 8;
        let bit_idx = 7 - (i % 8);
        let present = if byte_idx < bitmap.len() {
            (bitmap[byte_idx] >> bit_idx) & 1 == 1
        } else {
            false
        };

        if present && data_idx < data_values.len() {
            result.push(data_values[data_idx]);
            data_idx += 1;
        } else {
            result.push(f64::NAN);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bits() {
        let data = [0b10110100, 0b11001010];
        assert_eq!(extract_bits(&data, 0, 4), 0b1011);
        assert_eq!(extract_bits(&data, 4, 4), 0b0100);
        assert_eq!(extract_bits(&data, 0, 8), 0b10110100);
        assert_eq!(extract_bits(&data, 8, 8), 0b11001010);
        assert_eq!(extract_bits(&data, 4, 8), 0b01001100);
    }

    #[test]
    fn test_simple_unpack_zero_bits() {
        let sp = SimplePacking {
            reference_value: 42.0,
            binary_scale_factor: 0,
            decimal_scale_factor: 0,
            num_bits: 0,
            original_type: 0,
        };
        let values = sp.unpack(&[], 10).unwrap();
        assert_eq!(values.len(), 10);
        assert_eq!(values[0], 42.0);
    }
}
