#[derive(Clone, Debug)]
pub struct DataPathBuf {
    path: String,
}

impl DataPathBuf {
    fn from_str(path: &str) -> Self {
        Self {
            path: fix_path(path),
        }
    }

    pub fn push_front(&mut self, path: &str) {
        let mut path = fix_path(path);
        while path.ends_with("/") {
            path.truncate(path.len() - 1);
        }

        self.path.insert_str(0, &path);
    }

    pub fn file_name(&self) -> Option<&str> {
        let start = self.path.rfind("/")?;

        // If the last char is a / there is no file.
        if start == self.path.len() - 1 {
            return None;
        }

        Some(&self.path[start + 1..])
    }

    pub fn file_stem(&self) -> Option<&str> {
        let file_name = self.file_name()?;
        let end = file_name.rfind(".").unwrap_or(file_name.len());
        Some(&file_name[..end])
    }

    pub fn extension(&self) -> Option<&str> {
        let file_name = self.file_name()?;
        let start = file_name.rfind(".")?;
        Some(&self.path[start + 1..])
    }

    pub fn set_extension(&mut self, extension: &str) {
        let Some(file_name_start) = self.path.rfind("/") else {
            return;
        };

        if let Some(extension_start) = self.path[file_name_start + 1..].rfind(".") {
            let start = file_name_start + extension_start + 2;
            self.path.truncate(start);
            self.path.push_str(extension);
        } else {
            self.path.push('.');
            self.path.push_str(extension);
        }
    }
}

impl From<String> for DataPathBuf {
    fn from(path: String) -> Self {
        Self::from_str(&path)
    }
}

impl AsRef<str> for DataPathBuf {
    fn as_ref(&self) -> &str {
        &self.path
    }
}

/// Normalizes the asset path.
fn fix_path(path: &str) -> String {
    let mut path = path.trim().replace("\\", "/");
    path.make_ascii_lowercase();

    let mut start = 0;
    let mut acc = 0;
    for seg in path.split('/') {
        if seg != "data" {
            acc += seg.len() + 1;
            continue;
        }

        start = acc + seg.len() + 1;
    }
    let path = &path[start..];

    // The file paths are all over the place.
    // Some start with data\, some with \data, some are even absolute
    // and start at c:\ and some have no prefix at all.
    let path = path.strip_prefix("/").unwrap_or(&path);
    let path = path.strip_prefix("data/").unwrap_or(&path);
    let path = path.strip_prefix("data").unwrap_or(&path);

    path.to_owned()
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
        let mut buf = DataPathBuf::from_str("testa/testb/testc.testd");
        buf.set_extension("xxx");
        assert_eq!(buf.path, "testa/testb/testc.xxx");
    }

    #[test]
    fn path_buf_set_extension_add() {
        let mut buf = DataPathBuf::from_str("testa/testb");
        buf.set_extension("abc");
        assert_eq!(buf.path, "testa/testb.abc");
    }
}
