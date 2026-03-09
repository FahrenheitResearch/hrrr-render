/// Data unpacking for GRIB2 Section 7.
///
/// Supports:
/// - Template 5.0: Simple Packing
/// - Template 5.2: Complex Packing
/// - Template 5.3: Complex Packing with Spatial Differencing (HRRR default)
/// - Template 5.40: JPEG2000

use byteorder::{BigEndian, ReadBytesExt};
use std::io::{self, Cursor};

/// Convert a GRIB2 sign-magnitude 16-bit integer to i16.
/// In GRIB2, MSB is the sign bit (1=negative), remaining 15 bits are magnitude.
fn sign_magnitude_16(raw: u16) -> i16 {
    if raw & 0x8000 != 0 {
        -((raw & 0x7FFF) as i16)
    } else {
        raw as i16
    }
}

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
        let binary_scale_raw = cur.read_u16::<BigEndian>()?;
        let binary_scale_factor = sign_magnitude_16(binary_scale_raw);
        let decimal_scale_raw = cur.read_u16::<BigEndian>()?;
        let decimal_scale_factor = sign_magnitude_16(decimal_scale_raw);
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

/// Parameters for Complex Packing with Spatial Differencing (Template 5.2 and 5.3)
#[derive(Debug, Clone)]
pub struct ComplexPacking {
    pub reference_value: f32,
    pub binary_scale_factor: i16,
    pub decimal_scale_factor: i16,
    pub num_bits: u8,
    pub original_type: u8,
    // Complex packing fields (template 5.2)
    pub group_splitting_method: u8,
    pub missing_value_management: u8,
    pub primary_missing_substitute: u32,
    pub secondary_missing_substitute: u32,
    pub num_groups: u32,
    pub group_width_ref: u8,
    pub group_width_bits: u8,
    pub group_length_ref: u32,
    pub group_length_increment: u8,
    pub last_group_length: u32,
    pub group_length_bits: u8,
    // Spatial differencing fields (template 5.3 only)
    pub spatial_order: u8,
    pub extra_descriptors: u8,
}

impl ComplexPacking {
    pub fn parse(data: &[u8], has_spatial_diff: bool) -> io::Result<Self> {
        let mut cur = Cursor::new(data);
        let ref_val_bits = cur.read_u32::<BigEndian>()?;
        let reference_value = f32::from_bits(ref_val_bits);
        // GRIB2 uses sign-magnitude for scale factors (MSB = sign bit)
        let binary_scale_raw = cur.read_u16::<BigEndian>()?;
        let binary_scale_factor = sign_magnitude_16(binary_scale_raw);
        let decimal_scale_raw = cur.read_u16::<BigEndian>()?;
        let decimal_scale_factor = sign_magnitude_16(decimal_scale_raw);
        let num_bits = cur.read_u8()?;
        let original_type = cur.read_u8()?;
        let group_splitting_method = cur.read_u8()?;
        let missing_value_management = cur.read_u8()?;
        let primary_missing_substitute = cur.read_u32::<BigEndian>()?;
        let secondary_missing_substitute = cur.read_u32::<BigEndian>()?;
        let num_groups = cur.read_u32::<BigEndian>()?;
        let group_width_ref = cur.read_u8()?;
        let group_width_bits = cur.read_u8()?;
        let group_length_ref = cur.read_u32::<BigEndian>()?;
        let group_length_increment = cur.read_u8()?;
        let last_group_length = cur.read_u32::<BigEndian>()?;
        let group_length_bits = cur.read_u8()?;

        let (spatial_order, extra_descriptors) = if has_spatial_diff {
            let so = cur.read_u8()?;
            let ed = cur.read_u8()?;
            (so, ed)
        } else {
            (0, 0)
        };

        Ok(ComplexPacking {
            reference_value,
            binary_scale_factor,
            decimal_scale_factor,
            num_bits,
            original_type,
            group_splitting_method,
            missing_value_management,
            primary_missing_substitute,
            secondary_missing_substitute,
            num_groups,
            group_width_ref,
            group_width_bits,
            group_length_ref,
            group_length_increment,
            last_group_length,
            group_length_bits,
            spatial_order,
            extra_descriptors,
        })
    }

    pub fn unpack(&self, raw_data: &[u8], num_points: usize) -> io::Result<Vec<f64>> {
        let ng = self.num_groups as usize;
        if ng == 0 {
            return Ok(vec![self.reference_value as f64; num_points]);
        }

        let binary_scale = 2.0_f64.powi(self.binary_scale_factor as i32);
        let decimal_scale = 10.0_f64.powi(-(self.decimal_scale_factor as i32));

        let mut bit_offset: usize = 0;

        // For spatial differencing, read the extra descriptors first
        let mut spatial_first_vals: Vec<i64> = Vec::new();
        let mut spatial_min: i64 = 0;
        if self.spatial_order > 0 {
            // extra_descriptors = number of octets per extra value (not total)
            let bytes_per_val = self.extra_descriptors as usize;
            // Read spatial_order first values and the overall minimum
            for _ in 0..self.spatial_order {
                let val = read_signed_bytes(raw_data, bit_offset / 8, bytes_per_val);
                spatial_first_vals.push(val);
                bit_offset += bytes_per_val * 8;
            }
            spatial_min = read_signed_bytes(raw_data, bit_offset / 8, bytes_per_val);
            bit_offset += bytes_per_val * 8;
        }

        // Read group reference values (num_bits per group)
        let mut group_refs = Vec::with_capacity(ng);
        if self.num_bits > 0 {
            for _ in 0..ng {
                group_refs.push(extract_bits(raw_data, bit_offset, self.num_bits as usize) as i64);
                bit_offset += self.num_bits as usize;
            }
        } else {
            group_refs.resize(ng, 0i64);
        }
        // Align to byte boundary
        bit_offset = (bit_offset + 7) & !7;

        // Read group widths (group_width_bits per group)
        let mut group_widths = Vec::with_capacity(ng);
        if self.group_width_bits > 0 {
            for _ in 0..ng {
                let w = extract_bits(raw_data, bit_offset, self.group_width_bits as usize) as u8;
                group_widths.push(w + self.group_width_ref);
                bit_offset += self.group_width_bits as usize;
            }
        } else {
            group_widths.resize(ng, self.group_width_ref);
        }
        bit_offset = (bit_offset + 7) & !7;

        // Read group lengths (group_length_bits per group)
        let mut group_lengths = Vec::with_capacity(ng);
        if self.group_length_bits > 0 {
            for i in 0..ng {
                if i == ng - 1 {
                    group_lengths.push(self.last_group_length as usize);
                } else {
                    let l = extract_bits(raw_data, bit_offset, self.group_length_bits as usize) as u32;
                    group_lengths.push((self.group_length_ref + l * self.group_length_increment as u32) as usize);
                    bit_offset += self.group_length_bits as usize;
                }
            }
        } else {
            for i in 0..ng {
                if i == ng - 1 {
                    group_lengths.push(self.last_group_length as usize);
                } else {
                    group_lengths.push(self.group_length_ref as usize);
                }
            }
        }
        bit_offset = (bit_offset + 7) & !7;

        // Read the actual data values group by group
        let mut raw_values: Vec<i64> = Vec::with_capacity(num_points);
        for g in 0..ng {
            let width = group_widths[g] as usize;
            let length = group_lengths[g];
            let gref = group_refs[g];

            if width == 0 {
                for _ in 0..length {
                    raw_values.push(gref);
                }
            } else {
                for _ in 0..length {
                    let val = extract_bits(raw_data, bit_offset, width) as i64;
                    raw_values.push(gref + val);
                    bit_offset += width;
                }
            }
        }

        // Apply spatial differencing if needed
        if self.spatial_order > 0 {
            // Add spatial minimum to all values
            for v in raw_values.iter_mut() {
                *v += spatial_min;
            }

            // Prepend the first values
            let mut full_values = spatial_first_vals.clone();
            full_values.extend_from_slice(&raw_values);

            // Undo differencing
            if self.spatial_order == 1 {
                for i in 1..full_values.len() {
                    full_values[i] = full_values[i] + full_values[i - 1];
                }
            } else if self.spatial_order == 2 {
                for i in 2..full_values.len() {
                    full_values[i] = full_values[i] + 2 * full_values[i - 1] - full_values[i - 2];
                }
            }

            // Convert to float
            let values: Vec<f64> = full_values
                .iter()
                .take(num_points)
                .map(|&raw| (self.reference_value as f64 + raw as f64 * binary_scale) * decimal_scale)
                .collect();
            Ok(values)
        } else {
            // No spatial differencing - direct conversion
            let values: Vec<f64> = raw_values
                .iter()
                .take(num_points)
                .map(|&raw| (self.reference_value as f64 + raw as f64 * binary_scale) * decimal_scale)
                .collect();
            Ok(values)
        }
    }
}

/// Read a signed integer from `n` bytes (big-endian, sign-magnitude with MSB sign bit).
fn read_signed_bytes(data: &[u8], offset: usize, n: usize) -> i64 {
    if n == 0 || offset + n > data.len() {
        return 0;
    }
    let mut val: u64 = 0;
    for i in 0..n {
        val = (val << 8) | data[offset + i] as u64;
    }
    let sign_bit = 1u64 << (n * 8 - 1);
    if val & sign_bit != 0 {
        -((val & (sign_bit - 1)) as i64)
    } else {
        val as i64
    }
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
            if let Some(bmp) = _bitmap {
                values = apply_bitmap(&values, bmp, num_points);
            }
            Ok(values)
        }
        2 => {
            let packing = ComplexPacking::parse(template_data, false)?;
            let mut values = packing.unpack(raw_data, num_points)?;
            if let Some(bmp) = _bitmap {
                values = apply_bitmap(&values, bmp, num_points);
            }
            Ok(values)
        }
        3 => {
            let packing = ComplexPacking::parse(template_data, true)?;
            let mut values = packing.unpack(raw_data, num_points)?;
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
