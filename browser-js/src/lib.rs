//! NGOS Browser JavaScript Engine
//!
//! Simple JavaScript interpreter - 100% Proprietary
//! Note: This is a minimal JS engine for basic scripting.
//! For full ECMAScript support, consider integrating QuickJS or V8.

pub use browser_core::{BrowserError, BrowserResult};
pub use browser_dom::{Document, Node};

/// JavaScript Value
#[derive(Debug, Clone)]
pub enum JsValue {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Object(Vec<(String, JsValue)>),
    Array(Vec<JsValue>),
    Function(String, Vec<String>, String), // name, params, body
}

impl core::fmt::Display for JsValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            JsValue::Undefined => f.write_str("undefined"),
            JsValue::Null => f.write_str("null"),
            JsValue::Boolean(b) => write!(f, "{b}"),
            JsValue::Number(n) => write!(f, "{n}"),
            JsValue::String(s) => f.write_str(s),
            JsValue::Object(_) => f.write_str("[object Object]"),
            JsValue::Array(_) => f.write_str("[object Array]"),
            JsValue::Function(name, _, _) => write!(f, "[Function: {name}]"),
        }
    }
}

/// JavaScript Runtime
pub struct JsRuntime {
    globals: Vec<(String, JsValue)>,
}

impl JsRuntime {
    pub fn new() -> BrowserResult<Self> {
        let mut runtime = Self {
            globals: Vec::new(),
        };

        // Register built-in functions
        runtime.register_builtin(
            "console.log",
            JsValue::Function(
                String::from("log"),
                vec![String::from("message")],
                String::from("builtin"),
            ),
        );

        Ok(runtime)
    }

    fn register_builtin(&mut self, name: &str, value: JsValue) {
        self.globals.push((String::from(name), value));
    }

    /// Execute simple JavaScript code
    pub fn eval(&self, code: &str) -> BrowserResult<String> {
        // Very simplified JS interpreter
        // Only supports:
        // - console.log("message")
        // - Variable declarations: let x = 5
        // - Simple expressions: 1 + 2

        let code = code.trim();

        // Handle console.log
        if let Some(msg) = code.strip_prefix("console.log(")
            && let Some(msg) = msg.strip_suffix(")")
        {
            let msg = msg.trim().trim_matches('"').trim_matches('\'');
            println!("[JS Console] {}", msg);
            return Ok(String::from("undefined"));
        }

        // Handle simple arithmetic
        if code.contains('+') && !code.contains("let") && !code.contains("var") {
            let parts: Vec<&str> = code.split('+').collect();
            if parts.len() == 2
                && let (Ok(a), Ok(b)) = (
                    parts[0].trim().parse::<f64>(),
                    parts[1].trim().parse::<f64>(),
                )
            {
                return Ok((a + b).to_string());
            }
        }

        // Handle string concatenation
        if code.contains('+') {
            let parts: Vec<&str> = code.split('+').collect();
            if parts.len() == 2 {
                let a = parts[0].trim().trim_matches('"').trim_matches('\'');
                let b = parts[1].trim().trim_matches('"').trim_matches('\'');
                return Ok(format!("{}{}", a, b));
            }
        }

        // Unknown syntax
        Ok(String::from("undefined"))
    }

    /// Execute with DOM support (stub)
    pub fn eval_with_dom(&self, code: &str, _document: &Document) -> BrowserResult<String> {
        // TODO: Implement DOM bindings
        self.eval(code)
    }
}

impl Default for JsRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to create JavaScript runtime")
    }
}

/// JavaScript Console API
pub struct Console;

impl Console {
    pub fn log(message: &str) {
        println!("[JS Console] {}", message);
    }

    pub fn error(message: &str) {
        eprintln!("[JS Error] {}", message);
    }

    pub fn warn(message: &str) {
        eprintln!("[JS Warning] {}", message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_js_runtime() {
        let runtime = JsRuntime::new();
        assert!(runtime.is_ok());
    }

    #[test]
    fn eval_console_log() {
        let runtime = JsRuntime::new().unwrap();
        let result = runtime.eval("console.log(\"Hello World\")").unwrap();
        assert_eq!(result, "undefined");
    }

    #[test]
    fn eval_arithmetic() {
        let runtime = JsRuntime::new().unwrap();
        let result = runtime.eval("1 + 2").unwrap();
        assert_eq!(result, "3");
    }

    #[test]
    fn eval_string_concat() {
        let runtime = JsRuntime::new().unwrap();
        let result = runtime.eval("\"Hello\" + \" World\"").unwrap();
        assert_eq!(result, "Hello World");
    }
}
