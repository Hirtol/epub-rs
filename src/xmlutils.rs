use std::borrow::Cow;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum XMLError {
    #[error("No XML Elements Found")]
    NoElements,
    #[error("Error in HTML writer")]
    LolHtmlError(#[from] lol_html::errors::RewritingError),
}

pub trait RoxmlNodeExt {
    /// Find an attribute with the given `name`, ignoring any namespaces in the process.
    fn attr_no_namespace(&self, name: impl AsRef<str>) -> Option<&str>;
}

impl<'a, 'b> RoxmlNodeExt for roxmltree::Node<'a, 'b> {
    fn attr_no_namespace(&self, name: impl AsRef<str>) -> Option<&str> {
        self.attributes()
            .find(|attr| attr.name() == name.as_ref())
            .map(|attr| attr.value())
    }
}

/// Most XML documents are technically allowed to be UTF-16.
///
/// The majority of the Rust ecosystem relies on UTF-8, and very few parsers therefore support UTF-16.
/// In order to work around that we must therefore ensure that we get a UTF-8 representation, which is done here.
///
/// So long as the XML document was originally UTF-8 no new allocation is performed here, merely validation.
pub fn ensure_utf8(content: &[u8]) -> Cow<'_, str> {
    let (encoding, skip) =
        encoding_rs::Encoding::for_bom(content).unwrap_or((encoding_rs::UTF_8, 0));
    let (out, _) = encoding.decode_without_bom_handling(&content[skip..]);

    out
}

pub fn replace_attributes(html: &str, settings: lol_html::Settings) -> Result<Vec<u8>, XMLError> {
    let mut output = Vec::with_capacity(html.len());
    let mut rewriter =
        lol_html::HtmlRewriter::new(settings, |c: &[u8]| output.extend_from_slice(c));

    rewriter.write(html.as_bytes())?;
    rewriter.end()?;

    Ok(output)
}

#[derive(Debug, Clone, PartialEq)]
pub struct OwnedAttribute {
    pub name: OwnedName,
    pub value: Arc<str>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OwnedName {
    pub namespace: Option<String>,
    pub tag: String,
}
