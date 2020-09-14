use super::{Entry, EntryHandle};
use crate::handle::{Fdflags, Filestat, Fstflags, HandleRights, Lookupflags, Oflags, Rights, Size};
use crate::sched::Timestamp;
use crate::Result;
use std::convert::TryInto;
use tracing::trace;

impl Entry {
    pub fn path_create_directory(&self, path: &str) -> Result<()> {
        let required_rights =
            HandleRights::from_base(Rights::PATH_OPEN | Rights::PATH_CREATE_DIRECTORY);
        let (dirfd, path) =
            crate::path::get(&self, &required_rights, Lookupflags::empty(), path, false)?;
        dirfd.create_directory(&path)
    }

    pub fn path_filestat_get(&self, flags: Lookupflags, path: &str) -> Result<Filestat> {
        let required_rights = HandleRights::from_base(Rights::PATH_FILESTAT_GET);
        let (dirfd, path) = crate::path::get(&self, &required_rights, flags, path, false)?;
        let host_filestat =
            dirfd.filestat_get_at(&path, flags.contains(&Lookupflags::SYMLINK_FOLLOW))?;
        Ok(host_filestat)
    }

    pub fn path_filestat_set_times(
        &self,
        flags: Lookupflags,
        path: &str,
        atim: Timestamp,
        mtim: Timestamp,
        fst_flags: Fstflags,
    ) -> Result<()> {
        let required_rights = HandleRights::from_base(Rights::PATH_FILESTAT_SET_TIMES);
        let (dirfd, path) = crate::path::get(&self, &required_rights, flags, path, false)?;
        dirfd.filestat_set_times_at(
            &path,
            atim,
            mtim,
            fst_flags,
            flags.contains(&Lookupflags::SYMLINK_FOLLOW),
        )?;
        Ok(())
    }

    pub fn path_link(
        &self,
        old_flags: Lookupflags,
        old_path: &str,
        new_entry: &Entry,
        new_path: &str,
    ) -> Result<()> {
        let required_rights = HandleRights::from_base(Rights::PATH_LINK_SOURCE);
        let (old_dirfd, old_path) = crate::path::get(
            &self,
            &required_rights,
            Lookupflags::empty(),
            old_path,
            false,
        )?;
        let required_rights = HandleRights::from_base(Rights::PATH_LINK_TARGET);
        let (new_dirfd, new_path) = crate::path::get(
            &new_entry,
            &required_rights,
            Lookupflags::empty(),
            new_path,
            false,
        )?;
        old_dirfd.link(
            &old_path,
            new_dirfd,
            &new_path,
            old_flags.contains(&Lookupflags::SYMLINK_FOLLOW),
        )
    }

    pub fn path_open(
        &self,
        dirflags: Lookupflags,
        path: &str,
        oflags: Oflags,
        fs_rights_base: Rights,
        fs_rights_inheriting: Rights,
        fdflags: Fdflags,
    ) -> Result<Entry> {
        let needed_rights = crate::path::open_rights(
            &HandleRights::new(fs_rights_base, fs_rights_inheriting),
            oflags,
            fdflags,
        );
        trace!(needed_rights = tracing::field::debug(&needed_rights));
        let (dirfd, path) = crate::path::get(
            &self,
            &needed_rights,
            dirflags,
            path,
            oflags & Oflags::CREAT != Oflags::empty(),
        )?;
        let read = fs_rights_base & (Rights::FD_READ | Rights::FD_READDIR) != Rights::empty();
        let write = fs_rights_base
            & (Rights::FD_DATASYNC
                | Rights::FD_WRITE
                | Rights::FD_ALLOCATE
                | Rights::FD_FILESTAT_SET_SIZE)
            != Rights::empty();
        trace!(read = read, write = write, "dirfd.openat");
        let fd = dirfd.openat(&path, read, write, oflags, fdflags)?;
        let entry = Entry::new(EntryHandle::from(fd));

        // We need to manually deny the rights which are not explicitly requested
        // because Entry::from will assign maximal consistent rights.
        let mut rights = entry.get_rights();
        rights.base &= fs_rights_base;
        rights.inheriting &= fs_rights_inheriting;
        entry.set_rights(rights);
        Ok(entry)
    }

    pub fn path_readlink(&self, path: &str, buf: &mut [u8]) -> Result<Size> {
        let required_rights = HandleRights::from_base(Rights::PATH_READLINK);
        let (dirfd, path) =
            crate::path::get(&self, &required_rights, Lookupflags::empty(), path, false)?;
        let host_bufused = dirfd.readlink(&path, buf)?.try_into()?;
        Ok(host_bufused)
    }

    pub fn path_remove_directory(&self, path: &str) -> Result<()> {
        let required_rights = HandleRights::from_base(Rights::PATH_REMOVE_DIRECTORY);
        let (dirfd, path) =
            crate::path::get(&self, &required_rights, Lookupflags::empty(), path, true)?;
        dirfd.remove_directory(&path)
    }

    pub fn path_rename(&self, old_path: &str, new_entry: &Entry, new_path: &str) -> Result<()> {
        let required_rights = HandleRights::from_base(Rights::PATH_RENAME_SOURCE);
        let (old_dirfd, old_path) = crate::path::get(
            &self,
            &required_rights,
            Lookupflags::empty(),
            old_path,
            true,
        )?;
        let (new_dirfd, new_path) = crate::path::get(
            &new_entry,
            &required_rights,
            Lookupflags::empty(),
            new_path,
            true,
        )?;
        old_dirfd.rename(&old_path, new_dirfd, &new_path)
    }
}