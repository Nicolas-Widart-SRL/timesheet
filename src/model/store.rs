//! The JSON data file, its backups and the signed PDF copies kept next to it.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::{Data, Month};

/// Portable copy of everything the app keeps: the data file fields plus the signed PDFs,
/// base64-encoded and keyed by [`Month::key`]. A plain data file is a valid backup without signed PDFs.
#[derive(Serialize, Deserialize)]
struct Backup {
    #[serde(flatten)]
    data: Data,
    #[serde(default)]
    signed_pdfs: BTreeMap<String, String>,
}

/// JSON file holding all data, written atomically after every change.
pub struct Store {
    path: PathBuf,
    pub data: Data,
    /// Set when an existing file could not be read nor moved aside: saving would overwrite it.
    save_blocked: bool,
}

impl Store {
    /// Loads the store. An unparsable file is moved aside (never overwritten) and a warning is returned.
    /// A file that can be neither read nor moved aside is left untouched: saving is blocked for this session.
    pub fn open(path: PathBuf) -> (Self, Option<String>) {
        let mut warning = None;
        let mut save_blocked = false;
        let data = match fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice(&bytes) {
                Ok(data) => data,
                Err(err) => {
                    let backup = path.with_extension(format!(
                        "corrupt-{}.json",
                        chrono::Local::now().format("%Y%m%d-%H%M%S")
                    ));
                    warning = Some(match fs::rename(&path, &backup) {
                        Ok(()) => format!(
                            "Saved data could not be read ({err}). It was moved to {}.",
                            backup.display()
                        ),
                        Err(e) => {
                            save_blocked = true;
                            format!(
                                "Saved data could not be read ({err}) nor moved aside ({e}). Changes will not be saved."
                            )
                        }
                    });
                    Data::default()
                }
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => Data::default(),
            Err(e) => {
                save_blocked = true;
                warning = Some(format!(
                    "Could not read {}: {e}. Changes will not be saved.",
                    path.display()
                ));
                Data::default()
            }
        };
        (
            Store {
                path,
                data,
                save_blocked,
            },
            warning,
        )
    }

    /// Where the signed PDF of `month` is kept, next to the data file.
    pub fn signed_pdf_path(&self, month: Month) -> PathBuf {
        self.path
            .with_file_name("signed")
            .join(format!("{}.pdf", month.key()))
    }

    /// Copies `source` into the data folder as the signed PDF of `month`.
    /// A signed copy means the month was sent and approved, so missing dates are set to `today`.
    pub fn attach_signed_pdf(
        &mut self,
        month: Month,
        source: &Path,
        today: NaiveDate,
    ) -> io::Result<()> {
        let target = self.signed_pdf_path(month);
        if let Some(dir) = target.parent() {
            fs::create_dir_all(dir)?;
        }
        write_atomic(&target, &fs::read(source)?)?;
        let approval = self.data.approval_mut(month);
        approval.signed_pdf = Some(
            source
                .file_name()
                .map_or_else(|| "signed.pdf".into(), |n| n.to_string_lossy().into_owned()),
        );
        approval.set_sent(true, today);
        approval.approved.get_or_insert(today);
        Ok(())
    }

    pub fn remove_signed_pdf(&mut self, month: Month) -> io::Result<()> {
        match fs::remove_file(self.signed_pdf_path(month)) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
            _ => {}
        }
        self.data.approval_mut(month).signed_pdf = None;
        Ok(())
    }

    /// All data and signed PDFs as one JSON file. A signed copy missing on disk is left out.
    pub fn export_backup(&self) -> io::Result<Vec<u8>> {
        let mut signed_pdfs = BTreeMap::new();
        for (key, approval) in &self.data.approvals {
            let Some(month) = Month::from_key(key).filter(|_| approval.signed_pdf.is_some()) else {
                continue;
            };
            match fs::read(self.signed_pdf_path(month)) {
                Ok(bytes) => {
                    signed_pdfs.insert(key.clone(), BASE64.encode(bytes));
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
        }
        serde_json::to_vec_pretty(&Backup {
            data: self.data.clone(),
            signed_pdfs,
        })
        .map_err(io::Error::other)
    }

    /// Replaces all data and signed PDFs with a backup from [`Store::export_backup`].
    /// The backup is fully checked before anything is written. Unless the store is empty, the current
    /// data is first written to a backup next to the data file, whose path is returned.
    pub fn import_backup(
        &mut self,
        bytes: &[u8],
        now: chrono::NaiveDateTime,
    ) -> io::Result<Option<PathBuf>> {
        let invalid = |msg: String| io::Error::new(io::ErrorKind::InvalidData, msg);
        if self.save_blocked {
            return Err(io::Error::other(
                "the existing data file could not be read, so it is left untouched",
            ));
        }
        let backup: Backup = serde_json::from_slice(bytes)
            .map_err(|e| invalid(format!("not a timesheet backup ({e})")))?;
        let mut ids = HashSet::new();
        if let Some(e) = backup.data.entries.iter().find(|e| !ids.insert(e.id)) {
            return Err(invalid(format!("two lines share the id {}", e.id)));
        }
        let mut pdfs = Vec::new();
        for (key, encoded) in &backup.signed_pdfs {
            let month =
                Month::from_key(key).ok_or_else(|| invalid(format!("invalid month \"{key}\"")))?;
            let pdf = BASE64
                .decode(encoded)
                .map_err(|e| invalid(format!("signed PDF of {key} is damaged ({e})")))?;
            pdfs.push((month, pdf));
        }

        let previous = if self.data == Data::default() {
            None
        } else {
            let previous = self.path.with_extension(format!(
                "before-import-{}.json",
                now.format("%Y%m%d-%H%M%S")
            ));
            if let Some(dir) = previous.parent() {
                fs::create_dir_all(dir)?;
            }
            write_atomic(&previous, &self.export_backup()?)?;
            Some(previous)
        };
        for (month, pdf) in &pdfs {
            let target = self.signed_pdf_path(*month);
            if let Some(dir) = target.parent() {
                fs::create_dir_all(dir)?;
            }
            write_atomic(&target, pdf)?;
        }
        let mut data = backup.data;
        for (key, approval) in &mut data.approvals {
            if Month::from_key(key).is_none_or(|m| !self.signed_pdf_path(m).exists()) {
                approval.signed_pdf = None;
            }
        }
        self.data = data;
        self.save()?;
        Ok(previous)
    }

    pub fn save(&self) -> io::Result<()> {
        if self.save_blocked {
            return Err(io::Error::other(
                "the existing data file could not be read, so it is left untouched",
            ));
        }
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_vec_pretty(&self.data).map_err(io::Error::other)?;
        write_atomic(&self.path, &json)
    }
}

/// Writes to a sibling temp file, flushes it to disk, then renames it over the target
/// so a crash never leaves a half-written file.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    let mut file = fs::File::create(&tmp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::holidays::Country;
    use crate::model::{Appearance, DayOff};

    fn d(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
    }

    #[test]
    fn signed_pdf_is_copied_and_removed() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("my-signed.pdf");
        fs::write(&source, b"%PDF").unwrap();
        let (mut store, _) = Store::open(dir.path().join("data").join("timesheet.json"));
        let sep = Month {
            year: 2026,
            month: 9,
        };
        store.attach_signed_pdf(sep, &source, d(8)).unwrap();
        let copy = store.signed_pdf_path(sep);
        assert_eq!(fs::read(&copy).unwrap(), b"%PDF");
        let a = store.data.approval(sep);
        assert_eq!(
            (a.sent, a.approved, a.signed_pdf.as_deref()),
            (Some(d(8)), Some(d(8)), Some("my-signed.pdf"))
        );
        store.remove_signed_pdf(sep).unwrap();
        assert!(!copy.exists() && source.exists());
        assert_eq!(store.data.approval(sep).signed_pdf, None);
    }

    #[test]
    fn appearance_is_saved() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("timesheet.json");
        let (mut store, _) = Store::open(path.clone());
        store.data.settings.appearance = Appearance::Dark;
        store.save().unwrap();
        assert!(
            fs::read_to_string(&path)
                .unwrap()
                .contains(r#""appearance": "dark""#)
        );
        assert_eq!(
            Store::open(path).0.data.settings.appearance,
            Appearance::Dark
        );
    }

    #[test]
    fn store_roundtrip_and_corrupt_file_is_kept() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("timesheet.json");
        let (mut store, warning) = Store::open(path.clone());
        assert!(warning.is_none());
        store.data.add_line(d(1));
        store.save().unwrap();
        assert_eq!(Store::open(path.clone()).0.data, store.data);

        fs::write(&path, b"{not json").unwrap();
        let (store, warning) = Store::open(path.clone());
        assert!(warning.is_some());
        assert!(store.data.entries.is_empty());
        let moved = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .any(|e| e.file_name().to_string_lossy().contains("corrupt"));
        assert!(moved);
    }

    #[test]
    fn backup_moves_data_and_signed_pdfs_to_another_store() {
        let now = d(9).and_hms_opt(10, 0, 0).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("signed.pdf");
        fs::write(&source, b"%PDF-signed").unwrap();
        let (mut from, _) = Store::open(dir.path().join("a").join("timesheet.json"));
        from.data.settings.consultant = "Jane Doe".into();
        from.data.settings.country = Some(Country::Belgium);
        from.data.add_line(d(1));
        from.data.set_day_off(d(2), Some(DayOff::Unpaid));
        from.data
            .set_day_off(NaiveDate::from_ymd_opt(2026, 11, 11).unwrap(), None);
        let sep = Month {
            year: 2026,
            month: 9,
        };
        from.attach_signed_pdf(sep, &source, d(8)).unwrap();
        let backup = from.export_backup().unwrap();

        let (mut to, _) = Store::open(dir.path().join("b").join("timesheet.json"));
        assert_eq!(
            to.import_backup(&backup, now).unwrap(),
            None,
            "an empty store needs no backup"
        );
        assert_eq!(to.data, from.data);
        assert_eq!(to.data.days_off.len(), 2);
        assert_eq!(fs::read(to.signed_pdf_path(sep)).unwrap(), b"%PDF-signed");
        assert_eq!(
            Store::open(dir.path().join("b").join("timesheet.json"))
                .0
                .data,
            from.data,
            "import is saved"
        );

        // Importing again keeps the replaced data in a backup that can itself be imported.
        to.data.add_line(d(2));
        let replaced = to.data.clone();
        let previous = to
            .import_backup(&backup, now)
            .unwrap()
            .expect("current data is backed up");
        assert_eq!(to.data, from.data);
        to.import_backup(&fs::read(previous).unwrap(), now).unwrap();
        assert_eq!(to.data, replaced);
    }

    #[test]
    fn plain_data_file_imports_without_dangling_signed_pdf() {
        let dir = tempfile::tempdir().unwrap();
        let mut data = Data::default();
        data.add_line(d(1));
        data.approval_mut(Month {
            year: 2026,
            month: 9,
        })
        .signed_pdf = Some("lost.pdf".into());
        let (mut store, _) = Store::open(dir.path().join("timesheet.json"));
        store
            .import_backup(&serde_json::to_vec(&data).unwrap(), d(9).into())
            .unwrap();
        assert_eq!(store.data.entries, data.entries);
        assert_eq!(
            store
                .data
                .approval(Month {
                    year: 2026,
                    month: 9
                })
                .signed_pdf,
            None
        );
    }

    #[test]
    fn invalid_backup_changes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("timesheet.json");
        let (mut store, _) = Store::open(path.clone());
        store.data.add_line(d(1));
        store.save().unwrap();
        let before = store.data.clone();
        let line = r#"{"id":0,"date":"2026-10-01","kind":"backend","desc":"","hours":"8"}"#;
        let settings = r#""settings":{"consultant":"","client":"","project":""}"#;
        for bad in [
            "{not json".to_owned(),
            format!(r#"{{{settings},"entries":[{line},{line}],"next_id":1}}"#),
            format!(r#"{{{settings},"entries":[],"next_id":0,"signed_pdfs":{{"../evil":""}}}}"#),
            format!(r#"{{{settings},"entries":[],"next_id":0,"signed_pdfs":{{"2026-09":"%%%"}}}}"#),
        ] {
            assert!(
                store.import_backup(bad.as_bytes(), d(9).into()).is_err(),
                "{bad}"
            );
            assert_eq!(store.data, before);
        }
        assert_eq!(
            fs::read_dir(dir.path()).unwrap().count(),
            1,
            "nothing is written"
        );
    }

    #[test]
    fn unreadable_file_is_never_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        // A directory in place of the file cannot be read.
        let path = dir.path().join("timesheet.json");
        fs::create_dir(&path).unwrap();
        let (mut store, warning) = Store::open(path.clone());
        assert!(warning.is_some());
        store.data.add_line(d(1));
        assert!(store.save().is_err());
        assert!(path.is_dir());
        assert!(
            !dir.path().join("timesheet.tmp").exists(),
            "nothing is written"
        );
    }
}
