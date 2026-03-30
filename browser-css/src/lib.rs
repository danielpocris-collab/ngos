//! NGOS Browser CSS Parser
//!
//! CSS parser - 100% Proprietary, no external deps

pub use browser_core::{BrowserError, BrowserResult};
pub use browser_dom::{Document, Node};

/// CSS Stylesheet
pub struct Stylesheet {
    pub rules: Vec<CssRule>,
}

/// CSS Rule
pub struct CssRule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
}

/// CSS Selector
#[derive(Debug, Clone)]
pub enum Selector {
    Universal,
    Tag(String),
    Class(String),
    Id(String),
    Compound(Vec<Selector>),
    Descendant(Box<Selector>, Box<Selector>),
}

/// CSS Declaration
#[derive(Debug, Clone)]
pub struct Declaration {
    pub property: Property,
    pub value: String,
    pub important: bool,
}

/// CSS Properties (subset)
#[derive(Debug, Clone)]
pub enum Property {
    Color,
    BackgroundColor,
    FontSize,
    FontFamily,
    FontWeight,
    Margin(EdgeValues),
    Padding(EdgeValues),
    Border(BorderValues),
    Width(Length),
    Height(Length),
    Display,
    Position,
    Top(Length),
    Right(Length),
    Bottom(Length),
    Left(Length),
    Unknown(String),
}

#[derive(Debug, Clone, Default)]
pub struct EdgeValues {
    pub top: Length,
    pub right: Length,
    pub bottom: Length,
    pub left: Length,
}

#[derive(Debug, Clone, Default)]
pub struct BorderValues {
    pub width: Length,
    pub style: String,
    pub color: String,
}

#[derive(Debug, Clone, Default)]
pub enum Length {
    Auto,
    #[default]
    Zero,
    Pixels(f32),
    Percent(f32),
    Em(f32),
    Rem(f32),
}

/// Parse CSS text into stylesheet
pub fn parse_css(css: &str) -> Result<Stylesheet, BrowserError> {
    let mut rules = Vec::new();
    let mut current_pos = 0;
    let bytes = css.as_bytes();

    while current_pos < bytes.len() {
        // Find '{'
        let brace_start = bytes[current_pos..].iter().position(|&b| b == b'{');

        if brace_start.is_none() {
            break;
        }
        let brace_start = brace_start.unwrap() + current_pos;

        // Find '}'
        let brace_end = bytes[brace_start..].iter().position(|&b| b == b'}');

        if brace_end.is_none() {
            break;
        }
        let brace_end = brace_end.unwrap() + brace_start;

        // Parse selector (before '{')
        let selector_str = std::str::from_utf8(&bytes[current_pos..brace_start])
            .unwrap_or("")
            .trim();

        // Parse declarations (between '{' and '}')
        let decl_str = std::str::from_utf8(&bytes[brace_start + 1..brace_end])
            .unwrap_or("")
            .trim();

        if !selector_str.is_empty() {
            let selectors = parse_selectors(selector_str)?;
            let declarations = parse_declarations(decl_str);

            if !selectors.is_empty() {
                rules.push(CssRule {
                    selectors,
                    declarations,
                });
            }
        }

        current_pos = brace_end + 1;
    }

    Ok(Stylesheet { rules })
}

fn parse_selectors(selector_str: &str) -> Result<Vec<Selector>, BrowserError> {
    let mut selectors = Vec::new();

    for part in selector_str.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        let selector = parse_single_selector(part)?;
        selectors.push(selector);
    }

    Ok(selectors)
}

fn parse_single_selector(s: &str) -> Result<Selector, BrowserError> {
    let s = s.trim();

    if s == "*" {
        return Ok(Selector::Universal);
    }

    if let Some(id) = s.strip_prefix('#') {
        return Ok(Selector::Id(String::from(id)));
    }

    if let Some(class) = s.strip_prefix('.') {
        return Ok(Selector::Class(String::from(class)));
    }

    Ok(Selector::Tag(String::from(s)))
}

fn parse_declarations(decl_str: &str) -> Vec<Declaration> {
    let mut declarations = Vec::new();

    for decl in decl_str.split(';') {
        let decl = decl.trim();
        if decl.is_empty() {
            continue;
        }

        if let Some((property, value)) = decl.split_once(':') {
            let mut value = value.trim().to_string();
            let important = value.ends_with("!important");
            if important {
                value = value[..value.len() - 10].trim().to_string();
            }

            let prop = parse_property(property.trim(), &value);

            declarations.push(Declaration {
                property: prop,
                value,
                important,
            });
        }
    }

    declarations
}

fn parse_property(name: &str, value: &str) -> Property {
    match name.to_lowercase().as_str() {
        "color" => Property::Color,
        "background-color" => Property::BackgroundColor,
        "font-size" => Property::FontSize,
        "font-family" => Property::FontFamily,
        "font-weight" => Property::FontWeight,
        "margin" => Property::Margin(parse_edge_values(value)),
        "padding" => Property::Padding(parse_edge_values(value)),
        "border" => Property::Border(parse_border_values(value)),
        "width" => Property::Width(parse_length(value)),
        "height" => Property::Height(parse_length(value)),
        "display" => Property::Display,
        "position" => Property::Position,
        "top" => Property::Top(parse_length(value)),
        "right" => Property::Right(parse_length(value)),
        "bottom" => Property::Bottom(parse_length(value)),
        "left" => Property::Left(parse_length(value)),
        _ => Property::Unknown(String::from(name)),
    }
}

fn parse_length(value: &str) -> Length {
    let value = value.trim().to_lowercase();

    if value == "auto" {
        return Length::Auto;
    }

    if value == "0" || value == "0px" || value == "0%" {
        return Length::Zero;
    }

    if let Some(num) = value.strip_suffix("px")
        && let Ok(n) = num.trim().parse::<f32>()
    {
        return Length::Pixels(n);
    }

    if let Some(num) = value.strip_suffix("%")
        && let Ok(n) = num.trim().parse::<f32>()
    {
        return Length::Percent(n);
    }

    if let Some(num) = value.strip_suffix("em")
        && let Ok(n) = num.trim().parse::<f32>()
    {
        return Length::Em(n);
    }

    if let Some(num) = value.strip_suffix("rem")
        && let Ok(n) = num.trim().parse::<f32>()
    {
        return Length::Rem(n);
    }

    Length::Zero
}

fn parse_edge_values(value: &str) -> EdgeValues {
    let parts: Vec<&str> = value.split_whitespace().collect();

    match parts.len() {
        1 => {
            let v = parse_length(parts[0]);
            EdgeValues {
                top: v.clone(),
                right: v.clone(),
                bottom: v.clone(),
                left: v,
            }
        }
        2 => {
            let v1 = parse_length(parts[0]);
            let v2 = parse_length(parts[1]);
            EdgeValues {
                top: v1.clone(),
                right: v2.clone(),
                bottom: v1,
                left: v2,
            }
        }
        4 => EdgeValues {
            top: parse_length(parts[0]),
            right: parse_length(parts[1]),
            bottom: parse_length(parts[2]),
            left: parse_length(parts[3]),
        },
        _ => EdgeValues::default(),
    }
}

fn parse_border_values(value: &str) -> BorderValues {
    let parts: Vec<&str> = value.split_whitespace().collect();

    BorderValues {
        width: parts.first().map(|&s| parse_length(s)).unwrap_or_default(),
        style: parts.get(1).map(|&s| s.to_string()).unwrap_or_default(),
        color: parts.get(2).map(|&s| s.to_string()).unwrap_or_default(),
    }
}

/// Compute styles for a document
pub fn compute_styles(doc: &Document, stylesheet: &Stylesheet) -> ComputedStyles {
    let _ = (doc, stylesheet);
    ComputedStyles::new()
}

/// Computed styles
pub struct ComputedStyles {
    // Computed CSS values for each node
}

impl ComputedStyles {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for ComputedStyles {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_css() {
        let css = r#"
            body { margin: 0; padding: 0; }
            .container { width: 100%; }
            #main { background-color: white; }
        "#;
        let sheet = parse_css(css).unwrap();
        assert_eq!(sheet.rules.len(), 3);
    }

    #[test]
    fn parse_selectors() {
        assert!(matches!(
            parse_single_selector("*"),
            Ok(Selector::Universal)
        ));
        assert!(matches!(parse_single_selector("div"), Ok(Selector::Tag(_))));
        assert!(matches!(
            parse_single_selector(".class"),
            Ok(Selector::Class(_))
        ));
        assert!(matches!(parse_single_selector("#id"), Ok(Selector::Id(_))));
    }

    #[test]
    fn parse_length_values() {
        assert!(matches!(super::parse_length("auto"), Length::Auto));
        assert!(matches!(super::parse_length("0"), Length::Zero));
        assert!(matches!(
            super::parse_length("100px"),
            Length::Pixels(100.0)
        ));
        assert!(matches!(super::parse_length("50%"), Length::Percent(50.0)));
    }
}
