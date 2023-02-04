use crate::archive::EpubArchive;
use crate::doc::{MetadataNode, NavPoint};
use crate::error::{ArchiveError, Result};
use crate::parsers::{EpubMetadata, EpubParser, RootXml};
use crate::{utils, xmlutils};
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};

pub struct EpubV2Parser;

impl EpubParser for EpubV2Parser {
    fn parse<R: Read + Seek, PATH: AsRef<Path>>(
        epub: &mut EpubMetadata,
        root_base: PATH,
        xml: &RootXml,
        archive: &mut EpubArchive<R>,
    ) -> Result<()> {
        let root = xml.borrow();
        let root_base = root_base.as_ref();
        let unique_identifier_id = root.get_attr("unique-identifier");

        // resources from manifest
        let manifest = root.find("manifest").ok_or(ArchiveError::ParsingFailure)?;
        for r in manifest.borrow().children.iter() {
            let item = r.borrow();
            let _ = epub.insert_resource(root_base, &item);
        }

        // items from spine
        let spine = root.find("spine").ok_or(ArchiveError::ParsingFailure)?;
        for r in spine.borrow().children.iter() {
            let item = r.borrow();
            let _ = Self::insert_spine(epub, &item);
        }

        // toc.ncx
        if let Some(toc) = spine.borrow().get_attr("toc") {
            let _ = Self::fill_toc(epub, root_base, archive, toc);
        }

        // metadata
        let metadata = root.find("metadata").ok_or(ArchiveError::ParsingFailure)?;
        for r in metadata.borrow().children.iter() {
            let item = r.borrow();
            if item.name.local_name == "meta" {
                if let (Some(k), Some(v)) = (item.get_attr("name"), item.get_attr("content")) {
                    epub.metadata
                        .entry(k.to_string())
                        .or_insert(vec![])
                        .push(MetadataNode::from_attr(v, &item));
                } else if let Some(k) = item.get_attr("property") {
                    let v = match item.text {
                        Some(ref x) => x.to_string(),
                        None => String::from(""),
                    };

                    let node = MetadataNode::from_attr(v, &item);

                    epub.metadata
                        .entry(k.to_string())
                        .or_insert(vec![])
                        .push(node);
                }
            } else {
                let k = &item.name.local_name;
                let v = match item.text {
                    Some(ref x) => x.to_string(),
                    None => String::from(""),
                };
                if k == "identifier"
                    && epub.unique_identifier.is_none()
                    && unique_identifier_id.is_some()
                {
                    if let Some(id) = item.get_attr("id") {
                        if &id == unique_identifier_id.as_ref().unwrap() {
                            epub.unique_identifier = Some(v.to_string());
                        }
                    }
                }

                let node = MetadataNode::from_attr(v, &item);

                epub.metadata
                    .entry(k.to_string())
                    .or_insert(vec![])
                    .push(node);
            }
        }

        // Cover
        if epub.metadata.contains_key("cover") {
            epub.cover_id = epub
                .metadata
                .get("cover")
                .and_then(|i| i.get(0))
                .map(|i| i.content.to_string());
        }

        Ok(())
    }
}

impl EpubV2Parser {
    fn insert_spine(epub: &mut EpubMetadata, item: &xmlutils::XMLNode) -> Option<()> {
        let id = item.get_attr("idref")?;

        epub.spine.push(id.to_string());

        Some(())
    }

    fn fill_toc<R: Read + Seek, PATH: AsRef<Path>>(
        epub: &mut EpubMetadata,
        root_base: PATH,
        archive: &mut EpubArchive<R>,
        id: &str,
    ) -> Option<()> {
        let toc_res = epub.resources.get(id)?;

        let container = archive.get_entry(&toc_res.path).ok()?;
        let root = xmlutils::XMLReader::parse(container.as_slice()).ok()?;

        let mapnode = root.borrow().find("navMap")?;

        epub.toc
            .append(&mut Self::get_navpoints(root_base, &mapnode.borrow()));
        epub.toc.sort();

        Some(())
    }

    /// Recursively extract all navpoints from a node.
    fn get_navpoints(root_base: impl AsRef<Path>, parent: &xmlutils::XMLNode) -> Vec<NavPoint> {
        let mut navpoints = Vec::new();
        let root_base = root_base.as_ref();

        // TODO: get docTitle
        // TODO: parse metadata (dtb:totalPageCount, dtb:depth, dtb:maxPageNumber)

        for nav in parent.children.iter() {
            let item = nav.borrow();
            if item.name.local_name != "navPoint" {
                continue;
            }
            let play_order = item
                .get_attr("playOrder")
                .and_then(|n| n.parse::<usize>().ok());
            let content = item
                .find("content")
                .and_then(|c| c.borrow().get_attr("src").map(|p| root_base.join(p)));
            let label = item.find("navLabel").and_then(|l| {
                l.borrow()
                    .children
                    .get(0)
                    .and_then(|t| t.borrow().text.clone())
            });

            if let (Some(o), Some(c), Some(l)) = (play_order, content, label) {
                if let Some(href) = utils::percent_decode(&c.to_string_lossy()) {
                    let navpoint = NavPoint {
                        label: l.clone(),
                        content: PathBuf::from(href.as_ref()),
                        children: Self::get_navpoints(root_base, &item),
                        play_order: o,
                    };

                    navpoints.push(navpoint);
                } else {
                    println!("Failure in v2 parser, invalid ToC href entry: {:?}", c);
                }
            }
        }

        navpoints.sort();
        navpoints
    }
}
