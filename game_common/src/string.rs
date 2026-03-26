use std::borrow::Borrow;
use std::fmt::{self, Debug, Display, Formatter};
use std::ops::Deref;

/// A string value that is guaranteed to be lowercase.
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
        while i < s.as_bytes().len() {
            if !s.as_bytes()[i].is_ascii_lowercase() {
                panic!("string passed to Lowercase::<str>::from_static is not ASCII lowercase (non-ascii is not yet supported)");
            }

            i += 1;
        }

        // SAFETY: We have validated that all chars are lowercase.
        unsafe { Self::from_str(s) }
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

    /// Creates a nwe `Lowercast<str>`.
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

impl<'a> ToLowercase for &'a str {
    type Target = String;

    fn to_lowercase(&self) -> Self::Target {
        str::to_lowercase(self)
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
