//! Parser for Epub Spec version 3.0/3.2

use crate::archive::EpubArchive;
use crate::doc::NavPoint;
use crate::error::Result;
use crate::parsers::{EpubMetadata, EpubParser};
use crate::utils;
use crate::xmlutils::RoxmlNodeExt;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};

pub struct EpubV3Parser;

impl EpubParser for EpubV3Parser {
    fn parse<R: Read + Seek, PATH: AsRef<Path>>(
        epub: &mut EpubMetadata,
        root_base: PATH,
        _xml: &roxmltree::Document<'_>,
        archive: &mut EpubArchive<R>,
    ) -> Result<()> {
        // Cover
        if epub.cover_id.is_none() {
            // In the Epub 3.2 specification an `item` element in the `manifest` can have the `cover-image` property.
            for (key, item) in epub.resources.iter() {
                if matches!(&item.property, Some(property) if property == "cover-image") {
                    epub.cover_id = Some(key.clone());
                    break;
                }
            }
        }

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
) -> Option<()> {
    let toc_res = epub.resources.get(id)?;

    let toc_xml = archive.get_entry(&toc_res.path).ok()?;
    let txt = crate::xmlutils::ensure_utf8(&toc_xml);
    let root = roxmltree::Document::parse(&txt).ok()?;

    let mut navs = root.descendants().filter(|r| r.has_tag_name("nav"));

    let toc = navs.find(|nav| {
        nav.attr_no_namespace("type")
            .map(|name| name == "toc")
            .unwrap_or_default()
    })?;

    epub.toc.append(&mut get_navpoints(root_base, &toc));
    epub.toc.sort();

    Some(())
}

/// Recursively extract all navpoints from a node.
fn get_navpoints(root_base: impl AsRef<Path>, parent: &roxmltree::Node<'_, '_>) -> Vec<NavPoint> {
    let mut navpoints = Vec::new();
    let root_base = root_base.as_ref();
    let link_elements = parent
        .descendants()
        .filter(|r| r != parent)
        .filter(|r| r.has_tag_name("a"));

    for (i, item) in link_elements.enumerate() {
        let content = item.attr_no_namespace("href").map(|i| root_base.join(i));

        if let (Some(label), Some(content)) = (item.text(), content) {
            if let Some(href) = utils::percent_decode(&content.to_string_lossy()) {
                let navpoint = NavPoint {
                    label: label.to_owned(),
                    content: PathBuf::from(href.as_ref()),
                    children: get_navpoints(root_base, &item),
                    play_order: i,
                };

                navpoints.push(navpoint);
            } else {
                println!("Failure in v3 parser, invalid ToC href entry: {content:?}",);
            }
        }
    }

    navpoints
}
