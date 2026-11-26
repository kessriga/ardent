//! The two shifts file their notes under the same station name.

pub mod log {
    #[derive(Debug, PartialEq, Eq)]
    pub struct Entry(pub u16, pub &'static str);

    pub mod morning {
        use super::Entry;

        pub fn entries() -> [Entry; 2] {
            [
                Entry(370, "Ferry wake at the east ladder"),
                Entry(380, "Tag tied to the west rail"),
            ]
        }
    }

    pub mod evening {
        use super::Entry;

        pub fn entries() -> [Entry; 2] {
            [
                Entry(1110, "Spray covered the red stripe"),
                Entry(1120, "Water reached the brass plate"),
            ]
        }
    }

    impl Entry {
        /// Hour and minute on the station clock, in UTC.
        pub fn clock_time(&self) -> (u16, u16) {
            (self.0 / 60, self.0 % 60)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::log::{evening, morning};

    #[test]
    fn shifts_use_the_same_clock() {
        assert_eq!(morning::entries()[0].clock_time(), (6, 10));
        assert_eq!(evening::entries()[1].clock_time(), (18, 40));
    }
}
