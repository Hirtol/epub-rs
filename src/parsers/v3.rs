//! Parser for Epub Spec version 3.0/3.2

use crate::archive::EpubArchive;
use crate::doc::NavPoint;
use crate::parsers::{EpubMetadata, EpubParser, RootXml};
use crate::xmlutils;
use anyhow::anyhow;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub struct EpubV3Parser;

impl EpubParser for EpubV3Parser {
    fn parse<R: Read + Seek, PATH: AsRef<Path>>(
        epub: &mut EpubMetadata,
        root_base: PATH,
        xml: &RootXml,
        archive: &mut EpubArchive<R>,
    ) -> anyhow::Result<()> {
        // ToC, only done if the book didn't contain a V2 fallback
        if epub.toc.is_empty() {
            // toc.ncx is not in spine, thus we need to find it in manifest
            let mut nav = None;
            // Find nav item, see: https://www.w3.org/publishing/epub3/epub-packages.html#sec-nav
            for (k, item) in epub.resources.iter() {
                if matches!(&item.property, Some(property) if property == "nav") {
                    nav = Some(k.clone());
                    break;
                }
            }

            if let Some(nav) = nav {
                // We ignore the error here as failing to parse the ToC is not fatal.
                let _ = fill_toc(epub, root_base, archive, &nav);
            }
        }

        Ok(())
    }
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

    let navs = root.borrow().find_all_children("nav");

    let toc = navs
        .into_iter()
        .find_map(|nav| {
            //TODO: The Attribute is epub:type, but we only search for local name at the moment
            let name = nav.borrow().get_attr("type").ok()?.to_string();

            if name == "toc" {
                Some(nav)
            } else {
                None
            }
        })
        .ok_or(anyhow!("No toc found"))?;

    epub.toc
        .append(&mut get_navpoints(root_base, &toc.borrow()));
    epub.toc.sort();

    Ok(())
}

/// Recursively extract all navpoints from a node.
fn get_navpoints(root_base: impl AsRef<Path>, parent: &xmlutils::XMLNode) -> Vec<NavPoint> {
    let mut navpoints = Vec::new();
    let root_base = root_base.as_ref();
    let link_elements = parent.find_all_children("a");

    for (i, link) in link_elements.iter().enumerate() {
        let item = link.borrow();

        let label = item.text.clone();
        let content = item
            .get_attr("href")
            .ok()
            .and_then(|i| PathBuf::from_str(i).ok());

        if let (Some(label), Some(content)) = (label, content) {
            let navpoint = NavPoint {
                label,
                content,
                children: get_navpoints(root_base, &item),
                play_order: i,
            };

            navpoints.push(navpoint);
        }
    }

    navpoints.sort();

    navpoints
}
