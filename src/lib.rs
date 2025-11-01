use scraper::{ElementRef, Html, Selector};
use regex::Regex;
use worker::*;use serde::Serialize;

pub mod selector_generator;
use selector_generator::generate_selector_candidates;

// --- セレクター検証API用のデータ構造 ---
#[derive(Serialize, Debug, Clone)]
struct VerificationResult {
    url: String,
    selector: String,
    is_valid_syntax: bool,
    error_message: Option<String>,
    match_count: usize,
    matches: Vec<ElementInfo>,
}

#[derive(Serialize, Debug, Clone)]
struct ElementInfo {
    tag: String,
    text: String,
    html: String,
}

// --- データ構造 ---
#[derive(Serialize, Debug, Clone)]
pub struct StockData {
    pub name: String,
    pub code: String,
    pub price: String,
    pub change_abs: String,
    pub change_pct: String,
    pub update_time: String,
}

#[derive(Serialize, Debug, Clone)]
struct RankedCandidate {
    text: String,
    score: u32,
    reason: String,
}

#[derive(Serialize, Debug)]
struct DiscoveredData {
    code: String,
    url: String,
    name_candidates: Vec<RankedCandidate>,
    price_candidates: Vec<RankedCandidate>,
    change_abs_candidates: Vec<RankedCandidate>,
    change_pct_candidates: Vec<RankedCandidate>,
}

#[derive(Serialize, Debug)]
struct DynamicScrapeResult {
    data: StockData,
    used_selectors: std::collections::HashMap<String, String>,
}

// --- 汎用セルフヒーリング探索ユーティリティ ---

/// セルフヒーリング対応：フォールバック付きセレクター探索
fn find_with_fallback(document: &Html, selectors: &[&str]) -> Option<String> {
    for &sel_str in selectors {
        match Selector::parse(sel_str) {
            Ok(sel) => {
                let found = document.select(&sel).next();
                console_log!(
                    "[SelectorCheck] {:<60} => {}",
                    sel_str,
                    if found.is_some() { "✅ FOUND" } else { "❌ NONE" }
                );
                if let Some(el) = found {
                    let text = el.text().collect::<String>().trim().to_string();
                    return Some(text);
                }
            }
            Err(err) => {
                console_log!(
                    "[SelectorParseError] {:<60} => ❌ {:?}",
                    sel_str,
                    err
                );
            }
        }
    }
    None
}

fn parse_change_string(combined: &str) -> (String, String) {
    let re = Regex::new(r"([\-+]?[\d,]+(?:\.\d+)?).*?\((.*%?.*)\)").unwrap();
    if let Some(caps) = re.captures(combined) {
        let abs = caps.get(1).map_or("", |m| m.as_str()).trim().to_string();
        let pct = caps.get(2).map_or("", |m| m.as_str()).trim().to_string();
        (abs, pct)
    } else {
        (combined.trim().to_string(), "".to_string())
    }
}

// --- 改良版：セルフヒーリング付きスクレイピング本体 ---
pub fn scrape_stock_page_data(document: &Html) -> Result<StockData> {
    let container_selectors = &[
        "div[class*='PriceBoard__main']",
        "section[class*='PriceBoard']",
        "div[class*='BoardMain']",
        "main section div[class*='price']",
    ];
    let mut container_element = None;
    for &sel_str in container_selectors {
        match Selector::parse(sel_str) {
            Ok(sel) => {
                if let Some(el) = document.select(&sel).next() {
                    container_element = Some(el);
                    break;
                }
            }
            Err(_) => {}
        }
    }
    let container_el = container_element.ok_or_else(|| worker::Error::from("Main container not found"))?;
    let container_html = container_el.html();
    let container = Html::parse_fragment(&container_html);

    let name = find_with_fallback(&container, &["header h2", "div[class*='StockName__name']", "h1", "title"]).unwrap_or_else(|| "UNKNOWN".into());
    let code = find_with_fallback(&container, &["span[class*='PriceBoard__code']", "div[class*='Symbol'] span", "h2 span"]).unwrap_or_else(|| "N/A".into());
    let price = find_with_fallback(&container, &["span[class*='PriceBoard__price'] span[class*='StyledNumber__value']", "div[class*='price'] span", "div.price span"]).unwrap_or_else(|| "N/A".into());
    let combined_change = find_with_fallback(&container, &["div[class*='PriceChangeLabel']", "div[class*='change']", "span[class*='diff']"]).unwrap_or_default();
    let (change_abs, change_pct) = parse_change_string(&combined_change);
    let update_time = find_with_fallback(&container, &["ul[class*='PriceBoard__times'] time", "time[class*='timestamp']", "div[class*='time'] time"]).unwrap_or_else(|| "N/A".into());

    Ok(StockData { name, code, price, change_abs, change_pct, update_time })
}


fn scrape_priceboard_data(document: &Html) -> Result<StockData> {
    let container_selectors = &["div[class*='PriceBoard__main']", "section[class*='PriceBoard']", "div[class*='BoardMain']"];
    let mut container_element = None;
    for &sel_str in container_selectors {
        if let Ok(sel) = Selector::parse(sel_str) {
            if let Some(el) = document.select(&sel).next() {
                container_element = Some(el);
                break;
            }
        }
    }
    let container_el = container_element.ok_or_else(|| worker::Error::from("PriceBoard container not found"))?;
    let container_html = container_el.html();
    let container = Html::parse_fragment(&container_html);

    let name = find_with_fallback(&container, &["header h2", "div[class*='StockName__name']", "h1", "title"]).unwrap_or_else(|| "UNKNOWN".into());
    let code = find_with_fallback(&container, &["span[class*='PriceBoard__code']", "div[class*='Symbol'] span", "h2 span"]).unwrap_or_else(|| "N/A".into());
    let price = find_with_fallback(&container, &["span[class*='PriceBoard__price'] span[class*='StyledNumber__value']", "div[class*='price'] span", "div.price span"]).unwrap_or_else(|| "N/A".into());
    let combined_change = find_with_fallback(&container, &["div[class*='PriceChangeLabel']", "div[class*='change']", "span[class*='diff']"]).unwrap_or_default();
    let (change_abs, change_pct) = parse_change_string(&combined_change);
    let update_time = find_with_fallback(&container, &["ul[class*='PriceBoard__times'] time", "time[class*='timestamp']", "div[class*='time'] time"]).unwrap_or_else(|| "N/A".into());

    Ok(StockData { name, code, price, change_abs, change_pct, update_time })
}

async fn discover_data(code: &str) -> Result<DiscoveredData> {
    let url = format!("https://finance.yahoo.co.jp/quote/{}", code);
    let mut res = Fetch::Url(Url::parse(&url)?).send().await?;
    let html = res.text().await?;
    let document = Html::parse_document(&html);

    let mut name_candidates: Vec<RankedCandidate> = Vec::new();
    let mut base_name = String::new();
    if let Ok(title_selector) = Selector::parse("title") {
        if let Some(title_el) = document.select(&title_selector).next() {
            let title_text = title_el.text().collect::<String>();
            base_name = title_text.split('【').next().unwrap_or("")
                .split('(').next().unwrap_or("")
                .split('：').next().unwrap_or("")
                .trim().to_string();
            if !base_name.is_empty() {
                name_candidates.push(RankedCandidate { text: title_text.clone(), score: 50, reason: "Original <title> text".to_string() });
            }
        }
    }
    if !base_name.is_empty() {
        // Safely parse the heading selector; fall back to simpler selectors if parsing fails.
        let heading_selectors = match Selector::parse("h1, h2") {
            Ok(sel) => sel,
            Err(_) => {
                console_log!("[WARN] Failed to parse selector 'h1, h2', falling back to 'h1'");
                match Selector::parse("h1") {
                    Ok(s) => s,
                    Err(_) => {
                        console_log!("[WARN] Failed to parse fallback selector 'h1', using universal '*' selector");
                        // '*' should always be a valid selector; unwrap is safe here
                        Selector::parse("*").unwrap()
                    }
                }
            }
        };

        for element in document.select(&heading_selectors) {
            let text = element.text().collect::<String>().trim().to_string();
            if text.is_empty() { continue; }
            if text == base_name {
                name_candidates.push(RankedCandidate { text, score: 110, reason: format!("Exact match in <{}>", element.value().name()) });
            } else if text.contains(&base_name) {
                name_candidates.push(RankedCandidate { text, score: 100, reason: format!("Contains base name in <{}>", element.value().name()) });
            }
        }
    }

    let mut price_candidates: Vec<RankedCandidate> = Vec::new();
    // より広いセレクターパターンを試す
    for selector_str in &[
        "[class*='price'], [class*='Price']",
        "span[class*='value'], div[class*='value']",
        "[class*='board'] span, [class*='Board'] span",
        "[data-field='regularMarketPrice']",
        "[class*='quote'], [class*='Quote']",
        "span[class*='last'], div[class*='last']",
        "[class*='current'], [class*='Current']"
    ] {
        if let Ok(sel) = Selector::parse(selector_str) {
            for element in document.select(&sel) {
                let text = element.text().collect::<String>().trim().to_string();
                // 数値っぽい文字列かどうかをチェック（より緩やかな判定）
                if text.chars().any(|c| c.is_ascii_digit()) {
                    let cleaned_text = text.replace(",", "");
                    if let Ok(parsed_price) = cleaned_text.parse::<f64>() {
                        if parsed_price >= 0.0 {
                            let mut score = 50;
                            let class_attr = element.value().attr("class").unwrap_or("");
                            if text.contains(',') { score += 30; }
                            if class_attr.contains("value") { score += 20; }
                            if class_attr.contains("large") { score += 10; }
                            if class_attr.contains("code") || class_attr.contains("symbol") { score -= 40; }
                            price_candidates.push(RankedCandidate { 
                                text: text.clone(), 
                                score, 
                                reason: format!("Found in element with class: {} (selector: {})", class_attr, selector_str) 
                            });
                            
                            // デバッグログ
                            console_log!(
                                "Found price candidate: {} (score: {}, selector: {})", 
                                text, score, selector_str
                            );
                        }
                    }
                }
            }
        }
    }

    // 候補が見つからなかった場合のフォールバック
    if price_candidates.is_empty() {
        console_log!("No price candidates found, trying fallback selectors...");
        // フォールバック: より広いセレクターで数値を探す
        if let Ok(sel) = Selector::parse("span, div") {
            for element in document.select(&sel) {
                let text = element.text().collect::<String>().trim().to_string();
                if text.chars().any(|c| c.is_ascii_digit()) {
                    let cleaned_text = text.replace(",", "");
                    if let Ok(parsed_price) = cleaned_text.parse::<f64>() {
                        if parsed_price >= 0.0 {
                            price_candidates.push(RankedCandidate { 
                                text, 
                                score: 10, // フォールバックなので低いスコア
                                reason: format!("Fallback: found number in {}", element.value().name()) 
                            });
                        }
                    }
                }
            }
        }
    }

    let mut change_abs_candidates: Vec<RankedCandidate> = Vec::new();
    let mut change_pct_candidates: Vec<RankedCandidate> = Vec::new();

    if let Ok(sel) = Selector::parse("[class*='PriceChangeLabel__primary']") {
        for element in document.select(&sel) {
            let text = element.text().collect::<String>().trim().to_string();
            if (text.starts_with('+') || text.starts_with('-')) && text.chars().any(|c| c.is_digit(10)) {
                change_abs_candidates.push(RankedCandidate { text, score: 100, reason: "Found in primary change label".to_string() });
            }
        }
    }

    if let Ok(sel) = Selector::parse("[class*='PriceChangeLabel__secondary']") {
        for element in document.select(&sel) {
            let text = element.text().collect::<String>().trim().to_string();
            if text.contains('%') && text.contains('(') {
                change_pct_candidates.push(RankedCandidate { text, score: 100, reason: "Found in secondary change label".to_string() });
            }
        }
    }

    let mut name_map: std::collections::HashMap<String, RankedCandidate> = std::collections::HashMap::new();
    for candidate in name_candidates { name_map.entry(candidate.text.clone()).and_modify(|e| { if candidate.score > e.score { *e = candidate.clone(); } }).or_insert(candidate); }
    let mut final_name_candidates: Vec<RankedCandidate> = name_map.into_values().collect();
    final_name_candidates.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.text.cmp(&b.text)));

    let mut price_map: std::collections::HashMap<String, RankedCandidate> = std::collections::HashMap::new();
    for candidate in price_candidates { price_map.entry(candidate.text.clone()).and_modify(|e| { if candidate.score > e.score { *e = candidate.clone(); } }).or_insert(candidate); }
    let mut final_price_candidates: Vec<RankedCandidate> = price_map.into_values().collect();
    final_price_candidates.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.text.cmp(&b.text)));

    let mut change_abs_map: std::collections::HashMap<String, RankedCandidate> = std::collections::HashMap::new();
    for candidate in change_abs_candidates { change_abs_map.entry(candidate.text.clone()).and_modify(|e| { if candidate.score > e.score { *e = candidate.clone(); } }).or_insert(candidate); }
    let mut final_change_abs_candidates: Vec<RankedCandidate> = change_abs_map.into_values().collect();
    final_change_abs_candidates.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.text.cmp(&b.text)));

    let mut change_pct_map: std::collections::HashMap<String, RankedCandidate> = std::collections::HashMap::new();
    for candidate in change_pct_candidates { change_pct_map.entry(candidate.text.clone()).and_modify(|e| { if candidate.score > e.score { *e = candidate.clone(); } }).or_insert(candidate); }
    let mut final_change_pct_candidates: Vec<RankedCandidate> = change_pct_map.into_values().collect();
    final_change_pct_candidates.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.text.cmp(&b.text)));

    Ok(DiscoveredData {
        code: code.to_string(),
        url,
        name_candidates: final_name_candidates,
        price_candidates: final_price_candidates,
        change_abs_candidates: final_change_abs_candidates,
        change_pct_candidates: final_change_pct_candidates,
    })
}

async fn discover_index_data(code: &str) -> Result<DiscoveredData> {
    let url = format!("https://finance.yahoo.co.jp/quote/{}", code);
    let mut res = Fetch::Url(Url::parse(&url)?).send().await?;
    let html = res.text().await?;
    let document = Html::parse_document(&html);

    let mut name_candidates: Vec<RankedCandidate> = Vec::new();
    let mut price_candidates: Vec<RankedCandidate> = Vec::new();
    let mut change_abs_candidates: Vec<RankedCandidate> = Vec::new();
    let mut change_pct_candidates: Vec<RankedCandidate> = Vec::new();

    // Try to extract data from window.__PRELOADED_STATE__ JSON
    let re_preloaded_state = Regex::new(r"window\.__PRELOADED_STATE__ = (\{.*?\});").unwrap();
    if let Some(caps) = re_preloaded_state.captures(&html) {
        if let Some(json_str) = caps.get(1).map(|m| m.as_str()) {
            if let Ok(parsed_json) = serde_json::from_str::<serde_json::Value>(json_str) {
                // Extract Name
                if let Some(name_val) = parsed_json["pageInfo"]["title"].as_str() {
                    let cleaned_name = name_val.split(" - ").next().unwrap_or("").trim().to_string();
                    if !cleaned_name.is_empty() {
                        name_candidates.push(RankedCandidate { text: cleaned_name.clone(), score: 100, reason: "Found in __PRELOADED_STATE__ (title)".to_string() });
                        console_log!("[DEBUG] discover_index_data: JSON Name: {}", cleaned_name);
                    }
                }

                // Extract Price, Change_abs, Change_pct from priceBoard
                if let Some(price_board) = parsed_json["priceBoard"].as_object() {
                    if let Some(price_val) = price_board["price"].as_str() {
                        price_candidates.push(RankedCandidate { text: price_val.to_string(), score: 100, reason: "Found in __PRELOADED_STATE__ (price)".to_string() });
                        console_log!("[DEBUG] discover_index_data: JSON Price: {}", price_val);
                    }
                    if let Some(change_val) = price_board["change"].as_str() {
                        change_abs_candidates.push(RankedCandidate { text: change_val.to_string(), score: 100, reason: "Found in __PRELOADED_STATE__ (change_abs)".to_string() });
                        console_log!("[DEBUG] discover_index_data: JSON Change Abs: {}", change_val);
                    }
                    if let Some(change_pct_val) = price_board["changePct"].as_str() {
                        change_pct_candidates.push(RankedCandidate { text: change_pct_val.to_string(), score: 100, reason: "Found in __PRELOADED_STATE__ (change_pct)".to_string() });
                        console_log!("[DEBUG] discover_index_data: JSON Change Pct: {}", change_pct_val);
                    }
                }
            }
        }
    }

    // Fallback for Name if JSON extraction fails
    if name_candidates.is_empty() {
        console_log!("[DEBUG] discover_index_data: JSON name extraction failed, falling back to DOM scraping.");
        // Use title tag as a primary fallback
        if let Ok(sel) = Selector::parse("title") {
            if let Some(el) = document.select(&sel).next() {
                let title_text = el.text().collect::<String>();
                let cleaned_name = title_text.split(" - ").next().unwrap_or("").trim().to_string();
                 if !cleaned_name.is_empty() {
                    name_candidates.push(RankedCandidate { text: cleaned_name, score: 80, reason: "Found in <title> tag (fallback)".to_string() });
                }
            }
        }
        // Use h1 tag as a secondary fallback
        if name_candidates.is_empty() {
             if let Ok(sel) = Selector::parse("h1") {
                if let Some(el) = document.select(&sel).next() {
                    let h1_text = el.text().collect::<String>().trim().to_string();
                    if !h1_text.is_empty() {
                        name_candidates.push(RankedCandidate { text: h1_text, score: 70, reason: "Found in <h1> tag (fallback)".to_string() });
                    }
                }
            }
        }
    }

    // Fallback to DOM scraping for price, change_abs, change_pct if JSON extraction fails or is incomplete
    if price_candidates.is_empty() || change_abs_candidates.is_empty() || change_pct_candidates.is_empty() {
        console_log!("[DEBUG] discover_index_data: JSON price/change extraction failed or incomplete, falling back to DOM scraping.");
        // Price
        if price_candidates.is_empty() {
            if let Ok(sel) = Selector::parse("div[class*='_CommonPriceBoard__priceBlock'] span[class*='_StyledNumber__value']") {
                for element in document.select(&sel) {
                    let text = element.text().collect::<String>().trim().to_string();
                    if !text.starts_with('+') && !text.starts_with('-') {
                        if let Ok(parsed_price) = text.replace(",", "").parse::<f64>() {
                            if parsed_price >= 0.0 {
                                price_candidates.push(RankedCandidate { text: text.clone(), score: 90, reason: "Found in _CommonPriceBoard__priceBlock (fallback)".to_string() });
                                console_log!("[DEBUG] discover_index_data: DOM Fallback Price: {}", text);
                            }
                        }
                    }
                }
            }
        }

        // Broader fallback for price within the main price information block
        if price_candidates.is_empty() {
            if let Ok(sel) = Selector::parse("div[class*='_BasePriceBoard__priceInformation'] span, div[class*='_BasePriceBoard__priceInformation'] div") {
                for element in document.select(&sel) {
                    let text = element.text().collect::<String>().trim().to_string();
                    // Heuristic to distinguish price from change values
                    if text.chars().any(|c| c.is_ascii_digit()) && !text.starts_with('+') && !text.starts_with('-') && !text.contains('%') {
                        let cleaned_text = text.replace(",", "");
                        if let Ok(parsed_price) = cleaned_text.parse::<f64>() {
                            if parsed_price >= 0.0 {
                                price_candidates.push(RankedCandidate { 
                                    text: text.clone(), 
                                    score: 70, // Lower score for broader fallback
                                    reason: format!("Broader fallback in _BasePriceBoard__priceInformation: {}", element.value().name()) 
                                });
                                console_log!("[DEBUG] discover_index_data: Broader DOM Fallback Price: {}", text);
                            }
                        }
                    }
                }
            }
        }

        // Change Absolute
        if change_abs_candidates.is_empty() {
            if let Ok(sel) = Selector::parse("span[class*='_PriceChangeLabel__primary'] span[class*='_StyledNumber__value']") {
                for element in document.select(&sel) {
                    let text = element.text().collect::<String>().trim().to_string();
                    if text.starts_with('+') || text.starts_with('-') {
                        change_abs_candidates.push(RankedCandidate { text: text.clone(), score: 90, reason: "Found in _PriceChangeLabel__primary (fallback)".to_string() });
                        console_log!("[DEBUG] discover_index_data: DOM Fallback Change Abs: {}", text);
                    }
                }
            }
        }

        // Change Percentage
        if change_pct_candidates.is_empty() {
            if let Ok(sel) = Selector::parse("span[class*='_PriceChangeLabel__secondary'] span[class*='_StyledNumber__value']") {
                for element in document.select(&sel) {
                    let text = element.text().collect::<String>().trim().to_string();
                    if !text.is_empty() {
                        change_pct_candidates.push(RankedCandidate { text: text.clone(), score: 90, reason: "Found in _PriceChangeLabel__secondary (fallback)".to_string() });
                        console_log!("[DEBUG] discover_index_data: DOM Fallback Change Pct: {}", text);
                    }
                }
            }
        }
    }

    // Deduplicate and Sort (simplified as JSON should provide unique, high-score candidates)
    let mut final_name_candidates: Vec<RankedCandidate> = name_candidates.into_iter().collect();
    final_name_candidates.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.text.cmp(&b.text)));

    let mut final_price_candidates: Vec<RankedCandidate> = price_candidates.into_iter().collect();
    final_price_candidates.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.text.cmp(&b.text)));

    let mut final_change_abs_candidates: Vec<RankedCandidate> = change_abs_candidates.into_iter().collect();
    final_change_abs_candidates.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.text.cmp(&b.text)));

    let mut final_change_pct_candidates: Vec<RankedCandidate> = change_pct_candidates.into_iter().collect();
    final_change_pct_candidates.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.text.cmp(&b.text)));

    Ok(DiscoveredData {
        code: code.to_string(),
        url,
        name_candidates: final_name_candidates,
        price_candidates: final_price_candidates,
        change_abs_candidates: final_change_abs_candidates,
        change_pct_candidates: final_change_pct_candidates,
    })
}

async fn scrape_dynamically(code: &str) -> Result<DynamicScrapeResult> {
    let url = format!("https://finance.yahoo.co.jp/quote/{}", code);
    let mut res = Fetch::Url(Url::parse(&url)?).send().await?;
    let html = res.text().await?;
    let document = Html::parse_document(&html);

    let discovered = if code.starts_with('^') {
        discover_index_data(code).await?
    } else {
        discover_data(code).await?
    };
    
    let top_name = discovered.name_candidates.get(0).ok_or_else(|| Error::from("Could not find a name candidate."))?;
    // 価格候補がない場合はデバッグ情報を出力
    if discovered.price_candidates.is_empty() {
        console_log!("No price candidates found for code: {}", code);
    }
    let top_price = discovered.price_candidates.get(0).ok_or_else(|| Error::from("Could not find a price candidate."))?;
    let top_change_abs = discovered.change_abs_candidates.get(0).ok_or_else(|| Error::from("Could not find an absolute change candidate."))?;
    let top_change_pct = discovered.change_pct_candidates.get(0).ok_or_else(|| Error::from("Could not find a percentage change candidate."))?;

    let name_selectors = generate_selector_candidates(&html, &top_name.text);
    let price_selectors = generate_selector_candidates(&html, &top_price.text);
    let change_abs_selectors = generate_selector_candidates(&html, &top_change_abs.text);
    let change_pct_selectors = generate_selector_candidates(&html, &top_change_pct.text);

    let best_name_selector = name_selectors.get(0).ok_or_else(|| Error::from("No selector for name"))?;
    let best_price_selector = price_selectors.get(0).ok_or_else(|| Error::from("No selector for price"))?;
    let best_change_abs_selector = change_abs_selectors.get(0).ok_or_else(|| Error::from("No selector for absolute change"))?;
    let best_change_pct_selector = change_pct_selectors.get(0).ok_or_else(|| Error::from("No selector for percentage change"))?;

    // Safely parse generated selectors. If parsing fails, log a warning and use empty string as fallback.
    let name = top_name.text.clone();

    let price = if let Ok(sel) = Selector::parse(best_price_selector) {
        document.select(&sel).find(|el| el.text().collect::<String>().trim() == top_price.text).map(|_| top_price.text.clone()).unwrap_or_default()
    } else {
        console_log!("[WARN] Invalid price selector generated: {}", best_price_selector);
        String::new()
    };

    let change_abs = if let Ok(sel) = Selector::parse(best_change_abs_selector) {
        document.select(&sel).find(|el| el.text().collect::<String>().trim() == top_change_abs.text).map(|_| top_change_abs.text.clone()).unwrap_or_default()
    } else {
        console_log!("[WARN] Invalid change_abs selector generated: {}", best_change_abs_selector);
        String::new()
    };

    let change_pct = if let Ok(sel) = Selector::parse(best_change_pct_selector) {
        document.select(&sel).find(|el| el.text().collect::<String>().trim() == top_change_pct.text).map(|_| top_change_pct.text.clone()).unwrap_or_default()
    } else {
        console_log!("[WARN] Invalid change_pct selector generated: {}", best_change_pct_selector);
        String::new()
    };

    let update_time = find_with_fallback(&document, &["ul[class*='PriceBoard__times'] time", "time[class*='timestamp']"]).unwrap_or_else(|| "N/A".into());

    let stock_data = StockData { name, code: code.to_string(), price, change_abs, change_pct, update_time };

    let mut used_selectors = std::collections::HashMap::new();
    used_selectors.insert("name".to_string(), best_name_selector.clone());
    used_selectors.insert("price".to_string(), best_price_selector.clone());
    used_selectors.insert("change_abs".to_string(), best_change_abs_selector.clone());
    used_selectors.insert("change_pct".to_string(), best_change_pct_selector.clone());

    Ok(DynamicScrapeResult { data: stock_data, used_selectors })
}

async fn scrape_data(code: &str) -> Result<StockData> {
    // 指数コードの場合は、JSON解析を含む新しい動的ロジックを使用
    if code.starts_with('^') {
        let dynamic_result = scrape_dynamically(code).await?;
        return Ok(dynamic_result.data);
    }

    // 指数以外は、既存のロジックを維持
    let url = format!("https://finance.yahoo.co.jp/quote/{}", code);
    let mut res = Fetch::Url(Url::parse(&url)?).send().await?;
    let html = res.text().await?;
    let document = Html::parse_document(&html);

    if code.ends_with(".O") || code.ends_with("=X") {
        scrape_priceboard_data(&document)
    } else {
        scrape_stock_page_data(&document)
    }
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
enum ScrapeResult {
    Success(StockData),
    Error { code: String, error: String },
}

async fn scrape_multiple_data(codes: Vec<String>) -> Vec<ScrapeResult> {
    let mut results = Vec::new();
    for code in codes {
        match scrape_data(&code).await {
            Ok(stock_data) => results.push(ScrapeResult::Success(stock_data)),
            Err(e) => {
                results.push(ScrapeResult::Error { code: code.clone(), error: e.to_string() });
            }
        }
    }
    results
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();
    let router = Router::new();
    router
        .get("/health", |_, _| Response::ok("OK"))
        .get_async("/quote", |req, _ctx| async move {
            let url = req.url()?;
            let mut codes: Vec<String> = Vec::new();
            for (key, value) in url.query_pairs() {
                if key == "code" {
                    for part in value.split(',') {
                        let trimmed_part = part.trim();
                        if !trimmed_part.is_empty() {
                            codes.push(trimmed_part.to_string());
                        }
                    }
                }
            }
            if codes.is_empty() {
                return Response::error("Missing stock code query parameter", 400);
            }
            let results = scrape_multiple_data(codes).await;
            Response::from_json(&results)
        })
        .get_async("/discover-data", |req, _ctx| async move {
            let url = req.url()?;
            let mut code = None;
            for (key, value) in url.query_pairs() {
                if key == "code" {
                    code = Some(value.to_string());
                    break;
                }
            }
            let code = match code {
                Some(c) => c,
                None => return Response::error("Missing 'code' query parameter", 400),
            };
            match discover_data(&code).await {
                Ok(results) => Response::from_json(&results),
                Err(e) => Response::error(format!("Failed to discover data: {}", e), 500),
            }
        })
    .get_async("/scrape-dynamic", |req, _ctx| async move {
            let url = req.url()?;
            let mut codes: Vec<String> = Vec::new();
            for (key, value) in url.query_pairs() {
                if key == "code" {
                    codes.extend(value.split(',').map(|s| s.trim().to_string()));
                }
            }
            if codes.is_empty() {
                return Response::error("Missing 'code' query parameter", 400);
            }
            let futures = codes.iter().map(|code| scrape_dynamically(code));
            let results = futures::future::join_all(futures).await;

            let mut response_data = Vec::new();
            for result in results {
                match result {
                    Ok(data) => match serde_json::to_value(data) {
                        Ok(v) => response_data.push(v),
                        Err(e) => response_data.push(serde_json::json!({ "error": format!("serialization error: {}", e) })),
                    },
                    Err(e) => response_data.push(serde_json::json!({ "error": e.to_string() })),
                }
            }
            Response::from_json(&response_data)
        })
        .get_async("/generate-selectors", |req, _ctx| async move {
            let url = req.url()?;
            let mut target_url = None;
            let mut target_text = None;
            for (key, value) in url.query_pairs() {
                match key.as_ref() {
                    "url" => target_url = Some(value.to_string()),
                    "text" => target_text = Some(value.to_string()),
                    _ => {}
                }
            }
            let (target_url, target_text) = match (target_url, target_text) {
                (Some(u), Some(t)) => (u, t),
                _ => return Response::error("Missing 'url' and 'text' query parameters", 400),
            };
            let mut res = match Fetch::Url(Url::parse(&target_url)?).send().await {
                Ok(res) => res,
                Err(e) => return Response::error(format!("Failed to fetch URL: {}", e), 500),
            };
            let html = match res.text().await {
                Ok(html) => html,
                Err(e) => return Response::error(format!("Failed to read response text: {}", e), 500),
            };

            let selectors = generate_selector_candidates(&html, &target_text);
            Response::from_json(&selectors)
        })
        .get_async("/verify-selector", |req, _ctx| async move {
            let url = req.url()?;
            let mut target_url = None;
            let mut selector_str = None;
            for (key, value) in url.query_pairs() {
                match key.as_ref() {
                    "url" => target_url = Some(value.to_string()),
                    "selector" => selector_str = Some(value.to_string()),
                    _ => {}
                }
            }
            let (target_url, selector_str) = match (target_url, selector_str) {
                (Some(u), Some(s)) => (u, s),
                _ => return Response::error("Missing 'url' and 'selector' query parameters", 400),
            };
            let mut res = match Fetch::Url(Url::parse(&target_url)?).send().await {
                Ok(res) => res,
                Err(e) => return Response::error(format!("Failed to fetch URL: {}", e), 500),
            };
            let html = match res.text().await {
                Ok(html) => html,
                Err(e) => return Response::error(format!("Failed to read response text: {}", e), 500),
            };

            let document = Html::parse_document(&html);
            let mut result = VerificationResult {
                url: target_url,
                selector: selector_str.clone(),
                is_valid_syntax: false,
                error_message: None,
                match_count: 0,
                matches: vec![],
            };

            match Selector::parse(&selector_str) {
                Ok(selector) => {
                    result.is_valid_syntax = true;
                    let matches: Vec<ElementRef> = document.select(&selector).collect();
                    result.match_count = matches.len();
                    for element in matches.iter().take(5) {
                        result.matches.push(ElementInfo {
                            tag: element.value().name().to_string(),
                            text: element.text().collect::<String>().trim().to_string(),
                            html: element.html(),
                        });
                    }
                }
                Err(e) => {
                    result.is_valid_syntax = false;
                    result.error_message = Some(format!("{:?}", e));
                }
            };

            Response::from_json(&result)
        })
        .run(req, env)
        .await
}
