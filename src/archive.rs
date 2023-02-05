//! Manages the zip component part of the epub doc.
//!
//! Provides easy methods to navigate througth the epub parts and to get
//! the content as string.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use crate::error::{ArchiveError, Result};
use std::io::{Read, Seek};

/// Epub archive struct. Here it's stored the file path and the list of
/// files in the zip archive.
pub struct EpubArchive<R: Read + Seek> {
    zip: zip::ZipArchive<R>,
}

impl EpubArchive<BufReader<File>> {
    /// Opens the epub file in `path`.
    ///
    /// # Errors
    ///
    /// Returns an error if the zip is broken or if the file doesn't
    /// exists.
    pub fn new(path: impl AsRef<Path>) -> Result<EpubArchive<BufReader<File>>> {
        let file = File::open(path)?;
        let archive = EpubArchive::from_reader(BufReader::new(file))?;

        Ok(archive)
    }
}

impl<R: Read + Seek> EpubArchive<R> {
    /// Opens the epub contained in `reader`.
    ///
    /// # Errors
    ///
    /// Returns an error if the zip is broken.
    pub fn from_reader(reader: R) -> Result<Self> {
        let zip = zip::ZipArchive::new(reader)?;

        Ok(EpubArchive { zip })
    }

    /// Returns the content of the file by the `name` as `Vec<u8>`.
    ///
    /// # Errors
    ///
    /// Returns an error if the name doesn't exists in the zip archive.
    pub fn get_entry(&mut self, name: impl AsRef<Path>) -> Result<Vec<u8>> {
        let mut entry = Vec::new();
        let path = name.as_ref();
        let name = path.to_string_lossy();

        match self.zip.by_name(&name) {
            Ok(mut zipfile) => {
                zipfile.read_to_end(&mut entry)?;
                return Ok(entry);
            }
            Err(zip::result::ZipError::FileNotFound) => {}
            Err(e) => {
                return Err(e.into());
            }
        };

        // try percent encoding
        let name = percent_encoding::percent_decode(name.as_bytes()).decode_utf8()?;
        let mut zipfile = self.zip.by_name(&name)?;
        zipfile.read_to_end(&mut entry)?;
        Ok(entry)
    }

    /// Returns the content of the file by the `name` as `String`.
    ///
    /// # Errors
    ///
    /// Returns an error if the name doesn't exists in the zip archive.
    pub fn get_entry_as_str(&mut self, name: impl AsRef<Path>) -> Result<String> {
        let content = self.get_entry(name)?;
        String::from_utf8(content).map_err(ArchiveError::from)
    }

    /// Returns the content of container file "META-INF/container.xml".
    ///
    /// # Errors
    ///
    /// Returns an error if the epub doesn't have the container file.
    pub fn get_container_file(&mut self) -> Result<Vec<u8>> {
        self.get_entry("META-INF/container.xml")
    }
}
