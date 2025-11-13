pub const MAX_SELECTION_DURATION_SECS: f64 = 300.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RangeSelection {
    pub start_sec: f64,
    pub end_sec: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SelectionOutput {
    pub changed: bool,
    pub selection: Option<RangeSelection>,
    pub validation_error: Option<SelectionError>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionError {
    ExceedsMaxDuration,
    InvalidRange,
}

pub fn validate_selection(start: f64, end: f64, max_duration: f64) -> Result<(), SelectionError> {
    if start >= end {
        return Err(SelectionError::InvalidRange);
    }
    let span = end - start;
    if span > max_duration {
        return Err(SelectionError::ExceedsMaxDuration);
    }
    Ok(())
}

pub fn fraction_to_time(fraction: f32, total_duration: f64) -> f64 {
    fraction.clamp(0.0, 1.0) as f64 * total_duration
}

pub fn time_to_fraction(time_sec: f64, total_duration: f64) -> f32 {
    if total_duration <= 0.0 {
        return 0.0;
    }
    (time_sec / total_duration).clamp(0.0, 1.0) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_selection_valid() {
        assert!(validate_selection(0.0, 10.0, MAX_SELECTION_DURATION_SECS).is_ok());
        assert!(validate_selection(0.0, 300.0, MAX_SELECTION_DURATION_SECS).is_ok());
        assert!(validate_selection(50.0, 100.0, MAX_SELECTION_DURATION_SECS).is_ok());
    }

    #[test]
    fn test_validate_selection_exceeds_max() {
        assert_eq!(
            validate_selection(0.0, 301.0, MAX_SELECTION_DURATION_SECS).err(),
            Some(SelectionError::ExceedsMaxDuration)
        );
        assert_eq!(
            validate_selection(0.0, 600.0, MAX_SELECTION_DURATION_SECS).err(),
            Some(SelectionError::ExceedsMaxDuration)
        );
    }

    #[test]
    fn test_validate_selection_inverted() {
        assert_eq!(
            validate_selection(10.0, 5.0, MAX_SELECTION_DURATION_SECS).err(),
            Some(SelectionError::InvalidRange)
        );
        assert_eq!(
            validate_selection(100.0, 50.0, MAX_SELECTION_DURATION_SECS).err(),
            Some(SelectionError::InvalidRange)
        );
    }

    #[test]
    fn test_validate_selection_equal() {
        assert_eq!(
            validate_selection(10.0, 10.0, MAX_SELECTION_DURATION_SECS).err(),
            Some(SelectionError::InvalidRange)
        );
    }

    #[test]
    fn test_fraction_to_time() {
        assert_eq!(fraction_to_time(0.0, 100.0), 0.0);
        assert_eq!(fraction_to_time(0.5, 100.0), 50.0);
        assert_eq!(fraction_to_time(1.0, 100.0), 100.0);
        assert_eq!(fraction_to_time(0.25, 200.0), 50.0);
    }

    #[test]
    fn test_fraction_to_time_clamps() {
        assert_eq!(fraction_to_time(-0.1, 100.0), 0.0);
        assert_eq!(fraction_to_time(1.5, 100.0), 100.0);
    }

    #[test]
    fn test_time_to_fraction() {
        assert_eq!(time_to_fraction(0.0, 100.0), 0.0);
        assert_eq!(time_to_fraction(50.0, 100.0), 0.5);
        assert_eq!(time_to_fraction(100.0, 100.0), 1.0);
        assert_eq!(time_to_fraction(25.0, 100.0), 0.25);
    }

    #[test]
    fn test_time_to_fraction_clamps() {
        assert_eq!(time_to_fraction(-10.0, 100.0), 0.0);
        assert_eq!(time_to_fraction(150.0, 100.0), 1.0);
    }

    #[test]
    fn test_time_to_fraction_zero_duration() {
        assert_eq!(time_to_fraction(50.0, 0.0), 0.0);
        assert_eq!(time_to_fraction(0.0, 0.0), 0.0);
    }

    #[test]
    fn test_roundtrip_conversion() {
        let total_duration = 200.0;
        let times = vec![0.0, 25.0, 50.0, 75.0, 100.0, 150.0, 200.0];
        for time in times {
            let fraction = time_to_fraction(time, total_duration);
            let converted_time = fraction_to_time(fraction, total_duration);
            assert!((time - converted_time).abs() < 0.0001);
            assert!((converted_time - time).abs() < 0.0001);
        }
    }

    #[test]
    fn test_selection_output_default() {
        let output = SelectionOutput::default();
        assert!(!output.changed);
        assert!(output.selection.is_none());
        assert!(output.validation_error.is_none());
    }

    #[test]
    fn test_max_selection_duration_constant() {
        assert_eq!(MAX_SELECTION_DURATION_SECS, 300.0);
    }
}
