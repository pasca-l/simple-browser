use alloc::rc::Rc;
// use alloc::rc::Weak;
use crate::renderer::dom::node::Element;
use crate::renderer::dom::node::ElementKind;
use crate::renderer::dom::node::Node;
use crate::renderer::dom::node::NodeKind;
use crate::renderer::dom::node::Window;
use crate::renderer::html::attribute::Attribute;
use crate::renderer::html::token::HtmlToken;
use crate::renderer::html::token::HtmlTokenizer;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::str::FromStr;

#[derive(Debug, Clone)]
pub struct HtmlParser {
    // browser: Weak<RefCell<Browser>>,
    window: Rc<RefCell<Window>>,
    mode: InsertionMode,
    // https://html.spec.whatwg.org/multipage/parsing.html#original-insertion-mode
    original_insertion_mode: InsertionMode,
    // https://html.spec.whatwg.org/multipage/parsing.html#the-stack-of-open-elements
    stack_of_open_elements: Vec<Rc<RefCell<Node>>>,
    t: HtmlTokenizer,
}

impl HtmlParser {
    // pub fn new(browser: Weak<RefCell<Browser>>, t: HtmlTokenizer) -> Self {
    pub fn new(t: HtmlTokenizer) -> Self {
        Self {
            // browser: browser.clone(),
            // window: Rc::new(RefCell::new(Window::new(browser))),
            window: Rc::new(RefCell::new(Window::new())),
            mode: InsertionMode::Initial,
            original_insertion_mode: InsertionMode::Initial,
            stack_of_open_elements: Vec::new(),
            t,
        }
    }

    // creates new element node
    fn create_element(&self, tag: &str, attributes: Vec<Attribute>) -> Node {
        Node::new(NodeKind::Element(Element::new(tag, attributes)))
    }

    // inserts the node in the appropriate place
    fn insert_element(&mut self, tag: &str, attributes: Vec<Attribute>) {
        let window = self.window.borrow();
        // if there is nothing on stack, current node is the root element
        let current = match self.stack_of_open_elements.last() {
            Some(n) => n.clone(),
            None => window.document(),
        };

        let node = Rc::new(RefCell::new(self.create_element(tag, attributes)));

        // if the node has a child, the node is appended to an existing sibling
        if current.borrow().first_child().is_some() {
            let mut last_sibiling = current.borrow().first_child();
            loop {
                last_sibiling = match last_sibiling {
                    Some(ref node) => {
                        if node.borrow().next_sibling().is_some() {
                            node.borrow().next_sibling()
                        } else {
                            break;
                        }
                    }
                    None => unimplemented!("last_sibiling should be Some"),
                };
            }

            last_sibiling
                .unwrap()
                .borrow_mut()
                .set_next_sibling(Some(node.clone()));
            node.borrow_mut().set_previous_sibling(Rc::downgrade(
                &current
                    .borrow()
                    .first_child()
                    .expect("failed to get a first child"),
            ))
        } else {
            current.borrow_mut().set_first_child(Some(node.clone()));
        }

        // creates a weak reference with the appended node
        current.borrow_mut().set_last_child(Rc::downgrade(&node));
        node.borrow_mut().set_parent(Rc::downgrade(&current));

        self.stack_of_open_elements.push(node);
    }

    // creates new text node
    fn create_char(&self, c: char) -> Node {
        let mut s = String::new();
        s.push(c);
        Node::new(NodeKind::Text(s))
    }

    // inserts a character in the text node
    fn insert_char(&mut self, c: char) {
        let current = match self.stack_of_open_elements.last() {
            Some(n) => n.clone(),
            None => return,
        };

        // if current is a text node, append the character
        if let NodeKind::Text(ref mut s) = current.borrow_mut().kind {
            s.push(c);
            return;
        }

        // do not append the character, if it is '\n' or ' '
        if c == '\n' || c == ' ' {
            return;
        }

        let node = Rc::new(RefCell::new(self.create_char(c)));

        if current.borrow().first_child().is_some() {
            current
                .borrow()
                .first_child()
                .unwrap()
                .borrow_mut()
                .set_next_sibling(Some(node.clone()));
            node.borrow_mut().set_previous_sibling(Rc::downgrade(
                &current
                    .borrow()
                    .first_child()
                    .expect("failed to get a first child"),
            ));
        } else {
            current.borrow_mut().set_first_child(Some(node.clone()));
        }

        current.borrow_mut().set_last_child(Rc::downgrade(&node));
        node.borrow_mut().set_parent(Rc::downgrade(&current));

        self.stack_of_open_elements.push(node);
    }

    // looks if the last element on stack is of the same kind
    // TODO: rename to is_last_element_in_stack
    fn pop_current_node(&mut self, element_kind: ElementKind) -> bool {
        let current = match self.stack_of_open_elements.last() {
            Some(n) => n,
            None => return false,
        };

        if current.borrow().element_kind() == Some(element_kind) {
            self.stack_of_open_elements.pop();
            return true;
        }

        false
    }

    // looks if the any element on stack is of the same kind
    // TODO: rename to is_in_stack
    fn contain_in_stack(&mut self, element_kind: ElementKind) -> bool {
        for i in 0..self.stack_of_open_elements.len() {
            if self.stack_of_open_elements[i].borrow().element_kind() == Some(element_kind) {
                return true;
            }
        }

        false
    }

    // pops until the given element kind
    fn pop_until(&mut self, element_kind: ElementKind) {
        assert!(
            self.contain_in_stack(element_kind),
            "stack doesn't have an element {:?}",
            element_kind,
        );

        loop {
            let current = match self.stack_of_open_elements.pop() {
                Some(n) => n,
                None => return,
            };

            if current.borrow().element_kind() == Some(element_kind) {
                return;
            }
        }
    }

    pub fn construct_tree(&mut self) -> Rc<RefCell<Window>> {
        let mut token = self.t.next();

        while token.is_some() {
            match self.mode {
                InsertionMode::Initial => {
                    // as DOCTYPE is not supported,
                    // <!doctype html> is handled as Char token
                    if let Some(HtmlToken::Char(_)) = token {
                        token = self.t.next();
                        continue;
                    }

                    self.mode = InsertionMode::BeforeHtml;
                    continue;
                }

                InsertionMode::BeforeHtml => {
                    match token {
                        Some(HtmlToken::Char(c)) => {
                            if c == ' ' || c == '\n' {
                                token = self.t.next();
                                continue;
                            }
                        }

                        Some(HtmlToken::StartTag {
                            ref tag,
                            self_closing: _,
                            ref attributes,
                        }) => {
                            if tag == "html" {
                                self.insert_element(tag, attributes.to_vec());
                                self.mode = InsertionMode::BeforeHead;
                                token = self.t.next();
                                continue;
                            }
                        }

                        // Some(HtmlToken::EndTag { ref tag }) => {
                        //     // Any other end tag
                        //     // Parse error. Ignore the token.
                        //     if tag != "head" || tag != "body" || tag != "html" || tag != "br" {
                        //         // Ignore the token.
                        //         token = self.t.next();
                        //         continue;
                        //     }
                        // }
                        Some(HtmlToken::Eof) | None => {
                            return self.window.clone();
                        }

                        _ => {}
                    }

                    self.insert_element("html", Vec::new());
                    self.mode = InsertionMode::BeforeHead;
                    continue;
                }

                InsertionMode::BeforeHead => {
                    match token {
                        Some(HtmlToken::Char(c)) => {
                            if c == ' ' || c == '\n' {
                                token = self.t.next();
                                continue;
                            }
                        }

                        Some(HtmlToken::StartTag {
                            ref tag,
                            self_closing: _,
                            ref attributes,
                        }) => {
                            if tag == "head" {
                                self.insert_element(tag, attributes.to_vec());
                                self.mode = InsertionMode::InHead;
                                token = self.t.next();
                                continue;
                            }
                        }

                        Some(HtmlToken::Eof) | None => {
                            return self.window.clone();
                        }

                        _ => {}
                    }

                    self.insert_element("head", Vec::new());
                    self.mode = InsertionMode::InHead;
                    continue;
                }

                InsertionMode::InHead => {
                    match token {
                        Some(HtmlToken::Char(c)) => {
                            if c == ' ' || c == '\n' {
                                token = self.t.next();
                                continue;
                            }
                        }

                        Some(HtmlToken::StartTag {
                            ref tag,
                            self_closing: _,
                            ref attributes,
                        }) => {
                            if tag == "style" || tag == "script" {
                                self.insert_element(tag, attributes.to_vec());
                                self.original_insertion_mode = self.mode;
                                self.mode = InsertionMode::Text;
                                token = self.t.next();
                                continue;
                            }
                            // below is not included in spec,
                            // but is necessary for avoiding infinite loop,
                            // which occurs with HTML without <head> tag
                            if tag == "body" {
                                self.pop_until(ElementKind::Head);
                                self.mode = InsertionMode::AfterHead;
                                continue;
                            }
                            if let Ok(_element_kind) = ElementKind::from_str(tag) {
                                self.pop_until(ElementKind::Head);
                                self.mode = InsertionMode::AfterHead;
                                continue;
                            }
                        }

                        Some(HtmlToken::EndTag { ref tag }) => {
                            if tag == "head" {
                                self.mode = InsertionMode::AfterHead;
                                token = self.t.next();
                                self.pop_until(ElementKind::Head);
                                continue;
                            }
                        }

                        Some(HtmlToken::Eof) | None => {
                            return self.window.clone();
                        }
                    }

                    // ignore unsupported tags like <meta> and <title>
                    token = self.t.next();
                    continue;
                }

                InsertionMode::AfterHead => {
                    match token {
                        Some(HtmlToken::Char(c)) => {
                            if c == ' ' || c == '\n' {
                                self.insert_char(c);
                                token = self.t.next();
                                continue;
                            }
                        }

                        Some(HtmlToken::StartTag {
                            ref tag,
                            self_closing: _,
                            ref attributes,
                        }) => {
                            if tag == "body" {
                                self.insert_element(tag, attributes.to_vec());
                                token = self.t.next();
                                self.mode = InsertionMode::InBody;
                                continue;
                            }
                        }

                        Some(HtmlToken::Eof) | None => {
                            return self.window.clone();
                        }

                        _ => {}
                    }

                    self.insert_element("body", Vec::new());
                    self.mode = InsertionMode::InBody;
                    continue;
                }

                InsertionMode::InBody => {
                    match token {
                        Some(HtmlToken::StartTag {
                            ref tag,
                            self_closing: _,
                            ref attributes,
                        }) => {
                            match tag.as_str() {
                                // "script" | "style" => {
                                //     // Process the token using the rules for the "in head" insertion mode.
                                //     //
                                //     // https://html.spec.whatwg.org/multipage/parsing.html#parsing-html-fragments
                                //     // Switch the tokenizer to the script data state.
                                //     self.insert_element(tag, attributes.to_vec());
                                //     self.t.switch_context(State::ScriptData);
                                //     self.original_insertion_mode = self.mode;
                                //     self.mode = InsertionMode::Text;
                                //     token = self.t.next();
                                //     continue;
                                // }
                                "h1" | "h2" => {
                                    self.insert_element(tag, attributes.to_vec());
                                    token = self.t.next();
                                    continue;
                                }

                                "p" => {
                                    self.insert_element(tag, attributes.to_vec());
                                    token = self.t.next();
                                    continue;
                                }

                                "a" => {
                                    self.insert_element(tag, attributes.to_vec());
                                    token = self.t.next();
                                    continue;
                                }

                                _ => {
                                    token = self.t.next();
                                }
                            }
                        }

                        Some(HtmlToken::EndTag { ref tag }) => match tag.as_str() {
                            "body" => {
                                self.mode = InsertionMode::AfterBody;
                                token = self.t.next();
                                if !self.contain_in_stack(ElementKind::Body) {
                                    continue;
                                }
                                self.pop_until(ElementKind::Body);
                                continue;
                            }

                            "html" => {
                                if self.pop_current_node(ElementKind::Body) {
                                    self.mode = InsertionMode::AfterBody;
                                    assert!(self.pop_current_node(ElementKind::Html));
                                } else {
                                    token = self.t.next();
                                }
                                continue;
                            }

                            "h1" | "h2" => {
                                let element_kind = ElementKind::from_str(tag)
                                    .expect("failed to convert string to ElementKind");
                                token = self.t.next();
                                self.pop_until(element_kind);
                                continue;
                            }

                            "p" => {
                                let element_kind = ElementKind::from_str(tag)
                                    .expect("failed to convert string to ElementKind");
                                token = self.t.next();
                                self.pop_until(element_kind);
                                continue;
                            }

                            "a" => {
                                let element_kind = ElementKind::from_str(tag)
                                    .expect("failed to convert string to ElementKind");
                                token = self.t.next();
                                self.pop_until(element_kind);
                                continue;
                            }

                            _ => {
                                token = self.t.next();
                            }
                        },

                        // any other character token, so _ => {} is unnecessary
                        Some(HtmlToken::Char(c)) => {
                            self.insert_char(c);
                            token = self.t.next();
                            continue;
                        }

                        Some(HtmlToken::Eof) | None => {
                            return self.window.clone();
                        }
                    }
                }

                InsertionMode::Text => {
                    match token {
                        Some(HtmlToken::EndTag { ref tag }) => {
                            if tag == "style" {
                                self.pop_until(ElementKind::Style);
                                self.mode = self.original_insertion_mode;
                                token = self.t.next();
                                continue;
                            }
                            if tag == "script" {
                                self.pop_until(ElementKind::Script);
                                self.mode = self.original_insertion_mode;
                                token = self.t.next();
                                continue;
                            }
                        }

                        Some(HtmlToken::Char(c)) => {
                            self.insert_char(c);
                            token = self.t.next();
                            continue;
                        }

                        Some(HtmlToken::Eof) | None => {
                            return self.window.clone();
                        }

                        _ => {}
                    }

                    self.mode = self.original_insertion_mode;
                }

                InsertionMode::AfterBody => {
                    match token {
                        Some(HtmlToken::Char(_c)) => {
                            // not aligned with spec
                            token = self.t.next();
                            continue;
                        }

                        Some(HtmlToken::EndTag { ref tag }) => {
                            if tag == "html" {
                                self.mode = InsertionMode::AfterAfterBody;
                                token = self.t.next();
                                continue;
                            }
                        }

                        Some(HtmlToken::Eof) | None => {
                            return self.window.clone();
                        }

                        _ => {}
                    }

                    self.mode = InsertionMode::InBody;
                }

                InsertionMode::AfterAfterBody => {
                    match token {
                        Some(HtmlToken::Char(_c)) => {
                            // not align with the spec
                            token = self.t.next();
                            continue;
                        }

                        Some(HtmlToken::Eof) | None => {
                            return self.window.clone();
                        }

                        _ => {}
                    }

                    self.mode = InsertionMode::InBody;
                }
            }
        }

        self.window.clone()
    }
}

// There are 23 states defined, but only 9 will be implemented
// https://html.spec.whatwg.org/multipage/parsing.html#original-insertion-mode
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum InsertionMode {
    // https://html.spec.whatwg.org/multipage/parsing.html#the-initial-insertion-mode
    Initial,
    // https://html.spec.whatwg.org/multipage/parsing.html#the-before-html-insertion-mode
    BeforeHtml,
    // https://html.spec.whatwg.org/multipage/parsing.html#the-before-head-insertion-mode
    BeforeHead,
    // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-inhead
    InHead,
    // https://html.spec.whatwg.org/multipage/parsing.html#the-after-head-insertion-mode
    AfterHead,
    // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-inbody
    InBody,
    // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-incdata
    Text,
    // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-afterbody
    AfterBody,
    // https://html.spec.whatwg.org/multipage/parsing.html#the-after-after-body-insertion-mode
    AfterAfterBody,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alloc::string::ToString;
    use alloc::vec;

    #[test]
    fn test_empty() {
        // let browser = Browser::new();
        let html = "".to_string();
        let t = HtmlTokenizer::new(html);
        // let window = HtmlParser::new(Rc::downgrade(&browser), t).construct_tree();
        let window = HtmlParser::new(t).construct_tree();
        let expected = Rc::new(RefCell::new(Node::new(NodeKind::Document)));

        assert_eq!(expected, window.borrow().document());
    }

    #[test]
    fn test_body() {
        // let browser = Browser::new();
        let html = "<html><head></head><body></body></html>".to_string();
        let t = HtmlTokenizer::new(html);
        // let window = HtmlParser::new(Rc::downgrade(&browser), t).construct_tree();
        let window = HtmlParser::new(t).construct_tree();
        let document = window.borrow().document();

        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Document))),
            document
        );

        let html = document
            .borrow()
            .first_child()
            .expect("failed to get a first child of document");
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new(
                "html",
                Vec::new()
            ))))),
            html
        );

        let head = html
            .borrow()
            .first_child()
            .expect("failed to get a first child of html");
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new(
                "head",
                Vec::new()
            ))))),
            head
        );

        let body = head
            .borrow()
            .next_sibling()
            .expect("failed to get a next sibling of head");
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new(
                "body",
                Vec::new()
            ))))),
            body
        );
    }

    #[test]
    fn test_text() {
        // let browser = Browser::new();
        let html = "<html><head></head><body>text</body></html>".to_string();
        let t = HtmlTokenizer::new(html);
        // let window = HtmlParser::new(Rc::downgrade(&browser), t).construct_tree();
        let window = HtmlParser::new(t).construct_tree();
        let document = window.borrow().document();

        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Document))),
            document
        );

        let html = document
            .borrow()
            .first_child()
            .expect("failed to get a first child of document");
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new(
                "html",
                Vec::new()
            ))))),
            html
        );

        let body = html
            .borrow()
            .first_child()
            .expect("failed to get a first child of document")
            .borrow()
            .next_sibling()
            .expect("failed to get a next sibling of head");
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new(
                "body",
                Vec::new()
            ))))),
            body
        );

        let text = body
            .borrow()
            .first_child()
            .expect("failed to get a first child of document");
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Text("text".to_string())))),
            text
        );
    }

    #[test]
    fn test_multiple_nodes() {
        // let browser = Browser::new();
        let html = "<html><head></head><body><p><a foo=bar>text</a></p></body></html>".to_string();
        let t = HtmlTokenizer::new(html);
        // let window = HtmlParser::new(Rc::downgrade(&browser), t).construct_tree();
        let window = HtmlParser::new(t).construct_tree();
        let document = window.borrow().document();

        let body = document
            .borrow()
            .first_child()
            .expect("failed to get a first child of document")
            .borrow()
            .first_child()
            .expect("failed to get a first child of document")
            .borrow()
            .next_sibling()
            .expect("failed to get a next sibling of head");
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new(
                "body",
                Vec::new()
            ))))),
            body
        );

        let p = body
            .borrow()
            .first_child()
            .expect("failed to get a first child of body");
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new(
                "p",
                Vec::new()
            ))))),
            p
        );

        let mut attr = Attribute::new();
        attr.add_char('f', true);
        attr.add_char('o', true);
        attr.add_char('o', true);
        attr.add_char('b', false);
        attr.add_char('a', false);
        attr.add_char('r', false);
        let a = p
            .borrow()
            .first_child()
            .expect("failed to get a first child of p");
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new(
                "a",
                vec![attr]
            ))))),
            a
        );

        let text = a
            .borrow()
            .first_child()
            .expect("failed to get a first child of a");
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Text("text".to_string())))),
            text
        );
    }

    // #[test]
    // fn test_no_body_tag() {
    //     let browser = Browser::new();
    //     let html = "<p>a</p>".to_string();
    //     let t = HtmlTokenizer::new(html);
    //     let window = HtmlParser::new(Rc::downgrade(&browser), t).construct_tree();
    //     let document = window.borrow().document();
    //     assert_eq!(
    //         Rc::new(RefCell::new(Node::new(NodeKind::Document))),
    //         document
    //     );

    //     let html = document
    //         .borrow()
    //         .first_child()
    //         .expect("failed to get a first child of document");
    //     assert_eq!(
    //         Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new(
    //             "html",
    //             Vec::new()
    //         ))))),
    //         html
    //     );

    //     let head = html
    //         .borrow()
    //         .first_child()
    //         .expect("failed to get a first child of html");
    //     assert_eq!(
    //         Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new(
    //             "head",
    //             Vec::new()
    //         ))))),
    //         head
    //     );

    //     let body = head
    //         .borrow()
    //         .next_sibling()
    //         .expect("failed to get a next sibling of head");
    //     assert_eq!(
    //         Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new(
    //             "body",
    //             Vec::new()
    //         ))))),
    //         body
    //     );

    //     let p = body
    //         .borrow()
    //         .first_child()
    //         .expect("failed to get a first child of body");
    //     assert_eq!(
    //         Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new(
    //             "p",
    //             Vec::new()
    //         ))))),
    //         p
    //     );
}