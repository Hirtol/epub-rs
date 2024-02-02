use crate::archive::EpubArchive;
use crate::doc::{MetadataNode, NavPoint};
use crate::error::{ArchiveError, Result};
use crate::parsers::{EpubMetadata, EpubParser};
use crate::utils;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};

pub struct EpubV2Parser;

impl EpubParser for EpubV2Parser {
    fn parse<R: Read + Seek, PATH: AsRef<Path>>(
        epub: &mut EpubMetadata,
        root_base: PATH,
        xml: &roxmltree::Document<'_>,
        archive: &mut EpubArchive<R>,
    ) -> Result<()> {
        let root = xml;
        let root_base = root_base.as_ref();
        let unique_identifier_id = root.root_element().attribute("unique-identifier");

        // resources from manifest
        let manifest = root
            .descendants()
            .find(|r| r.has_tag_name("manifest"))
            .ok_or(ArchiveError::ParsingFailure)?;
        for item in manifest.children() {
            let _ = epub.insert_resource(root_base, &item);
        }

        // items from spine
        let spine = root
            .descendants()
            .find(|r| r.has_tag_name("spine"))
            .ok_or(ArchiveError::ParsingFailure)?;
        for item in spine.children() {
            let _ = Self::insert_spine(epub, &item);
        }

        // toc.ncx
        if let Some(toc) = spine.attribute("toc") {
            let _ = Self::fill_toc(epub, root_base, archive, toc);
        }

        // metadata
        let metadata = root
            .descendants()
            .find(|r| r.has_tag_name("metadata"))
            .ok_or(ArchiveError::ParsingFailure)?;
        for item in metadata.children() {
            if item.has_tag_name("meta") {
                if let (Some(k), Some(v)) = (item.attribute("name"), item.attribute("content")) {
                    epub.metadata
                        .entry(k.to_string())
                        .or_insert(vec![])
                        .push(MetadataNode::from_attr(v, &item));
                } else if let Some(k) = item.attribute("property") {
                    let v = item.text().unwrap_or_default().to_owned();

                    let node = MetadataNode::from_attr(v, &item);

                    epub.metadata
                        .entry(k.to_string())
                        .or_insert(vec![])
                        .push(node);
                }
            } else {
                let v = item.text().unwrap_or_default().to_owned();
                if item.has_tag_name("identifier")
                    && epub.unique_identifier.is_none()
                    && unique_identifier_id.is_some()
                {
                    if let Some(id) = item.attribute("id") {
                        if &id == unique_identifier_id.as_ref().unwrap() {
                            epub.unique_identifier = Some(v.clone());
                        }
                    }
                }

                let node = MetadataNode::from_attr(v, &item);

                epub.metadata
                    .entry(item.tag_name().name().to_string())
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
                .filter(|i| epub.resources.contains_key(&i.content))
                .map(|i| i.content.to_string());
        }

        Ok(())
    }
}

impl EpubV2Parser {
    fn insert_spine(epub: &mut EpubMetadata, item: &roxmltree::Node<'_, '_>) -> Option<()> {
        let id = item.attribute("idref")?;

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

        let toc_xml = archive.get_entry(&toc_res.path).ok()?;
        let txt = crate::xmlutils::ensure_utf8(&toc_xml);
        let root = crate::xmlutils::parse_xml(&txt).ok()?;

        let map_node = root.descendants().find(|r| r.has_tag_name("navMap"))?;

        epub.toc
            .append(&mut Self::get_navpoints(root_base, &map_node));
        epub.toc.sort();

        Some(())
    }

    /// Recursively extract all nav-points from a node.
    fn get_navpoints(
        root_base: impl AsRef<Path>,
        parent: &roxmltree::Node<'_, '_>,
    ) -> Vec<NavPoint> {
        let root_base = root_base.as_ref();

        // TODO: get docTitle
        // TODO: parse metadata (dtb:totalPageCount, dtb:depth, dtb:maxPageNumber)

        let mut output: Vec<_> = parent
            .children()
            .flat_map(|item| Self::parse_nav_point(&item, root_base))
            .collect();

        output.sort();
        output
    }

    fn parse_nav_point(item: &roxmltree::Node<'_, '_>, root_base: &Path) -> Option<NavPoint> {
        if !item.has_tag_name("navPoint") {
            return None;
        }

        let play_order = item
            .attribute("playOrder")
            .and_then(|n| n.parse::<usize>().ok())?;
        let content = item
            .descendants()
            .find(|r| r.has_tag_name("content"))
            .and_then(|c| c.attribute("src").map(|p| root_base.join(p)))?;
        let label = item
            .descendants()
            .find(|r| r.has_tag_name("navLabel"))
            .and_then(|l| {
                l.first_element_child()
                    .and_then(|t| t.text())
                    .map(|t| t.to_owned())
            })?;

        if let Some(href) = utils::percent_decode(&content.to_string_lossy()) {
            let navpoint = NavPoint {
                label,
                content: PathBuf::from(href.as_ref()),
                children: Self::get_navpoints(root_base, item),
                play_order,
            };

            Some(navpoint)
        } else {
            // println!("Failure in v2 parser, invalid ToC href entry: {:?}", c);
            None
        }
    }
}
