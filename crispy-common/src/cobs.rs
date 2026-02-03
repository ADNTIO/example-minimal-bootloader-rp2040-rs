// SPDX-License-Identifier: MIT
// Copyright (c) 2026 ADNT Sarl <info@adnt.io>

//! COBS (Consistent Overhead Byte Stuffing) encoder/decoder.
//!
//! COBS is a framing algorithm that eliminates 0x00 bytes from data,
//! allowing 0x00 to be used as a packet delimiter.

#[cfg(feature = "std")]
extern crate alloc;

#[cfg(feature = "std")]
use alloc::vec::Vec;

use heapless::Vec as HeaplessVec;

/// COBS encode data into a heapless Vec (for no_std).
///
/// The output includes the trailing 0x00 delimiter.
pub fn encode_heapless<const N: usize>(data: &[u8]) -> HeaplessVec<u8, N> {
    let mut output = HeaplessVec::new();
    let mut code_idx = 0;
    let mut code: u8 = 1;

    // Placeholder for first code byte
    let _ = output.push(0);

    for &byte in data {
        if byte == 0 {
            if code_idx < output.len() {
                output[code_idx] = code;
            }
            code_idx = output.len();
            let _ = output.push(0); // placeholder
            code = 1;
        } else {
            let _ = output.push(byte);
            code += 1;
            if code == 255 {
                if code_idx < output.len() {
                    output[code_idx] = code;
                }
                code_idx = output.len();
                let _ = output.push(0); // placeholder
                code = 1;
            }
        }
    }

    if code_idx < output.len() {
        output[code_idx] = code;
    }
    let _ = output.push(0); // delimiter

    output
}

/// COBS decode data from a heapless Vec (for no_std).
///
/// Returns None if decoding fails.
pub fn decode_heapless<const N: usize>(data: &[u8]) -> Option<HeaplessVec<u8, N>> {
    if data.is_empty() {
        return None;
    }

    let mut output = HeaplessVec::new();
    let mut i = 0;

    while i < data.len() {
        let code = data[i] as usize;
        if code == 0 {
            break; // delimiter
        }
        i += 1;

        for _ in 1..code {
            if i >= data.len() {
                return None; // unexpected end
            }
            if output.push(data[i]).is_err() {
                return None; // buffer overflow
            }
            i += 1;
        }

        if code < 255 && i < data.len() && data[i] != 0 && output.push(0).is_err() {
            return None; // buffer overflow
        }
    }

    Some(output)
}

#[cfg(feature = "std")]
/// COBS encode data into a Vec (for std).
///
/// The output includes the trailing 0x00 delimiter.
pub fn encode(data: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(data.len() + data.len() / 254 + 2);
    let mut code_idx = 0;
    let mut code: u8 = 1;

    output.push(0); // placeholder for first code byte

    for &byte in data {
        if byte == 0 {
            output[code_idx] = code;
            code_idx = output.len();
            output.push(0); // placeholder
            code = 1;
        } else {
            output.push(byte);
            code += 1;
            if code == 255 {
                output[code_idx] = code;
                code_idx = output.len();
                output.push(0); // placeholder
                code = 1;
            }
        }
    }
    output[code_idx] = code;
    output.push(0); // delimiter

    output
}

#[cfg(feature = "std")]
/// COBS decode data from a slice (for std).
///
/// Returns None if decoding fails.
pub fn decode(data: &[u8]) -> Option<Vec<u8>> {
    if data.is_empty() {
        return None;
    }

    let mut output = Vec::with_capacity(data.len());
    let mut i = 0;

    while i < data.len() {
        let code = data[i] as usize;
        if code == 0 {
            break; // delimiter
        }
        i += 1;

        for _ in 1..code {
            if i >= data.len() {
                return None; // unexpected end
            }
            output.push(data[i]);
            i += 1;
        }

        if code < 255 && i < data.len() && data[i] != 0 {
            output.push(0);
        }
    }

    Some(output)
}

// Tests that work in both std and no_std modes
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heapless_encode_decode_roundtrip() {
        let data = [0x11, 0x22, 0x00, 0x33];
        let encoded: HeaplessVec<u8, 64> = encode_heapless(&data);
        let decoded: HeaplessVec<u8, 64> = decode_heapless(&encoded).unwrap();
        assert_eq!(&decoded[..], &data[..]);
    }

    #[test]
    fn test_heapless_encode_no_zeros_in_payload() {
        let data = [0x11, 0x22, 0x33];
        let encoded: HeaplessVec<u8, 64> = encode_heapless(&data);
        // Check no zeros except the delimiter at the end
        assert!(encoded[..encoded.len() - 1].iter().all(|&b| b != 0));
        assert_eq!(encoded[encoded.len() - 1], 0);
    }

    #[test]
    fn test_heapless_empty_data() {
        let data: [u8; 0] = [];
        let encoded: HeaplessVec<u8, 64> = encode_heapless(&data);
        let decoded: HeaplessVec<u8, 64> = decode_heapless(&encoded).unwrap();
        assert!(decoded.is_empty());
    }

    #[test]
    fn test_heapless_decode_invalid_returns_none() {
        let invalid: Option<HeaplessVec<u8, 64>> = decode_heapless(&[0x05, 0x01]); // Claims 4 more bytes
        assert!(invalid.is_none());
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_std_encode_decode_roundtrip() {
        let data = [0x11, 0x22, 0x00, 0x33];
        let encoded = encode(&data);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_std_encode_no_zeros() {
        let data = [0x11, 0x22, 0x33];
        let encoded = encode(&data);
        // Check no zeros except the delimiter at the end
        assert!(encoded[..encoded.len() - 1].iter().all(|&b| b != 0));
        assert_eq!(encoded[encoded.len() - 1], 0);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_std_large_data() {
        let data: Vec<u8> = (0..256).map(|i| i as u8).collect();
        let encoded = encode(&data);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_std_heapless_interop() {
        // Verify std and heapless produce compatible encodings
        let data = [0x11, 0x22, 0x00, 0x33, 0x44];

        let std_encoded = encode(&data);
        let heapless_encoded: HeaplessVec<u8, 64> = encode_heapless(&data);

        assert_eq!(&std_encoded[..], &heapless_encoded[..]);

        // Cross-decode
        let std_decoded = decode(&heapless_encoded).unwrap();
        let heapless_decoded: HeaplessVec<u8, 64> = decode_heapless(&std_encoded).unwrap();

        assert_eq!(std_decoded, data);
        assert_eq!(&heapless_decoded[..], &data[..]);
    }
}
