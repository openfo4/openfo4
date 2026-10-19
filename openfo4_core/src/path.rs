use std::borrow::Borrow;
use std::fmt::{self, Debug, Display, Formatter};
use std::ops::Deref;

use bstr::{BStr, BString, ByteSlice};
use openfo4_common::utils::vec_ext::VecExt;

/// A path to a data file.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct DataPathBuf {
    bytes: BString,
}

impl DataPathBuf {
    pub fn new<T>(bytes: &T) -> Self
    where
        T: AsRef<BStr> + ?Sized,
    {
        Self {
            bytes: fix_path(bytes),
        }
    }

    pub fn push_front<T>(&mut self, path: T)
    where
        T: AsRef<BStr>,
    {
        let mut path = path.as_ref();
        if path.is_empty() {
            return;
        }

        // No leading slash.
        if let Some(p) = path.strip_prefix(b"/") {
            path = BStr::new(p);
        }

        self.bytes.extend_at(0, path);

        // Separator between new path and existing path.
        if !path.ends_with(b"/") {
            self.bytes.insert(path.len(), b'/');
        }

        self.bytes = fix_path(&self.bytes);
    }

    pub fn push<T>(&mut self, path: T)
    where
        T: AsRef<BStr>,
    {
        let mut path = path.as_ref();
        if path.is_empty() {
            return;
        }

        if let Some(p) = path.strip_prefix(b"/") {
            path = BStr::new(p);
        }

        if !self.bytes.starts_with(b"/") {
            self.bytes.push(b'/');
        }

        self.bytes.extend_from_slice(&path);

        self.bytes = fix_path(&self.bytes);
    }

    pub fn set_extension<T>(&mut self, extension: &T)
    where
        T: AsRef<BStr> + ?Sized,
    {
        let Some(file_name_start) = self.bytes.rfind_byte(b'/') else {
            return;
        };

        if let Some(extension_start) = self.bytes[file_name_start + 1..].rfind_byte(b'.') {
            let start = file_name_start + extension_start + 2;
            self.bytes.truncate(start);
            self.bytes.extend_from_slice(extension.as_ref());
        } else {
            self.bytes.push(b'.');
            self.bytes.extend_from_slice(extension.as_ref());
        }

        self.bytes = fix_path(&self.bytes);
    }
}

impl Deref for DataPathBuf {
    type Target = DataPath;

    fn deref(&self) -> &Self::Target {
        DataPath::from_bytes_ref(&self.bytes)
    }
}

impl Borrow<DataPath> for DataPathBuf {
    fn borrow(&self) -> &DataPath {
        &*self
    }
}

impl AsRef<DataPath> for DataPathBuf {
    fn as_ref(&self) -> &DataPath {
        &*self
    }
}

impl Debug for DataPathBuf {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.bytes, f)
    }
}

impl Display for DataPathBuf {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.bytes, f)
    }
}

#[derive(PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DataPath {
    // Invariants:
    // - All UTF8-compatible are lowercase.
    // - Components are separated by a single /, unix style.
    // - All paths are relative and without a prefixed /.
    // - The path contains no parent-references (..).
    bytes: BStr,
}

impl DataPath {
    pub const fn from_str(s: &str) -> &Self {
        // TODO: Open this up to full UTF8.
        assert!(s.is_ascii(), "DataPath::from_str is currently ASCII only");

        let mut i = 0;
        while i < s.len() {
            let b = s.as_bytes()[i];

            assert!(!b.is_ascii_uppercase(), "non-lowercase char found");

            if b == b'\\' {
                panic!("backslashes are not allowed");
            }

            // Scan for parent references ("/..").
            if b == b'/'
                && s.len() - i >= 3
                && s.as_bytes()[i + 1] == b'.'
                && s.as_bytes()[i + 2] == b'.'
            {
                panic!("parent references are not allowed");
            }

            if i == 0 && b == b'/' {
                panic!("path must not start with a /");
            }

            i += 1;
        }

        Self::from_bytes_ref(s.as_bytes())
    }

    const fn from_bytes_ref(bytes: &[u8]) -> &Self {
        // SAFETY:
        // `&BStr` has the same layout as `&[u8]` and `&Self` has the same
        // layout as `&BStr`.
        unsafe { core::mem::transmute::<&[u8], &Self>(bytes) }
    }

    pub fn components(&self) -> Components<'_> {
        Components { bytes: &self.bytes }
    }

    pub fn parent(&self) -> Option<&Self> {
        let end = self.bytes.rfind_byte(b'/')?;
        Some(Self::from_bytes_ref(&self.bytes[..end]))
    }

    /// Returns the final name of the file, if there is any.
    pub fn file_name(&self) -> Option<&BStr> {
        let start = self.bytes.rfind_byte(b'/')?;

        // If the last char is a / there is no file.
        if start == self.bytes.len() - 1 {
            return None;
        }

        Some(&self.bytes[start + 1..])
    }

    pub fn file_stem(&self) -> Option<&BStr> {
        let file_name = self.file_name()?;
        let end = file_name.rfind_byte(b'.').unwrap_or(file_name.len());
        Some(&file_name[..end])
    }

    pub fn extension(&self) -> Option<&BStr> {
        let file_name = self.file_name()?;
        let start = file_name.rfind_byte(b'.')?;
        file_name.get(start + 1..).map(BStr::new)
    }

    /// Returns `true` if the path starts with the given `base`.
    pub fn starts_with<T>(&self, base: &T) -> bool
    where
        T: AsRef<Self> + ?Sized,
    {
        base.as_ref()
            .components()
            .zip(self.components())
            .all(|(lhs, rhs)| lhs == rhs)
    }

    pub fn join<T>(&self, path: &T) -> DataPathBuf
    where
        T: AsRef<BStr> + ?Sized,
    {
        let mut p = self.to_owned();
        p.push(path.as_ref());
        p
    }

    pub fn strip_prefix<T>(&self, prefix: &T) -> Option<&Self>
    where
        T: AsRef<Self> + ?Sized,
    {
        let mut bytes = self.bytes.strip_prefix(&*prefix.as_ref().bytes)?;

        while let Some(b) = bytes.strip_prefix(b"/") {
            bytes = b;
        }

        Some(Self::from_bytes_ref(bytes))
    }

    pub fn as_bstr(&self) -> &BStr {
        &self.bytes
    }
}

impl AsRef<BStr> for DataPath {
    fn as_ref(&self) -> &BStr {
        &self.bytes
    }
}

impl AsRef<DataPath> for DataPath {
    fn as_ref(&self) -> &DataPath {
        self
    }
}

impl Debug for DataPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.bytes, f)
    }
}

impl Display for DataPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.bytes, f)
    }
}

impl ToOwned for DataPath {
    type Owned = DataPathBuf;

    fn to_owned(&self) -> Self::Owned {
        DataPathBuf {
            bytes: self.bytes.to_owned(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Components<'a> {
    bytes: &'a BStr,
}

impl<'a> Iterator for Components<'a> {
    type Item = &'a BStr;

    fn next(&mut self) -> Option<Self::Item> {
        if self.bytes.is_empty() {
            return None;
        }

        let index = self.bytes.find_byte(b'/').unwrap_or(self.bytes.len());
        let segment = &self.bytes[..index];

        // Invariant
        debug_assert!(!segment.starts_with(b"/"));

        self.bytes = BStr::new(self.bytes.get(index + 1..).unwrap_or_default());
        Some(segment)
    }
}

/// Normalizes the asset path.
fn fix_path<T>(path: &T) -> BString
where
    T: AsRef<BStr> + ?Sized,
{
    let mut path = path.as_ref().trim().replace("\\", "/");
    path.make_ascii_lowercase();

    let mut start = 0;
    let mut acc = 0;
    for seg in path.split(|b| *b == b'/') {
        if seg != b"data" {
            acc += seg.len() + 1;
            continue;
        }

        start = acc + seg.len() + 1;
    }
    let path = &path[start..];

    // The file paths are all over the place.
    // Some start with data\, some with \data, some are even absolute
    // and start at c:\ and some have no prefix at all.
    let path = path.strip_prefix(b"/").unwrap_or(&path);
    let path = path.strip_prefix(b"data/").unwrap_or(&path);
    let path = path.strip_prefix(b"data").unwrap_or(&path);

    BString::new(path.to_owned())
}

#[cfg(test)]
mod tests {
    use crate::path::DataPathBuf;

    use super::fix_path;

    #[test]
    fn data_path_buf_from_str() {
        assert_eq!(fix_path("\\NO_UPPERCASE\\ALLOWED"), "no_uppercase/allowed");
        assert_eq!(fix_path("meshes\\my_mesh"), "meshes/my_mesh");
        assert_eq!(fix_path("\\meshes\\my_mesh"), "meshes/my_mesh");
        assert_eq!(fix_path("data\\meshes\\my_mesh"), "meshes/my_mesh");
        assert_eq!(fix_path("\\data\\meshes\\my_mesh"), "meshes/my_mesh");
        assert_eq!(
            fix_path("c:\\a\\b\\data\\meshes\\my_mesh"),
            "meshes/my_mesh"
        );
        assert_eq!(
            fix_path("c:\\a\\b\\data\\a\\b\\data\\meshes\\my_mesh"),
            "meshes/my_mesh"
        );
    }

    #[test]
    fn path_buf_set_extension_replace() {
        let mut buf = DataPathBuf::new("testa/testb/testc.testd");
        buf.set_extension("xxx");
        assert_eq!(buf.bytes, "testa/testb/testc.xxx");
    }

    #[test]
    fn path_buf_set_extension_add() {
        let mut buf = DataPathBuf::new("testa/testb");
        buf.set_extension("abc");
        assert_eq!(buf.bytes, "testa/testb.abc");
    }
}
