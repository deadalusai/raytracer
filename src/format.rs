use std::time::Duration;
use std::fmt::Display;

pub struct FormattedDuration(pub Duration);

impl Display for FormattedDuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self.0.as_secs();
        let m = s / 60;
        let h = m / 60;
        let d = h / 24;
        let ms = self.0.subsec_millis();
        match (d, h % 24, m % 60, s % 60) {
            (0, 0, 0, 0) => write!(f, "{ms}ms"),
            (0, 0, 0, s) => write!(f, "{:.3}s", secs(s, ms)),
            (0, 0, m, s) => write!(f, "{m}m {:.3}s", secs(s, ms)),
            (0, h, m, s) => write!(f, "{h}h {m}m {:.3}s", secs(s, ms)),
            (d, h, m, s) => write!(f, "{d}d {h}h {m}m {:.3}s", secs(s, ms)),
        }?;
        Ok(())
    }
}

fn secs(seconds: u64, millis: u32) -> f32 {
    seconds as f32 + (millis as f32 / 1000.0)
}

#[cfg(test)]
mod test_formatted_duration {
    use std::time::Duration;
    use crate::format::FormattedDuration;

    fn test(secs: f64, expected: &str) {
        let actual = format!("{}", FormattedDuration(Duration::from_secs_f64(secs)));
        assert_eq!(actual, expected);
    }

    #[test]
    fn millis() {
        test(0.015, "15ms");
        test(0.115, "115ms");
        test(0.915, "915ms");
    }

    #[test]
    fn seconds() {
        test(1.0, "1s");
        test(1.015, "1.015s");
        test(2.115, "2.115s");
        test(15.915, "15.915s");
    }

    #[test]
    fn minutes() {
        test(600.0, "10m 0s");
        test(915.015, "15m 15.015s");
    }

    #[test]
    fn hours() {
        test(3600.0, "1h 0m 0s");
        test(54915.015, "15h 15m 15.015s");
    }

    #[test]
    fn days() {
        test(86400.0, "1d 0h 0m 0s");
        test(1350915.015, "15d 15h 15m 15.015s");
    }
}
