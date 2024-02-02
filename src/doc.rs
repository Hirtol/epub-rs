//! Manages the epub doc.
//!
//! Provides easy methods to navigate through the epub content, cover,
//! chapters, etc.

use crate::archive::EpubArchive;
use crate::error::{ArchiveError, Result};
use crate::parsers::{EpubMetadata, EpubParser};
use roxmltree::StringStorage;
use std::borrow::Cow;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::io::{Read, Seek};
use std::path::{Component, Path, PathBuf};

use crate::parsers::v2::EpubV2Parser;
use crate::parsers::v3::EpubV3Parser;
use crate::xmlutils;
use crate::xmlutils::{OwnedAttribute, OwnedName, XMLError};

/// Struct that represent a navigation point in a table of content
#[derive(Debug, Eq, Clone)]
pub struct NavPoint {
    /// the title of this navpoint
    pub label: String,
    /// the resource path
    pub content: PathBuf,
    /// nested navpoints
    pub children: Vec<NavPoint>,
    /// the order in the toc
    pub play_order: usize,
}

impl Ord for NavPoint {
    fn cmp(&self, other: &NavPoint) -> Ordering {
        self.play_order.cmp(&other.play_order)
    }
}

impl PartialOrd for NavPoint {
    fn partial_cmp(&self, other: &NavPoint) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for NavPoint {
    fn eq(&self, other: &NavPoint) -> bool {
        self.play_order == other.play_order
    }
}

/// A resource item is any item that was listed in the `content.opf` as part of the manifest.
/// It is guaranteed to have a path within the Epub, and a mime type.
///
/// Optionally it can contain a property attribute, [see here](https://www.w3.org/publishing/epub3/epub-packages.html#sec-item-property-values).
/// Note that `cover-image` and `nav` properties are already handled in the Epub V3 parsing.
/// See [EpubDoc::get_cover] and [EpubDoc::get_toc] for more information.
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceItem {
    pub path: PathBuf,
    pub mime: String,
    pub property: Option<String>,
}

/// A Metadata Node represents a piece of metadata that is in the `content.opf` file of the Epub.
/// It contains its textual content, as well as any attributes that was on the XML node.
#[derive(Debug, Clone, PartialEq)]
pub struct MetadataNode {
    /// The textual content that was within the XML open and close tags
    pub content: String,
    /// The attributes of the XML node
    pub attr: Vec<OwnedAttribute>,
}

impl MetadataNode {
    pub fn from_content(content: impl Into<String>) -> MetadataNode {
        MetadataNode {
            content: content.into(),
            attr: Vec::new(),
        }
    }

    pub fn from_attr(content: impl Into<String>, node: &roxmltree::Node) -> MetadataNode {
        let attrs = node
            .attributes()
            .map(|attr| OwnedAttribute {
                name: OwnedName {
                    namespace: attr.namespace().map(|r| r.to_owned()),
                    tag: attr.name().to_owned(),
                },
                value: match attr.value_storage() {
                    StringStorage::Borrowed(val) => (*val).into(),
                    StringStorage::Owned(val) => val.clone(),
                },
            })
            .collect();
        MetadataNode {
            content: content.into(),
            attr: attrs,
        }
    }

    /// Find an attribute in the current node with the given `name`
    ///
    /// # Arguments
    ///
    /// * `name` - the name of the attribute to find
    pub fn find_attr(&self, name: &str) -> Option<&str> {
        self.attr
            .iter()
            .find(|a| a.name.tag == name)
            .map(|a| a.value.as_ref())
    }
}

/// Struct to control the epub document
pub struct EpubDoc<R: Read + Seek> {
    /// the zip archive
    archive: RefCell<EpubArchive<R>>,

    /// root file base path
    pub root_base: PathBuf,

    /// root file full path
    pub root_file: PathBuf,

    pub context: EpubMetadata,
}

impl EpubDoc<BufReader<File>> {
    /// Opens the epub file in `path`.
    ///
    /// Initialize some internal variables to be able to access to the epub
    /// spine definition and to navigate trhough the epub.
    ///
    /// # Examples
    ///
    /// ```
    /// use epub::doc::EpubDoc;
    ///
    /// let doc = EpubDoc::new("test.epub");
    /// assert!(doc.is_ok());
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the epub is broken or if the file doesn't
    /// exists.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, ArchiveError> {
        let path = path.as_ref();
        let file = File::open(path)?;
        let doc = EpubDoc::from_reader(BufReader::new(file))?;

        Ok(doc)
    }
}

impl<R: Read + Seek> EpubDoc<R> {
    /// Opens the epub contained in `reader`.
    ///
    /// Initialize some internal variables to be able to access to the epub
    /// spine definition and to navigate trhough the epub.
    ///
    /// # Examples
    ///
    /// ```
    /// use epub::doc::EpubDoc;
    /// use std::fs::File;
    /// use std::io::{Cursor, Read};
    ///
    /// let mut file = File::open("test.epub").unwrap();
    /// let mut buffer = Vec::new();
    /// file.read_to_end(&mut buffer).unwrap();
    ///
    /// let cursor = Cursor::new(buffer);
    ///
    /// let doc = EpubDoc::from_reader(cursor);
    /// assert!(doc.is_ok());
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the epub is broken.
    pub fn from_reader(reader: R) -> Result<Self> {
        let mut archive = EpubArchive::from_reader(reader)?;
        let resources = HashMap::new();

        let container = archive.get_container_file()?;
        let root_file = get_root_file(&container)?;
        let base_path = root_file.parent().expect("All files have a parent");

        let mut doc = EpubDoc {
            archive: RefCell::new(archive),
            root_base: base_path.to_path_buf(),
            root_file,
            context: EpubMetadata {
                spine: vec![],
                resources,
                toc: vec![],
                metadata: Default::default(),
                cover_id: None,
                unique_identifier: None,
            },
        };

        doc.fill_resources()?;

        Ok(doc)
    }

    /// Returns the content of the first metadata found with this name.
    ///
    /// #Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # let doc = EpubDoc::new("test.epub");
    /// # let doc = doc.unwrap();
    /// let title = doc.mdata("title");
    /// assert_eq!(title.unwrap(), "Todo es mío");
    pub fn mdata(&self, name: &str) -> Option<&str> {
        match self.context.metadata.get(name) {
            Some(v) => v.get(0).map(|m| m.content.as_str()),
            None => None,
        }
    }

    /// Returns the first full metadata found with this name.
    ///
    /// #Examples
    ///
    /// ```
    /// # use epub::doc::{EpubDoc, MetadataNode};
    /// # let doc = EpubDoc::new("test.epub");
    /// # let doc = doc.unwrap();
    /// let title = doc.mdata_full("title");
    /// assert_eq!(title.unwrap().content, "Todo es mío");
    pub fn mdata_full(&self, name: &str) -> Option<&MetadataNode> {
        match self.context.metadata.get(name) {
            Some(v) => v.get(0),
            None => None,
        }
    }

    /// Returns the id of the epub cover.
    ///
    /// The cover is searched in the doc metadata, by the tag <meta name="cover" value"..">
    ///
    /// # Examples
    ///
    /// ```rust
    /// use epub::doc::EpubDoc;
    ///
    /// let doc = EpubDoc::new("test.epub");
    /// assert!(doc.is_ok());
    /// let mut doc = doc.unwrap();
    ///
    /// let cover_id = doc.get_cover_id().unwrap();
    /// ```
    ///
    /// # Returns
    ///
    /// Returns `None` if the cover path can't be found.
    pub fn get_cover_id(&self) -> Option<&str> {
        self.context.cover_id.as_deref()
    }

    /// Returns the cover as Vec<u8>
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use std::fs;
    /// use std::io::Write;
    /// use epub::doc::EpubDoc;
    ///
    /// let doc = EpubDoc::new("test.epub");
    /// assert!(doc.is_ok());
    /// let mut doc = doc.unwrap();
    ///
    /// let cover_data = doc.get_cover().unwrap();
    ///
    /// let f = fs::File::create("/tmp/cover.png");
    /// assert!(f.is_ok());
    /// let mut f = f.unwrap();
    /// let resp = f.write_all(&cover_data);
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the cover can't be found.
    pub fn get_cover(&self) -> Option<Vec<u8>> {
        let cover_id = self.get_cover_id()?;

        let cover_data = self.get_resource(cover_id)?;
        Some(cover_data)
    }

    /// Returns the ToC as found in the Epub.
    ///
    /// Note that if no ToC was found this [Vec] will be empty
    pub fn get_toc(&self) -> &Vec<NavPoint> {
        &self.context.toc
    }

    /// Returns Release Identifier defined at
    /// https://www.w3.org/publishing/epub3/epub-packages.html#sec-metadata-elem-identifiers-pid
    pub fn get_release_identifier(&self) -> Option<String> {
        match (
            self.context.unique_identifier.as_ref(),
            self.mdata("dcterms:modified"),
        ) {
            (Some(unique_identifier), Some(modified)) => {
                Some(format!("{}@{}", unique_identifier, modified))
            }
            _ => None,
        }
    }

    /// Returns the resource content by full path in the epub archive
    ///
    /// # Errors
    ///
    /// Returns an error if the path doesn't exists in the epub
    pub fn get_resource_by_path<P: AsRef<Path>>(&self, path: P) -> Result<Vec<u8>> {
        let content = self.archive.borrow_mut().get_entry(path)?;
        Ok(content)
    }

    /// Returns the resource content by the id defined in the spine
    ///
    /// # Returns
    ///
    /// Returns `None` if the `id` doesn't exists in the epub
    pub fn get_resource(&self, id: &str) -> Option<Vec<u8>> {
        let res_item = self.context.resources.get(id)?;

        self.get_resource_by_path(&res_item.path).ok()
    }

    /// Returns the resource content by full path in the epub archive, as String
    ///
    /// # Returns
    ///
    /// Returns `None` if the path doesn't exists in the epub
    pub fn get_resource_str_by_path(&self, path: impl AsRef<Path>) -> Result<String, ArchiveError> {
        let content = self.archive.borrow_mut().get_entry_as_str(path)?;

        Ok(content)
    }

    /// Returns the resource content by the id defined in the spine, as String
    ///
    /// # Returns
    ///
    /// Returns `None` if the id doesn't exists in the epub
    pub fn get_resource_str(&self, id: &str) -> Option<String> {
        let res_item = self.context.resources.get(id)?;

        self.get_resource_str_by_path(&res_item.path).ok()
    }

    /// Returns the resource mime-type
    ///
    /// # Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # let doc = EpubDoc::new("test.epub");
    /// # let doc = doc.unwrap();
    /// let mime = doc.get_resource_mime("portada.png");
    /// assert_eq!("image/png", mime.unwrap());
    /// ```
    /// # Returns
    ///
    /// Fails if the resource can't be found.
    pub fn get_resource_mime(&self, id: &str) -> Option<&str> {
        self.context
            .resources
            .get(id)
            .map(|item| item.mime.as_str())
    }

    /// Returns the resource mime searching by source full path
    ///
    /// # Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # let doc = EpubDoc::new("test.epub");
    /// # let doc = doc.unwrap();
    /// let mime = doc.get_resource_mime_by_path("OEBPS/Images/portada.png");
    /// assert_eq!("image/png", mime.unwrap());
    /// ```
    ///
    /// # Errors
    ///
    /// Fails if the resource can't be found.
    pub fn get_resource_mime_by_path(&self, path: impl AsRef<Path>) -> Option<&str> {
        let path = path.as_ref();

        self.context
            .resources
            .values()
            .filter(|data| data.path == path)
            .map(|data| data.mime.as_str())
            .next()
    }

    /// Returns the chapter data at the provided spine id, with resource uris renamed so they
    /// have the `url_prepend` prefix and all are relative to the root file.
    ///
    /// This method is useful to render the content with a html engine, because inside the epub
    /// local paths are relatives, so you can provide that content, because the engine will look
    /// for the relative path in the filesystem and that file isn't there. You should provide files
    /// with `url_prepend` using the get_resource_by_path
    ///
    /// # Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # let mut doc = EpubDoc::new("test.epub").unwrap();
    /// let spine_id = doc.context.spine.get(1).unwrap();
    /// let current = doc.get_page_with_epub_uris(&spine_id, "epub://").unwrap();
    /// let text = String::from_utf8(current).unwrap();
    /// assert!(text.contains("epub://OEBPS/Styles/stylesheet.css"));
    /// assert!(text.contains("http://creativecommons.org/licenses/by-sa/3.0/"));
    /// ```
    pub fn get_page_with_epub_uris(&self, spine_id: &str, url_prepend: &str) -> Result<Vec<u8>> {
        let path = &self
            .context
            .resources
            .get(spine_id)
            .ok_or(ArchiveError::InvalidId)?
            .path;
        let html = self.get_resource_by_path(path)?;
        let content = xmlutils::ensure_utf8(&html);

        let settings = lol_html::Settings {
            element_content_handlers: vec![
                lol_html::element!("a[href], link[href], image[href]", |el| {
                    let current_val = el.get_attribute("href").ok_or(XMLError::NoElements)?;
                    let href = build_epub_uri(path, url_prepend, &current_val);

                    el.set_attribute("href", &href)?;

                    Ok(())
                }),
                lol_html::element!("img[src]", |el| {
                    let current_val = el.get_attribute("src").ok_or(XMLError::NoElements)?;
                    let href = build_epub_uri(path, url_prepend, &current_val);

                    el.set_attribute("src", &href)?;

                    Ok(())
                }),
            ],
            strict: false,
            ..lol_html::Settings::default()
        };
        let response = xmlutils::replace_attributes(&content, settings)?;

        Ok(response)
    }

    /// Returns the number of chapters
    ///
    /// # Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # let doc = EpubDoc::new("test.epub");
    /// # let mut doc = doc.unwrap();
    /// assert_eq!(17, doc.get_num_pages());
    /// ```
    pub fn get_num_pages(&self) -> usize {
        self.context.spine.len()
    }

    /// Function to convert a resource path to a chapter number in the spine
    /// If the resource isn't in the spine list, None will be returned
    ///
    /// This method is useful to convert a toc NavPoint content to a chapter number
    /// to be able to navigate easily
    pub fn resource_uri_to_chapter(&self, uri: impl AsRef<Path>) -> Option<usize> {
        for (key, item) in self.context.resources.iter() {
            if item.path == uri.as_ref() {
                return self.resource_id_to_chapter(key);
            }
        }

        None
    }

    /// Function to convert a resource id to a chapter number in the spine
    /// If the resource isn't in the spine list, None will be returned
    pub fn resource_id_to_chapter(&self, uri: &str) -> Option<usize> {
        self.context.spine.iter().position(|item| item == uri)
    }

    fn fill_resources(&mut self) -> Result<()> {
        let mut archive = self.archive.borrow_mut();
        let root_container = archive.get_entry(&self.root_file)?;
        let txt = xmlutils::ensure_utf8(&root_container);
        let root = crate::xmlutils::parse_xml(&txt)?;
        let epub_version = root
            .root_element()
            .attribute("version")
            .ok_or(ArchiveError::ParsingFailure)?;

        match epub_version {
            "2.0" => {
                // Parse with only the V2 parser
                EpubV2Parser::parse(&mut self.context, &self.root_base, &root, &mut archive)?;
            }
            _ => {
                // Always assume it's a V3 epub
                // Parse with the V2 parser, followed by the V3 parser
                EpubV2Parser::parse(&mut self.context, &self.root_base, &root, &mut archive)?;
                EpubV3Parser::parse(&mut self.context, &self.root_base, &root, &mut archive)?;
            }
        }

        Ok(())
    }
}

fn get_root_file(content: &[u8]) -> Result<PathBuf, ArchiveError> {
    let txt = xmlutils::ensure_utf8(content);
    let root = crate::xmlutils::parse_xml(&txt)?;
    let element = root
        .descendants()
        .find(|r| r.has_tag_name("rootfile"))
        .ok_or(ArchiveError::ParsingFailure)?;
    let attr = element
        .attribute("full-path")
        .ok_or(ArchiveError::ParsingFailure)?;

    Ok(PathBuf::from(attr))
}

fn build_epub_uri<'a>(path: impl AsRef<Path>, url_prepend: &str, append: &'a str) -> Cow<'a, str> {
    // allowing external links
    if append.starts_with("http") {
        return append.into();
    }

    let path = path.as_ref();
    let mut cpath = path.to_path_buf();

    // current file base dir
    cpath.pop();
    for p in Path::new(append).components() {
        match p {
            Component::ParentDir => {
                cpath.pop();
            }
            Component::Normal(s) => {
                cpath.push(s);
            }
            _ => {}
        };
    }

    // If on Windows, replace all Windows path separators with Unix path separators
    let path = if cfg!(windows) {
        cpath.display().to_string().replace('\\', "/")
    } else {
        cpath.display().to_string()
    };

    format!("{url_prepend}{path}").into()
}
