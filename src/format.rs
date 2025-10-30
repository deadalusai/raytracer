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
            (0, 0, 0, s) => write!(f, "{s}.{ms}s"),
            (0, 0, m, s) => write!(f, "{m}m {s}.{ms}s"),
            (0, h, m, s) => write!(f, "{h}h {m}m {s}.{ms}s"),
            (d, h, m, s) => write!(f, "{d}d {h}h {m}m {s}.{ms}s"),
        }?;
        Ok(())
    }
}

#[cfg(test)]
mod test_formatted_duration {
    use std::time::Duration;
    use crate::format::FormattedDuration;

    fn test(secs: f64, expected: &str) {
        let actual = format!("{}", FormattedDuration(Duration::from_secs_f64(secs)));
        assert_eq!(actual, expected);
    }

    #[test] fn seconds() {
        test(15.555, "15.555s");
    }

    #[test] fn minutes() {
        test(915.555, "15m 15.555s");
    }

    #[test] fn hours() {
        test(54915.555, "15h 15m 15.555s");
    }

    #[test] fn days() {
        test(1350915.555, "15d 15h 15m 15.555s");
    }
}
