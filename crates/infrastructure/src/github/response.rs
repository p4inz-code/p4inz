use serde::Deserialize;
use thiserror::Error;

/// Subset of GitHub's `GET /repos/{owner}/{repo}` response actually used.
#[derive(Debug, Clone, Deserialize)]
pub struct RepositoryResponse {
    pub full_name: String,
    pub description: Option<String>,
}

/// Subset of GitHub's `GET /repos/{owner}/{repo}/readme` response actually
/// used. GitHub returns file content base64-encoded.
#[derive(Debug, Clone, Deserialize)]
pub struct ReadmeResponse {
    pub content: String,
    pub encoding: String,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ReadmeDecodeError {
    #[error("unsupported readme encoding '{encoding}'")]
    UnsupportedEncoding { encoding: String },
    #[error("readme content is not valid base64")]
    InvalidBase64,
    #[error("readme content is not valid UTF-8")]
    InvalidUtf8,
}

/// Decodes a [`ReadmeResponse`]'s content into plain text.
///
/// GitHub's API sometimes wraps the base64 payload across multiple lines;
/// newlines are stripped before decoding since they aren't part of the
/// base64 alphabet.
pub fn decode_readme(readme: &ReadmeResponse) -> Result<String, ReadmeDecodeError> {
    if readme.encoding != "base64" {
        return Err(ReadmeDecodeError::UnsupportedEncoding { encoding: readme.encoding.clone() });
    }

    let cleaned: String = readme.content.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, cleaned)
        .map_err(|_| ReadmeDecodeError::InvalidBase64)?;
    String::from_utf8(bytes).map_err(|_| ReadmeDecodeError::InvalidUtf8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_single_line_base64() {
        let readme = ReadmeResponse {
            content: "aGVsbG8gd29ybGQ=".to_string(),
            encoding: "base64".to_string(),
        };
        assert_eq!(decode_readme(&readme).unwrap(), "hello world");
    }

    #[test]
    fn decodes_base64_wrapped_across_lines() {
        let readme = ReadmeResponse {
            content: "aGVs\nbG8g\nd29y\nbGQ=".to_string(),
            encoding: "base64".to_string(),
        };
        assert_eq!(decode_readme(&readme).unwrap(), "hello world");
    }

    #[test]
    fn rejects_unsupported_encoding() {
        let readme = ReadmeResponse { content: "hello".to_string(), encoding: "utf-8".to_string() };
        assert_eq!(
            decode_readme(&readme),
            Err(ReadmeDecodeError::UnsupportedEncoding { encoding: "utf-8".to_string() })
        );
    }

    #[test]
    fn rejects_invalid_base64() {
        let readme = ReadmeResponse {
            content: "not valid base64!!!".to_string(),
            encoding: "base64".to_string(),
        };
        assert_eq!(decode_readme(&readme), Err(ReadmeDecodeError::InvalidBase64));
    }
}
