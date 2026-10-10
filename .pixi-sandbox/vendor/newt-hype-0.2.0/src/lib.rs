#![no_std]
//! ## 🦎 Newt Hype
//! This crate provides a simple way to work around the orphan rule by creating newtype
//! structs using a small set of macros.
//!
//! **Features**
//! - `#![no_std]` compatible
//! - automatically forwards common trait implementations
//! - minimizes boilerplate when defining strongly typed wrappers
//!
//! The [`base_newtype!`] macro declares a base newtype struct with generic implementations for
//! common traits. This base struct is a transparent wrapper around a single value of generic
//! type `T` and is owned by the crate of the caller.
//!
//! Once you have your base newtype struct declared, you can use the [`newtype!`] macro to
//! create named newtype structs that wrap the specified type using the base newtype struct.
//!
//! ## Example Usage
//! ```
//! use newt_hype::*;
//! // Declare the base newtype struct
//! base_newtype!(); // Defaults to NewType
//!
//! newtype!(BetterBool, bool); // Creates a newtype struct `BetterBool` that wraps `bool` using `NewType` as the base.
//!
//! let a: BetterBool = BetterBool::new(true);
//!
//! assert!(*a);
//! assert_ne!(a, false);
//!
//! newtype!(MyNewType, NewType, u32); // Creates a newtype struct `MyNewType`that wraps `u32` using `NewType` as the base
//! assert_eq!(MyNewType::new(42), 42);
//! let a = MyNewType::new(11);
//! let b = MyNewType::new(37);
//! let c = 38u32;
//! let d = MyNewType::new(40);
//! assert_eq!(a + b, MyNewType::new(48));
//! assert_eq!(a + c, MyNewType::new(49));
//! assert_eq!(d - c, MyNewType::new(2));
//! ```
//! Note that we can't do `c - a` because the orphan rule prevents us from implementing binary
//! ops we don't own (such as [`core::ops::Sub`]) on `u32`, which we also don't own, however we
//! can do `d - c` because `d` is a newtype struct that wraps `u32`, and we own the `NewType`
//! base struct.

/// Creates a named newtype struct that wraps the specified type using the (optionally)
/// provided base newtype struct.
///
/// If a base newtype struct is not provided, it will default to `NewType`.
///
/// Note that the created newtype is actually a type alias for the base newtype struct with its
/// generic type parameter set to the specified inner type, allowing it to take advantage of
/// all the generic implementations optimistically provided by the base newtype struct.
///
/// # Example:
/// ```
/// use newt_hype::*;
/// base_newtype!(); // Defaults to NewType
/// newtype!(MyNewType, NewType, u32); // Creates a newtype struct `MyNewType` that wraps `u32` using `NewType` as the base
/// ```
#[macro_export]
macro_rules! newtype {
    ($name:ident, $base:ident, $inner:ty) => {
        pub type $name = $base<$inner>;
    };
    ($name:ident, $inner:ty) => {
        pub type $name = NewType<$inner>;
    };
}

/// Declares a base newtype struct with generic impls for common traits.
///
/// Takes a single argument which is the name of the base newtype struct. If a name is not
/// provided, it will default to `NewType`.
///
/// This `NewType` struct is a transparent wrapper around a single value of type `T`, and will
/// be owned by the crate of the caller. The base wrapper forwards implementations for many
/// traits including `Deref`, arithmetic operations and various iterator traits. The preferred
/// pattern is to implement additional traits for `NewType<T>` where `T` implements those traits.
///
/// You only need to declare one base newtype struct per crate as it handles all generic impls
/// for you, however you are welcome to declare multiple if you wish.
///
/// # Example:
/// ```
/// use newt_hype::*;
/// base_newtype!(); // Defaults to NewType
/// base_newtype!(MyBaseNewType);
/// ```
#[macro_export]
macro_rules! base_newtype {
    () => {
        $crate::base_newtype!(NewType);
    };
    ($name:ident) => {
        #[repr(transparent)]
        pub struct $name<T>(pub T);

        impl<T> $name<T> {
            #[inline(always)]
            pub const fn new(value: T) -> Self {
                $name(value)
            }

            #[inline(always)]
            pub const fn inner(&self) -> &T {
                &self.0
            }

            #[inline(always)]
            pub const fn inner_mut(&mut self) -> &mut T {
                &mut self.0
            }

            #[inline(always)]
            pub fn into_inner(self) -> T {
                self.0
            }
        }

        impl<T> From<T> for $name<T> {
            #[inline(always)]
            fn from(value: T) -> Self {
                $name(value)
            }
        }

        impl<T> AsRef<T> for $name<T> {
            #[inline(always)]
            fn as_ref(&self) -> &T {
                &self.0
            }
        }

        impl<T> AsMut<T> for $name<T> {
            #[inline(always)]
            fn as_mut(&mut self) -> &mut T {
                &mut self.0
            }
        }

        impl<T> core::ops::Deref for $name<T> {
            type Target = T;

            #[inline(always)]
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl<T> core::ops::DerefMut for $name<T> {
            #[inline(always)]
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl<T: Default> Default for $name<T> {
            #[inline(always)]
            fn default() -> Self {
                $name(T::default())
            }
        }

        impl<T: Clone> Clone for $name<T> {
            #[inline(always)]
            fn clone(&self) -> Self {
                $name(self.0.clone())
            }
        }

        impl<T: Copy> Copy for $name<T> {}

        impl<T: PartialEq> PartialEq for $name<T> {
            #[inline(always)]
            fn eq(&self, other: &Self) -> bool {
                self.0.eq(&other.0)
            }
        }

        impl<T: PartialEq> PartialEq<T> for $name<T> {
            #[inline(always)]
            fn eq(&self, other: &T) -> bool {
                self.0.eq(other)
            }
        }

        impl<T: Eq> Eq for $name<T> {}

        impl<T: PartialOrd> PartialOrd for $name<T> {
            #[inline(always)]
            fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
                self.0.partial_cmp(&other.0)
            }
        }

        impl<T: PartialOrd> PartialOrd<T> for $name<T> {
            #[inline(always)]
            fn partial_cmp(&self, other: &T) -> Option<core::cmp::Ordering> {
                self.0.partial_cmp(other)
            }
        }

        impl<T: Ord> Ord for $name<T> {
            #[inline(always)]
            fn cmp(&self, other: &Self) -> core::cmp::Ordering {
                self.0.cmp(&other.0)
            }
        }

        impl<T: core::fmt::Debug> core::fmt::Debug for $name<T> {
            #[inline(always)]
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{:?}", self.0)
            }
        }

        impl<T: core::fmt::Display> core::fmt::Display for $name<T> {
            #[inline(always)]
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl<T: core::hash::Hash> core::hash::Hash for $name<T> {
            #[inline(always)]
            fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
                self.0.hash(state);
            }
        }

        impl<T: core::iter::Iterator> core::iter::Iterator for $name<T> {
            type Item = T::Item;

            #[inline(always)]
            fn next(&mut self) -> Option<Self::Item> {
                self.0.next()
            }

            #[inline(always)]
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.0.size_hint()
            }
        }

        impl<T: core::iter::DoubleEndedIterator> core::iter::DoubleEndedIterator for $name<T> {
            #[inline(always)]
            fn next_back(&mut self) -> Option<Self::Item> {
                self.0.next_back()
            }
        }

        impl<T: core::iter::ExactSizeIterator> core::iter::ExactSizeIterator for $name<T> {
            #[inline(always)]
            fn len(&self) -> usize {
                self.0.len()
            }
        }

        impl<T: core::iter::FusedIterator> core::iter::FusedIterator for $name<T> {}

        impl<T: core::ops::Index<I>, I> core::ops::Index<I> for $name<T> {
            type Output = T::Output;

            #[inline(always)]
            fn index(&self, index: I) -> &Self::Output {
                &self.0[index]
            }
        }

        impl<T: core::ops::IndexMut<I>, I> core::ops::IndexMut<I> for $name<T> {
            #[inline(always)]
            fn index_mut(&mut self, index: I) -> &mut Self::Output {
                &mut self.0[index]
            }
        }

        impl<T: core::ops::Add<T>> core::ops::Add<$name<T>> for $name<T> {
            type Output = $name<<T as core::ops::Add<T>>::Output>;

            #[inline(always)]
            fn add(self, rhs: $name<T>) -> Self::Output {
                $name(self.0 + rhs.0)
            }
        }

        impl<T: core::ops::Add<T>> core::ops::Add<T> for $name<T> {
            type Output = $name<<T as core::ops::Add<T>>::Output>;

            #[inline(always)]
            fn add(self, rhs: T) -> Self::Output {
                $name(self.0 + rhs)
            }
        }

        impl<T: core::ops::AddAssign<T>> core::ops::AddAssign<$name<T>> for $name<T> {
            #[inline(always)]
            fn add_assign(&mut self, rhs: $name<T>) {
                self.0.add_assign(rhs.0);
            }
        }

        impl<T: core::ops::AddAssign<T>> core::ops::AddAssign<T> for $name<T> {
            #[inline(always)]
            fn add_assign(&mut self, rhs: T) {
                self.0.add_assign(rhs);
            }
        }

        impl<T: core::ops::Sub<T>> core::ops::Sub<$name<T>> for $name<T> {
            type Output = $name<<T as core::ops::Sub<T>>::Output>;

            #[inline(always)]
            fn sub(self, rhs: $name<T>) -> Self::Output {
                $name(self.0 - rhs.0)
            }
        }

        impl<T: core::ops::Sub<T>> core::ops::Sub<T> for $name<T> {
            type Output = $name<<T as core::ops::Sub<T>>::Output>;

            #[inline(always)]
            fn sub(self, rhs: T) -> Self::Output {
                $name(self.0 - rhs)
            }
        }

        impl<T: core::ops::SubAssign<T>> core::ops::SubAssign<$name<T>> for $name<T> {
            #[inline(always)]
            fn sub_assign(&mut self, rhs: $name<T>) {
                self.0.sub_assign(rhs.0);
            }
        }

        impl<T: core::ops::SubAssign<T>> core::ops::SubAssign<T> for $name<T> {
            #[inline(always)]
            fn sub_assign(&mut self, rhs: T) {
                self.0.sub_assign(rhs);
            }
        }

        impl<T: core::ops::Mul<T>> core::ops::Mul<$name<T>> for $name<T> {
            type Output = $name<<T as core::ops::Mul<T>>::Output>;

            #[inline(always)]
            fn mul(self, rhs: $name<T>) -> Self::Output {
                $name(self.0 * rhs.0)
            }
        }

        impl<T: core::ops::Mul<T>> core::ops::Mul<T> for $name<T> {
            type Output = $name<<T as core::ops::Mul<T>>::Output>;

            #[inline(always)]
            fn mul(self, rhs: T) -> Self::Output {
                $name(self.0 * rhs)
            }
        }

        impl<T: core::ops::MulAssign<T>> core::ops::MulAssign<$name<T>> for $name<T> {
            #[inline(always)]
            fn mul_assign(&mut self, rhs: $name<T>) {
                self.0.mul_assign(rhs.0);
            }
        }

        impl<T: core::ops::MulAssign<T>> core::ops::MulAssign<T> for $name<T> {
            #[inline(always)]
            fn mul_assign(&mut self, rhs: T) {
                self.0.mul_assign(rhs);
            }
        }

        impl<T: core::ops::Div<T>> core::ops::Div<$name<T>> for $name<T> {
            type Output = $name<<T as core::ops::Div<T>>::Output>;

            #[inline(always)]
            fn div(self, rhs: $name<T>) -> Self::Output {
                $name(self.0 / rhs.0)
            }
        }

        impl<T: core::ops::Div<T>> core::ops::Div<T> for $name<T> {
            type Output = $name<<T as core::ops::Div<T>>::Output>;

            #[inline(always)]
            fn div(self, rhs: T) -> Self::Output {
                $name(self.0 / rhs)
            }
        }

        impl<T: core::ops::DivAssign<T>> core::ops::DivAssign<$name<T>> for $name<T> {
            #[inline(always)]
            fn div_assign(&mut self, rhs: $name<T>) {
                self.0.div_assign(rhs.0);
            }
        }

        impl<T: core::ops::DivAssign<T>> core::ops::DivAssign<T> for $name<T> {
            #[inline(always)]
            fn div_assign(&mut self, rhs: T) {
                self.0.div_assign(rhs);
            }
        }

        impl<T: core::ops::Rem<T>> core::ops::Rem<$name<T>> for $name<T> {
            type Output = $name<<T as core::ops::Rem<T>>::Output>;

            #[inline(always)]
            fn rem(self, rhs: $name<T>) -> Self::Output {
                $name(self.0 % rhs.0)
            }
        }

        impl<T: core::ops::Rem<T>> core::ops::Rem<T> for $name<T> {
            type Output = $name<<T as core::ops::Rem<T>>::Output>;

            #[inline(always)]
            fn rem(self, rhs: T) -> Self::Output {
                $name(self.0 % rhs)
            }
        }

        impl<T: core::ops::RemAssign<T>> core::ops::RemAssign<$name<T>> for $name<T> {
            #[inline(always)]
            fn rem_assign(&mut self, rhs: $name<T>) {
                self.0.rem_assign(rhs.0);
            }
        }

        impl<T: core::ops::RemAssign<T>> core::ops::RemAssign<T> for $name<T> {
            #[inline(always)]
            fn rem_assign(&mut self, rhs: T) {
                self.0.rem_assign(rhs);
            }
        }

        impl<T: core::ops::Neg> core::ops::Neg for $name<T> {
            type Output = $name<T::Output>;

            #[inline(always)]
            fn neg(self) -> Self::Output {
                $name(-self.0)
            }
        }

        impl<T: core::ops::Not> core::ops::Not for $name<T> {
            type Output = $name<T::Output>;

            #[inline(always)]
            fn not(self) -> Self::Output {
                $name(!self.0)
            }
        }

        impl<T: core::ops::BitAnd<T>> core::ops::BitAnd<$name<T>> for $name<T> {
            type Output = $name<<T as core::ops::BitAnd<T>>::Output>;

            #[inline(always)]
            fn bitand(self, rhs: $name<T>) -> Self::Output {
                $name(self.0 & rhs.0)
            }
        }

        impl<T: core::ops::BitAnd<T>> core::ops::BitAnd<T> for $name<T> {
            type Output = $name<<T as core::ops::BitAnd<T>>::Output>;

            #[inline(always)]
            fn bitand(self, rhs: T) -> Self::Output {
                $name(self.0 & rhs)
            }
        }

        impl<T: core::ops::BitAndAssign<T>> core::ops::BitAndAssign<$name<T>> for $name<T> {
            #[inline(always)]
            fn bitand_assign(&mut self, rhs: $name<T>) {
                self.0.bitand_assign(rhs.0);
            }
        }

        impl<T: core::ops::BitAndAssign<T>> core::ops::BitAndAssign<T> for $name<T> {
            #[inline(always)]
            fn bitand_assign(&mut self, rhs: T) {
                self.0.bitand_assign(rhs);
            }
        }

        impl<T: core::ops::BitOr<T>> core::ops::BitOr<$name<T>> for $name<T> {
            type Output = $name<<T as core::ops::BitOr<T>>::Output>;

            #[inline(always)]
            fn bitor(self, rhs: $name<T>) -> Self::Output {
                $name(self.0 | rhs.0)
            }
        }

        impl<T: core::ops::BitOr<T>> core::ops::BitOr<T> for $name<T> {
            type Output = $name<<T as core::ops::BitOr<T>>::Output>;

            #[inline(always)]
            fn bitor(self, rhs: T) -> Self::Output {
                $name(self.0 | rhs)
            }
        }

        impl<T: core::ops::BitOrAssign<T>> core::ops::BitOrAssign<$name<T>> for $name<T> {
            #[inline(always)]
            fn bitor_assign(&mut self, rhs: $name<T>) {
                self.0.bitor_assign(rhs.0);
            }
        }

        impl<T: core::ops::BitOrAssign<T>> core::ops::BitOrAssign<T> for $name<T> {
            #[inline(always)]
            fn bitor_assign(&mut self, rhs: T) {
                self.0.bitor_assign(rhs);
            }
        }

        impl<T: core::ops::BitXor<T>> core::ops::BitXor<$name<T>> for $name<T> {
            type Output = $name<<T as core::ops::BitXor<T>>::Output>;

            #[inline(always)]
            fn bitxor(self, rhs: $name<T>) -> Self::Output {
                $name(self.0 ^ rhs.0)
            }
        }

        impl<T: core::ops::BitXor<T>> core::ops::BitXor<T> for $name<T> {
            type Output = $name<<T as core::ops::BitXor<T>>::Output>;

            #[inline(always)]
            fn bitxor(self, rhs: T) -> Self::Output {
                $name(self.0 ^ rhs)
            }
        }

        impl<T: core::ops::BitXorAssign<T>> core::ops::BitXorAssign<$name<T>> for $name<T> {
            #[inline(always)]
            fn bitxor_assign(&mut self, rhs: $name<T>) {
                self.0.bitxor_assign(rhs.0);
            }
        }

        impl<T: core::ops::BitXorAssign<T>> core::ops::BitXorAssign<T> for $name<T> {
            #[inline(always)]
            fn bitxor_assign(&mut self, rhs: T) {
                self.0.bitxor_assign(rhs);
            }
        }

        impl<T: core::ops::Shl<T>> core::ops::Shl<$name<T>> for $name<T> {
            type Output = $name<<T as core::ops::Shl<T>>::Output>;

            #[inline(always)]
            fn shl(self, rhs: $name<T>) -> Self::Output {
                $name(self.0 << rhs.0)
            }
        }

        impl<T: core::ops::Shl<T>> core::ops::Shl<T> for $name<T> {
            type Output = $name<<T as core::ops::Shl<T>>::Output>;

            #[inline(always)]
            fn shl(self, rhs: T) -> Self::Output {
                $name(self.0 << rhs)
            }
        }

        impl<T: core::ops::ShlAssign<T>> core::ops::ShlAssign<$name<T>> for $name<T> {
            #[inline(always)]
            fn shl_assign(&mut self, rhs: $name<T>) {
                self.0.shl_assign(rhs.0);
            }
        }

        impl<T: core::ops::ShlAssign<T>> core::ops::ShlAssign<T> for $name<T> {
            #[inline(always)]
            fn shl_assign(&mut self, rhs: T) {
                self.0.shl_assign(rhs);
            }
        }

        impl<T: core::ops::Shr<T>> core::ops::Shr<$name<T>> for $name<T> {
            type Output = $name<<T as core::ops::Shr<T>>::Output>;

            #[inline(always)]
            fn shr(self, rhs: $name<T>) -> Self::Output {
                $name(self.0 >> rhs.0)
            }
        }

        impl<T: core::ops::Shr<T>> core::ops::Shr<T> for $name<T> {
            type Output = $name<<T as core::ops::Shr<T>>::Output>;

            #[inline(always)]
            fn shr(self, rhs: T) -> Self::Output {
                $name(self.0 >> rhs)
            }
        }

        impl<T: core::ops::ShrAssign<T>> core::ops::ShrAssign<$name<T>> for $name<T> {
            #[inline(always)]
            fn shr_assign(&mut self, rhs: $name<T>) {
                self.0.shr_assign(rhs.0);
            }
        }

        impl<T: core::ops::ShrAssign<T>> core::ops::ShrAssign<T> for $name<T> {
            #[inline(always)]
            fn shr_assign(&mut self, rhs: T) {
                self.0.shr_assign(rhs);
            }
        }

        impl<T: core::ops::RangeBounds<R>, R> core::ops::RangeBounds<R> for $name<T> {
            #[inline(always)]
            fn start_bound(&self) -> core::ops::Bound<&R> {
                self.0.start_bound()
            }

            #[inline(always)]
            fn end_bound(&self) -> core::ops::Bound<&R> {
                self.0.end_bound()
            }
        }

        impl<T: core::str::FromStr> core::str::FromStr for $name<T> {
            type Err = T::Err;

            #[inline(always)]
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                T::from_str(s).map($name)
            }
        }

        impl<T> core::borrow::Borrow<T> for $name<T> {
            #[inline(always)]
            fn borrow(&self) -> &T {
                &self.0
            }
        }

        impl<T> core::borrow::BorrowMut<T> for $name<T> {
            #[inline(always)]
            fn borrow_mut(&mut self) -> &mut T {
                &mut self.0
            }
        }

        impl<U, T> core::iter::FromIterator<U> for $name<T>
        where
            T: core::iter::FromIterator<U>,
        {
            #[inline(always)]
            fn from_iter<I: core::iter::IntoIterator<Item = U>>(iter: I) -> Self {
                $name(T::from_iter(iter))
            }
        }

        impl<U, T> core::iter::Extend<U> for $name<T>
        where
            T: core::iter::Extend<U>,
        {
            #[inline(always)]
            fn extend<I: core::iter::IntoIterator<Item = U>>(&mut self, iter: I) {
                self.0.extend(iter);
            }
        }

        impl<T: core::fmt::Binary> core::fmt::Binary for $name<T> {
            #[inline(always)]
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::Binary::fmt(&self.0, f)
            }
        }

        impl<T: core::fmt::Octal> core::fmt::Octal for $name<T> {
            #[inline(always)]
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::Octal::fmt(&self.0, f)
            }
        }

        impl<T: core::fmt::LowerHex> core::fmt::LowerHex for $name<T> {
            #[inline(always)]
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::LowerHex::fmt(&self.0, f)
            }
        }

        impl<T: core::fmt::UpperHex> core::fmt::UpperHex for $name<T> {
            #[inline(always)]
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::UpperHex::fmt(&self.0, f)
            }
        }

        impl<T: core::fmt::LowerExp> core::fmt::LowerExp for $name<T> {
            #[inline(always)]
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::LowerExp::fmt(&self.0, f)
            }
        }

        impl<T: core::fmt::UpperExp> core::fmt::UpperExp for $name<T> {
            #[inline(always)]
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::UpperExp::fmt(&self.0, f)
            }
        }

        impl<T: core::fmt::Pointer> core::fmt::Pointer for $name<T> {
            #[inline(always)]
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::Pointer::fmt(&self.0, f)
            }
        }
    };
}

#[allow(unused)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::format;
    use alloc::vec;
    use alloc::vec::Vec;

    base_newtype!();

    newtype!(WU32, NewType, u32);
    newtype!(WI32, i32);

    pub trait Foo<T> {
        const DELTA: u8;
        type Item;

        fn fizz(&self) -> T;
        fn set(&mut self, v: T);
        fn into_inner(self) -> T;
    }

    pub struct Thing<T>(pub T);

    impl<T: Copy + Ord> Foo<T> for Thing<T> {
        const DELTA: u8 = 99;
        type Item = T;

        fn fizz(&self) -> T {
            self.0
        }
        fn set(&mut self, v: T) {
            self.0 = v;
        }
        fn into_inner(self) -> T {
            self.0
        }
    }

    #[derive(Default)]
    struct SimpleHasher(u64);

    impl core::hash::Hasher for SimpleHasher {
        fn write(&mut self, bytes: &[u8]) {
            for b in bytes {
                self.0 = self.0.wrapping_add(*b as u64);
            }
        }

        fn finish(&self) -> u64 {
            self.0
        }
    }

    #[test]
    fn test_deref() {
        let w: WU32 = WU32::new(42);
        assert_eq!(*w, 42);

        let mut w_mut: WU32 = WU32::new(100);
        *w_mut = 200;
        assert_eq!(*w_mut, 200);

        let i: WI32 = WI32::new(10);
        assert_eq!(*i, 10);
        let mut i_mut: WI32 = WI32::new(20);
        *i_mut = 30;
        assert_eq!(*i_mut, 30);
    }

    #[test]
    fn test_math_ops() {
        let a: WU32 = WU32::new(10);
        let b: WU32 = WU32::new(5);

        let sum: WU32 = a + b;
        assert_eq!(*sum, 15);
        let sum = a + 5;
        assert_eq!(*sum, 15);

        let diff = a - b;
        assert_eq!(*diff, 5);
        let diff = a - 5;
        assert_eq!(*diff, 5);

        let prod = a * b;
        assert_eq!(*prod, 50);
        let prod = a * 5;
        assert_eq!(*prod, 50);

        let quot = a / b;
        assert_eq!(*quot, 2);
        let quot = a / 5;
        assert_eq!(*quot, 2);

        let rem = a % b;
        assert_eq!(*rem, 0);
        let rem = a % 5;
        assert_eq!(*rem, 0);

        let c = WI32::new(10);
        let neg_c = -c;
        assert_eq!(*neg_c, -10);
    }

    #[test]
    fn test_math_ops_assign() {
        let mut x: WU32 = WU32::new(10);

        x += WU32::new(2);
        assert_eq!(*x, 12);

        x -= WU32::new(5);
        assert_eq!(*x, 7);

        x *= WU32::new(3);
        assert_eq!(*x, 21);

        x /= WU32::new(7);
        assert_eq!(*x, 3);

        x %= WU32::new(2);
        assert_eq!(*x, 1);

        let mut y: WU32 = WU32::new(10);

        y += 2u32;
        assert_eq!(*y, 12);

        y -= 5u32;
        assert_eq!(*y, 7);

        y *= 3u32;
        assert_eq!(*y, 21);

        y /= 7u32;
        assert_eq!(*y, 3);

        y %= 2u32;
        assert_eq!(*y, 1);
    }

    #[test]
    fn test_bool() {
        newtype!(BetterBool, bool); // Creates a newtype struct `BetterBool` that wraps `bool` using `NewType` as the base.

        let a: BetterBool = BetterBool::new(true);
        let b: BetterBool = BetterBool::new(false);

        assert_eq!(a, true);
        assert_ne!(a, false);
        assert_ne!(a, b)
    }

    #[test]
    fn test_default_clone_asref() {
        let mut x: WU32 = Default::default();
        assert_eq!(x, 0);

        let y = x;
        assert_eq!(x, y);

        *x.as_mut() = 5;
        assert_eq!(x.as_ref(), &5);
    }

    #[test]
    fn test_from_str() {
        newtype!(Num, u32);
        let n: Num = "42".parse().unwrap();
        assert_eq!(n, 42);
    }

    #[test]
    fn test_iterator() {
        newtype!(R, core::ops::Range<u8>);
        let mut r = R::new(1..4);

        assert_eq!(r.len(), 3);
        assert_eq!(r.next(), Some(1));
        assert_eq!(r.next_back(), Some(3));
        assert_eq!(r.next(), Some(2));
        assert_eq!(r.next(), None);
    }

    #[test]
    fn test_iterator_methods() {
        newtype!(R, core::ops::Range<u8>);

        let mut r = R::new(0..5);
        assert_eq!(r.nth(2), Some(2));
        assert_eq!(r.next_back(), Some(4));

        let mut fused = R::new(0..2);
        assert_eq!(fused.next(), Some(0));
        assert_eq!(fused.next(), Some(1));
        assert_eq!(fused.next(), None);
        assert_eq!(fused.next(), None); // ensure FusedIterator behavior

        let collected: Vec<u8> = R::new(3..6).collect();
        assert_eq!(collected, vec![3, 4, 5]);
    }

    #[test]
    fn test_inner_and_from() {
        let mut x: WU32 = 5u32.into();
        assert_eq!(*x.inner(), 5);
        *x.inner_mut() = 7;
        assert_eq!(x.into_inner(), 7);
    }

    #[test]
    fn test_display_debug_hash() {
        use core::hash::{Hash, Hasher};

        let x: WU32 = 42u32.into();
        assert_eq!(format!("{x}"), "42");
        assert_eq!(format!("{x:?}"), "42");

        let mut hx = SimpleHasher::default();
        x.hash(&mut hx);
        let mut hy = SimpleHasher::default();
        42u32.hash(&mut hy);
        assert_eq!(hx.finish(), hy.finish());
    }

    #[test]
    fn test_ordering() {
        let a = WU32::new(1);
        let b = WU32::new(2);
        assert!(a < b);
        assert!(b > a);
        assert!(b > 1);

        let mut v = vec![b, a];
        v.sort();
        assert_eq!(v, vec![a, b]);
    }

    #[test]
    fn test_indexing() {
        newtype!(WVec, Vec<u8>);
        let mut w = WVec::new(vec![1, 2, 3]);
        assert_eq!(w[1], 2);
        w[1] = 4;
        assert_eq!(w[1], 4);
    }

    #[test]
    fn test_bit_ops() {
        let mut x = WU32::new(0b1010u32);
        assert_eq!(*(x & 0b1100u32).inner(), 0b1000u32);
        x &= 0b1100u32;
        assert_eq!(x, 0b1000u32);

        assert_eq!(*(x | 0b0011u32).inner(), 0b1011u32);
        x |= 0b0011u32;
        assert_eq!(x, 0b1011u32);

        assert_eq!(*(x ^ 0b1111u32).inner(), 0b0100u32);
        x ^= 0b1111u32;
        assert_eq!(x, 0b0100u32);

        assert_eq!((!x).into_inner(), !0b0100u32);

        let shifted: WU32 = x << 2;
        assert_eq!(shifted.into_inner(), 0b0100u32 << 2);
        x <<= 1;
        assert_eq!(x.into_inner(), 0b0100u32 << 1);

        let mut y = WU32::new(0b1000u32);
        let shr: WU32 = y >> 2;
        assert_eq!(shr.into_inner(), 0b1000u32 >> 2);
        y >>= 1;
        assert_eq!(y.into_inner(), 0b1000u32 >> 1);
    }

    #[test]
    fn test_bit_ops_with_newtype_rhs() {
        let mut x = WU32::new(0b1010u32);
        let rhs = WU32::new(0b1100u32);
        assert_eq!((x & rhs).into_inner(), 0b1000u32);
        x &= rhs;
        assert_eq!(x.into_inner(), 0b1000u32);

        let rhs = WU32::new(0b0011u32);
        assert_eq!((x | rhs).into_inner(), 0b1011u32);
        x |= rhs;
        assert_eq!(x.into_inner(), 0b1011u32);

        let rhs = WU32::new(0b1111u32);
        assert_eq!((x ^ rhs).into_inner(), 0b0100u32);
        x ^= rhs;
        assert_eq!(x.into_inner(), 0b0100u32);

        let shifted: WU32 = x << WU32::new(2);
        assert_eq!(shifted.into_inner(), 0b0100u32 << 2);
        x <<= WU32::new(1);
        assert_eq!(x.into_inner(), 0b0100u32 << 1);

        let mut y = WU32::new(0b1000u32);
        let shr: WU32 = y >> WU32::new(2);
        assert_eq!(shr.into_inner(), 0b1000u32 >> 2);
        y >>= WU32::new(1);
        assert_eq!(y.into_inner(), 0b1000u32 >> 1);
    }

    #[test]
    fn test_range_bounds() {
        use core::ops::{Bound, RangeBounds};
        newtype!(R, core::ops::Range<u8>);
        let r = R::new(1..4);
        assert_eq!(RangeBounds::start_bound(&r), Bound::Included(&1));
        assert_eq!(RangeBounds::end_bound(&r), Bound::Excluded(&4));
    }

    #[test]
    fn test_borrow_traits() {
        use core::borrow::{Borrow, BorrowMut};
        let mut x = WU32::new(5);
        let r: &u32 = x.borrow();
        assert_eq!(*r, 5);
        *x.borrow_mut() = 7;
        assert_eq!(x, 7u32);
    }

    #[test]
    fn test_formatting_traits() {
        let x = WU32::new(42);
        assert_eq!(format!("{x:b}"), format!("{:b}", 42u32));
        assert_eq!(format!("{x:o}"), format!("{:o}", 42u32));
        assert_eq!(format!("{x:x}"), format!("{:x}", 42u32));
    }
}
