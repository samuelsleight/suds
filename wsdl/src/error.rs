use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Unable to parse provided URL")]
    UrlParseError(#[from] url::ParseError),

    #[error("Unable to convert provided path")]
    PathConversionError(Option<std::io::Error>),

    #[error("Unable to open file")]
    FileOpenError(quick_xml::Error),

    #[error("Unable to get file from server")]
    ReqwestError(#[from] reqwest::Error),

    #[error("Unsupported URL scheme {0}")]
    UnsupportedScheme(String),

    #[error("Error parsing XML input")]
    XmlParseError(#[from] quick_xml::Error),
}
