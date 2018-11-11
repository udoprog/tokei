// Copyright (c) 2015 Aaron Power
// Use of this source code is governed by the APACHE2.0/MIT licence that can be
// found in the LICENCE-{APACHE/MIT} file.

use std::collections::{btree_map, BTreeMap};
use std::iter::IntoIterator;
use std::ops::{AddAssign, Deref, DerefMut};

use rayon::prelude::*;

#[cfg(feature = "io")] use serde;

use super::{Language, LanguageType};
use utils;
use FileAccess;

/// A collection of existing languages([_List of Languages_](https://github.com/Aaronepower/tokei#supported-languages))
#[derive(Debug, Default)]
pub struct Languages {
    inner: BTreeMap<LanguageType, Language>,
}

#[cfg(feature = "io")]
impl serde::Serialize for Languages {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: serde::Serializer {
            let map = self.remove_empty();
            map.serialize(serializer)
        }
}

#[cfg(feature = "io")]
impl<'de> serde::Deserialize<'de> for Languages {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: serde::Deserializer<'de> {
            let map = <_>::deserialize(deserializer)?;

            Ok(Self::from_previous(map))
        }
}

impl Languages {
    #[cfg(feature = "io")]
    fn from_previous(map: BTreeMap<LanguageType, Language>) -> Self {
        use std::collections::btree_map::Entry::*;
        let mut _self = Self::new();

        for (name, input_language) in map {
            match _self.entry(name) {
                Occupied(mut entry) => {
                    *entry.get_mut() += input_language;
                }
                Vacant(entry) => {
                    entry.insert(input_language);
                }
            }
        }
        _self
    }

    /// Get statistics from the list of paths provided, and a list ignored
    /// keywords to ignore paths containing them.
    ///
    /// ```no_run
    /// # use tokei::*;
    /// let mut languages = Languages::new();
    /// languages.get_statistics(&["."], vec![".git", "target"], None);
    /// ```
    pub fn get_statistics(&mut self,
                          paths: &[&str],
                          ignored: Vec<&str>,
                          types: Option<Vec<LanguageType>>)
    {
        utils::fs::get_all_files(paths, ignored, &mut self.inner, types);

        self.inner.par_iter_mut().for_each(|(_, l)| l.total());
    }

    /// Get statistics from a collection of objects.
    ///
    /// In its simplest form, it permits analyzing specific files,
    /// But this can also be used to collect statistics about non-filesystem objects, like files
    /// from an archive by providing a custom implementation of `FileAccess`.
    ///
    /// ```no_run
    /// # use tokei::*;
    /// # use std::path::Path;
    /// let mut languages = Languages::new();
    /// let files = vec![
    ///     Path::new("foo.txt"),
    ///     Path::new("bar.txt"),
    /// ];
    /// languages.get_statistics_from(files, None);
    /// ```
    pub fn get_statistics_from<'a, 'b: 'a, I: 'b, F>(
        &mut self,
        files: I,
        types: Option<Vec<LanguageType>>
    )
        where I: IntoIterator<Item = F>,
              F: Send + FileAccess<'a>,
    {
        utils::fs::get_all_file_accesses(files, &mut self.inner, types);
        self.inner.par_iter_mut().for_each(|(_, l)| l.total());
    }

    /// Constructs a new, blank `Languages`.
    ///
    /// ```
    /// # use tokei::*;
    /// let languages = Languages::new();
    /// ```
    pub fn new() -> Self {
        Languages::default()
    }

    /// Creates a new map that only contains non empty languages.
    ///
    /// ```
    /// use tokei::*;
    /// use std::collections::BTreeMap;
    ///
    /// let mut languages = Languages::new();
    /// languages.get_statistics(&["doesnt/exist"], vec![".git"], None);
    ///
    /// let empty_map = languages.remove_empty();
    ///
    /// assert_eq!(empty_map.len(), 0);
    /// ```
    pub fn remove_empty(&self) -> BTreeMap<&LanguageType, &Language> {
        let mut map = BTreeMap::new();

        for (name, language) in &self.inner {
            if !language.is_empty() {
                map.insert(name, language);
            }
        }
        map
    }
}

impl IntoIterator for Languages {
    type Item = <BTreeMap<LanguageType, Language> as IntoIterator>::Item;
    type IntoIter =
        <BTreeMap<LanguageType, Language> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a> IntoIterator for &'a Languages {
    type Item = (&'a LanguageType, &'a Language);
    type IntoIter = btree_map::Iter<'a, LanguageType, Language>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl<'a> IntoIterator for &'a mut Languages {
    type Item = (&'a LanguageType, &'a mut Language);
    type IntoIter = btree_map::IterMut<'a, LanguageType, Language>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter_mut()
    }
}

impl AddAssign<BTreeMap<LanguageType, Language>> for Languages {
    fn add_assign(&mut self, rhs: BTreeMap<LanguageType, Language>) {

        for (name, language) in rhs {

            if let Some(result) = self.inner.get_mut(&name) {
                *result += language;
            }
        }
    }
}

impl Deref for Languages {
    type Target = BTreeMap<LanguageType, Language>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Languages {
    fn deref_mut(&mut self) -> &mut BTreeMap<LanguageType, Language> {
        &mut self.inner
    }
}
