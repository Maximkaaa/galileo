/// Represents an attribution, typically used for citing sources or providing credit.
///
/// This struct stores a text description along with an optional URL where more information
/// or the source can be found.
#[derive(Debug, Clone)]
pub struct Attribution {
    /// - `text`: A static string representing the attribution text. This is typically the citation
    ///   or credit message.
    pub text: &'static str,
    /// - `url`: An optional URL where more information about the attribution can be found.
    pub url: Option<&'static str>,
}

impl Attribution {
    /// Creates a new `Attribution` with the given text and optional URL.
    pub fn new(text: &'static str, url: Option<&'static str>) -> Self {
        Self { text, url }
    }
}
