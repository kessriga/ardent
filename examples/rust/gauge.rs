//! Heights from the gauge on the east ladder.

pub mod gauge {
    pub type Centimetres = i16;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Reading {
        Height(Centimetres),
        Gap { reason: &'static str },
    }

    /// The offset converts a reading on this gauge to a height above gauge zero.
    #[derive(Clone, Copy, Debug)]
    pub struct Gauge {
        pub station: &'static str,
        pub offset: Centimetres,
    }

    pub trait Observe {
        type Sample;
        const UNIT: &'static str;

        fn observe(&self, crests: [Self::Sample; 3]) -> Reading;
    }

    impl Observe for Gauge {
        type Sample = Centimetres;
        const UNIT: &'static str = "cm";

        fn observe(&self, mut crests: [Centimetres; 3]) -> Reading {
            crests.sort_unstable();
            match crests[1].checked_add(self.offset) {
                Some(height) => Reading::Height(height),
                None => Reading::Gap {
                    reason: "height outside gauge range",
                },
            }
        }
    }

    pub static EAST_LADDER: Gauge = Gauge {
        station: "North Pier",
        offset: 0,
    };
}

#[cfg(test)]
mod tests {
    use super::gauge::{EAST_LADDER, Gauge, Observe, Reading};

    #[test]
    fn ferry_wake_does_not_set_the_height() {
        assert_eq!(EAST_LADDER.observe([85, 84, 89]), Reading::Height(85));
    }

    #[test]
    fn offset_can_put_a_reading_out_of_range() {
        let gauge = Gauge {
            offset: 1,
            ..EAST_LADDER
        };
        assert!(matches!(gauge.observe([i16::MAX; 3]), Reading::Gap { .. }));
    }
}
