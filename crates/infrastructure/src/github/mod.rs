mod adapter;
mod response;

pub use adapter::GitHubSourceAdapter;
pub use response::{ReadmeDecodeError, ReadmeResponse, RepositoryResponse, decode_readme};
