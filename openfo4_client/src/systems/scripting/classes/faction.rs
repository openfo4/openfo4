use openfo4_script::runtime::{Caller, Exception};

use super::{Object, State};

/// `CanPayCrimeGold`
///
/// ```text
/// bool Function CanPayCrimeGold() native
/// ```
pub fn can_pay_crime_gold(caller: Caller<'_, State>, faction: Object) -> Result<bool, Exception> {
    todo!()
}
