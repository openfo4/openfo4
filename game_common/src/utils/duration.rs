use std::time::Duration;

pub trait DurationExt: Sized {
    fn rem(self, other: Self) -> Self {
        self.checked_rem(other)
            .expect("cannot compute remainder with 0")
    }

    fn checked_rem(self, other: Self) -> Option<Self>;
}

impl DurationExt for Duration {
    fn checked_rem(self, other: Self) -> Option<Self> {
        if other.is_zero() {
            None
        } else {
            // Cannot overflow because A % B is always at most A.
            Some(Self::from_nanos_u128(self.as_nanos() % other.as_nanos()))
        }
    }
}
