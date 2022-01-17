//! Parsers for the different Epub versions.
//!
//! Whilst slightly inefficient, all the parsers before and including to the specified version are ran.
//! This ensures the maximum amount of compatibility, whilst also ensuring that modified parts of the spec can be implemented
//! without compatibility crud.

use crate::archive::EpubArchive;
use crate::doc::{MetadataNode, NavPoint, ResourceItem};
use crate::xmlutils::{XMLError, XMLNode};
use crate::{doc, utils, xmlutils};
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{Read, Seek};
use std::path::Path;

type RootXml = RefCell<XMLNode>;

pub(crate) mod v2;
pub(crate) mod v3;

pub trait EpubParser {
    /// Parse the root xml `content.opf`.
    ///
    /// Optionally make use of the provided `archive` for additional files which were referred to by the `content.opf`.
    ///
    /// Modifications will be stored in the `epub` object.
    fn parse<R: Read + Seek, PATH: AsRef<Path>>(
        epub: &mut EpubMetadata,
        root_base: PATH,
        xml: &RootXml,
        archive: &mut EpubArchive<R>,
    ) -> anyhow::Result<()>;
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EpubMetadata {
    /// epub spine ids
    pub spine: Vec<String>,

    /// resource id -> (path, mime)
    pub resources: HashMap<String, ResourceItem>,

    /// table of content, list of `NavPoint` in the toc.ncx
    pub toc: Vec<NavPoint>,

    /// The epub metadata stored as key -> value
    ///
    /// #Examples
    ///
    /// ```
    /// # use epub::doc::{EpubDoc, MetadataNode};
    /// # let doc = EpubDoc::new("test.epub");
    /// # let doc = doc.unwrap();
    /// let title = doc.context.metadata.get("title");
    ///
    /// assert_eq!(title.unwrap(), &vec![MetadataNode::from_content("Todo es mío".to_string())]);
    /// ```
    pub metadata: HashMap<String, Vec<MetadataNode>>,

    /// unique identifier
    pub unique_identifier: Option<String>,
}

impl EpubMetadata {
    pub(crate) fn insert_resource(
        &mut self,
        root_base: impl AsRef<Path>,
        item: &xmlutils::XMLNode,
    ) -> Result<(), XMLError> {
        let id = item.get_attr("id")?;
        let href = item.get_attr("href")?;
        let mtype = item.get_attr("media-type")?;
        let path = utils::convert_path_separators(root_base, &href);
        self.resources.insert(
            id.to_string(),
            ResourceItem {
                path,
                mime: mtype.to_string(),
                property: item.get_attr("properties").ok().map(Into::into),
            },
        );
        Ok(())
    }
}
