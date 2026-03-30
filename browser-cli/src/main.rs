//! NGOS Browser CLI
//!
//! Command-line web browser with JavaScript and HTTPS support

use browser_core::Url;
use browser_css::parse_css;
use browser_html::parse_html;
use browser_http::HttpClient;
use browser_js::JsRuntime;
use browser_layout::{LayoutContext, Size, build_layout_tree};
use browser_paint::{AsciiRenderer, FrameScriptRenderer, Renderer};
use browser_tls::HttpsClient;
use browser_ui::{BrowserUiSurface, BrowserViewport};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("NGOS Browser v0.1.0");
        eprintln!();
        eprintln!("Usage: ngos-browser [options] <url>");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --format <tui|ascii|framescript>  Output format (default: ascii)");
        eprintln!("  --width <pixels>                  Viewport width (default: 80)");
        eprintln!("  --height <pixels>                 Viewport height (default: 24)");
        eprintln!("  --js                              Enable JavaScript");
        eprintln!("  --help                            Show this help");
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  ngos-browser http://example.com");
        eprintln!("  ngos-browser https://example.com --format ascii");
        eprintln!("  ngos-browser http://example.com --format framescript --js");
        std::process::exit(1);
    }

    // Parse arguments
    let mut format = "ascii";
    let mut width = 80u32;
    let mut height = 24u32;
    let mut enable_js = false;
    let mut url = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--format" | "-f" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: --format requires a value");
                    std::process::exit(1);
                }
                format = args[i].as_str();
            }
            "--width" | "-w" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: --width requires a value");
                    std::process::exit(1);
                }
                width = args[i].parse().unwrap_or(80);
            }
            "--height" | "-h" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: --height requires a value");
                    std::process::exit(1);
                }
                height = args[i].parse().unwrap_or(24);
            }
            "--js" => {
                enable_js = true;
            }
            "--help" => {
                println!("NGOS Browser v0.1.0 - Pragmatic Edition");
                println!();
                println!("Features:");
                println!("  ✓ HTTP/1.1 and HTTPS (TLS 1.3)");
                println!("  ✓ HTML5 Parser");
                println!("  ✓ CSS3 Parser");
                println!("  ✓ JavaScript (QuickJS)");
                println!("  ✓ FrameScript Renderer (NGOS GPU)");
                println!();
                println!("Licenses:");
                println!("  - NGOS Browser Core: Proprietary");
                println!("  - QuickJS: MIT License");
                println!("  - rustls: Apache-2.0 License");
                return;
            }
            arg if arg.starts_with('-') => {
                eprintln!("Error: Unknown option '{}'", arg);
                std::process::exit(1);
            }
            arg => {
                url = Some(arg.to_string());
            }
        }
        i += 1;
    }

    let url_str = match url {
        Some(u) => u,
        None => {
            eprintln!("Error: No URL provided");
            std::process::exit(1);
        }
    };

    // Parse URL
    let url = match Url::parse(&url_str) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("Error: Invalid URL: {:?}", e);
            std::process::exit(1);
        }
    };

    println!("NGOS Browser v0.1.0");
    println!("Fetching: {}", url);

    // Fetch page
    let html = match url.scheme.as_str() {
        "http" => {
            let client = HttpClient::new();
            match client.get(&url) {
                Ok(resp) => {
                    println!("Status: {} {}", resp.status, resp.status_text);
                    String::from_utf8_lossy(&resp.body).to_string()
                }
                Err(e) => {
                    eprintln!("Error: {:?}", e);
                    std::process::exit(1);
                }
            }
        }
        "https" => {
            let client = match HttpsClient::new() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error initializing HTTPS: {:?}", e);
                    std::process::exit(1);
                }
            };
            match client.get(&url) {
                Ok(resp) => {
                    println!("Status: {} {} (HTTPS)", resp.status, resp.status_text);
                    String::from_utf8_lossy(&resp.body).to_string()
                }
                Err(e) => {
                    eprintln!("Error: {:?}", e);
                    std::process::exit(1);
                }
            }
        }
        _ => {
            eprintln!("Error: Unsupported scheme: {}", url.scheme);
            std::process::exit(1);
        }
    };

    // Parse HTML
    println!("Parsing HTML...");
    let doc = match parse_html(&html) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error parsing HTML: {:?}", e);
            std::process::exit(1);
        }
    };

    println!("DOM: {} nodes", count_nodes(&doc));

    // Parse CSS (if any inlined)
    println!("Parsing CSS...");
    let css = ""; // Would extract from <style> tags
    let _stylesheet =
        parse_css(css).unwrap_or_else(|_| browser_css::Stylesheet { rules: Vec::new() });

    // Build layout
    println!("Building layout...");
    let ctx = LayoutContext {
        viewport: Size::new(width as f32, height as f32),
    };
    let styles = browser_css::compute_styles(&doc, &_stylesheet);
    let tree = build_layout_tree(&doc, &styles, &ctx);

    // JavaScript (if enabled)
    if enable_js {
        println!("Initializing JavaScript...");
        match JsRuntime::new() {
            Ok(js) => {
                println!("JavaScript runtime ready");
                // Would execute <script> tags here
                let _ = js;
            }
            Err(e) => {
                eprintln!("Warning: JavaScript failed to initialize: {:?}", e);
            }
        }
    }

    // Render
    println!("Rendering ({}x{}, format={})...", width, height, format);

    match format {
        "ascii" => {
            let mut renderer = AsciiRenderer::new(width as usize, height as usize);
            if let Err(e) = renderer.render(&tree) {
                eprintln!("Render error: {:?}", e);
                std::process::exit(1);
            }
            let _ = renderer.present();
        }
        "tui" => {
            let mut renderer = AsciiRenderer::new(width as usize, height as usize);
            if let Err(e) = renderer.render(&tree) {
                eprintln!("Render error: {:?}", e);
                std::process::exit(1);
            }
            println!("{}", renderer.get_output());
        }
        "framescript" | "fs" => {
            let mut surface = BrowserUiSurface::new(BrowserViewport::new(width, height));
            let frame = match surface.render_layout(&tree) {
                Ok(frame) => frame,
                Err(e) => {
                    eprintln!("Render error: {:?}", e);
                    std::process::exit(1);
                }
            };
            println!("{}", frame.script_text());
            println!("{}", frame.encoded.payload);
        }
        "framescript-raw" => {
            let mut renderer = FrameScriptRenderer::new(width, height);
            if let Err(e) = renderer.render(&tree) {
                eprintln!("Render error: {:?}", e);
                std::process::exit(1);
            }
            println!("{}", renderer.get_output());
            if let Err(e) = renderer.present() {
                eprintln!("Present error: {:?}", e);
            }
        }
        _ => {
            eprintln!("Error: Unknown format '{}'", format);
            std::process::exit(1);
        }
    }

    println!("Done!");
}

fn count_nodes(doc: &browser_dom::Document) -> usize {
    let mut count = 0;
    if let Some(ref root) = doc.document_element {
        count_nodes_recursive(root, &mut count);
    }
    count
}

fn count_nodes_recursive(node: &browser_dom::Node, count: &mut usize) {
    *count += 1;
    for child in &node.borrow().children {
        count_nodes_recursive(child, count);
    }
}
