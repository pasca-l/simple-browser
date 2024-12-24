use crate::alloc::string::ToString;
use crate::renderer::html::dom::Node as DomNode;
use crate::renderer::html::dom::NodeKind as DomNodeKind;
use crate::renderer::html::helper::api::get_element_by_id;
use crate::renderer::js::ast::Node;
use crate::renderer::js::ast::Program;
use alloc::format;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::borrow::Borrow;
use core::cell::RefCell;
use core::fmt::Display;
use core::fmt::Formatter;
use core::ops::Add;
use core::ops::Sub;

#[derive(Debug, Clone)]
pub struct JsRuntime {
    env: Rc<RefCell<Environment>>,
    functions: Vec<Function>,
    dom_root: Rc<RefCell<DomNode>>,
}

impl JsRuntime {
    pub fn new(dom_root: Rc<RefCell<DomNode>>) -> Self {
        Self {
            env: Rc::new(RefCell::new(Environment::new(None))),
            // all fuctions are able to be called anywhere (as global function)
            functions: Vec::new(),
            dom_root,
        }
    }

    pub fn execute(&mut self, program: &Program) {
        for node in program.body() {
            self.eval(&Some(node.clone()), self.env.clone());
        }
    }

    fn eval(
        &mut self,
        node: &Option<Rc<Node>>,
        env: Rc<RefCell<Environment>>,
    ) -> Option<RuntimeValue> {
        let node = match node {
            Some(n) => n,
            None => return None,
        };

        match node.borrow() {
            Node::BlockStatement { body } => {
                let mut result: Option<RuntimeValue> = None;
                for stmt in body {
                    result = self.eval(stmt, env.clone());
                }
                result
            }
            Node::FunctionDeclaration { id, params, body } => {
                if let Some(RuntimeValue::StringLiteral(id)) = self.eval(&id, env.clone()) {
                    let cloned_body = match body {
                        Some(b) => Some(b.clone()),
                        None => None,
                    };
                    self.functions
                        .push(Function::new(id, params.to_vec(), cloned_body));
                };
                None
            }
            Node::ReturnStatement { argument } => {
                return self.eval(argument, env.clone());
            }
            Node::VariableDeclaration { declarations } => {
                for declaration in declarations {
                    self.eval(&declaration, env.clone());
                }
                None
            }
            Node::VariableDeclarator { id, init } => {
                if let Some(node) = id {
                    if let Node::Identifier(id) = node.borrow() {
                        let init = self.eval(&init, env.clone());
                        env.borrow_mut().add_variable(id.to_string(), init);
                    }
                }
                None
            }
            Node::ExpressionStatement(expr) => return self.eval(&expr, env.clone()),
            Node::AssignmentExpression {
                operator,
                left,
                right,
            } => {
                if operator != &'=' {
                    return None;
                }

                if let Some(node) = left {
                    if let Node::Identifier(id) = node.borrow() {
                        let new_value = self.eval(right, env.clone());
                        env.borrow_mut().update_variable(id.to_string(), new_value);
                        return None;
                    }
                }

                if let Some(RuntimeValue::HtmlElement { object, property }) =
                    self.eval(left, env.clone())
                {
                    let right_value = match self.eval(right, env.clone()) {
                        Some(value) => value,
                        None => return None,
                    };

                    if let Some(p) = property {
                        if p == "textContent" {
                            object
                                .borrow_mut()
                                .set_first_child(Some(Rc::new(RefCell::new(DomNode::new(
                                    DomNodeKind::Text(right_value.to_string()),
                                )))));
                        }
                    }
                }

                None
            }
            Node::BinaryExpression {
                operator,
                left,
                right,
            } => {
                let left_value = match self.eval(left, env.clone()) {
                    Some(value) => value,
                    None => return None,
                };
                let right_value = match self.eval(right, env.clone()) {
                    Some(value) => value,
                    None => return None,
                };

                if operator == &'+' {
                    Some(left_value + right_value)
                } else if operator == &'-' {
                    Some(left_value - right_value)
                } else {
                    None
                }
            }
            Node::CallExpression { callee, arguments } => {
                // create new scope
                let new_env = Rc::new(RefCell::new(Environment::new(Some(env))));

                let callee_value = match self.eval(callee, new_env.clone()) {
                    Some(value) => value,
                    None => return None,
                };

                // try to call the browser api first
                let api_result = self.call_browser_api(&callee_value, arguments, new_env.clone());
                if api_result.0 {
                    return api_result.1;
                }

                // find a user defined function
                let function = {
                    let mut f: Option<Function> = None;

                    for func in &self.functions {
                        if callee_value == RuntimeValue::StringLiteral(func.id.to_string()) {
                            f = Some(func.clone());
                        }
                    }

                    match f {
                        Some(f) => f,
                        None => panic!("function {:?} doesn't exist", callee),
                    }
                };

                // assign arguments to params as local variables
                assert!(arguments.len() == function.params.len());
                for (i, item) in arguments.iter().enumerate() {
                    if let Some(RuntimeValue::StringLiteral(name)) =
                        self.eval(&function.params[i], new_env.clone())
                    {
                        new_env
                            .borrow_mut()
                            .add_variable(name, self.eval(item, new_env.clone()));
                    }
                }

                // call function with arguments
                self.eval(&function.body.clone(), new_env.clone())
            }
            Node::MemberExpression { object, property } => {
                let object_value = match self.eval(&object, env.clone()) {
                    Some(value) => value,
                    None => return None,
                };
                let property_value = match self.eval(&property, env.clone()) {
                    Some(value) => value,
                    None => return Some(object_value),
                };

                if let RuntimeValue::HtmlElement { object, property } = object_value {
                    assert!(property.is_none());
                    return Some(RuntimeValue::HtmlElement {
                        object,
                        property: Some(property_value.to_string()),
                    });
                }

                return Some(
                    object_value + RuntimeValue::StringLiteral(".".to_string()) + property_value,
                );
            }
            Node::Identifier(name) => {
                match env.borrow_mut().get_variable(name.to_string()) {
                    Some(v) => return Some(v),
                    // if variable used for the first time, handle as string
                    None => Some(RuntimeValue::StringLiteral(name.to_string())),
                }
            }
            Node::NumericLiteral(value) => Some(RuntimeValue::Number(*value)),
            Node::StringLiteral(value) => Some(RuntimeValue::StringLiteral(value.to_string())),
        }
    }

    // calls browser supported api
    fn call_browser_api(
        &mut self,
        func: &RuntimeValue,
        arguments: &[Option<Rc<Node>>],
        env: Rc<RefCell<Environment>>,
    ) -> (bool, Option<RuntimeValue>) {
        if func == &RuntimeValue::StringLiteral("document.getElementById".to_string()) {
            let arg = match self.eval(&arguments[0], env.clone()) {
                Some(a) => a,
                None => return (true, None),
            };
            let target = match get_element_by_id(Some(self.dom_root.clone()), &arg.to_string()) {
                Some(n) => n,
                None => return (true, None),
            };
            return (
                true,
                Some(RuntimeValue::HtmlElement {
                    object: target,
                    property: None,
                }),
            );
        }

        (false, None)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    id: String,
    params: Vec<Option<Rc<Node>>>,
    body: Option<Rc<Node>>,
}

impl Function {
    fn new(id: String, params: Vec<Option<Rc<Node>>>, body: Option<Rc<Node>>) -> Self {
        Self { id, params, body }
    }
}

type VariableMap = Vec<(String, Option<RuntimeValue>)>;

// https://262.ecma-international.org/12.0/#sec-environment-records
#[derive(Debug, Clone)]
pub struct Environment {
    variables: VariableMap,
    outer: Option<Rc<RefCell<Environment>>>,
}

impl Environment {
    fn new(outer: Option<Rc<RefCell<Environment>>>) -> Self {
        Self {
            variables: VariableMap::new(),
            outer,
        }
    }

    pub fn get_variable(&self, name: String) -> Option<RuntimeValue> {
        // looks into the current scope to find variable
        for variable in &self.variables {
            if variable.0 == name {
                return variable.1.clone();
            }
        }

        // goes to outer scope to find variable
        if let Some(env) = &self.outer {
            env.borrow_mut().get_variable(name)
        } else {
            None
        }
    }

    // adds variable to current scope
    fn add_variable(&mut self, name: String, value: Option<RuntimeValue>) {
        self.variables.push((name, value));
    }

    // updates variable from current scope
    fn update_variable(&mut self, name: String, value: Option<RuntimeValue>) {
        for i in 0..self.variables.len() {
            if self.variables[i].0 == name {
                self.variables.remove(i);
                self.variables.push((name, value));
                return;
            }
        }
    }
}

// https://262.ecma-international.org/13.0/#sec-ecmascript-language-types
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeValue {
    HtmlElement {
        object: Rc<RefCell<DomNode>>,
        property: Option<String>,
    },
    // https://tc39.es/ecma262/#sec-numeric-types
    Number(u64),
    // https://tc39.es/ecma262/#sec-ecmascript-language-types-string-type
    StringLiteral(String),
}

impl Display for RuntimeValue {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        let s = match self {
            RuntimeValue::HtmlElement {
                object,
                property: _,
            } => {
                format!("HtmlElement: {:#?}", object)
            }
            RuntimeValue::Number(value) => format!("{}", value),
            RuntimeValue::StringLiteral(value) => value.to_string(),
        };
        write!(f, "{}", s)
    }
}

impl Add<RuntimeValue> for RuntimeValue {
    type Output = RuntimeValue;

    fn add(self, rhs: RuntimeValue) -> RuntimeValue {
        if let (RuntimeValue::Number(left_num), RuntimeValue::Number(right_num)) = (&self, &rhs) {
            return RuntimeValue::Number(left_num + right_num);
        }

        RuntimeValue::StringLiteral(self.to_string() + &rhs.to_string())
    }
}

impl Sub<RuntimeValue> for RuntimeValue {
    type Output = RuntimeValue;

    fn sub(self, rhs: RuntimeValue) -> RuntimeValue {
        if let (RuntimeValue::Number(left_num), RuntimeValue::Number(right_num)) = (&self, &rhs) {
            return RuntimeValue::Number(left_num - right_num);
        }

        // NaN
        RuntimeValue::Number(u64::MIN)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::js::ast::JsParser;
    use crate::renderer::js::token::JsLexer;

    #[test]
    fn test_num() {
        let dom = Rc::new(RefCell::new(DomNode::new(DomNodeKind::Document)));
        let input = "42".to_string();
        let lexer = JsLexer::new(input);
        let mut parser = JsParser::new(lexer);
        let ast = parser.parse_ast();
        let mut runtime = JsRuntime::new(dom);
        let expected = [Some(RuntimeValue::Number(42))];
        let mut i = 0;

        for node in ast.body() {
            let result = runtime.eval(&Some(node.clone()), runtime.env.clone());
            assert_eq!(expected[i], result);
            i += 1;
        }
    }

    #[test]
    fn test_add_nums() {
        let dom = Rc::new(RefCell::new(DomNode::new(DomNodeKind::Document)));
        let input = "1 + 2".to_string();
        let lexer = JsLexer::new(input);
        let mut parser = JsParser::new(lexer);
        let ast = parser.parse_ast();
        let mut runtime = JsRuntime::new(dom);
        let expected = [Some(RuntimeValue::Number(3))];
        let mut i = 0;

        for node in ast.body() {
            let result = runtime.eval(&Some(node.clone()), runtime.env.clone());
            assert_eq!(expected[i], result);
            i += 1;
        }
    }

    #[test]
    fn test_sub_nums() {
        let dom = Rc::new(RefCell::new(DomNode::new(DomNodeKind::Document)));
        let input = "2 - 1".to_string();
        let lexer = JsLexer::new(input);
        let mut parser = JsParser::new(lexer);
        let ast = parser.parse_ast();
        let mut runtime = JsRuntime::new(dom);
        let expected = [Some(RuntimeValue::Number(1))];
        let mut i = 0;

        for node in ast.body() {
            let result = runtime.eval(&Some(node.clone()), runtime.env.clone());
            assert_eq!(expected[i], result);
            i += 1;
        }
    }

    #[test]
    fn test_assign_variable() {
        let dom = Rc::new(RefCell::new(DomNode::new(DomNodeKind::Document)));
        let input = "var foo=42;".to_string();
        let lexer = JsLexer::new(input);
        let mut parser = JsParser::new(lexer);
        let ast = parser.parse_ast();
        let mut runtime = JsRuntime::new(dom);
        let expected = [None];
        let mut i = 0;

        for node in ast.body() {
            let result = runtime.eval(&Some(node.clone()), runtime.env.clone());
            assert_eq!(expected[i], result);
            i += 1;
        }
    }

    #[test]
    fn test_add_variable_and_num() {
        let dom = Rc::new(RefCell::new(DomNode::new(DomNodeKind::Document)));
        let input = "var foo=42; foo+1".to_string();
        let lexer = JsLexer::new(input);
        let mut parser = JsParser::new(lexer);
        let ast = parser.parse_ast();
        let mut runtime = JsRuntime::new(dom);
        let expected = [None, Some(RuntimeValue::Number(43))];
        let mut i = 0;

        for node in ast.body() {
            let result = runtime.eval(&Some(node.clone()), runtime.env.clone());
            assert_eq!(expected[i], result);
            i += 1;
        }
    }

    #[test]
    fn test_reassign_variable() {
        let dom = Rc::new(RefCell::new(DomNode::new(DomNodeKind::Document)));
        let input = "var foo=42; foo=1; foo".to_string();
        let lexer = JsLexer::new(input);
        let mut parser = JsParser::new(lexer);
        let ast = parser.parse_ast();
        let mut runtime = JsRuntime::new(dom);
        let expected = [None, None, Some(RuntimeValue::Number(1))];
        let mut i = 0;

        for node in ast.body() {
            let result = runtime.eval(&Some(node.clone()), runtime.env.clone());
            assert_eq!(expected[i], result);
            i += 1;
        }
    }

    #[test]
    fn test_add_function_and_num() {
        let dom = Rc::new(RefCell::new(DomNode::new(DomNodeKind::Document)));
        let input = "function foo() { return 42; } foo()+1".to_string();
        let lexer = JsLexer::new(input);
        let mut parser = JsParser::new(lexer);
        let ast = parser.parse_ast();
        let mut runtime = JsRuntime::new(dom);
        let expected = [None, Some(RuntimeValue::Number(43))];
        let mut i = 0;

        for node in ast.body() {
            let result = runtime.eval(&Some(node.clone()), runtime.env.clone());
            assert_eq!(expected[i], result);
            i += 1;
        }
    }

    #[test]
    fn test_define_function_with_args() {
        let dom = Rc::new(RefCell::new(DomNode::new(DomNodeKind::Document)));
        let input = "function foo(a, b) { return a + b; } foo(1, 2) + 3;".to_string();
        let lexer = JsLexer::new(input);
        let mut parser = JsParser::new(lexer);
        let ast = parser.parse_ast();
        let mut runtime = JsRuntime::new(dom);
        let expected = [None, Some(RuntimeValue::Number(6))];
        let mut i = 0;

        for node in ast.body() {
            let result = runtime.eval(&Some(node.clone()), runtime.env.clone());
            assert_eq!(expected[i], result);
            i += 1;
        }
    }

    #[test]
    fn test_local_variable() {
        let dom = Rc::new(RefCell::new(DomNode::new(DomNodeKind::Document)));
        let input = "var a=42; function foo() { var a=1; return a; } foo()+a".to_string();
        let lexer = JsLexer::new(input);
        let mut parser = JsParser::new(lexer);
        let ast = parser.parse_ast();
        let mut runtime = JsRuntime::new(dom);
        let expected = [None, None, Some(RuntimeValue::Number(43))];
        let mut i = 0;

        for node in ast.body() {
            let result = runtime.eval(&Some(node.clone()), runtime.env.clone());
            assert_eq!(expected[i], result);
            i += 1;
        }
    }
}
