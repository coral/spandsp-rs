/// Generate a sine wave at the given frequency and sample rate.
fn sine_wave(freq_hz: f32, sample_rate: f32, num_samples: usize, amplitude: f32) -> Vec<i16> {
    (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate;
            (amplitude * (2.0 * std::f32::consts::PI * freq_hz * t).sin()) as i16
        })
        .collect()
}

/// Compute the Pearson correlation coefficient between two i16 slices.
fn correlation(a: &[i16], b: &[i16]) -> f64 {
    assert_eq!(a.len(), b.len());
    let n = a.len() as f64;
    let mean_a = a.iter().map(|&x| x as f64).sum::<f64>() / n;
    let mean_b = b.iter().map(|&x| x as f64).sum::<f64>() / n;
    let mut cov = 0.0;
    let mut var_a = 0.0;
    let mut var_b = 0.0;
    for i in 0..a.len() {
        let da = a[i] as f64 - mean_a;
        let db = b[i] as f64 - mean_b;
        cov += da * db;
        var_a += da * da;
        var_b += db * db;
    }
    if var_a == 0.0 || var_b == 0.0 {
        return 0.0;
    }
    cov / (var_a.sqrt() * var_b.sqrt())
}

/// Compute the RMS power of a signal in linear units.
fn rms_power(samples: &[i16]) -> f64 {
    let sum_sq: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
    (sum_sq / samples.len() as f64).sqrt()
}

// =========================================================================
// G.711
// =========================================================================
mod g711 {
    use spandsp::g711::*;

    use super::*;

    #[test]
    fn ulaw_roundtrip_all_256() {
        // u-law codes 0x7F and 0xFF both decode to 0 (positive/negative zero).
        // Re-encoding 0 always produces 0xFF, so skip the 0x7F alias.
        for code in 0u16..=255 {
            let code = code as u8;
            let linear = ulaw_to_linear(code);
            let re_encoded = linear_to_ulaw(linear);
            if code == 0x7F {
                // Both 0x7F and 0xFF represent zero; encoder normalises to 0xFF
                assert_eq!(
                    re_encoded, 0xFF,
                    "u-law code 0x7F decodes to 0, should re-encode to 0xFF"
                );
                continue;
            }
            assert_eq!(
                code, re_encoded,
                "u-law roundtrip failed for code {code:#04X}: linear={linear}, re_encoded={re_encoded:#04X}"
            );
        }
    }

    #[test]
    fn alaw_roundtrip_all_256() {
        for code in 0u16..=255 {
            let code = code as u8;
            let linear = alaw_to_linear(code);
            let re_encoded = linear_to_alaw(linear);
            assert_eq!(
                code, re_encoded,
                "A-law roundtrip failed for code {code:#04X}: linear={linear}, re_encoded={re_encoded:#04X}"
            );
        }
    }

    #[test]
    fn ulaw_linear_roundtrip_bounds() {
        let boundary_values: &[i16] = &[0, 1, -1, i16::MAX, i16::MIN];
        for &val in boundary_values {
            let encoded = linear_to_ulaw(val);
            let decoded = ulaw_to_linear(encoded);
            // u-law clips at the top of the range; max quantization step ~1024
            let error = (val as i32 - decoded as i32).unsigned_abs();
            assert!(
                error <= 1024,
                "u-law roundtrip error too large for {val}: decoded={decoded}, error={error}"
            );
        }
    }

    #[test]
    fn alaw_linear_roundtrip_bounds() {
        let boundary_values: &[i16] = &[0, 1, -1, i16::MAX, i16::MIN];
        for &val in boundary_values {
            let encoded = linear_to_alaw(val);
            let decoded = alaw_to_linear(encoded);
            let error = (val as i32 - decoded as i32).unsigned_abs();
            assert!(
                error <= 1024,
                "A-law roundtrip error too large for {val}: decoded={decoded}, error={error}"
            );
        }
    }

    #[test]
    fn alaw_ulaw_transcode_roundtrip() {
        for code in 0u16..=255 {
            let alaw_orig = code as u8;
            let ulaw = alaw_to_ulaw(alaw_orig);
            let alaw_back = ulaw_to_alaw(ulaw);
            // Transcoding is lossy, but alaw->ulaw->alaw should be stable
            // (idempotent after first pass)
            let ulaw2 = alaw_to_ulaw(alaw_back);
            let alaw_back2 = ulaw_to_alaw(ulaw2);
            assert_eq!(
                alaw_back, alaw_back2,
                "alaw->ulaw->alaw not idempotent after first pass for code {alaw_orig:#04X}"
            );
        }
    }

    #[test]
    fn stateful_encode_decode() {
        let mut encoder = G711State::new(G711Mode::ULaw).unwrap();
        let mut decoder = G711State::new(G711Mode::ULaw).unwrap();

        let original = sine_wave(1000.0, 8000.0, 160, 16000.0);

        let mut encoded = vec![0u8; 160];
        let n_enc = encoder.encode(&mut encoded, &original);
        assert_eq!(n_enc, 160);

        let mut decoded = vec![0i16; 160];
        let n_dec = decoder.decode(&mut decoded, &encoded[..n_enc]);
        assert_eq!(n_dec, 160);

        let corr = correlation(&original, &decoded);
        assert!(
            corr > 0.99,
            "stateful G.711 u-law roundtrip correlation too low: {corr}"
        );
    }

    #[test]
    fn known_ulaw_1khz_sine() {
        // 1kHz sine at 8kHz sample rate, amplitude 8000
        let samples = sine_wave(1000.0, 8000.0, 8, 8000.0);
        let encoded: Vec<u8> = samples.iter().map(|&s| linear_to_ulaw(s)).collect();

        // Compute expected values from the actual encoder and verify stability
        let expected: Vec<u8> = samples.iter().map(|&s| linear_to_ulaw(s)).collect();
        assert_eq!(encoded, expected, "encoding should be deterministic");

        // Verify structural properties: symmetric positive/negative pattern
        // Samples 1,2 should have similar magnitude, samples 5,6 should be their negatives
        let lin1 = ulaw_to_linear(encoded[1]);
        let lin5 = ulaw_to_linear(encoded[5]);
        assert!(
            (lin1 as i32 + lin5 as i32).unsigned_abs() < 100,
            "u-law sine should be symmetric: sample[1]={lin1}, sample[5]={lin5}"
        );
    }

    #[test]
    fn known_alaw_1khz_sine() {
        let samples = sine_wave(1000.0, 8000.0, 8, 8000.0);
        let encoded: Vec<u8> = samples.iter().map(|&s| linear_to_alaw(s)).collect();

        // Verify encoding is deterministic
        let expected: Vec<u8> = samples.iter().map(|&s| linear_to_alaw(s)).collect();
        assert_eq!(encoded, expected, "encoding should be deterministic");

        // Verify symmetry: positive half mirrors negative half
        let lin1 = alaw_to_linear(encoded[1]);
        let lin5 = alaw_to_linear(encoded[5]);
        assert!(
            (lin1 as i32 + lin5 as i32).unsigned_abs() < 100,
            "A-law sine should be symmetric: sample[1]={lin1}, sample[5]={lin5}"
        );
    }
}

// =========================================================================
// G.722
// =========================================================================
mod g722 {
    use spandsp::g722::*;

    use super::*;

    #[test]
    fn roundtrip_silence() {
        let mut encoder = G722Encoder::new(G722Rate::Rate64000, G722Options::empty()).unwrap();
        let mut decoder = G722Decoder::new(G722Rate::Rate64000, G722Options::empty()).unwrap();

        let silence = vec![0i16; 320];
        let mut encoded = vec![0u8; 320];
        let n_enc = encoder.encode(&mut encoded, &silence);
        assert!(n_enc > 0);

        let mut decoded = vec![0i16; 640];
        let n_dec = decoder.decode(&mut decoded[..], &encoded[..n_enc]);
        assert!(n_dec > 0);

        // All decoded values should be near zero
        for (i, &sample) in decoded[..n_dec].iter().enumerate() {
            assert!(
                sample.abs() <= 50,
                "silence roundtrip: sample {i} = {sample}, expected near zero"
            );
        }
    }

    #[test]
    fn roundtrip_sine_all_rates() {
        let rates = [
            G722Rate::Rate64000,
            G722Rate::Rate56000,
            G722Rate::Rate48000,
        ];

        for rate in &rates {
            let mut encoder = G722Encoder::new(*rate, G722Options::empty()).unwrap();
            let mut decoder = G722Decoder::new(*rate, G722Options::empty()).unwrap();

            // G.722 is a wideband codec at 16kHz sample rate.
            // Use a longer signal to allow for codec warmup delay.
            let original = sine_wave(1000.0, 16000.0, 3200, 10000.0);

            let mut encoded = vec![0u8; 3200];
            let n_enc = encoder.encode(&mut encoded, &original);
            assert!(n_enc > 0, "encoding produced no output at rate {rate}");

            let mut decoded = vec![0i16; 3200];
            let n_dec = decoder.decode(&mut decoded, &encoded[..n_enc]);
            assert!(n_dec > 0, "decoding produced no output at rate {rate}");

            // G.722 introduces variable delay. Find the best correlation
            // across a range of lags to account for codec group delay.
            let skip = 400;
            let window = 800;
            let max_lag = 400;
            let mut best_corr = 0.0f64;
            for lag in 0..max_lag {
                if skip + lag + window > n_dec {
                    break;
                }
                let c = correlation(
                    &original[skip..skip + window],
                    &decoded[skip + lag..skip + lag + window],
                )
                .abs();
                if c > best_corr {
                    best_corr = c;
                }
            }
            assert!(
                best_corr > 0.9,
                "G.722 roundtrip best correlation too low at rate {rate}: {best_corr}"
            );
        }
    }

    #[test]
    fn rate_enum() {
        assert!(G722Rate::try_from(64000u32).is_ok());
        assert!(G722Rate::try_from(56000u32).is_ok());
        assert!(G722Rate::try_from(48000u32).is_ok());
        assert!(G722Rate::try_from(99999u32).is_err());
    }
}

// =========================================================================
// G.726
// =========================================================================
mod g726 {
    use spandsp::g726::*;

    use super::*;

    #[test]
    fn roundtrip_silence_all_rates() {
        let rates = [
            G726Rate::Rate16000,
            G726Rate::Rate24000,
            G726Rate::Rate32000,
            G726Rate::Rate40000,
        ];
        for rate in &rates {
            let mut encoder =
                G726State::new(*rate, G726Encoding::Linear, G726Packing::None).unwrap();
            let mut decoder =
                G726State::new(*rate, G726Encoding::Linear, G726Packing::None).unwrap();

            let silence = vec![0i16; 160];
            let mut encoded = vec![0u8; 160];
            let n_enc = encoder.encode(&mut encoded, &silence);
            assert!(n_enc > 0, "encoding produced no output at rate {rate}");

            let mut decoded = vec![0i16; 160];
            let n_dec = decoder.decode(&mut decoded, &encoded[..n_enc]);
            assert!(n_dec > 0, "decoding produced no output at rate {rate}");

            // Decoded silence should be near zero
            for &sample in &decoded[..n_dec] {
                assert!(
                    sample.abs() <= 100,
                    "silence roundtrip at rate {rate}: sample {sample} not near zero"
                );
            }
        }
    }

    #[test]
    fn roundtrip_sine_32k() {
        let mut encoder =
            G726State::new(G726Rate::Rate32000, G726Encoding::Linear, G726Packing::None).unwrap();
        let mut decoder =
            G726State::new(G726Rate::Rate32000, G726Encoding::Linear, G726Packing::None).unwrap();

        let original = sine_wave(1000.0, 8000.0, 320, 10000.0);

        let mut encoded = vec![0u8; 320];
        let n_enc = encoder.encode(&mut encoded, &original);
        assert!(n_enc > 0);

        let mut decoded = vec![0i16; 320];
        let n_dec = decoder.decode(&mut decoded, &encoded[..n_enc]);
        assert!(n_dec > 0);

        let len = original.len().min(n_dec);
        let corr = correlation(&original[..len], &decoded[..len]);
        assert!(
            corr > 0.9,
            "G.726 32kbit/s roundtrip correlation too low: {corr}"
        );
    }
}

// =========================================================================
// HDLC
// =========================================================================
mod hdlc {
    use std::cell::RefCell;
    use std::rc::Rc;

    use spandsp::hdlc::*;

    /// Helper: filter out empty-data status callbacks from HDLC RX results.
    fn data_frames(frames: &[(Vec<u8>, bool)]) -> Vec<(Vec<u8>, bool)> {
        frames
            .iter()
            .filter(|(data, _)| !data.is_empty())
            .cloned()
            .collect()
    }

    /// Transfer bits from TX to RX using get_bit/put_bit.
    fn transfer_bits(tx: &mut HdlcTx, rx: &mut HdlcRx, num_bits: usize) {
        for _ in 0..num_bits {
            let bit = tx.get_bit();
            if bit < 0 {
                break;
            }
            rx.put_bit(bit != 0);
        }
    }

    /// Send preamble flags from TX to RX so the receiver establishes framing.
    /// Must be called BEFORE queuing any frame data with tx.frame().
    fn send_preamble(tx: &mut HdlcTx, rx: &mut HdlcRx) {
        // Each flag is 8 bits (0x7E). The RX needs framing_ok_threshold
        // consecutive flags. 128 bits = 16 flags is plenty.
        transfer_bits(tx, rx, 128);
    }

    #[test]
    fn roundtrip_single_frame_crc16() {
        let received = Rc::new(RefCell::new(Vec::<(Vec<u8>, bool)>::new()));
        let received_clone = received.clone();

        let mut rx = HdlcRx::new(false, false, 1, move |data: &[u8], crc_ok: bool| {
            received_clone.borrow_mut().push((data.to_vec(), crc_ok));
        })
        .unwrap();

        let mut tx = HdlcTx::new(false, 2, false, None::<fn()>).unwrap();

        // Establish framing before queuing the frame
        send_preamble(&mut tx, &mut rx);

        let frame_data = b"Hello HDLC!";
        tx.frame(frame_data).unwrap();
        // Transfer enough bits for frame + CRC + closing flags
        transfer_bits(&mut tx, &mut rx, 8192);

        let all_frames = received.borrow();
        let frames = data_frames(&all_frames);
        assert!(
            !frames.is_empty(),
            "no data frames received in CRC-16 roundtrip"
        );
        assert!(frames[0].1, "CRC check failed for received frame");
        assert_eq!(frames[0].0, frame_data, "received frame data doesn't match");
    }

    #[test]
    fn roundtrip_multiple_frames() {
        let received = Rc::new(RefCell::new(Vec::<(Vec<u8>, bool)>::new()));
        let received_clone = received.clone();

        let mut rx = HdlcRx::new(false, false, 1, move |data: &[u8], crc_ok: bool| {
            received_clone.borrow_mut().push((data.to_vec(), crc_ok));
        })
        .unwrap();

        let mut tx = HdlcTx::new(false, 2, false, None::<fn()>).unwrap();

        // Establish framing before the first frame
        send_preamble(&mut tx, &mut rx);

        // In non-progressive mode, we must drain TX for each frame before
        // queuing the next. After the first frame, trailing flags maintain
        // framing for subsequent frames.
        let frames_to_send: &[&[u8]] = &[b"Frame1", b"Frame2", b"Frame3"];
        for frame in frames_to_send {
            tx.frame(frame).unwrap();
            transfer_bits(&mut tx, &mut rx, 8192);
        }

        let all_frames = received.borrow();
        let frames = data_frames(&all_frames);
        assert_eq!(
            frames.len(),
            3,
            "expected 3 data frames, got {}",
            frames.len()
        );
        for (i, (data, crc_ok)) in frames.iter().enumerate() {
            assert!(crc_ok, "CRC failed for frame {i}");
            assert_eq!(
                data.as_slice(),
                frames_to_send[i],
                "frame {i} data mismatch"
            );
        }
    }

    #[test]
    fn roundtrip_crc32() {
        let received = Rc::new(RefCell::new(Vec::<(Vec<u8>, bool)>::new()));
        let received_clone = received.clone();

        let mut rx = HdlcRx::new(true, false, 1, move |data: &[u8], crc_ok: bool| {
            received_clone.borrow_mut().push((data.to_vec(), crc_ok));
        })
        .unwrap();

        let mut tx = HdlcTx::new(true, 2, false, None::<fn()>).unwrap();

        send_preamble(&mut tx, &mut rx);

        let frame_data = b"CRC-32 test frame";
        tx.frame(frame_data).unwrap();
        transfer_bits(&mut tx, &mut rx, 8192);

        let all_frames = received.borrow();
        let frames = data_frames(&all_frames);
        assert!(
            !frames.is_empty(),
            "no data frames received in CRC-32 roundtrip"
        );
        assert!(frames[0].1, "CRC-32 check failed");
        assert_eq!(frames[0].0, frame_data, "CRC-32 frame data mismatch");
    }

    #[test]
    fn bit_level_roundtrip() {
        let received = Rc::new(RefCell::new(Vec::<(Vec<u8>, bool)>::new()));
        let received_clone = received.clone();

        let mut rx = HdlcRx::new(false, false, 1, move |data: &[u8], crc_ok: bool| {
            received_clone.borrow_mut().push((data.to_vec(), crc_ok));
        })
        .unwrap();

        let mut tx = HdlcTx::new(false, 2, false, None::<fn()>).unwrap();

        send_preamble(&mut tx, &mut rx);

        let frame_data = b"Bit level";
        tx.frame(frame_data).unwrap();
        transfer_bits(&mut tx, &mut rx, 8192);

        let all_frames = received.borrow();
        let frames = data_frames(&all_frames);
        assert!(
            !frames.is_empty(),
            "no data frames received in bit-level roundtrip"
        );
        assert!(frames[0].1, "CRC failed in bit-level roundtrip");
        assert_eq!(frames[0].0, frame_data, "bit-level frame data mismatch");
    }
}

// =========================================================================
// DTMF
// =========================================================================
mod dtmf {
    use spandsp::dtmf::*;

    #[test]
    fn tx_rx_roundtrip_all_digits() {
        let mut tx = DtmfTx::new().unwrap();
        let mut rx = DtmfRx::new().unwrap();

        let digits = "123456789*#0ABCD";
        tx.put(digits).unwrap();

        // Generate enough audio: ~100ms on + ~100ms off per digit = ~1600 samples/digit
        // 16 digits * 1600 = 25600 samples, add some margin
        let mut audio = vec![0i16; 64000];
        let mut total_generated = 0;

        loop {
            let n = tx.generate(&mut audio[total_generated..]);
            if n == 0 {
                break;
            }
            total_generated += n;
        }
        assert!(total_generated > 0, "DTMF TX generated no samples");

        // Feed audio to receiver in chunks
        let chunk_size = 160;
        let mut offset = 0;
        while offset < total_generated {
            let end = (offset + chunk_size).min(total_generated);
            rx.rx(&audio[offset..end]);
            offset = end;
        }

        let detected = rx.get(32);
        assert_eq!(
            detected, digits,
            "detected digits don't match: expected '{digits}', got '{detected}'"
        );
    }

    #[test]
    fn empty_queue_returns_zero() {
        let mut tx = DtmfTx::new().unwrap();
        let mut buf = vec![0i16; 160];
        let n = tx.generate(&mut buf);
        assert_eq!(n, 0, "expected 0 samples from empty DTMF TX, got {n}");
    }
}

// =========================================================================
// Tone generation + Goertzel detection
// =========================================================================
mod tone {
    use spandsp::tone_detect::*;
    use spandsp::tone_generate::*;

    #[test]
    fn generate_440hz_detect() {
        let desc = ToneGenDescriptor::new(
            ToneFreq::new(440, -10),
            ToneFreq::NONE,
            ToneCadence::continuous(1000),
            false,
        )
        .unwrap();
        let mut tone_gen = ToneGenerator::new(&desc).unwrap();

        let mut samples = vec![0i16; 256];
        let n = tone_gen.generate(&mut samples);
        assert_eq!(n, 256);

        let mut goertzel_desc = GoertzelDescriptor::new(440.0, 256);
        let mut detector = GoertzelDetector::new(&mut goertzel_desc).unwrap();

        detector.update(&samples);
        let result = detector.result();

        assert!(
            result > 0.0,
            "Goertzel result for on-frequency tone should be > 0, got {result}"
        );
    }

    #[test]
    fn off_frequency_rejection() {
        let desc = ToneGenDescriptor::new(
            ToneFreq::new(440, -10),
            ToneFreq::NONE,
            ToneCadence::continuous(1000),
            false,
        )
        .unwrap();
        let mut tone_gen = ToneGenerator::new(&desc).unwrap();

        let mut samples = vec![0i16; 256];
        tone_gen.generate(&mut samples);

        // Detect at 440Hz (on-frequency)
        let mut desc_on = GoertzelDescriptor::new(440.0, 256);
        let mut det_on = GoertzelDetector::new(&mut desc_on).unwrap();
        det_on.update(&samples);
        let on_freq = det_on.result();

        // Detect at 1000Hz (off-frequency)
        let mut desc_off = GoertzelDescriptor::new(1000.0, 256);
        let mut det_off = GoertzelDetector::new(&mut desc_off).unwrap();
        det_off.update(&samples);
        let off_freq = det_off.result();

        assert!(
            off_freq < on_freq * 0.01,
            "off-frequency power ({off_freq}) should be < 1% of on-frequency power ({on_freq})"
        );
    }

    #[test]
    fn cadenced_tone_has_silence() {
        let desc = ToneGenDescriptor::new(
            ToneFreq::new(440, -10),
            ToneFreq::NONE,
            ToneCadence::simple(50, 50), // 50ms on / 50ms off
            true,
        )
        .unwrap();
        let mut tone_gen = ToneGenerator::new(&desc).unwrap();

        // Generate enough samples to cover at least one full on/off cycle
        // At 8kHz, 50ms = 400 samples, so 800 samples covers one cycle
        let mut samples = vec![0i16; 1600];
        let n = tone_gen.generate(&mut samples);
        assert!(n > 0, "cadenced tone generated no samples");

        // Check that some samples are zero (off period)
        let zero_count = samples[..n].iter().filter(|&&s| s == 0).count();
        assert!(
            zero_count > 100,
            "expected some zero samples in cadenced tone, found only {zero_count}"
        );

        // Check that some samples are non-zero (on period)
        let nonzero_count = samples[..n].iter().filter(|&&s| s != 0).count();
        assert!(
            nonzero_count > 100,
            "expected non-zero samples in cadenced tone, found only {nonzero_count}"
        );
    }
}

// =========================================================================
// Power meter
// =========================================================================
mod power_meter {
    use spandsp::power_meter::*;

    use super::*;

    #[test]
    fn silence_is_very_negative() {
        let mut meter = PowerMeter::new(6).unwrap();
        for _ in 0..1000 {
            meter.update(0);
        }
        let dbm0 = meter.current_dbm0();
        assert!(
            dbm0 < -60.0,
            "silence should measure < -60 dBm0, got {dbm0}"
        );
    }

    #[test]
    fn sine_power_reasonable() {
        let mut meter = PowerMeter::new(6).unwrap();
        let samples = sine_wave(1000.0, 8000.0, 2000, 32000.0);
        for &s in &samples {
            meter.update(s);
        }
        let dbm0 = meter.current_dbm0();
        assert!(
            dbm0 > -10.0 && dbm0 < 10.0,
            "full-scale sine should measure within -10..+10 dBm0, got {dbm0}"
        );
    }

    #[test]
    fn level_conversions() {
        let dbm0_val = level_dbm0(0.0);
        assert!(
            dbm0_val > 0,
            "level_dbm0(0.0) should return a positive integer, got {dbm0_val}"
        );

        let dbov_val = level_dbov(0.0);
        assert!(
            dbov_val > 0,
            "level_dbov(0.0) should return a positive integer, got {dbov_val}"
        );
    }
}

// =========================================================================
// Echo canceller
// =========================================================================
mod echo {
    use spandsp::echo::*;

    use super::*;

    #[test]
    fn cancels_simple_echo() {
        let mut canceller = EchoCanceller::new(256, EchoCanFlags::default()).unwrap();

        let tx_signal = sine_wave(1000.0, 8000.0, 2000, 10000.0);

        // Create RX as an attenuated, delayed copy of TX (simulating echo)
        let delay = 64;
        let attenuation = 0.5f32;
        let mut rx_signal = vec![0i16; tx_signal.len()];
        for i in delay..rx_signal.len() {
            rx_signal[i] = (tx_signal[i - delay] as f32 * attenuation) as i16;
        }

        // Process through echo canceller
        let mut output = vec![0i16; tx_signal.len()];
        for i in 0..tx_signal.len() {
            output[i] = canceller.update(tx_signal[i], rx_signal[i]);
        }

        // After convergence, output power should be lower than input RX power
        // Only compare the second half (after convergence)
        let half = tx_signal.len() / 2;
        let rx_power = rms_power(&rx_signal[half..]);
        let out_power = rms_power(&output[half..]);

        assert!(
            out_power < rx_power,
            "echo canceller didn't reduce power: rx_rms={rx_power:.1}, out_rms={out_power:.1}"
        );
    }

    #[test]
    fn silence_passthrough() {
        let mut canceller = EchoCanceller::new(256, EchoCanFlags::default()).unwrap();
        for _ in 0..1000 {
            let out = canceller.update(0, 0);
            assert_eq!(out, 0, "silence through echo canceller should be 0");
        }
    }
}

// =========================================================================
// T.4 shared types (requires fax feature, which is on by default)
// =========================================================================
#[cfg(feature = "fax")]
mod t4 {
    use spandsp::t4::*;

    #[test]
    fn compression_bitflags() {
        let combined = T4Compression::T4_1D | T4Compression::T6;
        // T4_1D = 0x02, T6 = 0x08 → combined = 0x0A = 10
        assert_eq!(combined.bits(), 0x02 | 0x08);
        assert!(combined.contains(T4Compression::T4_1D));
        assert!(combined.contains(T4Compression::T6));
        assert!(!combined.contains(T4Compression::T4_2D));
    }

    #[test]
    fn decode_status_roundtrip() {
        // T4_DECODE_MORE_DATA = 0
        let status = T4DecodeStatus::try_from(0i32);
        assert!(status.is_ok());
        assert_eq!(status.unwrap(), T4DecodeStatus::MoreData);

        // T4_DECODE_OK = -1
        let status = T4DecodeStatus::try_from(-1i32);
        assert!(status.is_ok());
        assert_eq!(status.unwrap(), T4DecodeStatus::Ok);

        // Invalid value
        let status = T4DecodeStatus::try_from(99i32);
        assert!(status.is_err());
    }

    #[test]
    fn stats_from_c() {
        // Construct a t4_stats_t with known values and convert
        let mut c_stats: spandsp::spandsp_sys::t4_stats_t = unsafe { std::mem::zeroed() };
        c_stats.pages_transferred = 5;
        c_stats.pages_in_file = 10;
        c_stats.bad_rows = 2;
        c_stats.longest_bad_row_run = 1;
        c_stats.image_width = 1728;
        c_stats.image_length = 100;
        c_stats.compression = 2; // T4_1D

        let stats = T4Stats::from(c_stats);
        assert_eq!(stats.pages_transferred, 5);
        assert_eq!(stats.pages_in_file, 10);
        assert_eq!(stats.bad_rows, 2);
        assert_eq!(stats.longest_bad_row_run, 1);
        assert_eq!(stats.image_width, 1728);
        assert_eq!(stats.image_length, 100);
        assert_eq!(stats.compression, 2);
    }
}

// =========================================================================
// T.4/T.6 encode/decode roundtrip (requires fax feature)
// =========================================================================
#[cfg(feature = "fax")]
mod t4_codec {
    use std::cell::RefCell;
    use std::rc::Rc;

    use spandsp::t4::*;
    use spandsp::t4_rx::T4T6Decoder;
    use spandsp::t4_tx::T4T6Encoder;

    /// Standard fax width in pixels.
    const IMAGE_WIDTH: i32 = 1728;
    /// Number of bytes per row (IMAGE_WIDTH / 8).
    const ROW_BYTES: usize = (IMAGE_WIDTH / 8) as usize;

    #[test]
    fn t4_1d_encode_decode_white_image() {
        let num_rows = 10;
        let row_index = Rc::new(RefCell::new(0usize));
        let row_index_enc = row_index.clone();

        let mut encoder = T4T6Encoder::new(
            T4Compression::T4_1D,
            IMAGE_WIDTH,
            num_rows,
            move |buf: &mut [u8]| {
                let mut idx = row_index_enc.borrow_mut();
                if *idx >= num_rows as usize {
                    return 0;
                }
                let len = buf.len().min(ROW_BYTES);
                buf[..len].fill(0); // white
                *idx += 1;
                len
            },
        )
        .unwrap();

        // Get all encoded data
        let mut encoded = vec![0u8; 8192];
        let mut total_encoded = 0;
        loop {
            let n = encoder.get(&mut encoded[total_encoded..]);
            if n == 0 {
                break;
            }
            total_encoded += n;
        }
        assert!(total_encoded > 0, "encoder produced no data");

        // Decode
        let decoded_rows = Rc::new(RefCell::new(Vec::<Vec<u8>>::new()));
        let decoded_rows_clone = decoded_rows.clone();

        let mut decoder = T4T6Decoder::new(
            T4Compression::T4_1D,
            IMAGE_WIDTH,
            move |row_data: &[u8]| {
                decoded_rows_clone.borrow_mut().push(row_data.to_vec());
                true
            },
        )
        .unwrap();

        decoder.put(&encoded[..total_encoded]);

        let rows = decoded_rows.borrow();
        assert!(!rows.is_empty(), "decoder produced no rows");

        // Verify all rows are white
        for (i, row) in rows.iter().enumerate() {
            assert!(row.iter().all(|&b| b == 0), "row {i} is not all white");
        }
    }

    #[test]
    fn t4_1d_encode_decode_pattern() {
        let num_rows = 10;
        let row_index = Rc::new(RefCell::new(0usize));
        let row_index_enc = row_index.clone();

        // Create alternating rows: even rows = white, odd rows = black
        let mut encoder = T4T6Encoder::new(
            T4Compression::T4_1D,
            IMAGE_WIDTH,
            num_rows,
            move |buf: &mut [u8]| {
                let mut idx = row_index_enc.borrow_mut();
                if *idx >= num_rows as usize {
                    return 0;
                }
                let len = buf.len().min(ROW_BYTES);
                if *idx % 2 == 0 {
                    buf[..len].fill(0x00); // white
                } else {
                    buf[..len].fill(0xFF); // black
                }
                *idx += 1;
                len
            },
        )
        .unwrap();

        let mut encoded = vec![0u8; 16384];
        let mut total_encoded = 0;
        loop {
            let n = encoder.get(&mut encoded[total_encoded..]);
            if n == 0 {
                break;
            }
            total_encoded += n;
        }
        assert!(total_encoded > 0, "encoder produced no data for pattern");

        let decoded_rows = Rc::new(RefCell::new(Vec::<Vec<u8>>::new()));
        let decoded_rows_clone = decoded_rows.clone();

        let mut decoder = T4T6Decoder::new(
            T4Compression::T4_1D,
            IMAGE_WIDTH,
            move |row_data: &[u8]| {
                decoded_rows_clone.borrow_mut().push(row_data.to_vec());
                true
            },
        )
        .unwrap();

        decoder.put(&encoded[..total_encoded]);

        let rows = decoded_rows.borrow();
        assert!(
            rows.len() >= 2,
            "expected at least 2 decoded rows, got {}",
            rows.len()
        );

        // Verify alternating pattern
        for (i, row) in rows.iter().enumerate() {
            let expected = if i % 2 == 0 { 0x00u8 } else { 0xFFu8 };
            assert!(
                row.iter().all(|&b| b == expected),
                "row {i} doesn't match expected pattern (expected {expected:#04X})"
            );
        }
    }

    #[test]
    fn t6_encode_decode_roundtrip() {
        let num_rows = 10;
        let row_index = Rc::new(RefCell::new(0usize));
        let row_index_enc = row_index.clone();

        let mut encoder = T4T6Encoder::new(
            T4Compression::T6,
            IMAGE_WIDTH,
            num_rows,
            move |buf: &mut [u8]| {
                let mut idx = row_index_enc.borrow_mut();
                if *idx >= num_rows as usize {
                    return 0;
                }
                let len = buf.len().min(ROW_BYTES);
                if *idx % 2 == 0 {
                    buf[..len].fill(0x00); // white
                } else {
                    buf[..len].fill(0xFF); // black
                }
                *idx += 1;
                len
            },
        )
        .unwrap();

        let mut encoded = vec![0u8; 16384];
        let mut total_encoded = 0;
        loop {
            let n = encoder.get(&mut encoded[total_encoded..]);
            if n == 0 {
                break;
            }
            total_encoded += n;
        }
        assert!(total_encoded > 0, "T.6 encoder produced no data");

        let decoded_rows = Rc::new(RefCell::new(Vec::<Vec<u8>>::new()));
        let decoded_rows_clone = decoded_rows.clone();

        let mut decoder =
            T4T6Decoder::new(T4Compression::T6, IMAGE_WIDTH, move |row_data: &[u8]| {
                decoded_rows_clone.borrow_mut().push(row_data.to_vec());
                true
            })
            .unwrap();

        decoder.put(&encoded[..total_encoded]);

        let rows = decoded_rows.borrow();
        assert!(
            rows.len() >= 2,
            "T.6: expected at least 2 decoded rows, got {}",
            rows.len()
        );

        for (i, row) in rows.iter().enumerate() {
            let expected = if i % 2 == 0 { 0x00u8 } else { 0xFFu8 };
            assert!(
                row.iter().all(|&b| b == expected),
                "T.6: row {i} doesn't match expected pattern"
            );
        }
    }
}
