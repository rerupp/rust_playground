//! This module contains the data structures and logic to support duplicate
//! file reporting.
//!
//! This module does have domain objects exposed to the caller. I decided to
//! separate from the objects module because they really are specific to this
//! module. If they were moved into the object module there would be a lot of
//! boiler place needed to support them not being within the same module.

// Here's the contract with domain. Using this approach, as the use case went
// through revisions, modules allowed developing new implementations and object
// models then easily swap the changes in to verify report changes.
pub(crate) use ver4::DuplicateFoldersBuilder;
pub use ver4::{
    DuplicateFolders, DuplicateFoldersMatch, FolderAnalysisMd, FolderGroupId, FolderGroupMd, FolderNoMatchMd,
    FoldersMatchMd, FoldersNoMatch,
};

// #[allow(unused)]
pub mod ver4 {
    use super::super::{DuplicateIds, Error, FileMd, FolderMd, Metadata, Result};
    use std::{
        cmp::{Ord, Ordering, PartialEq},
        collections::HashMap,
        ops::Index,
        path::PathBuf,
    };

    /// Consolidate converting a collection of string things to a collection of &str
    macro_rules! vstrs {
        ($strings:expr) => {
            $strings.iter().map(|s| s.as_str()).collect::<Vec<&str>>()
        };
    }

    /// A utility that will compare two strings as pathnames.
    ///
    /// # Arguments
    ///
    /// * `l` is the left hand side that compared as a filesystem pathname.
    /// * `r` is the right hand side of the comparison.
    fn paths_cmp(l: &str, r: &str) -> Ordering {
        let lhs = PathBuf::from(l);
        let rhs = PathBuf::from(r);
        lhs.cmp(&rhs)
    }

    /// A utility that sorts and truncates the capacity of a vector.
    ///
    /// # Arguments
    ///
    /// * `v` is the vector that will be sorted.
    /// * `f` is the function that will be used to sort the vector by.
    fn vsort_by<T, F>(v: &mut Vec<T>, f: F)
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        v.sort_by(f);
        v.shrink_to(0);
    }

    /// A utility that sorts and truncates the capacity of a vector.
    fn vsort<T: Ord>(v: &mut Vec<T>) {
        v.sort();
        v.shrink_to(0);
    }

    /// The builder that creates the duplicate folders metadata.
    ///
    /// It is used by the api to assemble the metadata then perform an analysis on that
    /// data. It needs to be public but only for the crate.
    #[derive(Debug)]
    pub(crate) struct DuplicateFoldersBuilder {
        /// A collection to lookup folder metadata based on its identifier.
        folders_md: FoldersMd,
        /// A collection of the folders that have duplicate filenames.
        folder_group_filenames: HashMap<FolderGroupId, Vec<String>>,
        /// A container for errors that may have occurred when building the metadata.
        errors: Vec<String>,
    }
    impl DuplicateFoldersBuilder {
        /// Create the builder.
        pub fn new() -> Self {
            DuplicateFoldersBuilder {
                folders_md: FoldersMd::new(),
                folder_group_filenames: HashMap::new(),
                errors: vec![],
            }
        }
        /// Add a folders metadata to the builder.
        ///
        /// An error will be added if the builder already contains the folders metadata.
        ///
        /// # Arguments
        ///
        /// * `folder_md` is the folders metadata being added to the builder.
        pub fn add_folder_md(&mut self, folder_md: FolderMd) -> &mut Self {
            if let Some(folder_md) = self.folders_md.add(folder_md) {
                self.errors.push(format!("Yikes... {:?} was already added!", folder_md));
            }
            self
        }
        /// Add the metadata that links filenames to folders.
        ///
        /// Internally the method ensures the list of folder identifiers resulting from the duplicate
        /// identifers is sorted in ascending order. This method should be called after all of the
        /// folder metadata has been added due to validation rules.
        ///
        /// # Arguments
        ///
        /// `duplicate_ids` is the metadata tying a filename to a group of folders.
        pub fn add_duplicate_ids(&mut self, duplicate_ids: DuplicateIds) -> &mut Self {
            if self.validate_duplicate_ids(&duplicate_ids) {
                // either update the existing folder group or create a new one
                let fgid = FolderGroupId::from(&duplicate_ids);
                if let Some(filenames) = self.folder_group_filenames.get_mut(&fgid) {
                    filenames.push(duplicate_ids.filename);
                } else {
                    self.folder_group_filenames.insert(fgid, vec![duplicate_ids.filename]);
                }
            }
            self
        }
        /// Verify the duplicate ids have corresponding folder metadata loaded.
        ///
        /// # Arguments
        ///
        /// * `duplicate_ids` is what will be verified against the loaded folder metadata.
        fn validate_duplicate_ids(&mut self, duplicate_ids: &DuplicateIds) -> bool {
            macro_rules! add_error {
                ($error:expr) => {
                    self.errors.push(format!("Yikes... {:?} {}!", duplicate_ids, $error))
                };
            }
            let starting_errors_cnt = self.errors.len();
            for (folder_id, child_id) in &duplicate_ids.ids {
                if let Some(folder_md) = self.folders_md.get(folder_id) {
                    if let Some(metadata) = folder_md.children.get(&duplicate_ids.filename) {
                        if metadata.id() != *child_id {
                            add_error!(format!("{:?} duplicate file id mismatch", folder_md));
                        }
                    } else {
                        add_error!(format!("{:?} filename not found", folder_md));
                    }
                } else {
                    add_error!(format!("FolderMd id {} not found", folder_id));
                }
            }
            let folder_group_id = FolderGroupId::from(duplicate_ids);
            if let Some(filenames) = self.folder_group_filenames.get(&folder_group_id) {
                if filenames.contains(&duplicate_ids.filename) {
                    add_error!(format!("{} already exists in {:?}", duplicate_ids.filename, filenames));
                }
            }
            self.errors.len() == starting_errors_cnt
        }
        /// Consumme the builder and create the duplicate folders metadata.
        ///
        /// An error will be returned if errors were encountered when adding metadata.
        pub fn build(self) -> Result<DuplicateFolders> {
            if self.errors.is_empty() {
                let mut folder_groups = vec![];
                for (fgid, filenames) in self.folder_group_filenames {
                    let folders_md = self.folders_md.get_group(&fgid);
                    let analysis = analyze_folders_files(folders_md, &filenames);
                    folder_groups.push(FolderGroup::new(fgid, filenames, analysis));
                }
                Ok(DuplicateFolders::new(self.folders_md, folder_groups))
            } else {
                Err(Error::from(self.errors.join("\n")))
            }
        }
    }

    #[derive(Debug, Default)]
    /// Consolidate the use cases for accessing folders md.
    struct FoldersMd(HashMap<i64, FolderMd>);
    impl FoldersMd {
        /// Creates a new instance of the `FoldersMd` container.
        fn new() -> Self {
            Default::default()
        }
        /// Add folder md to the container.
        ///
        /// # Arguments
        ///
        /// * `folder_md' is the metadata that will be added.
        fn add(&mut self, folder_md: FolderMd) -> Option<FolderMd> {
            self.0.insert(folder_md.id, folder_md)
        }
        #[inline]
        /// Get the folder md associated with the id.
        ///
        /// # Arguments
        ///
        /// * `id` is the folder id of the folder md.
        fn get(&self, id: &i64) -> Option<&FolderMd> {
            self.0.get(id)
        }
        /// Get a collection of folder md for the folder group.
        ///
        /// # Arguments
        ///
        /// * `fgid` is the folder group id whose folder md will be collected.
        fn get_group(&self, fgid: &FolderGroupId) -> Vec<&FolderMd> {
            let mut folder_group = fgid.0.iter().map(|folder_id| self.index(folder_id)).collect();
            vsort_by(&mut folder_group, |lhs, rhs| paths_cmp(&lhs.pathname, &rhs.pathname));
            folder_group
        }
    }
    impl std::ops::Index<&i64> for FoldersMd {
        type Output = FolderMd;
        #[inline]
        /// Get the folder md associate with the id.
        ///
        /// # Arguments
        ///
        /// * `i` is the folder id of the folder md.
        fn index(&self, i: &i64) -> &Self::Output {
            self.0.get(i).unwrap()
        }
    }

    /// A collection of folder ids used to identify a group of folders.
    #[derive(Debug, PartialEq, Hash, Eq, PartialOrd, Ord)]
    pub struct FolderGroupId(Vec<i64>);
    impl FolderGroupId {
        /// Creates the folder group identifier.
        ///
        /// The `folder_ids` collection will be sorted in ascending order as part
        /// of the initialization.
        ///
        /// # Arguments
        ///
        /// * `folder_ids` is the collection of folder identifiers.
        fn new(mut folder_ids: Vec<i64>) -> Self {
            vsort(&mut folder_ids);
            Self(folder_ids)
        }
    }
    impl Clone for FolderGroupId {
        fn clone(&self) -> Self {
            let mut folder_ids = self.0.clone();
            // not sure this is needed but it was 50-50 on clone maintaining the capacity.
            folder_ids.shrink_to(0);
            Self(folder_ids)
        }
    }
    impl From<&DuplicateIds> for FolderGroupId {
        /// Create a folder group identifier from the duplicate folder ids.
        fn from(duplicate_ids_: &DuplicateIds) -> Self {
            let folder_ids: Vec<i64> = duplicate_ids_.ids.iter().map(|(folder_id, _)| *folder_id).collect();
            Self::new(folder_ids)
        }
    }
    impl From<Vec<&FileMd>> for FolderGroupId {
        /// Create a folder group identifier for the file metadata parent ids.
        fn from(file_mds: Vec<&FileMd>) -> Self {
            let folder_ids: Vec<i64> = file_mds.iter().map(|&md| md.parent_id).collect();
            Self::new(folder_ids)
        }
    }
    impl From<&Vec<&FolderMd>> for FolderGroupId {
        /// Create a folder group identifier from the collection of folders using each folders id.
        fn from(folder_mds: &Vec<&FolderMd>) -> Self {
            let folder_ids: Vec<i64> = folder_mds.iter().map(|&md| md.id).collect();
            Self::new(folder_ids)
        }
    }

    /// This is the domains metadata model for folders that have duplicate files.
    #[derive(Debug)]
    pub struct DuplicateFolders {
        /// A collection to lookup folder metadata based on its identifier.
        folders_md: FoldersMd,
        /// A collection of the folders that have duplicate filenames.
        folder_groups: Vec<FolderGroup>,
    }
    impl DuplicateFolders {
        /// Creates a new instance of the duplicate folders metadata.
        ///
        /// The `folder_groups` collection will be sorted by the folder group id.
        ///
        /// # Arguments
        ///
        /// * `folders_md` is the folders metadata for the folder groups.
        /// * `folder_groups` is the collection of folders with duplicate filenames.
        fn new(folders_md: FoldersMd, mut folder_groups: Vec<FolderGroup>) -> Self {
            vsort_by(&mut folder_groups, |lhs, rhs| lhs.fgid.cmp(&rhs.fgid));
            Self { folders_md, folder_groups }
        }
        /// Retrieves folder group metadata by index for the internal folder groups.
        ///
        /// # Arguments
        ///
        /// * `index` is the index of the folder group.
        fn get_folder_group_md(&self, index: usize) -> Option<FolderGroupMd> {
            if let Some(folder_group) = self.folder_groups.get(index) {
                // convert the folders that had matches
                let mut matches: Vec<(Vec<&FolderMd>, Vec<&str>)> = vec![];
                for (fgid, filenames) in &folder_group.analysis.matches {
                    matches.push((self.folders_md.get_group(fgid), vstrs!(filenames)));
                }
                let mut no_matches: Vec<(&FolderMd, Vec<&str>)> = vec![];
                // convert the folders that did not have matches
                for (fid, filenames) in &folder_group.analysis.no_matches {
                    no_matches.push((&self.folders_md[fid], vstrs!(filenames)));
                }
                vsort_by(&mut no_matches, |(lhs, _), (rhs, _)| paths_cmp(&lhs.pathname, &rhs.pathname));
                // create the metadata
                Some(FolderGroupMd {
                    fgid: folder_group.fgid.clone(),
                    folders_md: self.folders_md.get_group(&folder_group.fgid),
                    filenames: vstrs!(&folder_group.filenames),
                    folder_analysis: FolderAnalysisMd {
                        // TODO: review the need to include fgid, folders_md, and filenames for the analysis
                        fgid: folder_group.fgid.clone(),
                        folders_md: self.folders_md.get_group(&folder_group.fgid),
                        filenames: vstrs!(&folder_group.filenames),
                        file_matches: matches,
                        files_without_match: no_matches,
                    },
                })
            } else {
                None
            }
        }
    }
    impl<'df> IntoIterator for &'df DuplicateFolders {
        type Item = FolderGroupMd<'df>;
        type IntoIter = DuplicateFolderIterator<'df>;
        /// Creates the iterator that can traverse the duplicate folders metadata.
        fn into_iter(self) -> Self::IntoIter {
            DuplicateFolderIterator { duplicate_folders: &self, index: 0 }
        }
    }

    /// The iterator over duplicate folders metadata.
    #[derive(Debug)]
    pub struct DuplicateFolderIterator<'df> {
        /// A reference to the duplicate folders metadata.
        duplicate_folders: &'df DuplicateFolders,
        /// Holds the index of the next duplicate folder metadata element.
        index: usize,
    }
    /// The implementation of the duplicate folder metadata iterator.
    impl<'df> Iterator for DuplicateFolderIterator<'df> {
        type Item = FolderGroupMd<'df>;
        fn next(&mut self) -> Option<Self::Item> {
            let next_group = self.duplicate_folders.get_folder_group_md(self.index);
            if next_group.is_some() {
                self.index += 1;
            }
            next_group
        }
    }

    /// The collection of folders that have filenames in common.
    #[derive(Debug)]
    pub struct FolderGroup {
        /// The folder ids that have filenames in common.
        fgid: FolderGroupId,
        /// The filenames that are common to each of the folders.
        filenames: Vec<String>,
        /// The analysis of which folders have matching files and those that do not.
        analysis: FolderAnalysis,
    }
    impl FolderGroup {
        /// Create a new folder group that have filenames in common.
        ///
        /// The filenames collection will be sorted into ascending order.
        ///
        /// # Arguments
        ///
        /// * `fgid` is id of the folder group.
        /// * `filenames` is the collection of filenames in common with each of the folders.
        /// * `analysis` describes the files that matched and the files that did not.
        fn new(fgid: FolderGroupId, mut filenames: Vec<String>, analysis: FolderAnalysis) -> Self {
            vsort(&mut filenames);
            FolderGroup { fgid, filenames, analysis }
        }
    }

    /// The analysis of folders that have filenames in common.
    #[derive(Debug)]
    struct FolderAnalysis {
        /// The collection of folder groups that have files that match.
        matches: Vec<(FolderGroupId, Vec<String>)>,
        /// The collection of folders and files that did not match.
        no_matches: Vec<(i64, Vec<String>)>,
    }
    impl FolderAnalysis {
        /// Creates an instance of the metadtata describing folder file matches and files that did not match.
        ///
        /// The `matches` collection will be orderd by ascending folder group id. The `no_matches` collection
        /// will be ordered by ascending folder id. The filenames in both collections will be sorted in ascending
        /// order.
        fn new(mut matches: Vec<(FolderGroupId, Vec<String>)>, mut no_matches: Vec<(i64, Vec<String>)>) -> Self {
            matches.iter_mut().for_each(|(_, filenames)| vsort(filenames));
            vsort_by(&mut matches, |(lhs, _), (rhs, _)| lhs.cmp(rhs));
            no_matches.iter_mut().for_each(|(_, filenames)| vsort(filenames));
            vsort_by(&mut no_matches, |(lhs, _), (rhs, _)| lhs.cmp(rhs));
            FolderAnalysis { matches, no_matches }
        }
    }

    // The metadata associated with folders that have common file names.
    #[derive(Debug)]
    pub struct FolderGroupMd<'df> {
        /// The folder group id
        pub fgid: FolderGroupId,
        /// The collection of folder metadata for the folder group.
        pub folders_md: Vec<&'df FolderMd>,
        /// The duplicate filenames that were analyzed for this group of folders.
        pub filenames: Vec<&'df str>,
        /// The folder analysis for this group
        pub folder_analysis: FolderAnalysisMd<'df>,
    }

    /// The metadata associated with the analysis of files within a folder group.
    #[derive(Debug)]
    pub struct FolderAnalysisMd<'fa> {
        /// The folder identifier of this group.
        pub fgid: FolderGroupId,
        /// The collection of folders metadata.
        pub folders_md: Vec<&'fa FolderMd>,
        /// The filenames analyzed in this group of folders.
        pub filenames: Vec<&'fa str>,
        /// The folder matches
        pub file_matches: Vec<(Vec<&'fa FolderMd>, Vec<&'fa str>)>,
        /// The folders with files that did not match
        pub files_without_match: Vec<(&'fa FolderMd, Vec<&'fa str>)>,
    }

    /// Used internally to check for file matches in folders that have duplicate
    /// filenames.
    ///
    /// > ### GIGO!
    /// > The caller guarantees all duplicate filenames can be found within
    /// > the collection of folders metadata.
    ///
    /// # Arguments
    ///
    /// * `folders_md` is the metadata for folders that contain the same filename.
    /// * `filenames` is the list of names to examine in the folders.
    fn analyze_folders_files(folders_md: Vec<&FolderMd>, filenames: &Vec<String>) -> FolderAnalysis {
        // track the folder matches, misses, and the files
        let mut folder_matches: HashMap<FolderGroupId, Vec<String>> = HashMap::new();
        let mut no_file_matches: HashMap<i64, Vec<String>> = HashMap::new();
        for filename in filenames.iter() {
            let (matches, no_matches) = analyze_folders_file(&folders_md, filename);
            for fgid in matches {
                if let Some(files_matched) = folder_matches.get_mut(&fgid) {
                    files_matched.push(filename.clone());
                } else {
                    folder_matches.insert(fgid, vec![filename.clone()]);
                }
            }
            for fid in no_matches {
                if let Some(no_files_matched) = no_file_matches.get_mut(&fid) {
                    no_files_matched.push(filename.clone())
                } else {
                    no_file_matches.insert(fid, vec![filename.clone()]);
                }
            }
        }
        FolderAnalysis::new(folder_matches.into_iter().collect(), no_file_matches.into_iter().collect())
    }

    /// Used internally to collect the folder metadata and analyze files by filename.
    ///
    /// > ### GIGO!
    /// > The caller guarantees the filename can be found in all folders metadata
    /// > otherwise it will panic.
    ///
    /// # Arguments
    ///
    /// * `folders_md` is the collection of folder metadata with a common filename.
    /// * `filename` is the name of the file to analyze.
    fn analyze_folders_file(folders_md: &Vec<&FolderMd>, filename: &str) -> (Vec<FolderGroupId>, Vec<i64>) {
        // collect all the file metdata from the folders
        let files_md: Vec<&FileMd> = folders_md
            .iter()
            .map(|&md| {
                if let Metadata::File(file_md) = &md.children[filename] {
                    file_md
                } else {
                    log::error!("Yikes... Expected FileMd: {:#?}.", &md.children[filename]);
                    panic!("Yikes... {filename} is not file metadata!")
                }
            })
            .collect();
        // analyze the files and collect the results
        let (matches, no_matches) = folder_file_matches(files_md);
        (
            matches.into_iter().map(|file_mds| FolderGroupId::from(file_mds)).collect(),
            no_matches.into_iter().map(|file_md| file_md.parent_id).collect(),
        )
    }

    /// Used internally to analyze files that share a common filename.
    ///
    /// The current scan to see if a file matches is simply looking at
    /// the size. At some point I'll look at adding something like a
    /// crc check to be more confident. There is no validation as part
    /// of the check (like do they really all share the same filename)
    /// so garbage in gargage out applies.
    ///
    /// Typically the list returned will only contain a single entry. The
    /// entry in the list will be a list of the file identifiers that
    /// match. The reason for it being a list comes from the following
    /// use case. Assume the following files and their content.
    ///
    /// | File | Content |
    /// | :---: | :---: |
    /// | file_1 | ABCD |
    /// | file_2 | XYZ |
    /// | file_3 | ABCD |
    /// | file_4 | XYZ |
    /// | file_5 | NOP |
    ///
    /// In this use case there are four file matches however there are two groups
    /// of folder matches.
    ///
    /// * *file_1* and *file_3*
    /// * *file_2* and *file_4*
    ///
    /// The function returns a tuple consisting of the folder group identifiers for
    /// matching folders, and a collection of the folder identifiers that did not
    /// match.
    ///
    /// > ### GIGO!
    /// > The collection of file metadata is not validated. The caller is responsible
    /// > for the content.
    ///
    /// # Arguments
    ///
    /// `files_md` is the collection of files to examine. The caller guarantees the
    /// file metadata otherwise GIGO.
    fn folder_file_matches(files_md: Vec<&FileMd>) -> (Vec<Vec<&FileMd>>, Vec<&FileMd>) {
        // the file match groupings
        let mut group_matches: Vec<Vec<&FileMd>> = vec![];
        // the filen ids that have matched
        let mut matched: Vec<i64> = vec![];
        // walk the files looking for matches
        let files_cnt = files_md.len();
        for i in 0..files_cnt {
            // filter files already matched
            let lhs_md = files_md[i];
            if matched.contains(&lhs_md.id) {
                continue;
            }
            // search for another file match
            let mut current_group = vec![lhs_md];
            for n in i + 1..files_cnt {
                // filter files already matched
                let rhs_md = files_md[n];
                if matched.contains(&rhs_md.id) {
                    continue;
                }
                // right now the test is only size however it really should have some crc validation
                if lhs_md.size == rhs_md.size {
                    current_group.push(rhs_md);
                }
            }
            if current_group.len() > 1 {
                // there were multiple matches so do the housekeeping
                current_group.iter().for_each(|&md| matched.push(md.id));
                group_matches.push(current_group);
            }
        }
        // collect up the files that did not have a match
        let not_matched: Vec<&FileMd> = files_md
            .iter()
            .filter_map(|&md| match matched.contains(&md.id) {
                true => None,
                false => Some(md),
            })
            .collect();
        (group_matches, not_matched)
    }

    /// The container of folders with common filenames that match.
    #[derive(Debug)]
    pub struct DuplicateFoldersMatch {
        /// A collection to lookup folder metadata based on its identifier.
        folders_md: FoldersMd,
        /// The collection of folder groups that have file which match.
        folder_matches: Vec<FoldersMatch>,
    }
    impl DuplicateFoldersMatch {
        /// Get the folders match metadata for one of the duplicate folder groups.
        ///
        /// # Arguments
        ///
        /// * `index` identifies which folder match group metaddata will be returned.
        pub fn get_md(&self, index: usize) -> Option<FoldersMatchMd> {
            match self.folder_matches.get(index) {
                Some(folders_match) => {
                    let fgid = folders_match.fgid.clone();
                    let folders_md = self.folders_md.get_group(&fgid);
                    // matches and except are already ordered
                    let matches: Vec<&str> = vstrs!(folders_match.matches);
                    let except: Vec<&str> = vstrs!(folders_match.except);
                    // collect up the other matches
                    let mut other_matches: Vec<(&FolderMd, Vec<Vec<&FolderMd>>)> = vec![];
                    for (id, fgids) in &folders_match.other_matches {
                        other_matches.push((
                            &self.folders_md[id],
                            fgids.iter().map(|fgid| self.folders_md.get_group(&fgid)).collect(),
                        ));
                    }
                    vsort_by(&mut other_matches, |(lhs, _), (rhs, _)| paths_cmp(&lhs.pathname, &rhs.pathname));
                    Some(FoldersMatchMd { fgid, folders_md, matches, except, other_matches })
                }
                None => None,
            }
        }
    }

    /// The internal metadata for a group of folders that have common file names with matching files.
    #[derive(Debug)]
    pub struct FoldersMatch {
        /// The folder group id for this group of folders
        fgid: FolderGroupId,
        /// The filenames these folders have in common.
        matches: Vec<String>,
        /// The names of files that did not match in this group.
        except: Vec<String>,
        /// For folders in this group, other folder group that have matching filenames.
        other_matches: Vec<(i64, Vec<FolderGroupId>)>,
    }
    impl FoldersMatch {
        /// Creates the metadata for matching folders.
        ///
        /// The `matches`, `except`, and `other_matches` collection will be sorted in ascending order.
        ///
        /// # Arguments
        ///
        /// * `matches` is the list of filenames this group of folders have in common.
        /// * `except` is the list of filenames that did not have matching file content.
        /// * `other_matches` identifies folders in this group that are part of other matching folder groups.
        fn new(
            fgid: FolderGroupId,
            mut matches: Vec<String>,
            mut except: Vec<String>,
            mut other_matches: Vec<(i64, Vec<FolderGroupId>)>,
        ) -> Self {
            vsort(&mut matches);
            vsort(&mut except);
            other_matches.iter_mut().for_each(|(_, folder_groups)| vsort(folder_groups));
            vsort_by(&mut other_matches, |(lhs, _), (rhs, _)| lhs.cmp(rhs));
            Self { fgid, matches, except, other_matches }
        }
    }
    impl From<DuplicateFolders> for DuplicateFoldersMatch {
        /// Convert the duplicate folders metadata into duplicate folders that match metadata.
        fn from(duplicate_folders: DuplicateFolders) -> Self {
            let mut folder_matches = builders::FoldersMatchBuilder::new();
            for folder_group in duplicate_folders.folder_groups {
                folder_matches.add(folder_group);
            }
            DuplicateFoldersMatch { folders_md: duplicate_folders.folders_md, folder_matches: folder_matches.build() }
        }
    }

    /// The folder match metadata for the domain API.
    #[derive(Debug)]
    pub struct FoldersMatchMd<'m> {
        /// The id for the matching folder group.
        pub fgid: FolderGroupId,
        /// The collection of folders metadata.
        pub folders_md: Vec<&'m FolderMd>,
        /// The filenames that this group of folders had in common.
        pub matches: Vec<&'m str>,
        /// The filenames that did not match.
        pub except: Vec<&'m str>,
        /// Other folder group matches folders in this group might have.
        pub other_matches: Vec<(&'m FolderMd, Vec<Vec<&'m FolderMd>>)>,
    }

    /// The iterator structure allowing the folder match metadata to be traversed.
    pub struct FoldersMatchIterator<'m> {
        folders_match: &'m DuplicateFoldersMatch,
        index: usize,
    }
    impl<'df> IntoIterator for &'df DuplicateFoldersMatch {
        type Item = FoldersMatchMd<'df>;
        type IntoIter = FoldersMatchIterator<'df>;
        /// Converts the duplicate folders metadata into an iterator.
        fn into_iter(self) -> Self::IntoIter {
            FoldersMatchIterator { folders_match: &self, index: 0 }
        }
    }
    impl<'m> Iterator for FoldersMatchIterator<'m> {
        type Item = FoldersMatchMd<'m>;
        /// Returns the next folders match metadata or `None` once traversal is complete.
        fn next(&mut self) -> Option<Self::Item> {
            let folders_match = self.folders_match.get_md(self.index);
            if folders_match.is_some() {
                self.index += 1;
            }
            folders_match
        }
    }

    #[derive(Debug)]
    /// The metadata for a folder that did not have at least one filename match.
    pub struct FolderNoMatch {
        /// The folder id.
        id: i64,
        /// The count of files this folder did match.
        matches: usize,
        /// The filenames that did not have a match.
        filenames: Vec<String>,
    }
    impl FolderNoMatch {
        /// Creates the metadata for a folder without a file match.
        ///
        /// The `filenames` collection is sorted by filename in ascending order.
        ///
        /// # Arguments
        ///
        /// * `id` is the folder id.
        /// * `matches` is that count of files the folder did match.
        /// * `filenames` is the collection of files that did not match.
        fn new(id: i64, matches: usize, mut filenames: Vec<String>) -> Self {
            vsort(&mut filenames);
            Self { id, matches, filenames }
        }
    }

    /// The folder that did not match files metadata for the domain API.
    #[derive(Debug)]
    pub struct FolderNoMatchMd<'m> {
        /// The folders metadata.
        pub folder_md: &'m FolderMd,
        /// The collection of filenames that did not match.
        pub filenames: Vec<&'m str>,
        /// The count of files the folder did match.
        pub other_matches: usize,
    }

    /// The internal metadata for a folder that did not match common file names.
    #[derive(Debug)]
    pub struct FoldersNoMatch {
        /// A collection to lookup folder metadata based on its identifier.
        folders_md: FoldersMd,
        /// The collection of folders that have group matches except for these files.
        // no_matches: Vec<(i64, Vec<String>)>,
        no_matches: Vec<FolderNoMatch>,
    }
    impl FoldersNoMatch {
        /// Creates the folders that do not have file matches metadata.
        ///
        /// The `no_matches` collection will be sorted by folder id.
        ///
        /// # Arguments
        ///
        /// * `folders_md` contains all of the duplicate folders, folder metadata.
        /// * `no_matches` is the collection of folders without file matches metadata.
        fn new(folders_md: FoldersMd, mut no_matches: Vec<FolderNoMatch>) -> Self {
            vsort_by(&mut no_matches, |lhs, rhs| lhs.id.cmp(&rhs.id));
            Self { folders_md, no_matches }
        }
        /// Get the folder no matches metadata by collection index.
        ///
        /// # Arguments
        ///
        /// * `index` identifies which entry in the no match metadata to retrieve.
        fn get_md(&self, index: usize) -> Option<FolderNoMatchMd> {
            match self.no_matches.get(index) {
                Some(folder_no_match) => {
                    let folder_md = &self.folders_md[&folder_no_match.id];
                    let filenames = vstrs!(&folder_no_match.filenames);
                    let other_matches = folder_no_match.matches;
                    Some(FolderNoMatchMd { folder_md, filenames, other_matches })
                }
                None => None,
            }
        }
    }
    impl From<DuplicateFolders> for FoldersNoMatch {
        /// Converts the duplicate folders metadata into folders that did not match metadata.
        fn from(duplicate_folders: DuplicateFolders) -> Self {
            let mut no_matches = builders::FoldersNoMatchBuilder::new();
            for folder_group in duplicate_folders.folder_groups {
                no_matches.add(folder_group);
            }
            FoldersNoMatch::new(duplicate_folders.folders_md, no_matches.build())
        }
    }

    /// The metadata supporting traversal of the folders with out matches data.
    #[derive(Debug)]
    pub struct FoldersNoMatchIterator<'n> {
        /// The live reference back to the folders without match metadata.
        folders_no_match: &'n FoldersNoMatch,
        /// The index of which metadata will be returned next.
        index: usize,
        /// Maps the iteration order by folder pathname not id.
        pathname_order: Vec<usize>,
    }
    impl<'n> IntoIterator for &'n FoldersNoMatch {
        type Item = FolderNoMatchMd<'n>;
        type IntoIter = FoldersNoMatchIterator<'n>;
        /// Creates the iterator that will traverse folders without file matches by folder pathname.
        fn into_iter(self) -> Self::IntoIter {
            let mut pathname_sorter: Vec<(usize, &str)> = self
                .no_matches
                .iter()
                .enumerate()
                .map(|(index, md)| (index, self.folders_md[&md.id].pathname.as_str()))
                .collect();
            vsort_by(&mut pathname_sorter, |(_, lhs), (_, rhs)| lhs.cmp(rhs));
            let mut pathname_order: Vec<usize> = Vec::with_capacity(pathname_sorter.len());
            pathname_sorter.iter().for_each(|(index, _)| pathname_order.push(*index));
            FoldersNoMatchIterator { folders_no_match: &self, index: 0, pathname_order }
        }
    }
    impl<'n> Iterator for FoldersNoMatchIterator<'n> {
        type Item = FolderNoMatchMd<'n>;
        /// Returns the next folder that did not match files metadata by folder pathname.
        fn next(&mut self) -> Option<Self::Item> {
            let folders_match = match self.pathname_order.get(self.index) {
                Some(index) => {
                    let md = self.folders_no_match.get_md(*index);
                    self.index += 1;
                    md
                }
                None => None,
            };
            folders_match
        }
    }

    mod builders {
        //! This module consolidates the duplicate folder metadata builders.
        use super::{vsort_by, FolderGroup, FolderGroupId, FolderNoMatch, FoldersMatch, HashMap};

        /// The builder that facilitates creating the folders match metadata.
        #[derive(Debug, Default)]
        pub struct FoldersMatchBuilder {
            /// The individual folder groups that have common filenames with matching files.
            matches: HashMap<FolderGroupId, (Vec<String>, Vec<String>)>,
            /// The filenames that were not matched for a given folder id.
            no_matches: HashMap<i64, Vec<String>>,
            /// The folder groups with matching folders, by folder id.
            matches_to_fgids: HashMap<i64, Vec<FolderGroupId>>,
        }
        impl FoldersMatchBuilder {
            /// Creates the new instances using `Default::default()`;
            pub fn new() -> Self {
                Default::default()
            }
            /// Adds a folder group to the builders.
            ///
            /// This will manage add matches and tracking the folders that did not match.
            ///
            /// # Arguments
            ///
            /// * `folder_group` contains the analysis that will be added to the builder.
            pub fn add(&mut self, folder_group: FolderGroup) {
                // the match metadata
                for (fgid, match_filenames) in folder_group.analysis.matches {
                    self.add_matches(&fgid, &folder_group.filenames, match_filenames);
                }
                // the no match metadata
                for (folder_id, filenames) in folder_group.analysis.no_matches {
                    self.add_no_matches(folder_id, filenames);
                }
            }
            /// Add the folders that had file matches.
            ///
            /// # Arguments
            ///
            /// * `fgid` is the identifier of the folder group.
            /// * `filenames` is the collection of file names that were examined for this group.
            /// * `matches` is the collection of file names that were matched.
            fn add_matches(&mut self, fgid: &FolderGroupId, filenames: &Vec<String>, mut matches: Vec<String>) {
                // remember what files  matched for the folder group
                let mut filenames = filenames.clone();
                if let Some((group_filenames, group_matches)) = self.matches.get_mut(fgid) {
                    group_filenames.append(&mut filenames);
                    group_matches.append(&mut matches);
                } else {
                    self.matches.insert(fgid.clone(), (filenames, matches));
                }
                // track the folder group associated with the folder
                fgid.0.iter().for_each(|id| match self.matches_to_fgids.get_mut(id) {
                    Some(folder_groups) => {
                        if !folder_groups.contains(&fgid) {
                            folder_groups.push(fgid.clone());
                        }
                    }
                    None => {
                        self.matches_to_fgids.insert(*id, vec![fgid.clone()]);
                    }
                });
            }
            /// Add the folders that did not match files.
            ///
            /// # Arguments
            ///
            /// * `folder_id` is the folder id.
            /// * `no_matches` has the file names that were not matched for the folder.
            fn add_no_matches(&mut self, folder_id: i64, mut no_matches: Vec<String>) {
                if let Some(folder_filenames) = self.no_matches.get_mut(&folder_id) {
                    folder_filenames.append(&mut no_matches);
                } else {
                    self.no_matches.insert(folder_id, no_matches);
                }
            }
            /// Create the collection of folders match metadata.
            ///
            /// The resulting folders match metadata will be ordered by the folder group id.
            pub fn build(self) -> Vec<FoldersMatch> {
                let mut folders_match: Vec<FoldersMatch> = Vec::with_capacity(self.matches.len());
                for (fgid, (group_filenames, group_matches)) in self.matches {
                    // get the difference between the group filenames and group matches
                    let excepts = group_filenames
                        .iter()
                        .filter_map(|filename| match group_matches.contains(filename) {
                            true => None,
                            false => Some(filename.clone()),
                        })
                        .collect();
                    let mut other_matches: Vec<(i64, Vec<FolderGroupId>)> = vec![];
                    for id in &fgid.0 {
                        let other_match_groups = filter_fgid(&fgid, &self.matches_to_fgids[id]);
                        if other_match_groups.len() > 0 {
                            other_matches.push((*id, other_match_groups));
                        }
                    }
                    // collect the other matches
                    folders_match.push(FoldersMatch::new(fgid, group_filenames, excepts, other_matches))
                }
                vsort_by(&mut folders_match, |lhs, rhs| lhs.fgid.cmp(&rhs.fgid));
                folders_match
            }
        }

        /// Removes a folder group identifier from  a collection of folder group identifies.
        ///
        /// # Arguments
        ///
        /// * `fgid` is the folder group that will be removed.
        /// * `fgids` is the collection of folder group identifiers that will be examined.
        fn filter_fgid(fgid: &FolderGroupId, fgids: &Vec<FolderGroupId>) -> Vec<FolderGroupId> {
            fgids
                .iter()
                .filter_map(|other| match fgid == other {
                    true => None,
                    false => Some(other.clone()),
                })
                .collect()
        }

        /// The builder that create the folders that did not match metadata.
        #[derive(Debug, Default)]
        pub struct FoldersNoMatchBuilder {
            /// The collection that tracks the count of folder file matches.
            folder_matches: HashMap<i64, usize>,
            /// The collection that contains the files that were not matched by a folder.
            folders_no_match: HashMap<i64, Vec<String>>,
        }
        impl FoldersNoMatchBuilder {
            /// Creates a new instance of the builder using `Default::default()`.
            pub fn new() -> Self {
                Default::default()
            }
            /// Adds a folder group analysis to the builder.
            ///
            /// # Arguments
            ///
            /// * `folder_group` holds the analysis that will be added to the builder.
            pub fn add(&mut self, folder_group: FolderGroup) {
                self.add_matches(folder_group.analysis.matches);
                self.add_no_matches(folder_group.analysis.no_matches);
            }
            /// Adds the folders that had matching files.
            ///
            /// # Arguments
            ///
            /// * `matches` has the folder group and file names that matched.
            fn add_matches(&mut self, matches: Vec<(FolderGroupId, Vec<String>)>) {
                for (fgid, filenames) in matches {
                    let match_count = filenames.len();
                    // remember the ids for folders and keep a count of files that were matched
                    fgid.0.iter().for_each(|id| {
                        if let Some(matches) = self.folder_matches.get_mut(id) {
                            *matches += match_count;
                        } else {
                            self.folder_matches.insert(*id, match_count);
                        }
                    });
                }
            }
            /// Add the folders that did not have matching files.
            ///
            /// # Arguments
            ///
            /// * `no_matches` is the folder and file names that were not matched.
            fn add_no_matches(&mut self, no_matches: Vec<(i64, Vec<String>)>) {
                for (folder_id, mut filenames) in no_matches {
                    if let Some(no_match_filenames) = self.folders_no_match.get_mut(&folder_id) {
                        no_match_filenames.append(&mut filenames);
                    } else {
                        self.folders_no_match.insert(folder_id, filenames);
                    }
                }
            }
            /// Creates the collection of metadata for folders that did not match.
            ///
            /// Note: For this use case the collection of metadata is not sorted in a
            /// predictable order. It is the owner of this collection that will manage
            /// ordering.
            pub fn build(mut self) -> Vec<FolderNoMatch> {
                // let mut folder_no_matches: Vec<FolderNoMatch> = self
                let folder_no_matches: Vec<FolderNoMatch> = self
                    .folders_no_match
                    .into_iter()
                    .map(|(id, filenames)| {
                        // there's either a match count or 0...
                        let matches = self.folder_matches.remove(&id).unwrap_or(0);
                        FolderNoMatch::new(id, matches, filenames)
                    })
                    .collect();
                folder_no_matches
            }
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;
        use std::path::PathBuf;
        macro_rules! filemd {
            (($id:expr, $parent_id:expr), $filename:expr, $size:expr) => {
                file_md($id, $parent_id, $filename, $size)
            };
            ($id:expr, $filename:expr) => {
                file_md($id, 0, $filename, 0)
            };
            ($id:expr, $filename:expr, $size:expr) => {
                file_md($id, 0, $filename, $size)
            };
        }
        fn file_md(id: i64, parent_id: i64, filename: &str, size: u64) -> FileMd {
            FileMd {
                id,
                parent_id,
                pathname: String::default(),
                name: filename.to_string(),
                is_symlink: false,
                size,
                created: 0,
                modified: 0,
            }
        }
        fn folder_md(id: i64, pathname: &str, children: Vec<FileMd>) -> FolderMd {
            let pathname: PathBuf = PathBuf::from(pathname).components().into_iter().collect();
            let children: Vec<(String, Metadata)> = children
                .into_iter()
                .map(|mut md| {
                    md.parent_id = id;
                    md.pathname = pathname.join(&md.name).as_path().display().to_string();
                    (md.name.clone(), Metadata::File(md))
                })
                .collect();
            let folder_md = FolderMd {
                id,
                parent_id: 0,
                pathname: pathname.to_string_lossy().to_string(),
                name: pathname.file_name().unwrap().to_string_lossy().to_string(),
                size: 0,
                created: 0,
                modified: 0,
                children: children.into_iter().collect(),
            };
            folder_md
        }
        fn duplicate_ids(filename: &str, ids: Vec<(i64, i64)>) -> DuplicateIds {
            DuplicateIds { filename: filename.to_string(), ids }
        }
        fn duplicate_folders_builder(md: Vec<FolderMd>) -> DuplicateFoldersBuilder {
            let mut builder = DuplicateFoldersBuilder::new();
            md.into_iter().for_each(|folder_md| {
                builder.add_folder_md(folder_md);
            });
            assert!(builder.errors.is_empty());
            builder
        }
        #[test]
        fn vec_sort() {
            let mut testcase: Vec<i64> = Vec::with_capacity(5);
            [3, 2, 1].into_iter().for_each(|id| testcase.push(id));
            assert_eq!(testcase, [3, 2, 1]);
            assert_eq!(testcase.capacity(), 5);
            vsort(&mut testcase);
            assert_eq!(testcase, [1, 2, 3]);
            assert_eq!(testcase.capacity(), 3);
            let mut testcase: Vec<(i64, String)> = Vec::with_capacity(10);
            let one = (1, "nop".to_string());
            let two = (2, "abc".to_string());
            let three = (3, "xyz".to_string());
            [three.clone(), two.clone(), one.clone()].iter().for_each(|(id, name)| testcase.push((*id, name.clone())));
            assert_eq!(testcase, [three.clone(), two.clone(), one.clone()]);
            assert_eq!(testcase.capacity(), 10);
            vsort_by(&mut testcase, |(_, lhs), (_, rhs)| lhs.cmp(rhs));
            assert_eq!(testcase, [two, one, three]);
            assert_eq!(testcase.capacity(), 3);
        }
        #[test]
        fn folder_file_matches_fn() {
            let filename = "file.dat";
            let file_mds = vec![
                filemd!((1, 1), filename, 256),
                filemd!((2, 2), filename, 8196),
                filemd!((3, 3), filename, 256),
                filemd!((4, 4), filename, 8196),
                filemd!((5, 5), filename, 0),
            ];
            let files_md: Vec<&FileMd> = file_mds.iter().map(|md| md).collect();
            let (matches, no_match) = folder_file_matches(files_md);
            assert_eq!(matches.len(), 2);
            for match_group in matches {
                assert_eq!(match_group.len(), 2);
                if match_group[0].size == 256 {
                    assert_eq!(FolderGroupId::from(match_group), FolderGroupId::new(vec![1, 3]));
                } else {
                    assert_eq!(FolderGroupId::from(match_group), FolderGroupId::new(vec![2, 4]));
                }
            }
            assert_eq!(no_match.len(), 1);
            assert_eq!(no_match[0].id, 5);
        }
        #[test]
        fn folders_file_matches_fn() {
            let filename = "fname";
            let folders_md = vec![
                folder_md(1, "/folder/one", vec![filemd!(11, filename, 100)]),
                folder_md(2, "/folder/two", vec![filemd!(21, filename, 1024)]),
                folder_md(3, "/folder/three", vec![filemd!(31, filename, 100)]),
                folder_md(4, "/folder/four", vec![filemd!(41, filename, 1024)]),
                folder_md(5, "/folder/five", vec![filemd!(51, filename, 512)]),
            ];
            let testcase: Vec<&FolderMd> = folders_md.iter().map(|md| md).collect();
            let (mut matches, no_matches) = super::analyze_folders_file(&testcase, filename);
            matches.sort();
            assert_eq!(matches.len(), 2);
            assert_eq!(matches[0], FolderGroupId::new(vec![1, 3]));
            assert_eq!(matches[1], FolderGroupId::new(vec![2, 4]));
            assert_eq!(no_matches.len(), 1);
            assert_eq!(no_matches[0], 5);
        }
        #[test]
        fn analyze_folders_files_fn() {
            let match1 = "match1";
            let match2 = "match2";
            let match3 = "match3";
            let no_match = "no_match";
            let duplicate_folders = vec![
                folder_md(
                    1,
                    "/folder/one",
                    vec![
                        filemd!(11, match1, 100),
                        filemd!(12, match2, 200),
                        filemd!(13, match3, 100),
                        filemd!(14, no_match, 1),
                    ],
                ),
                folder_md(
                    2,
                    "/folder/two",
                    vec![
                        filemd!(21, match1, 10),
                        filemd!(22, match2, 200),
                        filemd!(23, match3, 10),
                        filemd!(24, no_match, 2),
                    ],
                ),
                folder_md(
                    3,
                    "/folder/three",
                    vec![
                        filemd!(31, match1, 100),
                        filemd!(32, match2, 200),
                        filemd!(33, match3, 100),
                        filemd!(34, no_match, 3),
                    ],
                ),
                folder_md(
                    4,
                    "/folder/four",
                    vec![
                        filemd!(41, match1, 10),
                        filemd!(42, match2, 200),
                        filemd!(43, match3, 10),
                        filemd!(44, no_match, 4),
                    ],
                ),
            ];
            let folders_md: Vec<&FolderMd> = duplicate_folders.iter().map(|md| md).collect();
            let filenames = vec![match1.to_string(), match2.to_string(), match3.to_string(), no_match.to_string()];
            let mut folder_analysis = analyze_folders_files(folders_md, &filenames);
            assert_eq!(folder_analysis.matches.len(), 3);
            folder_analysis.matches.sort_by(|(lhs, _), (rhs, _)| lhs.cmp(&rhs));
            let testcase = vec![
                (FolderGroupId::new(vec![1, 2, 3, 4]), vec![match2]),
                (FolderGroupId::new(vec![1, 3]), vec![match1, match3]),
                (FolderGroupId::new(vec![2, 4]), vec![match1, match3]),
            ];
            for (i, (fgid, fnames)) in testcase.iter().enumerate() {
                assert_eq!(folder_analysis.matches[i].0, *fgid);
                assert_eq!(folder_analysis.matches[i].1, *fnames);
            }
            assert_eq!(folder_analysis.no_matches.len(), 4);
            folder_analysis.no_matches.sort_by(|(lhs, _), (rhs, _)| lhs.cmp(rhs));
            for (i, id) in [1, 2, 3, 4].iter().enumerate() {
                assert_eq!(folder_analysis.no_matches[i].0, *id);
                assert_eq!(folder_analysis.no_matches[i].1, vec![no_match]);
            }
        }
        #[test]
        fn validate_duplicate_ids() {
            let filename = "a_file";
            let mut builder = duplicate_folders_builder(vec![
                folder_md(1, "folder1", vec![filemd!(1, filename)]),
                folder_md(2, "folder2", vec![filemd!(2, filename)]),
            ]);
            assert!(builder.validate_duplicate_ids(&duplicate_ids(filename, vec![(1, 1), (2, 2)])));
            assert!(builder.errors.is_empty());
            assert!(!builder.validate_duplicate_ids(&duplicate_ids(filename, vec![(3, 1), (2, 2)])));
            assert!(builder.errors.len() == 1);
            builder.errors.clear();
            assert!(!builder.validate_duplicate_ids(&duplicate_ids(filename, vec![(1, 1), (2, 3)])));
            assert!(builder.errors.len() == 1);
            builder.errors.clear();
            // make sure the filename is a child of the folder
            builder.add_folder_md(folder_md(3, "folder3", vec![filemd!(3, "file")]));
            assert!(!builder.validate_duplicate_ids(&duplicate_ids(filename, vec![(1, 1), (3, 3),])));
            assert!(builder.errors.len() == 1);
        }
    }
}
