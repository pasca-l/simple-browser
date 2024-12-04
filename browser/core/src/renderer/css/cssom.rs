use crate::renderer::css::token::CssToken;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;

// https://www.w3.org/TR/cssom-1/#cssstylesheet
#[derive(Debug, Clone, PartialEq)]
pub struct StyleSheet {
    pub rules: Vec<QualifiedRule>,
}

impl StyleSheet {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn set_rules(&mut self, rules: Vec<QualifiedRule>) {
        self.rules = rules;
    }
}

// https://www.w3.org/TR/css-syntax-3/#qualified-rule
#[derive(Debug, Clone, PartialEq)]
pub struct QualifiedRule {
    pub selector: Selector,
    pub declarations: Vec<Declaration>,
}

impl QualifiedRule {
    pub fn new() -> Self {
        Self {
            selector: Selector::TypeSelector("".to_string()),
            declarations: Vec::new(),
        }
    }

    pub fn set_selector(&mut self, selector: Selector) {
        self.selector = selector;
    }

    pub fn set_declarations(&mut self, declarations: Vec<Declaration>) {
        self.declarations = declarations;
    }
}

// https://www.w3.org/TR/selectors-4/
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selector {
    // https://www.w3.org/TR/selectors-4/#type-selectors
    TypeSelector(String),
    // https://www.w3.org/TR/selectors-4/#class-html
    ClassSelector(String),
    // https://www.w3.org/TR/selectors-4/#id-selectors
    IdSelector(String),
    // dummy selector for unparsables
    UnknownSelector,
}

// https://www.w3.org/TR/css-syntax-3/#declaration
#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    pub property: String,
    pub value: ComponentValue,
}

impl Declaration {
    pub fn new() -> Self {
        Self {
            property: String::new(),
            value: ComponentValue::Ident(String::new()),
        }
    }

    pub fn set_property(&mut self, property: String) {
        self.property = property;
    }

    pub fn set_value(&mut self, value: ComponentValue) {
        self.value = value;
    }
}

// supports only component values that are preserved as tokens
pub type ComponentValue = CssToken;
