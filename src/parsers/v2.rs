use crate::archive::EpubArchive;
use crate::doc::{MetadataNode, NavPoint};
use crate::parsers::{EpubMetadata, EpubParser, RootXml};
use crate::xmlutils;
use crate::xmlutils::XMLError;
use anyhow::anyhow;
use std::io::{Read, Seek};
use std::path::Path;

pub struct EpubV2Parser;

impl EpubParser for EpubV2Parser {
    fn parse<R: Read + Seek, PATH: AsRef<Path>>(
        epub: &mut EpubMetadata,
        root_base: PATH,
        xml: &RootXml,
        archive: &mut EpubArchive<R>,
    ) -> anyhow::Result<()> {
        let root = xml.borrow();
        let root_base = root_base.as_ref();
        let unique_identifier_id = &root.get_attr("unique-identifier").ok();

        // resources from manifest
        let manifest = root.find("manifest")?;
        for r in manifest.borrow().childs.iter() {
            let item = r.borrow();
            let _ = epub.insert_resource(root_base, &item);
        }

        // items from spine
        let spine = root.find("spine")?;
        for r in spine.borrow().childs.iter() {
            let item = r.borrow();
            let _ = Self::insert_spine(epub, &item);
        }

        // toc.ncx
        if let Ok(toc) = spine.borrow().get_attr("toc") {
            let _ = Self::fill_toc(epub, root_base, archive, &toc);
        }

        // metadata
        let metadata = root.find("metadata")?;
        for r in metadata.borrow().childs.iter() {
            let item = r.borrow();
            if item.name.local_name == "meta" {
                if let (Ok(k), Ok(v)) = (item.get_attr("name"), item.get_attr("content")) {
                    epub.metadata
                        .entry(k.to_string())
                        .or_insert(vec![])
                        .push(MetadataNode::from_attr(v, &item));
                } else if let Ok(k) = item.get_attr("property") {
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
                    if let Ok(id) = item.get_attr("id") {
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

        Ok(())
    }
}

impl EpubV2Parser {
    fn insert_spine(epub: &mut EpubMetadata, item: &xmlutils::XMLNode) -> Result<(), XMLError> {
        let id = item.get_attr("idref")?;
        epub.spine.push(id.to_string());
        Ok(())
    }

    fn fill_toc<R: Read + Seek, PATH: AsRef<Path>>(
        epub: &mut EpubMetadata,
        root_base: PATH,
        archive: &mut EpubArchive<R>,
        id: &str,
    ) -> anyhow::Result<()> {
        let toc_res = epub
            .resources
            .get(id)
            .ok_or_else(|| anyhow!("No toc found"))?;

        let container = archive.get_entry(&toc_res.path)?;
        let root = xmlutils::XMLReader::parse(container.as_slice())?;

        let mapnode = root.borrow().find("navMap")?;

        epub.toc
            .append(&mut Self::get_navpoints(root_base, &mapnode.borrow()));
        epub.toc.sort();

        Ok(())
    }

    /// Recursively extract all navpoints from a node.
    fn get_navpoints(root_base: impl AsRef<Path>, parent: &xmlutils::XMLNode) -> Vec<NavPoint> {
        let mut navpoints = Vec::new();
        let root_base = root_base.as_ref();

        // TODO: get docTitle
        // TODO: parse metadata (dtb:totalPageCount, dtb:depth, dtb:maxPageNumber)

        for nav in parent.childs.iter() {
            let item = nav.borrow();
            if item.name.local_name != "navPoint" {
                continue;
            }
            let play_order = item
                .get_attr("playOrder")
                .ok()
                .and_then(|n| usize::from_str_radix(&n, 10).ok());
            let content = match item.find("content") {
                Ok(c) => c.borrow().get_attr("src").ok().map(|p| root_base.join(p)),
                _ => None,
            };
            let label = match item.find("navLabel") {
                Ok(l) => l
                    .borrow()
                    .childs
                    .get(0)
                    .and_then(|t| t.borrow().text.clone()),
                _ => None,
            };

            if let (Some(o), Some(c), Some(l)) = (play_order, content, label) {
                let navpoint = NavPoint {
                    label: l.clone(),
                    content: c.clone(),
                    children: Self::get_navpoints(root_base, &item),
                    play_order: o,
                };
                navpoints.push(navpoint);
            }
        }

        navpoints.sort();
        navpoints
    }
}
