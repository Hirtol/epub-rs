//! EPUB library
//! lib to read and navigate throught an epub file contents
//!
//! # Examples
//!
//! ## Opening
//!
//! ```
//! use epub::doc::EpubDoc;
//! let doc = EpubDoc::new("test.epub");
//! assert!(doc.is_ok());
//! let doc = doc.unwrap();
//!
//! ```
//!
//! ## Getting doc metatada
//!
//! Metadata is a HashMap storing all metadata defined in the epub
//!
//! ```
//! # use epub::doc::EpubDoc;
//! # let doc = EpubDoc::new("test.epub");
//! # let doc = doc.unwrap();
//! let title = doc.mdata("title");
//! assert_eq!(title.unwrap(), "Todo es mío");
//! ```
//!
//! ## Accessing resources
//!
//! In the resources var is stored each resource defined
//! in the epub indexed by the id and with the full internal
//! path and mimetype. It's a HashMap<a: String, (b: String, c: String)>
//! where 'a' is the resource id, 'b' is the resource full path and
//! 'c' is the resource mimetype
//!
//! ```
//! # use epub::doc::EpubDoc;
//! # use std::path::Path;
//! # let doc = EpubDoc::new("test.epub");
//! # let doc = doc.unwrap();
//! assert_eq!(23, doc.context.resources.len());
//! let tpage = doc.context.resources.get("titlepage.xhtml");
//! assert_eq!(tpage.unwrap().path, Path::new("OEBPS/Text/titlepage.xhtml"));
//! assert_eq!(tpage.unwrap().mime, "application/xhtml+xml");
//! ```
//!
//! ## Navigating using the spine
//!
//! Spine is a Vec<String> storing the epub spine as resources ids
//!
//! ```
//! # use epub::doc::EpubDoc;
//! # let doc = EpubDoc::new("test.epub");
//! # let doc = doc.unwrap();
//! assert_eq!(17, doc.context.spine.len());
//! assert_eq!("titlepage.xhtml", doc.context.spine[0]);
//! ```
//!
//! ## Getting the cover
//!
//! ```ignore
//! use std::fs;
//! use std::io::Write;
//! use epub::doc::EpubDoc;
//!
//! let doc = EpubDoc::new("test.epub");
//! assert!(doc.is_ok());
//! let mut doc = doc.unwrap();
//!
//! let cover_data = doc.get_cover().unwrap();
//!
//! let f = fs::File::create("/tmp/cover.png");
//! assert!(f.is_ok());
//! let mut f = f.unwrap();
//! let resp = f.write_all(&cover_data);
//! ```

mod xmlutils;

pub mod archive;
pub mod doc;
pub mod error;
pub(crate) mod parsers;
mod utils;
