use std::path::Path;
use url::Url;

mod parser;

pub mod error;
pub mod types;

pub fn parse<S: AsRef<str>>(
    url: S,
) -> Result<(types::Definition, types::Namespaces), error::Error> {
    let url = {
        match Url::parse(url.as_ref()) {
            Ok(url) => url,
            Err(url::ParseError::RelativeUrlWithoutBase) => Url::from_file_path(
                &Path::new(url.as_ref())
                    .canonicalize()
                    .map_err(|err| error::Error::PathConversionError(Some(err)))?,
            )
            .unwrap(),
            Err(err) => return Err(err.into()),
        }
    };

    parser::parse(url)
}
