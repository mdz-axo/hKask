//! Ad-hoc test: Search Kindle library, extract matching books.
//! Run: cargo test -p hkask-mcp-docproc --test kindle_discover -- --nocapture
//! Delete after validation per tool policy.

#[test]
fn search_and_extract_kindle_books() {
    // Load .env file for credentials
    let env_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.env");
    if env_path.exists() {
        for line in std::fs::read_to_string(&env_path)
            .unwrap_or_default()
            .lines()
        {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                let v = v.trim().trim_matches('"');
                // SAFETY: single-threaded test, no other code accessing env
                unsafe {
                    std::env::set_var(k.trim(), v);
                }
            }
        }
    }
    let chrome_bin = "/snap/chromium/current/usr/lib/chromium-browser/chrome";

    // Load credentials from .env
    let email = std::env::var("AMAZON_EMAIL").expect("AMAZON_EMAIL not set");
    let password = std::env::var("AMAZON_PASSWORD").expect("AMAZON_PASSWORD not set");
    println!("Using Amazon account: {}", email);

    assert!(
        std::path::Path::new(chrome_bin).exists(),
        "Chrome binary not found at {}",
        chrome_bin
    );

    use headless_chrome::{Browser, LaunchOptionsBuilder};

    let launch_opts = LaunchOptionsBuilder::default()
        .headless(true)
        .window_size(Some((1280, 900)))
        .sandbox(false)
        .path(Some(std::path::PathBuf::from(chrome_bin)))
        .args(vec![
            std::ffi::OsStr::new("--disable-blink-features=AutomationControlled"),
            std::ffi::OsStr::new("--no-first-run"),
            std::ffi::OsStr::new("--no-default-browser-check"),
        ])
        .build()
        .expect("LaunchOptions build");

    let browser = Browser::new(launch_opts).expect("Launch browser");
    let tab = browser.new_tab().expect("New tab");

    // Navigate to Kindle library
    println!("Navigating to Kindle library...");
    tab.navigate_to("https://read.amazon.com/kindle-library")
        .expect("Nav to library");
    tab.wait_until_navigated().expect("Wait nav");
    std::thread::sleep(std::time::Duration::from_secs(3));

    // ── Auth state machine ─────────────────────────────────────────────────
    // Amazon may redirect headless clients: landing → sign-in → 2FA → library
    let mut current_url = tab.get_url();
    println!("URL after nav: {}", current_url);

    // Phase 1: Landing page → click sign-in
    if current_url.contains("/landing") {
        println!("On landing page — finding sign-in link...");
        for selector in &["a[href*='signin']", "a[href*='ap/signin']", "a"] {
            if let Ok(el) = tab.find_element(selector) {
                if let Ok(Some(href)) = el.get_attribute_value("href") {
                    if href.contains("signin") || href.contains("ap/sign") {
                        println!("  Clicking: {}", href);
                        el.click().ok();
                        std::thread::sleep(std::time::Duration::from_secs(3));
                        break;
                    }
                }
            }
        }
        current_url = tab.get_url();
        println!("URL after sign-in click: {}", current_url);
    }

    // Phase 2: Sign-in → enter credentials
    if current_url.contains("/ap/signin") {
        println!("Login required — entering credentials...");
        kindle_login_flow(&tab, &email, &password);
        current_url = tab.get_url();
        println!("URL after login: {}", current_url);
    }

    // Phase 3: 2FA wait
    if current_url.contains("/ap/mfa") || current_url.contains("/ap/cvf") {
        println!("2FA detected — waiting 60s for manual entry...");
        std::thread::sleep(std::time::Duration::from_secs(60));
        current_url = tab.get_url();
    }

    // Phase 4: Re-navigate to library
    if !current_url.contains("kindle-library") && !current_url.contains("/landing") {
        println!("Navigating to Kindle library...");
        tab.navigate_to("https://read.amazon.com/kindle-library")
            .expect("Nav to library");
        tab.wait_until_navigated().expect("Wait nav");
        std::thread::sleep(std::time::Duration::from_secs(5));
        current_url = tab.get_url();
    }

    let final_url = tab.get_url();
    println!("Final URL: {}", final_url);

    if final_url.contains("/ap/signin") || final_url.contains("/landing") {
        println!("FAILED: Still not authenticated. URL: {}", final_url);
        // Take debug screenshot
        let screenshot = tab
            .capture_screenshot(
                headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
                None,
                None,
                true,
            )
            .expect("Screenshot");
        std::fs::write("/tmp/kindle-debug.png", &screenshot).ok();
        println!("Debug screenshot saved to /tmp/kindle-debug.png");
        return;
    }

    // Try to get page content
    println!("\n--- Page content ---");
    std::thread::sleep(std::time::Duration::from_secs(3));

    match tab.find_element("body") {
        Ok(body) => {
            if let Ok(text) = body.get_inner_text() {
                let snippet: String = text.chars().take(1000).collect();
                println!("{}", snippet);

                // Check if we have book titles
                if text.contains("knowledge graph") || text.contains("Knowledge Graph") {
                    println!("\n*** Found knowledge graph book(s)! ***");
                }
                if text.contains("knowledge production")
                    || text.contains("Knowledge Production")
                    || text.contains("Knowledge production")
                {
                    println!("\n*** Found knowledge production book(s)! ***");
                }
            }
        }
        Err(e) => println!("Body element not found: {:?}", e),
    }

    // Look for book titles and ASINs
    println!("\n--- Looking for books with selectors ---");
    for selector in &[
        "h2",
        "[data-asin]",
        ".bc-heading",
        ".bc-text",
        ".book-title",
        "a[href*='asin']",
    ] {
        match tab.find_elements(selector) {
            Ok(elements) => {
                println!("Selector '{}': {} elements", selector, elements.len());
                for el in elements.iter().take(10) {
                    if let Ok(text) = el.get_inner_text() {
                        let t = text.trim();
                        if !t.is_empty() {
                            // Check for knowledge-related terms
                            let lower = t.to_lowercase();
                            let marker = if lower.contains("knowledge") {
                                " ★★★"
                            } else {
                                ""
                            };
                            println!("  {}{}", t, marker);
                        }
                    }
                }
            }
            Err(_) => {}
        }
    }

    let screenshot = tab
        .capture_screenshot(
            headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
            None,
            None,
            true,
        )
        .expect("Screenshot");
    std::fs::write("/tmp/kindle-library.png", &screenshot).expect("Write screenshot");
    println!("\nScreenshot saved to /tmp/kindle-library.png");
}

/// Login flow: fill email → submit → fill password → submit
fn kindle_login_flow(tab: &headless_chrome::Tab, email: &str, password: &str) {
    // Fill email
    if let Ok(el) = tab.wait_for_element("input[type=\"email\"]") {
        el.click().ok();
        std::thread::sleep(std::time::Duration::from_millis(500));
        tab.type_str(email).ok();
        println!("  Email entered");
    }
    // Submit email
    if let Ok(el) = tab.wait_for_element("input[type=\"submit\"]") {
        el.click().ok();
        std::thread::sleep(std::time::Duration::from_secs(2));
        println!("  Submitted email");
    }
    // Fill password
    if let Ok(el) = tab.wait_for_element("input[type=\"password\"]") {
        el.click().ok();
        std::thread::sleep(std::time::Duration::from_millis(500));
        tab.type_str(password).ok();
        println!("  Password entered");
    }
    // Submit password
    if let Ok(el) = tab.find_element("input[type=\"submit\"]") {
        el.click().ok();
        std::thread::sleep(std::time::Duration::from_secs(3));
        println!("  Submitted password");
    }
}
