use crate::classes::{Object, State};
use crate::runtime::{Caller, Exception};

/// `CanPayCrimeGold`
///
/// ```text
/// bool Function CanPayCrimeGold() native
/// ```
pub fn can_pay_crime_gold(caller: Caller<'_, State>, faction: Object) -> Result<bool, Exception> {
    todo!()
}
