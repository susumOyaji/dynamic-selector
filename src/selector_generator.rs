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
                // Update best_match if none exists or current text is shorter than previous best
                let should_replace = match best_match.as_ref() {
                    Some((_, prev_len)) => text_len < *prev_len,
                    None => true,
                };
                if should_replace {
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

    // 1. IDセレクター (最高スコア)
    if let Some(id) = element.value().id() {
        if !id.trim().is_empty() {
            add_candidate(candidates, format!("#{}", id), 100);
        }
    }

    // 2. Classセレクター
    let classes: Vec<_> = element.value().classes().filter(|c| !c.trim().is_empty()).collect();
    if !classes.is_empty() {
        let class_selector = classes.iter().map(|c| format!(".{}", c)).collect::<String>();
        // 全クラス結合 (例: tag.class1.class2)
        add_candidate(candidates, format!("{}{}", tag_name, class_selector), 60);

        for class in &classes {
            // 個別クラス (例: tag.class1)
            add_candidate(candidates, format!("{}.{}", tag_name, class), 40);
            // BEMライクなクラスの基底部分 (例: [class*="block__element"])
            if class.contains("__") {
                 if let Some(base) = class.split("__").next() {
                     if !base.is_empty() {
                        add_candidate(candidates, format!("{}[class*='{}']", tag_name, base), 50);
                     }
                 }
            }
        }
    }

    // 3. 親要素のコンテキストを利用したセレクター
    let mut current = Some(element);
    let mut path_parts = vec![tag_name.to_string()];
    let mut level = 1;

    while let Some(parent) = current.and_then(|el| el.parent_element()) {
        if level > 3 { break; } // 3階層まで遡る

        let parent_tag = parent.value().name();

        // 親がIDを持つ場合 (高スコア)
        if let Some(id) = parent.value().id() {
            let mut parent_path = path_parts.clone();
            parent_path.reverse();
            let selector = format!("#{}{}", id, parent_path.join(" > "));
            add_candidate(candidates, selector, 90 - level * 5); // 階層が浅いほど高スコア
            break; // IDが見つかったらそこで打ち切り
        }

        // 親が特徴的なクラスを持つ場合
        let parent_classes: Vec<_> = parent.value().classes().filter(|c| !c.trim().is_empty()).collect();
        if !parent_classes.is_empty() {
            let specific_class = parent_classes.iter().find(|c| c.contains("__") || c.contains('-'));
            if let Some(s_class) = specific_class {
                let mut parent_path = path_parts.clone();
                parent_path.reverse();
                let selector = format!("{}.{} > {}", parent_tag, s_class, parent_path.join(" > "));
                add_candidate(candidates, selector, 70 - level * 5);
            }
        }

        path_parts.push(parent_tag.to_string());
        current = Some(parent);
        level += 1;
    }

    // 4. その他の属性セレクター
    for (attr, value) in element.value().attrs() {
        let lower_attr = attr.to_lowercase();
        if lower_attr != "class" && lower_attr != "id" && !value.trim().is_empty() {
            add_candidate(candidates, format!("{}[{}='{}']", tag_name, attr, value), 30);
        }
    }

    // 5. タグ名のみ (最低スコア)
    add_candidate(candidates, tag_name.to_string(), 1);
}