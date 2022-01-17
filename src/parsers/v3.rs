//! Parser for Epub Spec version 3.0/3.2

use crate::archive::EpubArchive;
use crate::parsers::{EpubMetadata, EpubParser, RootXml};
use std::io::{Read, Seek};
use std::path::Path;

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
            // toc.ncx is not in spine, assuming Epub v3.2 so we need to find it in manifest
            let mut nav = None;
            // Find nav item, see: https://www.w3.org/publishing/epub3/epub-packages.html#sec-nav
            for (k, item) in epub.resources.iter() {
                if matches!(&item.property, Some(property) if property == "nav") {
                    nav = Some(k.clone());
                    break;
                }
            }

            if let Some(nav) = nav {
                //todo!();
                // let _ = Self::fill_toc(&nav);
            }
        }

        Ok(())
    }
}
