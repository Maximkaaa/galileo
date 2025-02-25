//! This module provides functionality for handling attributions.
//! Represents an attribution, typically used for citing sources or providing credit.
///
/// This struct stores a text description along with an optional URL where more information
/// or the source can be found.
#[derive(Debug, Clone)]
pub struct Attribution {
    /// - `text`: A static string representing the attribution text. This is typically the citation
    ///   or credit message.
    text: String,
    /// - `url`: An optional URL where more information about the attribution can be found.
    url: Option<String>,
}

impl Attribution {
    /// Creates a new `Attribution` with the given text and optional URL.
    pub fn new(text: String, url: Option<String>) -> Self {
        Self { text, url }
    }

    /// Returns a reference to the text of the attribution.
    pub fn get_text(&self) -> &str {
        &self.text
    }

    /// Returns a reference to the URL associated with the attribution, if any.
    pub fn get_url(&self) -> Option<&str> {
        self.url.as_deref()
    }
}
