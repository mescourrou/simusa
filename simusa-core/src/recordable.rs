/*!
The Recordable module defines the [`Recordable`] trait which provides method
to save the state of a struct to be analysed afterward (or during with GUI).
*/

use crate::{context::Context, utils::GeneralValue};

/// Trait providing save state method.
///
/// The generic `RecordType` is the Record which is produced.
/// See structs like [`RobotRecord`](crate::node::node_factory::RobotRecord),
/// [`StateEstimatorRecord`](crate::state_estimators::StateEstimatorRecord) or
/// [`PIDRecord`](crate::controllers::pid::PIDRecord).
///
/// The Record should contain the elements worth to be used in the result
/// analysis.
pub trait Recordable<RecordType> {
    /// Generate the current state Record.
    fn record(&self, context: &Context) -> RecordType;
}

macro_rules! impl_identity_recordable {
    ($t: ty) => {
        impl Recordable<$t> for $t {
            fn record(&self, _context: &Context) -> $t {
                self.clone()
            }
        }
    };
}

impl_identity_recordable!(f32);
impl_identity_recordable!(f64);
impl_identity_recordable!((f32, f32));
impl_identity_recordable!(GeneralValue);
impl_identity_recordable!(i8);
impl_identity_recordable!(i16);
impl_identity_recordable!(i32);
impl_identity_recordable!(i64);
