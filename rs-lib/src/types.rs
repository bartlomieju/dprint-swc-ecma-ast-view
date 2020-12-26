use crate::comments::*;
use crate::generated::*;
use crate::tokens::*;
use swc_common::{comments::SingleThreadedComments, BytePos, Span, Spanned};
use swc_ecmascript::parser::token::TokenAndSpan;

pub enum NodeOrToken<'a> {
  Node(Node<'a>),
  Token(&'a TokenAndSpan),
}

impl<'a> NodeOrToken<'a> {
  pub fn unwrap_token(&self) -> &'a TokenAndSpan {
    match self {
      NodeOrToken::Token(token) => token,
      NodeOrToken::Node(node) => panic!(
        "Expected to unwrap a token, but it was a node of kind {}.",
        node.kind()
      ),
    }
  }

  pub fn unwrap_node(&self) -> &Node<'a> {
    match self {
      NodeOrToken::Node(node) => node,
      NodeOrToken::Token(token) => panic!(
        "Expected to unwrap a node, but it was a token with text '{:?}'.",
        token.token
      ),
    }
  }
}

impl<'a> Spanned for NodeOrToken<'a> {
  fn span(&self) -> Span {
    match self {
      NodeOrToken::Node(node) => node.span(),
      NodeOrToken::Token(token) => token.span(),
    }
  }
}

pub trait SpannedExt {
  fn lo(&self) -> BytePos;
  fn hi(&self) -> BytePos;
  fn start_line_fast(&self, module: &Module) -> usize;
  fn end_line_fast(&self, module: &Module) -> usize;
  fn start_column_fast(&self, module: &Module) -> usize;
  fn end_column_fast(&self, module: &Module) -> usize;
  fn width_fast(&self, module: &Module) -> usize;
  fn tokens_fast<'a>(&self, module: &Module<'a>) -> &'a [TokenAndSpan];
  fn text_fast<'a>(&self, module: &Module<'a>) -> &'a str;
  fn leading_comments_fast<'a>(&self, module: &Module<'a>) -> CommentsIterator<'a>;
  fn trailing_comments_fast<'a>(&self, module: &Module<'a>) -> CommentsIterator<'a>;

  fn previous_token_fast<'a>(&self, module: &Module<'a>) -> Option<&'a TokenAndSpan> {
    let token_container = module_to_token_container(module);
    let index = token_container.get_token_index_at_lo(self.lo());
    if index == 0 {
      None
    } else {
      token_container.get_token_at_index(index - 1)
    }
  }

  fn next_token_fast<'a>(&self, module: &Module<'a>) -> Option<&'a TokenAndSpan> {
    let token_container = module_to_token_container(module);
    let index = token_container.get_token_index_at_hi(self.hi());
    token_container.get_token_at_index(index + 1)
  }

  fn previous_tokens_fast<'a>(
    &self,
    module: &Module<'a>,
  ) -> std::iter::Rev<std::slice::Iter<'a, TokenAndSpan>> {
    let token_container = module_to_token_container(module);
    let index = token_container.get_token_index_at_lo(self.lo());
    token_container.tokens[0..index].iter().rev()
  }

  fn next_tokens_fast<'a>(&self, module: &Module<'a>) -> &'a [TokenAndSpan] {
    let token_container = module_to_token_container(module);
    let index = token_container.get_token_index_at_hi(self.hi());
    &token_container.tokens[index + 1..]
  }
}

impl<T> SpannedExt for T
where
  T: Spanned,
{
  fn lo(&self) -> BytePos {
    self.span().lo
  }

  fn hi(&self) -> BytePos {
    self.span().hi
  }

  fn start_line_fast(&self, module: &Module) -> usize {
    module_to_source_file(module)
      .lookup_line(self.lo())
      .unwrap_or(0)
  }

  fn end_line_fast(&self, module: &Module) -> usize {
    module_to_source_file(module)
      .lookup_line(self.hi())
      .unwrap_or(0)
  }

  fn start_column_fast(&self, module: &Module) -> usize {
    get_column_at_pos(module, self.lo())
  }

  fn end_column_fast(&self, module: &Module) -> usize {
    get_column_at_pos(module, self.hi())
  }

  fn width_fast(&self, module: &Module) -> usize {
    self.text_fast(module).chars().count()
  }

  fn tokens_fast<'a>(&self, module: &Module<'a>) -> &'a [TokenAndSpan] {
    let span = self.span();
    let token_container = module_to_token_container(module);
    token_container.get_tokens_in_range(span.lo, span.hi)
  }

  fn text_fast<'a>(&self, module: &Module<'a>) -> &'a str {
    let span = self.span();
    let source_file = module_to_source_file(module);
    &source_file.src[(span.lo.0 as usize)..(span.hi.0 as usize)]
  }

  fn leading_comments_fast<'a>(&self, module: &Module<'a>) -> CommentsIterator<'a> {
    module_to_comment_container(module).leading_comments(self.lo())
  }

  fn trailing_comments_fast<'a>(&self, module: &Module<'a>) -> CommentsIterator<'a> {
    module_to_comment_container(module).trailing_comments(self.hi())
  }
}

pub trait NodeTrait<'a>: SpannedExt {
  fn parent(&self) -> Option<Node<'a>>;
  fn children(&self) -> Vec<Node<'a>>;
  fn into_node(&self) -> Node<'a>;
  fn kind(&self) -> NodeKind;

  fn ancestors(&self) -> AncestorIterator<'a> {
    AncestorIterator::new(self.into_node())
  }

  fn start_line(&self) -> usize {
    self.start_line_fast(self.module())
  }

  fn end_line(&self) -> usize {
    self.end_line_fast(self.module())
  }

  fn start_column(&self) -> usize {
    self.start_column_fast(self.module())
  }

  fn end_column(&self) -> usize {
    self.end_column_fast(self.module())
  }

  fn width(&self) -> usize {
    self.width_fast(self.module())
  }

  fn child_index(&self) -> usize {
    if let Some(parent) = self.parent() {
      let lo = self.lo();
      for (i, child) in parent.children().iter().enumerate() {
        if child.span().lo == lo {
          return i;
        }
      }
      panic!("Could not find the child index for some reason.");
    } else {
      0
    }
  }

  fn previous_sibling(&self) -> Option<Node<'a>> {
    if let Some(parent) = self.parent() {
      let child_index = self.child_index();
      if child_index > 0 {
        Some(parent.children().remove(child_index - 1))
      } else {
        None
      }
    } else {
      None
    }
  }

  fn next_sibling(&self) -> Option<Node<'a>> {
    if let Some(parent) = self.parent() {
      let next_index = self.child_index() + 1;
      let mut children = parent.children();
      if next_index < children.len() {
        Some(children.remove(next_index))
      } else {
        None
      }
    } else {
      None
    }
  }

  fn tokens(&self) -> &'a [TokenAndSpan] {
    self.tokens_fast(self.module())
  }

  fn children_with_tokens(&self) -> Vec<NodeOrToken<'a>> {
    self.children_with_tokens_fast(self.module())
  }

  fn children_with_tokens_fast(&self, module: &Module<'a>) -> Vec<NodeOrToken<'a>> {
    let children = self.children();
    let tokens = self.tokens_fast(module);
    let mut result = Vec::new();
    let mut tokens_index = 0;

    for child in children {
      let child_span = child.span();

      // get the tokens before the current child
      for token in &tokens[tokens_index..] {
        if token.span.lo() < child_span.lo {
          result.push(NodeOrToken::Token(token));
          tokens_index += 1;
        } else {
          break;
        }
      }

      // push current child
      result.push(NodeOrToken::Node(child));

      // skip past all the tokens within the token
      for token in &tokens[tokens_index..] {
        if token.span.hi() <= child_span.hi {
          tokens_index += 1;
        } else {
          break;
        }
      }
    }

    // get the tokens after the children
    for token in &tokens[tokens_index..] {
      result.push(NodeOrToken::Token(token));
    }

    result
  }

  fn leading_comments(&self) -> CommentsIterator<'a> {
    self.leading_comments_fast(self.module())
  }

  fn trailing_comments(&self) -> CommentsIterator<'a> {
    self.trailing_comments_fast(self.module())
  }

  fn module(&self) -> &Module<'a> {
    let mut current: Node<'a> = self.into_node();
    while let Some(parent) = current.parent() {
      current = parent;
    }

    // the top-most node will always be a module
    current.expect::<Module>()
  }

  fn text(&self) -> &'a str {
    self.text_fast(&self.module())
  }

  fn previous_token(&self) -> Option<&'a TokenAndSpan> {
    self.previous_token_fast(self.module())
  }

  fn next_token(&self) -> Option<&'a TokenAndSpan> {
    self.next_token_fast(self.module())
  }

  fn previous_tokens(&self) -> std::iter::Rev<std::slice::Iter<'a, TokenAndSpan>> {
    self.previous_tokens_fast(self.module())
  }

  fn next_tokens(&self) -> &'a [TokenAndSpan] {
    self.next_tokens_fast(self.module())
  }
}

pub trait TokenExt {
  fn token_index(&self, module: &Module) -> usize;
}

impl TokenExt for TokenAndSpan {
  fn token_index(&self, module: &Module) -> usize {
    let token_container = module_to_token_container(module);
    token_container.get_token_index_at_lo(self.span.lo)
  }
}

pub trait ModuleExt<'a> {
  fn token_at_index(&self, index: usize) -> Option<&'a TokenAndSpan>;
}

impl<'a> ModuleExt<'a> for Module<'a> {
  fn token_at_index(&self, index: usize) -> Option<&'a TokenAndSpan> {
    let token_container = module_to_token_container(self);
    token_container.get_token_at_index(index)
  }
}

fn module_to_source_file<'a>(module: &Module<'a>) -> &'a swc_common::SourceFile {
  module
    .source_file
    .expect("The source file must be provided to `with_view` in order to use this method.")
}

fn module_to_token_container<'a>(module: &Module<'a>) -> &'a TokenContainer<'a> {
  module
    .tokens
    .as_ref()
    .expect("The tokens must be provided to `with_view` in order to use this method.")
}

fn module_to_comment_container<'a>(module: &Module<'a>) -> &'a CommentContainer<'a> {
  module
    .comments
    .as_ref()
    .expect("The comments must be provided to `with_view` in order to use this method.")
}

fn get_column_at_pos(module: &Module, pos: BytePos) -> usize {
  let source_file = module_to_source_file(module);
  let text_bytes = source_file.src.as_bytes();
  let pos = pos.0 as usize;
  let mut line_start = 0;
  for i in (0..pos).rev() {
    if text_bytes[i] == '\n' as u8 {
      line_start = i + 1;
      break;
    }
  }
  let text_slice = &source_file.src[line_start..pos];
  text_slice.chars().count()
}

pub trait CastableNode<'a> {
  fn to(node: &Node<'a>) -> Option<&'a Self>;
  fn kind() -> NodeKind;
}

pub struct SourceFileInfo<'a> {
  pub module: &'a swc_ecmascript::ast::Module,
  pub source_file: Option<&'a swc_common::SourceFile>,
  pub tokens: Option<&'a Vec<TokenAndSpan>>,
  pub comments: Option<&'a SingleThreadedComments>,
}

#[derive(Clone)]
pub struct AncestorIterator<'a> {
  current: Node<'a>,
}

impl<'a> AncestorIterator<'a> {
  pub fn new(node: Node<'a>) -> AncestorIterator<'a> {
    AncestorIterator { current: node }
  }
}

impl<'a> Iterator for AncestorIterator<'a> {
  type Item = Node<'a>;

  fn next(&mut self) -> Option<Node<'a>> {
    let parent = self.current.parent();
    if let Some(parent) = parent {
      self.current = parent;
    }
    parent
  }
}
