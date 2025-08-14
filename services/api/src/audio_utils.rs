use base64::Engine;
use rubato::{FastFixedIn, PolynomialDegree};

// Define standard sample rates for clarity and consistency
pub const OPENAI_REALTIME_API_PCM16_SAMPLE_RATE: f64 = 24000.0;
pub const GEMINI_LIVE_API_PCM16_SAMPLE_RATE: f64 = 16000.0;
pub const FRONTEND_AUDIO_PLAYER_SAMPLE_RATE: f64 = 24000.0; // Frontend expects 24kHz for consistent playback

/// Creates a resampler to convert between audio sample rates.
pub fn create_resampler(
    in_sampling_rate: f64,
    out_sampling_rate: f64,
    chunk_size: usize,
) -> anyhow::Result<FastFixedIn<f32>> {
    let resampler = FastFixedIn::<f32>::new(
        out_sampling_rate / in_sampling_rate,
        1.0,                     // No cutoff frequency, pass all frequencies
        PolynomialDegree::Cubic, // Cubic interpolation for quality
        chunk_size,
        1, // 1 channel (mono)
    )?;
    Ok(resampler)
}

/// Decodes a base64 string representing PCM16 audio into a vector of f32 samples.
/// The function converts the string to a binary vector of u8, interprets chunks as i16 values,
/// and then normalizes them to f32 values between -1.0 and 1.0.
pub fn decode_f32_from_base64_i16(base64_fragment: &str) -> Vec<f32> {
    if let Ok(pcm16_bytes) = base64::engine::general_purpose::STANDARD.decode(base64_fragment) {
        pcm16_bytes
            .chunks_exact(2)
            .map(|chunk| {
                let v = i16::from_le_bytes([chunk[0], chunk[1]]);
                (v as f32 / 32768.0).clamp(-1.0, 1.0)
            })
            .collect()
    } else {
        tracing::error!("Failed to decode base64 fragment to f32");
        Vec::new()
    }
}

/// Encodes a slice of f32 samples into a base64 string (converting to i16 PCM first).
pub fn encode_f32_to_base64_i16(pcm32: &[f32]) -> String {
    let pcm16: Vec<u8> = pcm32
        .iter()
        .flat_map(|&sample| {
            let v = (sample * 32768.0).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            v.to_le_bytes().to_vec()
        })
        .collect();
    base64::engine::general_purpose::STANDARD.encode(&pcm16)
}

/// Converts a slice of f32 samples to a vector of i16 samples.
pub fn convert_f32_to_i16(pcm32: &[f32]) -> Vec<i16> {
    pcm32
        .iter()
        .map(|&sample| (sample * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16)
        .collect()
}

/// Converts a slice of i16 samples to a vector of f32 samples.
pub fn convert_i16_to_f32(pcm16: &[i16]) -> Vec<f32> {
    pcm16
        .iter()
        .map(|&sample| sample as f32 / 32768.0)
        .collect()
}

// Basic encoding/decoding for i16 (useful for direct pass-through or debugging)
pub fn encode_i16(pcm16: &[i16]) -> String {
    let pcm16_bytes: Vec<u8> = pcm16
        .iter()
        .flat_map(|&sample| sample.to_le_bytes().to_vec())
        .collect();
    base64::engine::general_purpose::STANDARD.encode(&pcm16_bytes)
}

pub fn decode_i16(base64_fragment: &str) -> Vec<i16> {
    if let Ok(pcm16_bytes) = base64::engine::general_purpose::STANDARD.decode(base64_fragment) {
        pcm16_bytes
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect()
    } else {
        tracing::error!("Failed to decode base64 fragment to i16");
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    #[test]
    fn test_create_resampler() {
        // Test creating a resampler with valid parameters
        let result = create_resampler(16000.0, 24000.0, 1024);
        assert!(result.is_ok());

        // Test creating a resampler with same input and output rates
        let result = create_resampler(24000.0, 24000.0, 1024);
        assert!(result.is_ok());

        // Test creating a resampler with downsampling
        let result = create_resampler(48000.0, 24000.0, 1024);
        assert!(result.is_ok());
    }

    #[test]
    fn test_decode_f32_from_base64_i16() {
        // Test with known values
        // i16 value 16384 = 0x4000 in little endian = [0x00, 0x40]
        // When normalized: 16384 / 32768.0 = 0.5
        let test_data = vec![0x00u8, 0x40u8]; // 16384 in little endian
        let base64_input = base64::engine::general_purpose::STANDARD.encode(&test_data);

        let result = decode_f32_from_base64_i16(&base64_input);
        assert_eq!(result.len(), 1);
        assert_abs_diff_eq!(result[0], 0.5, epsilon = 0.0001);

        // Test with multiple samples
        let test_data = vec![0x00u8, 0x40u8, 0x00u8, 0x80u8]; // [16384, -32768]
        let base64_input = base64::engine::general_purpose::STANDARD.encode(&test_data);

        let result = decode_f32_from_base64_i16(&base64_input);
        assert_eq!(result.len(), 2);
        assert_abs_diff_eq!(result[0], 0.5, epsilon = 0.0001);
        assert_abs_diff_eq!(result[1], -1.0, epsilon = 0.0001);

        // Test with invalid base64
        let result = decode_f32_from_base64_i16("invalid_base64!");
        assert!(result.is_empty());

        // Test with empty string
        let result = decode_f32_from_base64_i16("");
        assert!(result.is_empty());

        // Test with odd number of bytes (should handle gracefully)
        let test_data = vec![0x00u8]; // Only 1 byte, can't form i16
        let base64_input = base64::engine::general_purpose::STANDARD.encode(&test_data);
        let result = decode_f32_from_base64_i16(&base64_input);
        assert!(result.is_empty()); // chunks_exact(2) will skip incomplete chunks
    }

    #[test]
    fn test_encode_f32_to_base64_i16() {
        // Test with known values
        let input = vec![0.5f32, -1.0f32, 0.0f32];
        let result = encode_f32_to_base64_i16(&input);

        // Decode back to verify
        let decoded = decode_f32_from_base64_i16(&result);
        assert_eq!(decoded.len(), 3);
        assert_abs_diff_eq!(decoded[0], 0.5, epsilon = 0.001);
        assert_abs_diff_eq!(decoded[1], -1.0, epsilon = 0.001);
        assert_abs_diff_eq!(decoded[2], 0.0, epsilon = 0.001);

        // Test with values that need clamping
        let input = vec![2.0f32, -2.0f32]; // Should be clamped to ±1.0
        let result = encode_f32_to_base64_i16(&input);
        let decoded = decode_f32_from_base64_i16(&result);
        assert_eq!(decoded.len(), 2);
        // Values should be clamped to valid range
        assert!(decoded[0] <= 1.0);
        assert!(decoded[1] >= -1.0);

        // Test with empty input
        let result = encode_f32_to_base64_i16(&[]);
        let decoded = decode_f32_from_base64_i16(&result);
        assert!(decoded.is_empty());
    }

    #[test]
    fn test_convert_f32_to_i16() {
        // Test with known values
        let input = vec![1.0f32, -1.0f32, 0.0f32, 0.5f32];
        let result = convert_f32_to_i16(&input);

        assert_eq!(result.len(), 4);
        assert_eq!(result[0], i16::MAX);
        // -1.0 * 32767 = -32767, not i16::MIN (-32768)
        assert_eq!(result[1], -32767);
        assert_eq!(result[2], 0);
        assert_eq!(result[3], (0.5 * i16::MAX as f32) as i16);

        // Test with values that need clamping
        let input = vec![2.0f32, -2.0f32];
        let result = convert_f32_to_i16(&input);
        assert_eq!(result[0], i16::MAX);
        assert_eq!(result[1], i16::MIN);

        // Test with empty input
        let result = convert_f32_to_i16(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_convert_i16_to_f32() {
        // Test with known values
        let input = vec![i16::MAX, i16::MIN, 0i16, 16384i16];
        let result = convert_i16_to_f32(&input);

        assert_eq!(result.len(), 4);
        assert_abs_diff_eq!(result[0], i16::MAX as f32 / 32768.0, epsilon = 0.0001);
        assert_abs_diff_eq!(result[1], i16::MIN as f32 / 32768.0, epsilon = 0.0001);
        assert_abs_diff_eq!(result[2], 0.0, epsilon = 0.0001);
        assert_abs_diff_eq!(result[3], 0.5, epsilon = 0.0001);

        // Test with empty input
        let result = convert_i16_to_f32(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_encode_i16() {
        // Test with known values
        let input = vec![256i16, -256i16, 0i16];
        let result = encode_i16(&input);

        // Verify by decoding
        let decoded = decode_i16(&result);
        assert_eq!(decoded, input);

        // Test with empty input
        let result = encode_i16(&[]);
        let decoded = decode_i16(&result);
        assert!(decoded.is_empty());
    }

    #[test]
    fn test_decode_i16() {
        // Test with known values
        let original = vec![256i16, -256i16, 0i16];
        let encoded = encode_i16(&original);
        let decoded = decode_i16(&encoded);

        assert_eq!(decoded, original);

        // Test with invalid base64
        let result = decode_i16("invalid_base64!");
        assert!(result.is_empty());

        // Test with empty string
        let result = decode_i16("");
        assert!(result.is_empty());

        // Test with odd number of bytes
        let test_data = vec![0x00u8]; // Only 1 byte
        let base64_input = base64::engine::general_purpose::STANDARD.encode(&test_data);
        let result = decode_i16(&base64_input);
        assert!(result.is_empty());
    }

    #[test]
    fn test_round_trip_conversions() {
        // Test f32 -> base64 -> f32
        let original_f32 = vec![0.1f32, -0.7f32, 0.0f32, 0.99f32];
        let encoded = encode_f32_to_base64_i16(&original_f32);
        let decoded = decode_f32_from_base64_i16(&encoded);

        assert_eq!(decoded.len(), original_f32.len());
        for (original, decoded) in original_f32.iter().zip(decoded.iter()) {
            assert_abs_diff_eq!(*original, *decoded, epsilon = 0.001);
        }

        // Test i16 -> base64 -> i16
        let original_i16 = vec![1000i16, -2000i16, 0i16, i16::MAX, i16::MIN];
        let encoded = encode_i16(&original_i16);
        let decoded = decode_i16(&encoded);

        assert_eq!(decoded, original_i16);

        // Test f32 -> i16 -> f32
        let original_f32 = vec![0.5f32, -0.25f32, 0.0f32];
        let as_i16 = convert_f32_to_i16(&original_f32);
        let back_to_f32 = convert_i16_to_f32(&as_i16);

        for (original, converted) in original_f32.iter().zip(back_to_f32.iter()) {
            assert_abs_diff_eq!(*original, *converted, epsilon = 0.001);
        }
    }

    #[test]
    fn test_sample_rate_constants() {
        // Verify the constants are reasonable values
        assert_eq!(OPENAI_REALTIME_API_PCM16_SAMPLE_RATE, 24000.0);
        assert_eq!(GEMINI_LIVE_API_PCM16_SAMPLE_RATE, 16000.0);
        assert_eq!(FRONTEND_AUDIO_PLAYER_SAMPLE_RATE, 24000.0);

        // Test that constants work with resampler creation
        let result = create_resampler(
            GEMINI_LIVE_API_PCM16_SAMPLE_RATE,
            OPENAI_REALTIME_API_PCM16_SAMPLE_RATE,
            1024,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_edge_cases() {
        // Test with maximum and minimum f32 values
        let extreme_values = vec![
            f32::MAX,
            f32::MIN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
        ];
        let encoded = encode_f32_to_base64_i16(&extreme_values);
        let decoded = decode_f32_from_base64_i16(&encoded);

        // All should be clamped to valid range
        for value in decoded {
            assert!(value >= -1.0 && value <= 1.0);
        }

        // Test with very large chunk size for resampler
        let result = create_resampler(24000.0, 48000.0, 1_000_000);
        assert!(result.is_ok());
    }
}
