use std::{ffi::OsString, num::NonZeroUsize, path::PathBuf};

use lru::LruCache;
use windows::Win32::Foundation::HANDLE;

use crate::utils;

pub struct PathResolver {
    volume_handle: HANDLE,
    drive_letter: char,
    fid_path_cache: LruCache<u64, PathBuf>,
}

impl PathResolver {
    pub fn new(volume_handle: HANDLE, drive_letter: char) -> Self {
        let fid_path_cache = LruCache::new(NonZeroUsize::new(4 * 1024).unwrap());
        PathResolver {
            volume_handle,
            drive_letter,
            fid_path_cache,
        }
    }

    pub fn resolve_path(
        &mut self,
        fid: u64,
        parent_fid: u64,
        file_name: &OsString,
    ) -> Option<PathBuf> {
        if let Some(path) = self.fid_path_cache.get(&fid) {
            return Some(path.clone());
        }

        if let Some(parent_path) = self.fid_path_cache.get(&parent_fid) {
            let path = parent_path.join(file_name);
            self.fid_path_cache.put(fid, path.clone());
            return Some(path);
        }

        // If not in cache, try to get parent path from file system
        if let Ok(parent_path) =
            utils::file_id_to_path(self.volume_handle, self.drive_letter, parent_fid)
        {
            let path = parent_path.join(file_name);
            self.fid_path_cache.put(parent_fid, parent_path);
            self.fid_path_cache.put(fid, path.clone());
            return Some(path);
        }

        None
    }
}
