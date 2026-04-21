//! NGOS Browser HTML Parser
//!
//! HTML5 parser - 100% Proprietary, no external deps
//!
//! Canonical subsystem role:
//! - subsystem: browser HTML support
//! - owner layer: application support layer
//! - semantic owner: `browser-html`
//! - truth path role: browser-facing HTML parsing support for browser
//!   application flows
//!
//! Canonical contract families defined here:
//! - HTML parsing contracts
//! - browser document construction support contracts
//! - parser state support contracts
//!
//! This crate may define browser HTML support behavior, but it must not
//! redefine kernel, runtime, or product-level OS truth.

pub use browser_core::{BrowserError, BrowserResult};
use browser_dom::{Document, Node, NodeData, NodeType};

/// Parse HTML string into DOM tree
pub fn parse_html(html: &str) -> BrowserResult<Document> {
    let mut parser = HtmlParser::new(html);
    parser.parse()
}

/// HTML Parser
struct HtmlParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> HtmlParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn parse(&mut self) -> BrowserResult<Document> {
        let mut doc = Document::new();
        let mut stack: Vec<Node> = Vec::new();

        while let Some(token) = self.next_token()? {
            match token {
                Token::Doctype(name) => {
                    // Skip doctype for now
                    let _ = name;
                }
                Token::StartTag(name, attrs) => {
                    let mut node = NodeData::new(NodeType::Element, &name);
                    for (k, v) in attrs {
                        node.set_attribute(&k, &v);
                    }
                    let node_rc = std::rc::Rc::new(std::cell::RefCell::new(node));

                    if let Some(parent) = stack.last() {
                        parent
                            .borrow_mut()
                            .children
                            .push(std::rc::Rc::clone(&node_rc));
                    } else {
                        doc.document_element = Some(std::rc::Rc::clone(&node_rc));
                    }

                    // Self-closing tags
                    let void_elements = [
                        "br", "hr", "img", "input", "meta", "link", "area", "base", "col", "embed",
                        "param", "source", "track", "wbr",
                    ];
                    if !void_elements.contains(&name.to_lowercase().as_str()) {
                        stack.push(node_rc);
                    }
                }
                Token::EndTag(_name) => {
                    stack.pop();
                }
                Token::SelfClosingTag(name, attrs) => {
                    let mut node = NodeData::new(NodeType::Element, &name);
                    for (k, v) in attrs {
                        node.set_attribute(&k, &v);
                    }
                    let node_rc = std::rc::Rc::new(std::cell::RefCell::new(node));

                    if let Some(parent) = stack.last() {
                        parent
                            .borrow_mut()
                            .children
                            .push(std::rc::Rc::clone(&node_rc));
                    } else if doc.document_element.is_none() {
                        doc.document_element = Some(std::rc::Rc::clone(&node_rc));
                    }
                }
                Token::Text(text) => {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        let node = NodeData::new(NodeType::Text, "#text");
                        let text_rc = std::rc::Rc::new(std::cell::RefCell::new(node));
                        text_rc.borrow_mut().value = Some(trimmed.to_string());

                        if let Some(parent) = stack.last() {
                            parent.borrow_mut().children.push(text_rc);
                        }
                    }
                }
                Token::Comment(text) => {
                    let node = NodeData::new(NodeType::Comment, "#comment");
                    let comment_rc = std::rc::Rc::new(std::cell::RefCell::new(node));
                    comment_rc.borrow_mut().value = Some(text);

                    if let Some(parent) = stack.last() {
                        parent.borrow_mut().children.push(comment_rc);
                    }
                }
            }
        }

        // Find <head> and <body>
        if let Some(ref html_elem) = doc.document_element {
            for child in &html_elem.borrow().children {
                let tag = &child.borrow().name;
                if tag == "head" {
                    doc.head = Some(std::rc::Rc::clone(child));
                } else if tag == "body" {
                    doc.body = Some(std::rc::Rc::clone(child));
                }
            }
        }

        Ok(doc)
    }

    fn next_token(&mut self) -> BrowserResult<Option<Token>> {
        self.skip_whitespace();

        if self.pos >= self.input.len() {
            return Ok(None);
        }

        if self.input[self.pos..].starts_with("<!") {
            return self.read_doctype_or_comment();
        }

        if self.input[self.pos..].starts_with("<") {
            return self.read_tag();
        }

        self.read_text()
    }

    fn read_tag(&mut self) -> BrowserResult<Option<Token>> {
        self.pos += 1; // skip '<'

        if self.pos >= self.input.len() {
            return Err(BrowserError::Parse("Unexpected end in tag".into()));
        }

        let is_closing = self.input.as_bytes()[self.pos] as char == '/';
        if is_closing {
            self.pos += 1;
        }

        // Read tag name
        let tag_name = self.read_while(|c| c.is_alphanumeric() || c == '-' || c == '_');
        if tag_name.is_empty() {
            self.skip_until('>');
            if self.pos < self.input.len() {
                self.pos += 1;
            }
            return Ok(None);
        }

        // Read attributes
        let mut attrs = Vec::new();
        loop {
            self.skip_whitespace();

            if self.pos >= self.input.len() {
                return Err(BrowserError::Parse("Unexpected end in tag".into()));
            }

            let ch = self.input.as_bytes()[self.pos] as char;

            if ch == '>' {
                self.pos += 1;
                break;
            }

            if ch == '/' {
                self.pos += 1;
                if self.pos < self.input.len() && self.input.as_bytes()[self.pos] as char == '>' {
                    self.pos += 1;
                    return Ok(Some(Token::SelfClosingTag(tag_name, attrs)));
                }
                break;
            }

            // Read attribute name
            let attr_name = self.read_while(|c| !c.is_whitespace() && c != '=');
            if attr_name.is_empty() {
                break;
            }

            self.skip_whitespace();

            // Read '=' and value
            if self.pos < self.input.len() && self.input.as_bytes()[self.pos] as char == '=' {
                self.pos += 1;
                self.skip_whitespace();
                let attr_value = self.read_attr_value()?;
                attrs.push((attr_name, attr_value));
            } else {
                attrs.push((attr_name.clone(), String::new()));
            }
        }

        if is_closing {
            Ok(Some(Token::EndTag(tag_name)))
        } else {
            let void_elements = [
                "br", "hr", "img", "input", "meta", "link", "area", "base", "col", "embed",
                "param", "source", "track", "wbr",
            ];
            if void_elements.contains(&tag_name.to_lowercase().as_str()) {
                Ok(Some(Token::SelfClosingTag(tag_name, attrs)))
            } else {
                Ok(Some(Token::StartTag(tag_name, attrs)))
            }
        }
    }

    fn read_attr_value(&mut self) -> BrowserResult<String> {
        if self.pos >= self.input.len() {
            return Ok(String::new());
        }

        let quote = self.input.as_bytes()[self.pos] as char;

        if quote == '"' || quote == '\'' {
            self.pos += 1;
            let value = self.read_until(quote);
            if self.pos < self.input.len() {
                self.pos += 1; // skip closing quote
            }
            Ok(value)
        } else {
            Ok(self.read_while(|c| !c.is_whitespace() && c != '>'))
        }
    }

    fn read_text(&mut self) -> BrowserResult<Option<Token>> {
        let start = self.pos;

        while self.pos < self.input.len() {
            if self.input.as_bytes()[self.pos] as char == '<' {
                break;
            }
            self.pos += 1;
        }

        let text = &self.input[start..self.pos];

        if text.is_empty() {
            return self.next_token();
        }

        Ok(Some(Token::Text(String::from(text))))
    }

    fn read_doctype_or_comment(&mut self) -> BrowserResult<Option<Token>> {
        self.pos += 2; // skip '<!'

        if self.input[self.pos..].starts_with("--") {
            // Comment
            self.pos += 2;
            let comment = self.read_until_str("-->");
            if self.pos < self.input.len() {
                self.pos += 3; // skip '-->'
            }
            return Ok(Some(Token::Comment(comment)));
        }

        // DOCTYPE
        let name = self.read_while(|c| !c.is_whitespace() && c != '>');
        self.skip_until('>');
        if self.pos < self.input.len() {
            self.pos += 1;
        }

        Ok(Some(Token::Doctype(name)))
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() {
            let ch = self.input.as_bytes()[self.pos] as char;
            if ch.is_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn skip_until(&mut self, ch: char) {
        while self.pos < self.input.len() {
            if self.input.as_bytes()[self.pos] as char == ch {
                break;
            }
            self.pos += 1;
        }
    }

    fn read_while<F>(&mut self, mut predicate: F) -> String
    where
        F: FnMut(char) -> bool,
    {
        let start = self.pos;
        while self.pos < self.input.len() {
            let ch = self.input.as_bytes()[self.pos] as char;
            if predicate(ch) {
                self.pos += 1;
            } else {
                break;
            }
        }
        String::from(&self.input[start..self.pos])
    }

    fn read_until(&mut self, end: char) -> String {
        let start = self.pos;
        while self.pos < self.input.len() {
            if self.input.as_bytes()[self.pos] as char == end {
                break;
            }
            self.pos += 1;
        }
        String::from(&self.input[start..self.pos])
    }

    fn read_until_str(&mut self, end: &str) -> String {
        let start = self.pos;
        while self.pos < self.input.len() {
            if self.input[self.pos..].starts_with(end) {
                break;
            }
            self.pos += 1;
        }
        String::from(&self.input[start..self.pos])
    }
}

/// HTML Tokens
enum Token {
    Doctype(String),
    StartTag(String, Vec<(String, String)>),
    EndTag(String),
    SelfClosingTag(String, Vec<(String, String)>),
    Text(String),
    Comment(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_html() {
        let html = "<!DOCTYPE html><html><body><p>Hello</p></body></html>";
        let doc = parse_html(html).unwrap();
        assert!(doc.document_element.is_some());
    }

    #[test]
    fn parse_with_attributes() {
        let html = r#"<div id="main" class="container">Content</div>"#;
        let doc = parse_html(html).unwrap();
        assert!(doc.document_element.is_some());
    }

    #[test]
    fn parse_self_closing() {
        let html = "<br><hr><img src='test.jpg'>";
        let doc = parse_html(html).unwrap();
        assert!(doc.document_element.is_some());
    }
}
