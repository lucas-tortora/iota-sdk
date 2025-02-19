// Copyright 2021-2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod address;
mod expiration;
mod governor_address;
mod immutable_alias_address;
mod state_controller_address;
mod storage_deposit_return;
mod timelock;

use alloc::{boxed::Box, collections::BTreeSet, vec::Vec};

use bitflags::bitflags;
use derive_more::{Deref, From};
use iterator_sorted::is_unique_sorted;
use packable::{
    bounded::BoundedU8,
    error::{UnpackError, UnpackErrorExt},
    packer::Packer,
    prefix::BoxedSlicePrefix,
    unpacker::Unpacker,
    Packable,
};

pub use self::{
    address::AddressUnlockCondition, expiration::ExpirationUnlockCondition,
    governor_address::GovernorAddressUnlockCondition, immutable_alias_address::ImmutableAliasAddressUnlockCondition,
    state_controller_address::StateControllerAddressUnlockCondition,
    storage_deposit_return::StorageDepositReturnUnlockCondition, timelock::TimelockUnlockCondition,
};
use crate::types::block::{address::Address, create_bitflags, protocol::ProtocolParameters, Error};

///
#[derive(Clone, Eq, PartialEq, Hash, From)]
pub enum UnlockCondition {
    /// An address unlock condition.
    Address(AddressUnlockCondition),
    /// A storage deposit return unlock condition.
    StorageDepositReturn(StorageDepositReturnUnlockCondition),
    /// A timelock unlock condition.
    Timelock(TimelockUnlockCondition),
    /// An expiration unlock condition.
    Expiration(ExpirationUnlockCondition),
    /// A state controller address unlock condition.
    StateControllerAddress(StateControllerAddressUnlockCondition),
    /// A governor address unlock condition.
    GovernorAddress(GovernorAddressUnlockCondition),
    /// An immutable alias address unlock condition.
    ImmutableAliasAddress(ImmutableAliasAddressUnlockCondition),
}

impl PartialOrd for UnlockCondition {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for UnlockCondition {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.kind().cmp(&other.kind())
    }
}

impl core::fmt::Debug for UnlockCondition {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Address(unlock_condition) => unlock_condition.fmt(f),
            Self::StorageDepositReturn(unlock_condition) => unlock_condition.fmt(f),
            Self::Timelock(unlock_condition) => unlock_condition.fmt(f),
            Self::Expiration(unlock_condition) => unlock_condition.fmt(f),
            Self::StateControllerAddress(unlock_condition) => unlock_condition.fmt(f),
            Self::GovernorAddress(unlock_condition) => unlock_condition.fmt(f),
            Self::ImmutableAliasAddress(unlock_condition) => unlock_condition.fmt(f),
        }
    }
}

impl UnlockCondition {
    /// Return the output kind of an `Output`.
    pub fn kind(&self) -> u8 {
        match self {
            Self::Address(_) => AddressUnlockCondition::KIND,
            Self::StorageDepositReturn(_) => StorageDepositReturnUnlockCondition::KIND,
            Self::Timelock(_) => TimelockUnlockCondition::KIND,
            Self::Expiration(_) => ExpirationUnlockCondition::KIND,
            Self::StateControllerAddress(_) => StateControllerAddressUnlockCondition::KIND,
            Self::GovernorAddress(_) => GovernorAddressUnlockCondition::KIND,
            Self::ImmutableAliasAddress(_) => ImmutableAliasAddressUnlockCondition::KIND,
        }
    }

    /// Returns the [`UnlockConditionFlags`] for the given [`UnlockCondition`].
    pub(crate) fn flag(&self) -> UnlockConditionFlags {
        match self {
            Self::Address(_) => UnlockConditionFlags::ADDRESS,
            Self::StorageDepositReturn(_) => UnlockConditionFlags::STORAGE_DEPOSIT_RETURN,
            Self::Timelock(_) => UnlockConditionFlags::TIMELOCK,
            Self::Expiration(_) => UnlockConditionFlags::EXPIRATION,
            Self::StateControllerAddress(_) => UnlockConditionFlags::STATE_CONTROLLER_ADDRESS,
            Self::GovernorAddress(_) => UnlockConditionFlags::GOVERNOR_ADDRESS,
            Self::ImmutableAliasAddress(_) => UnlockConditionFlags::IMMUTABLE_ALIAS_ADDRESS,
        }
    }

    /// Checks whether the unlock condition is an [`AddressUnlockCondition`].
    pub fn is_address(&self) -> bool {
        matches!(self, Self::Address(_))
    }

    /// Gets the unlock condition as an actual [`AddressUnlockCondition`].
    /// NOTE: Will panic if the unlock condition is not an [`AddressUnlockCondition`].
    pub fn as_address(&self) -> &AddressUnlockCondition {
        if let Self::Address(unlock_condition) = self {
            unlock_condition
        } else {
            panic!("invalid downcast of non-AddressUnlockCondition");
        }
    }

    /// Checks whether the unlock condition is a [`StorageDepositReturnUnlockCondition`].
    pub fn is_storage_deposit_return(&self) -> bool {
        matches!(self, Self::StorageDepositReturn(_))
    }

    /// Gets the unlock condition as an actual [`StorageDepositReturnUnlockCondition`].
    /// NOTE: Will panic if the unlock condition is not a [`StorageDepositReturnUnlockCondition`].
    pub fn as_storage_deposit_return(&self) -> &StorageDepositReturnUnlockCondition {
        if let Self::StorageDepositReturn(unlock_condition) = self {
            unlock_condition
        } else {
            panic!("invalid downcast of non-StorageDepositReturnUnlockCondition");
        }
    }

    /// Checks whether the unlock condition is a [`TimelockUnlockCondition`].
    pub fn is_timelock(&self) -> bool {
        matches!(self, Self::Timelock(_))
    }

    /// Gets the unlock condition as an actual [`TimelockUnlockCondition`].
    /// NOTE: Will panic if the unlock condition is not a [`TimelockUnlockCondition`].
    pub fn as_timelock(&self) -> &TimelockUnlockCondition {
        if let Self::Timelock(unlock_condition) = self {
            unlock_condition
        } else {
            panic!("invalid downcast of non-TimelockUnlockCondition");
        }
    }

    /// Checks whether the unlock condition is an [`ExpirationUnlockCondition`].
    pub fn is_expiration(&self) -> bool {
        matches!(self, Self::Expiration(_))
    }

    /// Gets the unlock condition as an actual [`ExpirationUnlockCondition`].
    /// NOTE: Will panic if the unlock condition is not an [`ExpirationUnlockCondition`].
    pub fn as_expiration(&self) -> &ExpirationUnlockCondition {
        if let Self::Expiration(unlock_condition) = self {
            unlock_condition
        } else {
            panic!("invalid downcast of non-ExpirationUnlockCondition");
        }
    }

    /// Checks whether the unlock condition is a [`StateControllerAddressUnlockCondition`].
    pub fn is_state_controller_address(&self) -> bool {
        matches!(self, Self::StateControllerAddress(_))
    }

    /// Gets the unlock condition as an actual [`StateControllerAddressUnlockCondition`].
    /// NOTE: Will panic if the unlock condition is not a [`StateControllerAddressUnlockCondition`].
    pub fn as_state_controller_address(&self) -> &StateControllerAddressUnlockCondition {
        if let Self::StateControllerAddress(unlock_condition) = self {
            unlock_condition
        } else {
            panic!("invalid downcast of non-StateControllerAddressUnlockCondition");
        }
    }

    /// Checks whether the unlock condition is a [`GovernorAddressUnlockCondition`].
    pub fn is_governor_address(&self) -> bool {
        matches!(self, Self::GovernorAddress(_))
    }

    /// Gets the unlock condition as an actual [`GovernorAddressUnlockCondition`].
    /// NOTE: Will panic if the unlock condition is not a [`GovernorAddressUnlockCondition`].
    pub fn as_governor_address(&self) -> &GovernorAddressUnlockCondition {
        if let Self::GovernorAddress(unlock_condition) = self {
            unlock_condition
        } else {
            panic!("invalid downcast of non-GovernorAddressUnlockCondition");
        }
    }

    /// Checks whether the unlock condition is an [`ImmutableAliasAddressUnlockCondition`].
    pub fn is_immutable_alias_address(&self) -> bool {
        matches!(self, Self::ImmutableAliasAddress(_))
    }

    /// Gets the unlock condition as an actual [`ImmutableAliasAddressUnlockCondition`].
    /// NOTE: Will panic if the unlock condition is not an [`ImmutableAliasAddressUnlockCondition`].
    pub fn as_immutable_alias_address(&self) -> &ImmutableAliasAddressUnlockCondition {
        if let Self::ImmutableAliasAddress(unlock_condition) = self {
            unlock_condition
        } else {
            panic!("invalid downcast of non-ImmutableAliasAddressUnlockCondition");
        }
    }
}

create_bitflags!(
    /// A bitflags-based representation of the set of active [`UnlockCondition`]s.
    pub UnlockConditionFlags,
    u16,
    [
        (ADDRESS, AddressUnlockCondition),
        (STORAGE_DEPOSIT_RETURN, StorageDepositReturnUnlockCondition),
        (TIMELOCK, TimelockUnlockCondition),
        (EXPIRATION, ExpirationUnlockCondition),
        (STATE_CONTROLLER_ADDRESS, StateControllerAddressUnlockCondition),
        (GOVERNOR_ADDRESS, GovernorAddressUnlockCondition),
        (IMMUTABLE_ALIAS_ADDRESS, ImmutableAliasAddressUnlockCondition),
    ]
);

impl Packable for UnlockCondition {
    type UnpackError = Error;
    type UnpackVisitor = ProtocolParameters;

    fn pack<P: Packer>(&self, packer: &mut P) -> Result<(), P::Error> {
        match self {
            Self::Address(unlock_condition) => {
                AddressUnlockCondition::KIND.pack(packer)?;
                unlock_condition.pack(packer)
            }
            Self::StorageDepositReturn(unlock_condition) => {
                StorageDepositReturnUnlockCondition::KIND.pack(packer)?;
                unlock_condition.pack(packer)
            }
            Self::Timelock(unlock_condition) => {
                TimelockUnlockCondition::KIND.pack(packer)?;
                unlock_condition.pack(packer)
            }
            Self::Expiration(unlock_condition) => {
                ExpirationUnlockCondition::KIND.pack(packer)?;
                unlock_condition.pack(packer)
            }
            Self::StateControllerAddress(unlock_condition) => {
                StateControllerAddressUnlockCondition::KIND.pack(packer)?;
                unlock_condition.pack(packer)
            }
            Self::GovernorAddress(unlock_condition) => {
                GovernorAddressUnlockCondition::KIND.pack(packer)?;
                unlock_condition.pack(packer)
            }
            Self::ImmutableAliasAddress(unlock_condition) => {
                ImmutableAliasAddressUnlockCondition::KIND.pack(packer)?;
                unlock_condition.pack(packer)
            }
        }?;

        Ok(())
    }

    fn unpack<U: Unpacker, const VERIFY: bool>(
        unpacker: &mut U,
        visitor: &Self::UnpackVisitor,
    ) -> Result<Self, UnpackError<Self::UnpackError, U::Error>> {
        Ok(match u8::unpack::<_, VERIFY>(unpacker, &()).coerce()? {
            AddressUnlockCondition::KIND => {
                Self::from(AddressUnlockCondition::unpack::<_, VERIFY>(unpacker, &()).coerce()?)
            }
            StorageDepositReturnUnlockCondition::KIND => {
                Self::from(StorageDepositReturnUnlockCondition::unpack::<_, VERIFY>(unpacker, visitor).coerce()?)
            }
            TimelockUnlockCondition::KIND => {
                Self::from(TimelockUnlockCondition::unpack::<_, VERIFY>(unpacker, &()).coerce()?)
            }
            ExpirationUnlockCondition::KIND => {
                Self::from(ExpirationUnlockCondition::unpack::<_, VERIFY>(unpacker, &()).coerce()?)
            }
            StateControllerAddressUnlockCondition::KIND => {
                Self::from(StateControllerAddressUnlockCondition::unpack::<_, VERIFY>(unpacker, &()).coerce()?)
            }
            GovernorAddressUnlockCondition::KIND => {
                Self::from(GovernorAddressUnlockCondition::unpack::<_, VERIFY>(unpacker, &()).coerce()?)
            }
            ImmutableAliasAddressUnlockCondition::KIND => {
                Self::from(ImmutableAliasAddressUnlockCondition::unpack::<_, VERIFY>(unpacker, &()).coerce()?)
            }
            k => return Err(Error::InvalidOutputKind(k)).map_err(UnpackError::Packable),
        })
    }
}

pub(crate) type UnlockConditionCount = BoundedU8<0, { UnlockConditions::COUNT_MAX }>;

///
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Deref, Packable)]
#[packable(unpack_error = Error, with = |e| e.unwrap_item_err_or_else(|p| Error::InvalidUnlockConditionCount(p.into())))]
#[packable(unpack_visitor = ProtocolParameters)]
pub struct UnlockConditions(
    #[packable(verify_with = verify_unique_sorted_packable)] BoxedSlicePrefix<UnlockCondition, UnlockConditionCount>,
);

impl TryFrom<Vec<UnlockCondition>> for UnlockConditions {
    type Error = Error;

    #[inline(always)]
    fn try_from(unlock_conditions: Vec<UnlockCondition>) -> Result<Self, Self::Error> {
        Self::from_vec(unlock_conditions)
    }
}

impl TryFrom<BTreeSet<UnlockCondition>> for UnlockConditions {
    type Error = Error;

    #[inline(always)]
    fn try_from(unlock_conditions: BTreeSet<UnlockCondition>) -> Result<Self, Self::Error> {
        Self::from_set(unlock_conditions)
    }
}

impl IntoIterator for UnlockConditions {
    type Item = UnlockCondition;
    type IntoIter = alloc::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        Vec::from(Into::<Box<[UnlockCondition]>>::into(self.0)).into_iter()
    }
}

impl UnlockConditions {
    ///
    pub const COUNT_MAX: u8 = 7;

    /// Creates a new [`UnlockConditions`] from a vec.
    pub fn from_vec(unlock_conditions: Vec<UnlockCondition>) -> Result<Self, Error> {
        let mut unlock_conditions =
            BoxedSlicePrefix::<UnlockCondition, UnlockConditionCount>::try_from(unlock_conditions.into_boxed_slice())
                .map_err(Error::InvalidUnlockConditionCount)?;

        unlock_conditions.sort_by_key(UnlockCondition::kind);
        // Sort is obviously fine now but uniqueness still needs to be checked.
        verify_unique_sorted::<true>(&unlock_conditions)?;

        Ok(Self(unlock_conditions))
    }

    /// Creates a new [`UnlockConditions`] from an ordered set.
    pub fn from_set(unlock_conditions: BTreeSet<UnlockCondition>) -> Result<Self, Error> {
        Ok(Self(
            unlock_conditions
                .into_iter()
                .collect::<Box<[_]>>()
                .try_into()
                .map_err(Error::InvalidUnlockConditionCount)?,
        ))
    }

    /// Gets a reference to an [`UnlockCondition`] from an unlock condition kind, if any.
    #[inline(always)]
    pub fn get(&self, key: u8) -> Option<&UnlockCondition> {
        self.0
            .binary_search_by_key(&key, UnlockCondition::kind)
            // PANIC: indexation is fine since the index has been found.
            .map(|index| &self.0[index])
            .ok()
    }

    /// Gets a reference to an [`AddressUnlockCondition`], if any.
    #[inline(always)]
    pub fn address(&self) -> Option<&AddressUnlockCondition> {
        self.get(AddressUnlockCondition::KIND).map(UnlockCondition::as_address)
    }

    /// Gets a reference to a [`StorageDepositReturnUnlockCondition`], if any.
    #[inline(always)]
    pub fn storage_deposit_return(&self) -> Option<&StorageDepositReturnUnlockCondition> {
        self.get(StorageDepositReturnUnlockCondition::KIND)
            .map(UnlockCondition::as_storage_deposit_return)
    }

    /// Gets a reference to a [`TimelockUnlockCondition`], if any.
    #[inline(always)]
    pub fn timelock(&self) -> Option<&TimelockUnlockCondition> {
        self.get(TimelockUnlockCondition::KIND)
            .map(UnlockCondition::as_timelock)
    }

    /// Gets a reference to an [`ExpirationUnlockCondition`], if any.
    #[inline(always)]
    pub fn expiration(&self) -> Option<&ExpirationUnlockCondition> {
        self.get(ExpirationUnlockCondition::KIND)
            .map(UnlockCondition::as_expiration)
    }

    /// Gets a reference to a [`StateControllerAddressUnlockCondition`], if any.
    #[inline(always)]
    pub fn state_controller_address(&self) -> Option<&StateControllerAddressUnlockCondition> {
        self.get(StateControllerAddressUnlockCondition::KIND)
            .map(UnlockCondition::as_state_controller_address)
    }

    /// Gets a reference to a [`GovernorAddressUnlockCondition`], if any.
    #[inline(always)]
    pub fn governor_address(&self) -> Option<&GovernorAddressUnlockCondition> {
        self.get(GovernorAddressUnlockCondition::KIND)
            .map(UnlockCondition::as_governor_address)
    }

    /// Gets a reference to an [`ImmutableAliasAddressUnlockCondition`], if any.
    #[inline(always)]
    pub fn immutable_alias_address(&self) -> Option<&ImmutableAliasAddressUnlockCondition> {
        self.get(ImmutableAliasAddressUnlockCondition::KIND)
            .map(UnlockCondition::as_immutable_alias_address)
    }

    /// Returns the address to be unlocked.
    #[inline(always)]
    pub fn locked_address<'a>(&'a self, address: &'a Address, milestone_timestamp: u32) -> &'a Address {
        self.expiration()
            .and_then(|e| e.return_address_expired(milestone_timestamp))
            .unwrap_or(address)
    }

    /// Returns whether a time lock exists and is still relevant.
    #[inline(always)]
    pub fn is_time_locked(&self, milestone_timestamp: u32) -> bool {
        self.timelock()
            .map_or(false, |timelock| milestone_timestamp < timelock.timestamp())
    }

    /// Returns whether an expiration exists and is expired.
    #[inline(always)]
    pub fn is_expired(&self, milestone_timestamp: u32) -> bool {
        self.expiration()
            .map_or(false, |expiration| milestone_timestamp >= expiration.timestamp())
    }
}

#[inline]
fn verify_unique_sorted<const VERIFY: bool>(unlock_conditions: &[UnlockCondition]) -> Result<(), Error> {
    if VERIFY && !is_unique_sorted(unlock_conditions.iter().map(UnlockCondition::kind)) {
        Err(Error::UnlockConditionsNotUniqueSorted)
    } else {
        Ok(())
    }
}

#[inline]
fn verify_unique_sorted_packable<const VERIFY: bool>(
    unlock_conditions: &[UnlockCondition],
    _: &ProtocolParameters,
) -> Result<(), Error> {
    verify_unique_sorted::<VERIFY>(unlock_conditions)
}

pub(crate) fn verify_allowed_unlock_conditions(
    unlock_conditions: &UnlockConditions,
    allowed_unlock_conditions: UnlockConditionFlags,
) -> Result<(), Error> {
    for (index, unlock_condition) in unlock_conditions.iter().enumerate() {
        if !allowed_unlock_conditions.contains(unlock_condition.flag()) {
            return Err(Error::UnallowedUnlockCondition {
                index,
                kind: unlock_condition.kind(),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn all_flags_present() {
        assert_eq!(
            UnlockConditionFlags::ALL_FLAGS,
            &[
                UnlockConditionFlags::ADDRESS,
                UnlockConditionFlags::STORAGE_DEPOSIT_RETURN,
                UnlockConditionFlags::TIMELOCK,
                UnlockConditionFlags::EXPIRATION,
                UnlockConditionFlags::STATE_CONTROLLER_ADDRESS,
                UnlockConditionFlags::GOVERNOR_ADDRESS,
                UnlockConditionFlags::IMMUTABLE_ALIAS_ADDRESS
            ]
        );
    }
}

#[cfg(feature = "serde")]
pub mod dto {
    use alloc::format;

    use serde::{Deserialize, Serialize, Serializer};
    use serde_json::Value;

    pub use self::{
        address::dto::AddressUnlockConditionDto, expiration::dto::ExpirationUnlockConditionDto,
        governor_address::dto::GovernorAddressUnlockConditionDto,
        immutable_alias_address::dto::ImmutableAliasAddressUnlockConditionDto,
        state_controller_address::dto::StateControllerAddressUnlockConditionDto,
        storage_deposit_return::dto::StorageDepositReturnUnlockConditionDto, timelock::dto::TimelockUnlockConditionDto,
    };
    use super::*;
    use crate::types::{block::Error, TryFromDto, ValidationParams};

    #[derive(Clone, Debug, Eq, PartialEq, From)]
    pub enum UnlockConditionDto {
        /// An address unlock condition.
        Address(AddressUnlockConditionDto),
        /// A storage deposit return unlock condition.
        StorageDepositReturn(StorageDepositReturnUnlockConditionDto),
        /// A timelock unlock condition.
        Timelock(TimelockUnlockConditionDto),
        /// An expiration unlock condition.
        Expiration(ExpirationUnlockConditionDto),
        /// A state controller address unlock condition.
        StateControllerAddress(StateControllerAddressUnlockConditionDto),
        /// A governor address unlock condition.
        GovernorAddress(GovernorAddressUnlockConditionDto),
        /// An immutable alias address unlock condition.
        ImmutableAliasAddress(ImmutableAliasAddressUnlockConditionDto),
    }

    impl<'de> Deserialize<'de> for UnlockConditionDto {
        fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            let value = Value::deserialize(d)?;
            Ok(
                match value
                    .get("type")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| serde::de::Error::custom("invalid unlock condition type"))?
                    as u8
                {
                    AddressUnlockCondition::KIND => {
                        Self::Address(AddressUnlockConditionDto::deserialize(value).map_err(|e| {
                            serde::de::Error::custom(format!("cannot deserialize address unlock condition: {e}"))
                        })?)
                    }
                    StorageDepositReturnUnlockCondition::KIND => Self::StorageDepositReturn(
                        StorageDepositReturnUnlockConditionDto::deserialize(value).map_err(|e| {
                            serde::de::Error::custom(format!(
                                "cannot deserialize storage deposit unlock condition: {e}"
                            ))
                        })?,
                    ),
                    TimelockUnlockCondition::KIND => {
                        Self::Timelock(TimelockUnlockConditionDto::deserialize(value).map_err(|e| {
                            serde::de::Error::custom(format!("cannot deserialize timelock unlock condition: {e}"))
                        })?)
                    }
                    ExpirationUnlockCondition::KIND => {
                        Self::Expiration(ExpirationUnlockConditionDto::deserialize(value).map_err(|e| {
                            serde::de::Error::custom(format!("cannot deserialize expiration unlock condition: {e}"))
                        })?)
                    }
                    StateControllerAddressUnlockCondition::KIND => Self::StateControllerAddress(
                        StateControllerAddressUnlockConditionDto::deserialize(value).map_err(|e| {
                            serde::de::Error::custom(format!(
                                "cannot deserialize state controller unlock condition: {e}"
                            ))
                        })?,
                    ),
                    GovernorAddressUnlockCondition::KIND => {
                        Self::GovernorAddress(GovernorAddressUnlockConditionDto::deserialize(value).map_err(|e| {
                            serde::de::Error::custom(format!("cannot deserialize governor unlock condition: {e}"))
                        })?)
                    }
                    ImmutableAliasAddressUnlockCondition::KIND => Self::ImmutableAliasAddress(
                        ImmutableAliasAddressUnlockConditionDto::deserialize(value).map_err(|e| {
                            serde::de::Error::custom(format!(
                                "cannot deserialize immutable alias address unlock condition: {e}"
                            ))
                        })?,
                    ),
                    _ => return Err(serde::de::Error::custom("invalid unlock condition type")),
                },
            )
        }
    }

    impl Serialize for UnlockConditionDto {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            #[derive(Serialize)]
            #[serde(untagged)]
            enum UnlockConditionDto_<'a> {
                T1(&'a AddressUnlockConditionDto),
                T2(&'a StorageDepositReturnUnlockConditionDto),
                T3(&'a TimelockUnlockConditionDto),
                T4(&'a ExpirationUnlockConditionDto),
                T5(&'a StateControllerAddressUnlockConditionDto),
                T6(&'a GovernorAddressUnlockConditionDto),
                T7(&'a ImmutableAliasAddressUnlockConditionDto),
            }
            #[derive(Serialize)]
            struct TypedUnlockCondition<'a> {
                #[serde(flatten)]
                unlock_condition: UnlockConditionDto_<'a>,
            }
            let unlock_condition = match self {
                Self::Address(o) => TypedUnlockCondition {
                    unlock_condition: UnlockConditionDto_::T1(o),
                },
                Self::StorageDepositReturn(o) => TypedUnlockCondition {
                    unlock_condition: UnlockConditionDto_::T2(o),
                },
                Self::Timelock(o) => TypedUnlockCondition {
                    unlock_condition: UnlockConditionDto_::T3(o),
                },
                Self::Expiration(o) => TypedUnlockCondition {
                    unlock_condition: UnlockConditionDto_::T4(o),
                },
                Self::StateControllerAddress(o) => TypedUnlockCondition {
                    unlock_condition: UnlockConditionDto_::T5(o),
                },
                Self::GovernorAddress(o) => TypedUnlockCondition {
                    unlock_condition: UnlockConditionDto_::T6(o),
                },
                Self::ImmutableAliasAddress(o) => TypedUnlockCondition {
                    unlock_condition: UnlockConditionDto_::T7(o),
                },
            };
            unlock_condition.serialize(serializer)
        }
    }

    impl From<&UnlockCondition> for UnlockConditionDto {
        fn from(value: &UnlockCondition) -> Self {
            match value {
                UnlockCondition::Address(v) => Self::Address(AddressUnlockConditionDto::from(v)),
                UnlockCondition::StorageDepositReturn(v) => {
                    Self::StorageDepositReturn(StorageDepositReturnUnlockConditionDto::from(v))
                }
                UnlockCondition::Timelock(v) => Self::Timelock(TimelockUnlockConditionDto::from(v)),
                UnlockCondition::Expiration(v) => Self::Expiration(ExpirationUnlockConditionDto::from(v)),
                UnlockCondition::StateControllerAddress(v) => {
                    Self::StateControllerAddress(StateControllerAddressUnlockConditionDto::from(v))
                }
                UnlockCondition::GovernorAddress(v) => {
                    Self::GovernorAddress(GovernorAddressUnlockConditionDto::from(v))
                }
                UnlockCondition::ImmutableAliasAddress(v) => {
                    Self::ImmutableAliasAddress(ImmutableAliasAddressUnlockConditionDto::from(v))
                }
            }
        }
    }

    impl TryFromDto for UnlockCondition {
        type Dto = UnlockConditionDto;
        type Error = Error;

        fn try_from_dto_with_params_inner(dto: Self::Dto, params: ValidationParams<'_>) -> Result<Self, Self::Error> {
            Ok(match dto {
                UnlockConditionDto::Address(v) => Self::Address(AddressUnlockCondition::try_from(v)?),
                UnlockConditionDto::StorageDepositReturn(v) => Self::StorageDepositReturn(
                    StorageDepositReturnUnlockCondition::try_from_dto_with_params_inner(v, params)?,
                ),
                UnlockConditionDto::Timelock(v) => Self::Timelock(TimelockUnlockCondition::try_from(v)?),
                UnlockConditionDto::Expiration(v) => Self::Expiration(ExpirationUnlockCondition::try_from(v)?),
                UnlockConditionDto::StateControllerAddress(v) => {
                    Self::StateControllerAddress(StateControllerAddressUnlockCondition::try_from(v)?)
                }
                UnlockConditionDto::GovernorAddress(v) => {
                    Self::GovernorAddress(GovernorAddressUnlockCondition::try_from(v)?)
                }
                UnlockConditionDto::ImmutableAliasAddress(v) => {
                    Self::ImmutableAliasAddress(ImmutableAliasAddressUnlockCondition::try_from(v)?)
                }
            })
        }
    }

    impl UnlockConditionDto {
        /// Return the unlock condition kind of a `UnlockConditionDto`.
        pub fn kind(&self) -> u8 {
            match self {
                Self::Address(_) => AddressUnlockCondition::KIND,
                Self::StorageDepositReturn(_) => StorageDepositReturnUnlockCondition::KIND,
                Self::Timelock(_) => TimelockUnlockCondition::KIND,
                Self::Expiration(_) => ExpirationUnlockCondition::KIND,
                Self::StateControllerAddress(_) => StateControllerAddressUnlockCondition::KIND,
                Self::GovernorAddress(_) => GovernorAddressUnlockCondition::KIND,
                Self::ImmutableAliasAddress(_) => ImmutableAliasAddressUnlockCondition::KIND,
            }
        }
    }
}
