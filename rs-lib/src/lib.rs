mod comments;
#[allow(invalid_value)]
mod generated;
#[cfg(feature = "serialize")]
mod serialize;
mod tokens;
mod types;

pub use comments::CommentsIterator;
pub use generated::*;
pub use types::*;

#[cfg(feature = "serialize")]
pub use serialize::*;

// swc re-exports
pub use swc_common::comments::{Comment, CommentKind};
pub use swc_common::{BytePos, Span, Spanned};
pub use swc_ecmascript::parser::token::{Token, TokenAndSpan};
