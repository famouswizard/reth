use std::{
    fs::File,
    io::{Read, Write},
    ops::RangeInclusive,
    path::{Path, PathBuf},
};

use eyre::OptionExt;
use reth_tracing::tracing::debug;
use tracing::instrument;

use super::entry::WalEntry;

/// The underlying WAL storage backed by a directory of files.
///
/// Each notification is represented by a single file that contains a MessagePack-encoded
/// [`WalEntry`] struct.
#[derive(Debug)]
pub struct Storage {
    /// The path to the WAL file.
    path: PathBuf,
}

impl Storage {
    /// Creates a new instance of [`Storage`] backed by the file at the given path and creates
    /// it doesn't exist.
    pub(super) fn new(path: impl AsRef<Path>) -> eyre::Result<Self> {
        reth_fs_util::create_dir_all(&path)?;

        Ok(Self { path: path.as_ref().to_path_buf() })
    }

    fn file_path(&self, id: u64) -> PathBuf {
        self.path.join(format!("{id}.wal"))
    }

    fn parse_filename(filename: &str) -> eyre::Result<u64> {
        filename
            .strip_suffix(".wal")
            .and_then(|s| s.parse().ok())
            .ok_or_eyre(format!("failed to parse file name: {filename}"))
    }

    /// Removes entry for the given file ID from the storage.
    #[instrument(target = "exex::wal::storage", skip(self))]
    pub(super) fn remove_entry(&self, file_id: u64) -> eyre::Result<()> {
        if let Err(err) = reth_fs_util::remove_file(self.file_path(file_id)) {
            debug!(?err, "Failed to remove entry from the storage");
            return Err(err.into())
        }

        debug!("Entry was removed from the storage");
        Ok(())
    }

    /// Returns the range of file IDs in the storage.
    ///
    /// If there are no files in the storage, returns `None`.
    pub(super) fn files_range(&self) -> eyre::Result<Option<RangeInclusive<u64>>> {
        let mut min_id = None;
        let mut max_id = None;

        for entry in reth_fs_util::read_dir(&self.path)? {
            let entry = entry?;
            let file_name = entry.file_name();
            let file_id = Self::parse_filename(&file_name.to_string_lossy())?;

            min_id = min_id.map_or(Some(file_id), |min_id: u64| Some(min_id.min(file_id)));
            max_id = max_id.map_or(Some(file_id), |max_id: u64| Some(max_id.max(file_id)));
        }

        Ok(min_id.zip(max_id).map(|(min_id, max_id)| min_id..=max_id))
    }

    /// Removes entries from the storage according to the given file range.
    ///
    /// # Returns
    ///
    /// Number of removed entries.
    pub(super) fn remove_entries(&self, range: RangeInclusive<u64>) -> eyre::Result<usize> {
        for id in range.clone() {
            self.remove_entry(id)?;
        }

        Ok(range.count())
    }

    /// Removes entries from the storage according to the given range.
    ///
    /// # Returns
    ///
    /// Entries that were removed.
    pub(super) fn take_entries(&self, range: RangeInclusive<u64>) -> eyre::Result<Vec<WalEntry>> {
        let entries = self.entries(range).collect::<eyre::Result<Vec<_>>>()?;

        for (id, _) in &entries {
            self.remove_entry(*id)?;
        }

        Ok(entries.into_iter().map(|(_, entry)| entry).collect())
    }

    pub(super) fn entries(
        &self,
        range: RangeInclusive<u64>,
    ) -> impl DoubleEndedIterator<Item = eyre::Result<(u64, WalEntry)>> + '_ {
        range.map(move |id| self.read_entry(id).map(|entry| (id, entry)))
    }

    /// Reads the entry from the file with the given id.
    #[instrument(target = "exex::wal::storage", skip(self))]
    pub(super) fn read_entry(&self, file_id: u64) -> eyre::Result<WalEntry> {
        let file_path = self.file_path(file_id);
        debug!(?file_path, "Reading entry from WAL");

        let mut file = File::open(&file_path)?;
        read_entry(&mut file)
    }

    /// Writes the entry to the file with the given id.
    #[instrument(target = "exex::wal::storage", skip(self, entry))]
    pub(super) fn write_entry(&self, file_id: u64, entry: WalEntry) -> eyre::Result<()> {
        let file_path = self.file_path(file_id);
        debug!(?file_path, "Writing entry to WAL");

        let mut file = File::create_new(&file_path)?;
        write_entry(&mut file, &entry)?;

        Ok(())
    }
}

// TODO(alexey): use rmp-serde when Alloy and Reth serde issues are resolved

fn write_entry(mut w: &mut impl Write, entry: &WalEntry) -> eyre::Result<()> {
    // rmp_serde::encode::write(w, entry)?;
    serde_json::to_writer(&mut w, entry)?;
    w.flush()?;
    Ok(())
}

fn read_entry(r: &mut impl Read) -> eyre::Result<WalEntry> {
    // Ok(rmp_serde::from_read(r)?)
    Ok(serde_json::from_reader(r)?)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use eyre::OptionExt;
    use reth_exex_types::ExExNotification;
    use reth_provider::Chain;
    use reth_testing_utils::generators::{self, random_block};

    use crate::{wal::entry::WalEntry, NotificationCommitTarget};

    use super::Storage;

    #[test]
    fn test_roundtrip() -> eyre::Result<()> {
        let mut rng = generators::rng();

        let temp_dir = tempfile::tempdir()?;
        let storage = Storage::new(&temp_dir)?;

        let old_block = random_block(&mut rng, 0, Default::default())
            .seal_with_senders()
            .ok_or_eyre("failed to recover senders")?;
        let new_block = random_block(&mut rng, 0, Default::default())
            .seal_with_senders()
            .ok_or_eyre("failed to recover senders")?;

        let notification = ExExNotification::ChainReorged {
            new: Arc::new(Chain::new(vec![new_block], Default::default(), None)),
            old: Arc::new(Chain::new(vec![old_block], Default::default(), None)),
        };
        let entry = WalEntry { target: NotificationCommitTarget::Commit, notification };

        // Do a round trip serialization and deserialization
        let file_id = 0;
        storage.write_entry(file_id, entry.clone())?;
        let deserialized_entry = storage.read_entry(file_id)?;
        assert_eq!(deserialized_entry, entry);

        Ok(())
    }
}