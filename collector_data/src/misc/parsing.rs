pub mod duration {
    use std::ops::Deref;

    use color_eyre::{
        eyre::{bail, eyre, Context as _},
        Result,
    };
    use itertools::Itertools as _;

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
    #[serde(try_from = "String", into = "String")]
    pub struct DurationWrapper(pub chrono::Duration);
    use chrono::Duration;
    use serde::Deserialize;

    impl TryFrom<String> for DurationWrapper {
        type Error = color_eyre::Report;

        fn try_from(value: String) -> Result<Self, Self::Error> {
            let chars = value.chars().collect_vec();
            let (time, time_str, parse_fn): (&[char], &str, Box<dyn Fn(i64) -> Option<Duration>>) = match chars.as_slice() {
                [milliseconds @ .., 'm', 's'] => (milliseconds, "milliseconds", Box::new(Duration::try_milliseconds)),
                [seconds @ .., 's'] => (seconds, "seconds", Box::new(Duration::try_seconds)),
                [minutes @ .., 'm'] => (minutes, "minutes", Box::new(Duration::try_minutes)),
                [hours @ .., 'h'] => (hours, "hours", Box::new(Duration::try_hours)),
                x @ _ => bail!(
                    "parsing duration: {x}: invalid suffix (only h, m, s, ms)",
                    x = x.into_iter().collect::<String>()
                ),
            };
            let time = time.into_iter().collect::<String>();
            Ok(DurationWrapper(time.parse::<i64>().context("parsing duration from string").and_then(
                |dur| parse_fn(dur).ok_or_else(|| eyre!("Could not parse {} as {}", time, time_str)),
            )?))
        }
    }

    impl Deref for DurationWrapper {
        type Target = Duration;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
}

pub use duration::DurationWrapper as Duration;

#[allow(non_snake_case)]
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn Duration__try_from_string() {
        assert_eq!(Duration::try_from("35s").unwrap(), Duration(chrono::Duration::seconds(35)));
        assert_eq!(Duration::try_from("27m").unwrap(), Duration(chrono::Duration::minutes(27)));
        assert_eq!(Duration::try_from("3h").unwrap(), Duration(chrono::Duration::hours(3)));
        assert_eq!(Duration::try_from("500ms").unwrap(), Duration(chrono::Duration::milliseconds(500)));

        assert!(Duration::try_from("500").is_err());
        assert!(Duration::try_from("400k").is_err());
        assert!(Duration::try_from("400jkl").is_err());
    }
}
