use std::sync::Arc;

type V2SFormatter = dyn Fn(f32) -> String + Send + Sync;
type S2VFormatter = dyn Fn(&str) -> Option<f32> + Send + Sync;

/// Format a `f32` Hertz value as a rounded `Hz` below 1000 Hz, and as a rounded `kHz` value above
/// 1000 Hz. This already includes the unit.
pub(crate) fn v2s_f32_ms_then_s(sigfig: usize) -> Arc<V2SFormatter> {
    Arc::new(move |mut value| {
        let mut is_seconds = false;
        if value >= 1000.0 {
            value = value / 1000.0;
            is_seconds = true;
        }
        
        // calculate digits after dot based on value and precision
        // https://stackoverflow.com/questions/60497397/how-do-you-format-a-float-to-the-first-significant-decimal-and-with-specified-pr
        let digits = {
            // compute absolute value
            let value_abs = value.abs();

            if value_abs >= 1.0 {
                // reduce by number of digits, minimum 0
                let n = (1.0 + value_abs.log10().floor()) as usize;
                if n <= sigfig {
                    sigfig - n
                } else {
                    0
                }
            } else if value_abs > 0.0 {
                sigfig - 1
            } else {
                0
            }
        };

        if is_seconds {
            format!("{value:.digits$} s")
        } else {
            format!("{value:.digits$} ms")
        }
    })
}

/// Convert an input in the same format at that of [`v2s_f32_hz_then_khz()`] to a Hertz value. This
/// additionally also accepts note names in the same format as [`s2v_i32_note_formatter()`], and
/// optionally also with cents in the form of `D#5, -23 ct.`.
pub(crate) fn s2v_f32_ms_then_s() -> Arc<S2VFormatter> {
    Arc::new(move |string| {
        let string = string.trim();

        // Accept values in either ms (with or without unit) or s
        let duration_segment = string.trim();
        let result = duration_segment
            .trim_end_matches([' ', 'm', 'M', 's', 'S'])
            .parse()
            .ok();

        let last_2_chars = duration_segment.get(duration_segment.len().saturating_sub(2)..);
        match last_2_chars {
            // ends with 'ms', return the parsed value
            Some(unit) if unit.eq_ignore_ascii_case("ms") => result,
            // doesn't end with 'ms', but does end with 's', assume seconds and multiply by 1000.0
            Some(unit) if unit.ends_with("s") => result.map(|x| x * 1000.0),
            // otherwise, just assume ms
            _ => result,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    mod v2s_f32_ms_then_s {
        use super::*;

        #[test]
        fn test_01() {
            let input = 10.0;
            let expected = "10.000 ms";
            assert_eq!(v2s_f32_ms_then_s(3)(input), expected);
        }

        #[test]
        fn test_02() {
            let input = 123.4;
            let expected = "123.400 ms";
            assert_eq!(v2s_f32_ms_then_s(3)(input), expected);
        }

        #[test]
        fn test_03() {
            let input = 123.45678;
            let expected = "123.457 ms";
            assert_eq!(v2s_f32_ms_then_s(3)(input), expected);
        }

        #[test]
        fn test_04() {
            let input = 1234.0;
            let expected = "1.234 s";
            assert_eq!(v2s_f32_ms_then_s(3)(input), expected);
        }

        #[test]
        fn test_05() {
            let input = 1234.5678;
            let expected = "1.235 s";
            assert_eq!(v2s_f32_ms_then_s(3)(input), expected);
        }
    }

    mod s2v_f32_ms_then_s {
        use super::*;

        #[test]
        fn test_01() {
            let input = "10.000 ms";
            let expected = Some(10.0);
            assert_eq!(s2v_f32_ms_then_s()(input), expected);
        }

        #[test]
        fn test_02() {
            let input = "123.400 ms";
            let expected = Some(123.4);
            assert_eq!(s2v_f32_ms_then_s()(input), expected);
        }

        #[test]
        fn test_03() {
            let input = "123.457 ms";
            let expected = Some(123.457);
            assert_eq!(s2v_f32_ms_then_s()(input), expected);
        }

        #[test]
        fn test_04() {
            let input = "1.234 s";
            let expected = Some(1234.0);
            assert_eq!(s2v_f32_ms_then_s()(input), expected);
        }

        #[test]
        fn test_05() {
            let input = "1.235 s";
            let expected = Some(1235.0);
            assert_eq!(s2v_f32_ms_then_s()(input), expected);
        }

        #[test]
        fn test_06() {
            let input = "10.000";
            let expected = Some(10.0);
            assert_eq!(s2v_f32_ms_then_s()(input), expected);
        }

        #[test]
        fn test_07() {
            let input = "123.400";
            let expected = Some(123.4);
            assert_eq!(s2v_f32_ms_then_s()(input), expected);
        }

        #[test]
        fn test_08() {
            let input = "123.457";
            let expected = Some(123.457);
            assert_eq!(s2v_f32_ms_then_s()(input), expected);
        }
    }
}
