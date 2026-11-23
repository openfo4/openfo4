use std::borrow::Borrow;
use std::fmt::{self, Debug, Display, Formatter};
use std::ops::Deref;

use bstr::{BStr, BString, ByteSlice};

/// A string value that is guaranteed to be lowercase.
///
/// Note that `Lowercase<T>` does not expose a [`Deref`] implementation, as this would make it easy
/// to accidently compare to `T`. `Lowercase<T>` is only meant to be compared to itself. To access
/// to inner `T` use [`as_inner`] instead.
///
/// [`as_inner`]: Self::as_inner
#[derive(Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Lowercase<T>(T)
where
    T: ?Sized;

impl<T> Lowercase<T>
where
    T: ?Sized,
{
    /// Returns a reference to the wrapped value.
    pub fn as_inner(&self) -> &T {
        &self.0
    }
}

impl<T> Lowercase<T> {
    pub fn new<S>(value: S) -> Self
    where
        S: ToLowercase<Target = T>,
    {
        Self(value.to_lowercase())
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl Lowercase<String> {
    /// Returns a `Lowercase<str>` string slice of the string.
    pub fn as_str(&self) -> &Lowercase<str> {
        unsafe { Lowercase::<str>::from_str(self.0.as_str()) }
    }
}

impl Lowercase<str> {
    /// Creates a `&Lowercase<str>` from a static `str`.
    ///
    /// # Panics
    ///
    /// Panics if the given `str` is not fully lowercase.
    pub const fn from_static(s: &'static str) -> &'static Self {
        let mut i = 0;
        while i < s.len() {
            if s.as_bytes()[i].is_ascii_uppercase() {
                panic!(
                    "string passed to Lowercase::<str>::from_static is not ASCII lowercase (non-ascii is not yet supported)"
                );
            }

            i += 1;
        }

        // SAFETY: We have validated that all chars are lowercase.
        unsafe { Self::from_str(s) }
    }

    /// Returns the number of bytes in this string.
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the string is empty.
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a string slice with the prefix removed.
    pub fn strip_prefix(&self, prefix: &str) -> Option<&Self> {
        let s = self.0.strip_prefix(prefix)?;
        Some(unsafe { Self::from_str(s) })
    }

    /// Returns a string slice with the suffix removed.
    pub fn strip_suffix(&self, suffix: &str) -> Option<&Self> {
        let s = self.0.strip_suffix(suffix)?;
        Some(unsafe { Self::from_str(s) })
    }

    /// Creates a new `Lowercase<str>`.
    ///
    /// # Safety
    ///
    /// The string must represent an all-lowercase string.
    const unsafe fn from_str(s: &str) -> &Lowercase<str> {
        // SAFETY: Lowercase<T> is repr(transparent) with contents T.
        unsafe { core::mem::transmute::<&str, &Lowercase<str>>(s) }
    }
}

pub trait ToLowercase {
    type Target;

    fn to_lowercase(&self) -> Self::Target;
}

impl ToLowercase for &str {
    type Target = String;

    fn to_lowercase(&self) -> Self::Target {
        str::to_lowercase(self)
    }
}

impl ToLowercase for &BStr {
    type Target = BString;

    fn to_lowercase(&self) -> Self::Target {
        BString::new(ByteSlice::to_lowercase(&***self))
    }
}

impl<T> AsRef<T> for Lowercase<T>
where
    T: ?Sized,
{
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl Borrow<Lowercase<str>> for Lowercase<String> {
    fn borrow(&self) -> &Lowercase<str> {
        self.as_str()
    }
}

impl PartialEq<Lowercase<str>> for Lowercase<String> {
    fn eq(&self, other: &Lowercase<str>) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<Lowercase<String>> for Lowercase<str> {
    fn eq(&self, other: &Lowercase<String>) -> bool {
        self == other.as_str()
    }
}

impl<T> Debug for Lowercase<T>
where
    T: ?Sized + Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl<T> Display for Lowercase<T>
where
    T: ?Sized + Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Default for Lowercase<String> {
    fn default() -> Self {
        Self(String::new())
    }
}

impl Deref for Lowercase<String> {
    type Target = Lowercase<str>;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl ToOwned for Lowercase<str> {
    type Owned = Lowercase<String>;

    fn to_owned(&self) -> Self::Owned {
        let s = self.as_inner();
        Lowercase(s.to_owned())
    }
}

impl Lowercase<BString> {
    pub fn as_bstr(&self) -> &Lowercase<BStr> {
        unsafe { Lowercase::<BStr>::from_bstr(self.0.as_bstr()) }
    }
}

impl Lowercase<BStr> {
    const unsafe fn from_bstr(s: &BStr) -> &Lowercase<BStr> {
        // SAFETY: Lowercase<T> is repr(transparent) with contents T.
        unsafe { core::mem::transmute::<&BStr, &Lowercase<BStr>>(s) }
    }
}

impl Borrow<Lowercase<BStr>> for Lowercase<BString> {
    fn borrow(&self) -> &Lowercase<BStr> {
        self.as_bstr()
    }
}

impl PartialEq<Lowercase<BStr>> for Lowercase<BString> {
    fn eq(&self, other: &Lowercase<BStr>) -> bool {
        self.as_bstr() == other
    }
}

impl PartialEq<Lowercase<BString>> for Lowercase<BStr> {
    fn eq(&self, other: &Lowercase<BString>) -> bool {
        self == other.as_bstr()
    }
}

impl AsRef<Lowercase<BStr>> for Lowercase<BStr> {
    fn as_ref(&self) -> &Lowercase<BStr> {
        self
    }
}

impl AsRef<Lowercase<BStr>> for Lowercase<str> {
    fn as_ref(&self) -> &Lowercase<BStr> {
        // SAFETY: A `Lowercase<str>` is guaranteed to only contain lowercase bytes.
        unsafe { Lowercase::from_bstr(self.as_inner().as_bytes().as_bstr()) }
    }
}
