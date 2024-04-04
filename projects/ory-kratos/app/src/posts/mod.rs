use super::*;
mod post;
use post::Post;
pub mod posts_page;
pub use posts_page::PostPage;
mod create_posts;
use crate::posts_page::PostData;
use create_posts::CreatePost;
