use scraper::{Element, Html, ElementRef, Selector};
use std::collections::HashMap;

// セレクター候補を生成するメイン関数
pub fn generate_selector_candidates(html_str: &str, target_text: &str) -> Vec<String> {
    let document = Html::parse_document(html_str);
    // セレクター文字列をキー、最高スコアを値とするHashMapを使用
    let mut candidate_map: HashMap<String, u32> = HashMap::new();
    let mut best_match: Option<(ElementRef, usize)> = None;

    if let Ok(selector) = Selector::parse("*") {
        for element in document.select(&selector) {
            let element_text = element.text().collect::<String>();
            if element_text.contains(target_text) {
                let text_len = element_text.len();
                if best_match.is_none() || text_len < best_match.as_ref().unwrap().1 {
                    best_match = Some((element, text_len));
                }
            }
        }
    }

    if let Some((element, _)) = best_match {
        // HashMapを渡してセレクターを生成
        generate_for_element(element, &mut candidate_map);
    }

    // HashMapを(セレクター, スコア)のタプルのVecに変換
    let mut sorted_candidates: Vec<_> = candidate_map.into_iter().collect();
    
    // スコアの降順でソート
    sorted_candidates.sort_by(|a, b| b.1.cmp(&a.1));

    // セレクター文字列だけを抽出
    sorted_candidates.into_iter().map(|(s, _)| s).collect()
}

// 候補をHashMapに追加/更新するヘルパー関数
fn add_candidate(map: &mut HashMap<String, u32>, selector: String, score: u32) {
    // 新しいセレクターを挿入するか、既存のセレクターのスコアをより高い方に更新
    map.entry(selector)
       .and_modify(|e| *e = (*e).max(score))
       .or_insert(score);
}


// 単一の要素からセレクターを生成し、HashMapに追加する
fn generate_for_element(element: ElementRef, candidates: &mut HashMap<String, u32>) {
    let tag_name = element.value().name();

    // 1. IDセレクター
    if let Some(id) = element.value().id() {
        if !id.trim().is_empty() {
            add_candidate(candidates, format!("#{}", id), 100);
        }
    }

    // 2. Classセレクター
    let classes: Vec<_> = element.value().classes().filter(|c| !c.trim().is_empty()).collect();
    if !classes.is_empty() {
        let class_selector = classes.iter().map(|c| format!(".{}", c)).collect::<String>();
        add_candidate(candidates, format!("{}{}", tag_name, class_selector), 50);

        for class in classes {
            add_candidate(candidates, format!("{}.{}", tag_name, class), 40);
            if class.contains("__") { 
                 if let Some(base) = class.split("__").next() {
                     if !base.is_empty() {
                        add_candidate(candidates, format!("{}[class*='{}']", tag_name, base), 45);
                     }
                 }
            }
        }
    }
    
    // 3. その他の属性セレクター
    for (attr, value) in element.value().attrs() {
        let lower_attr = attr.to_lowercase();
        if lower_attr != "class" && lower_attr != "id" && !value.trim().is_empty() {
            add_candidate(candidates, format!("{}[{}='{}']", tag_name, attr, value), 30);
        }
    }

    // 4. 構造セレクター (親子関係)
    if let Some(parent) = element.parent_element() {
        let parent_tag = parent.value().name();
        if let Some(id) = parent.value().id() {
            add_candidate(candidates, format!("#{} > {}", id, tag_name), 80);
        } else {
             let parent_classes: Vec<_> = parent.value().classes().filter(|c| !c.trim().is_empty()).collect();
             if !parent_classes.is_empty() {
                let parent_class_selector = parent_classes.iter().map(|c| format!(".{}", c)).collect::<String>();
                add_candidate(candidates, format!("{}{} > {}", parent_tag, parent_class_selector, tag_name), 25);
             }
        }
    }
    
    // 5. タグ名のみ
    add_candidate(candidates, tag_name.to_string(), 1);
}