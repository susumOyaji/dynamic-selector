use scraper::{Html, Selector};
use worker::*;
use serde::Serialize;

#[derive(Serialize, Debug)]
struct StockData {
    name: String,
    code: String,
    price: String,
    change_abs: String, // 前日比（金額）
    change_pct: String, // 前日比（パーセント）
    update_time: String,
}

fn parse_change_string(combined: &str) -> (String, String) {
    if let Some(paren_index) = combined.find('(') {
        let abs = combined[..paren_index].trim().to_string();
        let pct_part = &combined[paren_index + 1..];
        let pct = if let Some(end_paren_index) = pct_part.find(')') {
            pct_part[..end_paren_index].trim().to_string()
        } else {
            "".to_string()
        };
        (abs, pct)
    } else {
        (combined.trim().to_string(), "".to_string())
    }
}

fn scrape_stock_page_data(document: &Html) -> Result<StockData> {
    let container_sel = Selector::parse("div[class*='PriceBoard__main']").unwrap();
    let container = document.select(&container_sel).next().ok_or_else(|| worker::Error::from("Main container not found"))?;

    let name_sel = Selector::parse("header h2").unwrap();
    let name = container
        .select(&name_sel)
        .next()
        .map(|e| e.text().collect::<String>().trim().to_string())
        .unwrap_or_default();

    let code_sel = Selector::parse("span[class*='PriceBoard__code']").unwrap();
    let code = container
        .select(&code_sel)
        .next()
        .map(|e| e.text().collect::<String>().trim().to_string())
        .unwrap_or_default();

    let price_sel =
        Selector::parse("span[class*='PriceBoard__price'] span[class*='StyledNumber__value']")
            .unwrap();
    let price = container
        .select(&price_sel)
        .next()
        .map(|e| e.text().collect::<String>().trim().to_string())
        .unwrap_or_default();

    let change_sel = Selector::parse("div[class*='PriceChangeLabel']").unwrap();
    let combined_change = container
        .select(&change_sel)
        .next()
        .map(|e| {
            e.text()
                .collect::<String>()
                .replace("前日比", "")
                .replace('\n', " ")
                .trim()
                .to_string()
        })
        .unwrap_or_default();
    let (change_abs, change_pct) = parse_change_string(&combined_change);

    let time_sel = Selector::parse("ul[class*='PriceBoard__times'] time").unwrap();
    let update_time = container
        .select(&time_sel)
        .next()
        .map(|e| e.text().collect::<String>().trim().to_string())
        .unwrap_or_default();

    Ok(StockData {
        name,
        code,
        price,
        change_abs,
        change_pct,
        update_time,
    })
}

fn scrape_index_data(document: &Html, code: &str) -> Result<StockData> {
    let name_sel = Selector::parse("h1").unwrap();
    let raw_name = document
        .select(&name_sel)
        .next()
        .map(|e| e.text().collect::<String>().trim().to_string())
        .unwrap_or_default();
    let name = raw_name.replace("の指数情報・推移", "").trim().to_string();

    let container_sel = Selector::parse("div[class*='_BasePriceBoard__main']").unwrap();
    let container = match document.select(&container_sel).next() {
        Some(c) => c,
        None => return Err(worker::Error::from(format!("Index container not found for {}.", code)))
    };

    let price_block_sel = Selector::parse("div[class*='_BasePriceBoard__price']").unwrap();
    let price_block_text = container
        .select(&price_block_sel)
        .next()
        .map(|e| e.text().collect::<String>())
        .unwrap_or_default();

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

    let mut update_time = "".to_string();
    let list_items_sel = Selector::parse("ul li").unwrap();
    let mut found_realtime = false;
    for li in document.select(&list_items_sel) {
        let text = li.text().collect::<String>();
        if found_realtime {
            update_time = text.trim().to_string();
            break;
        }
        if text.contains("リアルタイム") {
            found_realtime = true;
        }
    }

    Ok(StockData {
        name,
        code: code.to_string(),
        price,
        change_abs,
        change_pct,
        update_time,
    })
}

fn scrape_priceboard_data(document: &Html, code: &str) -> Result<StockData> {
    let container_sel = Selector::parse("div[class*='PriceBoard__main']").unwrap();
    let container = match document.select(&container_sel).next() {
        Some(c) => c,
        None => return Err(worker::Error::from(format!("PriceBoard container not found for {}.", code)))
    };

    let name_sel = Selector::parse("header h2").unwrap();
    let name = container
        .select(&name_sel)
        .next()
        .map(|e| e.text().collect::<String>().trim().to_string())
        .unwrap_or_default();

    let price_sel =
        Selector::parse("span[class*='PriceBoard__price'] span[class*='StyledNumber__value']")
            .unwrap();
    let price = container
        .select(&price_sel)
        .next()
        .map(|e| e.text().collect::<String>().trim().to_string())
        .unwrap_or_default();

    let change_sel = Selector::parse("div[class*='PriceChangeLabel']").unwrap();
    let combined_change = container
        .select(&change_sel)
        .next()
        .map(|e| e.text().collect::<String>().replace("前日比", "").trim().to_string())
        .unwrap_or_default();
    let (change_abs, change_pct) = parse_change_string(&combined_change);

    let time_sel = Selector::parse("ul[class*='PriceBoard__times'] time").unwrap();
    let update_time = container
        .select(&time_sel)
        .next()
        .map(|e| e.text().collect::<String>().trim().to_string())
        .unwrap_or_default();

    Ok(StockData {
        name,
        code: code.to_string(),
        price,
        change_abs,
        change_pct,
        update_time,
    })
}

async fn scrape_data(code: &str) -> Result<StockData> {
    let url = format!("https://finance.yahoo.co.jp/quote/{}", code);
    let mut res = Fetch::Url(Url::parse(&url)?).send().await?;
    let html = res.text().await?;
    let document = Html::parse_document(&html);

    if code.starts_with('^') {
        scrape_index_data(&document, code)
    } else if code.ends_with(".O") || code.ends_with("=X") {
        scrape_priceboard_data(&document, code)
    } else {
        scrape_stock_page_data(&document)
    }
}

async fn scrape_multiple_data(codes: Vec<String>) -> Vec<StockData> {
    let mut results = Vec::new();
    for code in codes {
        match scrape_data(&code).await {
            Ok(stock_data) => results.push(stock_data),
            Err(e) => {
                console_log!("Failed to scrape data for code {}: {}", code, e);
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