use scraper::{Html, Selector};
use worker::*;
use serde::Serialize;

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

// --- 前日比解析 ---
fn parse_change_string(combined: &str) -> (String, String) {
    if let Some(paren_index) = combined.find('(') {
        let abs = combined[..paren_index].trim().to_string();
        let pct_part = &combined[paren_index + 1..];
        let pct = pct_part.split(')').next().unwrap_or("").trim().to_string();
        (abs, pct)
    } else {
        (combined.trim().to_string(), "".to_string())
    }
}

// --- 改良版：セルフヒーリング付きスクレイピング本体 ---
pub fn scrape_stock_page_data(document: &Html) -> Result<StockData> {
    // 1️⃣ メインコンテナ探索 (ElementRef を取得)
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
                    console_log!("[ContainerCheck] {:<60} => ✅ FOUND", sel_str);
                    container_element = Some(el);
                    break; // Found it, stop searching
                } else {
                    console_log!("[ContainerCheck] {:<60} => ❌ NONE", sel_str);
                }
            }
            Err(err) => {
                console_log!("[SelectorParseError] {:<60} => ❌ {:?}", sel_str, err);
            }
        }
    }

    let container_el = container_element.ok_or_else(|| worker::Error::from("Main container not found"))?;

    // 2️⃣ container の内部 HTML を新たな Html にパース（部分解析）
    let container_html = container_el.html();
    let container = Html::parse_fragment(&container_html);

    // 3️⃣ 各項目を自己修復付きで抽出
    let name = find_with_fallback(
        &container,
        &[
            "header h2",
            "div[class*='StockName__name']",
            "h1",
            "title",
        ],
    )
    .unwrap_or_else(|| "UNKNOWN".into());

    let code = find_with_fallback(
        &container,
        &[
            "span[class*='PriceBoard__code']",
            "div[class*='Symbol'] span",
            "h2 span",
        ],
    )
    .unwrap_or_else(|| "N/A".into());

    let price = find_with_fallback(
        &container,
        &[
            "span[class*='PriceBoard__price'] span[class*='StyledNumber__value']",
            "div[class*='price'] span",
            "div.price span",
        ],
    )
    .unwrap_or_else(|| "N/A".into());

    let combined_change = find_with_fallback(
        &container,
        &[
            "div[class*='PriceChangeLabel']",
            "div[class*='change']",
            "span[class*='diff']",
        ],
    )
    .unwrap_or_default();
    let (change_abs, change_pct) = parse_change_string(&combined_change);

    let update_time = find_with_fallback(
        &container,
        &[
            "ul[class*='PriceBoard__times'] time",
            "time[class*='timestamp']",
            "div[class*='time'] time",
        ],
    )
    .unwrap_or_else(|| "N/A".into());

    // 4️⃣ 結果まとめ
    Ok(StockData {
        name,
        code,
        price,
        change_abs,
        change_pct,
        update_time,
    })
}

// --- 以下、src/lib.rs.original より --- 

fn find_time_after_realtime_label(document: &Html) -> Option<String> {
    if let Ok(sel) = Selector::parse("ul li") {
        let mut found_realtime = false;
        for li in document.select(&sel) {
            let text = li.text().collect::<String>();
            if found_realtime {
                return Some(text.trim().to_string());
            }
            if text.contains("リアルタイム") {
                found_realtime = true;
            }
        }
    }
    None
}

fn format_update_time(time_str: &str) -> String {
    if let Some(start) = time_str.find(':') {
        if let Some(end) = time_str.find('）') {
            if start < end {
                return time_str[start + 1..end].trim().to_string();
            }
        }
    }
    time_str.trim().to_string()
}

fn scrape_index_data(document: &Html, code: &str) -> Result<StockData> {
    let raw_name = find_with_fallback(document, &["h1[class*='title']", "h1"]).unwrap_or_default();
    let name = raw_name.replace("の指数情報・推移", "").trim().to_string();

    let price_block_text = find_with_fallback(document, &["div[class*='_BasePriceBoard__price']"]).unwrap_or_default();

    let (price, combined_change) = {
        let change_label = "前日比";
        let time_label = "リアルタイム";

        if let Some(change_start_index) = price_block_text.find(change_label) {
            let price_str = price_block_text[..change_start_index].trim().to_string();
            let rest_of_string = &price_block_text[change_start_index + change_label.len()..];

            let change_str = if let Some(time_start_index) = rest_of_string.find(time_label) {
                rest_of_string[..time_start_index].trim().to_string()
            } else {
                rest_of_string.trim().to_string()
            };
            (price_str, change_str)
        } else {
            (price_block_text.trim().to_string(), "".to_string())
        }
    };
    let (change_abs, change_pct) = parse_change_string(&combined_change);

    let mut update_time = find_with_fallback(document, &[
        "li[class*='__time--localUpdateTime'] > time", // Most specific, from user feedback
        "div[class*='_BasePriceBoard__time'] time"     // Generic fallback
    ]).unwrap_or_default();

    if update_time.is_empty() {
        if let Some(time_from_loop) = find_time_after_realtime_label(document) {
            update_time = time_from_loop;
        }
    }

    Ok(StockData {
        name,
        code: code.to_string(),
        price,
        change_abs,
        change_pct,
        update_time: format_update_time(&update_time),
    })
}

fn scrape_priceboard_data(document: &Html) -> Result<StockData> {
    // 1️⃣ メインコンテナ探索 (ElementRef を取得)
    let container_selectors = &[
        "div[class*='PriceBoard__main']",
        "section[class*='PriceBoard']",
        "div[class*='BoardMain']",
    ];
    let mut container_element = None;
    for &sel_str in container_selectors {
        match Selector::parse(sel_str) {
            Ok(sel) => {
                if let Some(el) = document.select(&sel).next() {
                    console_log!("[ContainerCheck] {:<60} => ✅ FOUND", sel_str);
                    container_element = Some(el);
                    break;
                } else {
                    console_log!("[ContainerCheck] {:<60} => ❌ NONE", sel_str);
                }
            }
            Err(err) => {
                console_log!("[SelectorParseError] {:<60} => ❌ {:?}", sel_str, err);
            }
        }
    }
    let container_el = container_element.ok_or_else(|| worker::Error::from("PriceBoard container not found"))?;

    // 2️⃣ container の内部 HTML を新たな Html にパース（部分解析）
    let container_html = container_el.html();
    let container = Html::parse_fragment(&container_html);

    // 3️⃣ 各項目を自己修復付きで抽出
    let name = find_with_fallback(&container, &["header h2", "div[class*='StockName__name']", "h1", "title"]).unwrap_or_else(|| "UNKNOWN".into());
    let code = find_with_fallback(&container, &["span[class*='PriceBoard__code']", "div[class*='Symbol'] span", "h2 span"]).unwrap_or_else(|| "N/A".into());
    let price = find_with_fallback(&container, &["span[class*='PriceBoard__price'] span[class*='StyledNumber__value']", "div[class*='price'] span", "div.price span"]).unwrap_or_else(|| "N/A".into());
    let combined_change = find_with_fallback(&container, &["div[class*='PriceChangeLabel']", "div[class*='change']", "span[class*='diff']"]).unwrap_or_default();
    let (change_abs, change_pct) = parse_change_string(&combined_change);
    let update_time = find_with_fallback(&container, &["ul[class*='PriceBoard__times'] time", "time[class*='timestamp']", "div[class*='time'] time"]).unwrap_or_else(|| "N/A".into());

    // 4️⃣ 結果まとめ
    Ok(StockData { name, code, price, change_abs, change_pct, update_time })
}

async fn scrape_data(code: &str) -> Result<StockData> {
    let url = format!("https://finance.yahoo.co.jp/quote/{}", code);
    let mut res = Fetch::Url(Url::parse(&url)?).send().await?;
    let html = res.text().await?;
    let document = Html::parse_document(&html);

    if code.starts_with('^') {
        scrape_index_data(&document, code)
    } else if code.ends_with(".O") || code.ends_with("=X") {
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
                results.push(ScrapeResult::Error {
                    code: code.clone(),
                    error: e.to_string(),
                });
            }
        }
    }
    results
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    // 簡易な panic hook（詳細なスタックを出す）
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
        .run(req, env)
        .await
}