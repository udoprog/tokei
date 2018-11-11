use std::io;
use std::fs;
use std::path::Path;
use std::borrow::Cow;

impl<'a> FileAccess<'a> for &'a Path {
    type Reader = fs::File;

    fn open(self) -> io::Result<Self::Reader> {
        fs::File::open(self)
    }

    fn name(self) -> Cow<'a, str> {
        self.to_string_lossy()
    }

    fn file_name(self) -> Option<Cow<'a, str>> {
        match self.file_name() {
            Some(filename_os) => {
                Some(Cow::from(filename_os.to_string_lossy().to_lowercase()))
            },
            None => None
        }
    }

    fn extension(self) -> Option<Cow<'a, str>> {
        match self.extension() {
            Some(extension_os) => {
                Some(Cow::from(extension_os.to_string_lossy().to_lowercase()))
            },
            None => None
        }
    }
}

/// Trait to access files for analysis.
///
/// It can be converted into a LanguageType (e.g. identify which language it belongs to).
/// The name of the file is typically its path, but this might be logical in case it's part of an
/// archive (tar, zip, ...).
pub trait FileAccess<'a>: Copy {
    type Reader: io::Read;

    /// Open the type for reading.
    fn open(self) -> io::Result<Self::Reader>;

    /// Get the name of the file object.
    fn name(self) -> Cow<'a, str>;

    /// Access the file name, if available.
    fn file_name(self) -> Option<Cow<'a, str>> {
        let name = match self.name() {
            Cow::Borrowed(n) => Cow::from(n.rsplit("/").next().unwrap_or_else(|| n)),
            Cow::Owned(n) => Cow::from(n.rsplit("/").next().unwrap_or_else(|| &n).to_string()),
        };

        Some(name)
    }

    /// Access the extension of the file, if available.
    fn extension(self) -> Option<Cow<'a, str>> {
        match self.file_name() {
            Some(Cow::Borrowed(n)) => n.rsplit(".").next().map(Cow::from),
            Some(Cow::Owned(n)) => n.rsplit(".").next().map(|s| s.to_string()).map(Cow::from),
            None => None,
        }
    }

    /// Rename the file access object.
    fn with_name(self, name: &'a str) -> WithName<'a, Self> {
        WithName {
            name,
            file_access: self,
        }
    }
}

/// Struct which causes a FileAccess object to be renamed.
///
/// Created using [`FileAccess::with_name`].
#[derive(Clone, Copy)]
pub struct WithName<'a, F> where F: FileAccess<'a> {
    name: &'a str,
    file_access: F,
}

impl<'a, F> FileAccess<'a> for WithName<'a, F> where F: FileAccess<'a> {
    type Reader = F::Reader;

    fn open(self) -> io::Result<Self::Reader> {
        self.file_access.open()
    }

    fn name(self) -> Cow<'a, str> {
        Cow::from(self.name)
    }
}
