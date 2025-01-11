use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use walkdir::WalkDir;
use tauri::{AppHandle, Emitter, Manager};
use serde_json::json;

#[derive(Deserialize, Serialize)]
enum RenameTarget {
    NAME,
    STEM,
    SUFFIX,
}

impl FromStr for RenameTarget {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "NAME" => Ok(RenameTarget::NAME),
            "STEM" => Ok(RenameTarget::STEM),
            "SUFFIX" => Ok(RenameTarget::SUFFIX),
            _ => Err(format!("Invalid RenameTarget value: {}", s)),
        }
    }
}

fn left_pad(s: &str, width: usize, fill_char: char) -> Result<String, String> {
    if s.len() >= width {
        return Ok(s.to_string());
    }
    let padding = width - s.len();
    let padding_str = fill_char.to_string().repeat(padding);
    Ok(format!("{}{}", padding_str, s))
}

fn right_pad(s: &str, width: usize, fill_char: char) -> Result<String, String> {
    if s.len() >= width {
        return Ok(s.to_string());
    }
    let padding = width - s.len();
    let padding_str = fill_char.to_string().repeat(padding);
    Ok(format!("{}{}", s, padding_str))
}

fn replacement_handler(replacement: &str, serial_number: i64) -> Result<String, String> {
    let mut new_replacement = replacement.to_string();
    let pattern = Regex::new(r"(@?)<([A-Za-z0-9_-]?)enum(-?[1-9]?)>").unwrap();

    if let Some(captures) = pattern.captures(&new_replacement) {
        let captured_str = captures.get(0).map_or("", |m| m.as_str());
        let at_sign = captures.get(1).map_or(false, |m| m.as_str() == "@");

        let padding = captures.get(2).map_or("", |m| m.as_str());
        let number = captures.get(3).map_or("", |m| m.as_str());

        let (padding, number) = if !padding.is_empty() {
            (
                padding.chars().next().unwrap_or('0'),
                number.parse::<i8>().unwrap_or(1),
            )
        } else if !number.is_empty() {
            ('0', number.parse::<i8>().unwrap_or(1))
        } else {
            (' ', 0)
        };

        let padded_serial = if at_sign {
            captured_str[1..].to_string()
        } else {
            if number >= 0 {
                left_pad(&serial_number.to_string(), number as usize, padding)?
            } else {
                right_pad(&serial_number.to_string(), -number as usize, padding)?
            }
        };

        new_replacement = pattern
            .replace_all(&new_replacement, padded_serial)
            .to_string();
    }

    Ok(new_replacement)
}

fn replace_with_count(
    use_regex: bool,
    text: &str,
    pattern: &str,
    replacement: &str,
    count: i8,
) -> Result<(String, String, String), String> {
    if use_regex {
        if let Ok(regex) = Regex::new(pattern) {
            match count {
                0 => {
                    let mut highlighted_parts = Vec::new();
                    let mut replaced_parts = Vec::new();
                    let mut highlighted_replaced_parts = Vec::new();
                    let mut remaining_text = text;
                    let mut count = 1;

                    loop {
                        if let Some(match_) = regex.find(&remaining_text) {
                            let end = match_.end();
                            let (left, right) = remaining_text.split_at(end);
                            let replaced_left = regex.replace(&left, replacement);
                            let highlighted_left = regex.replace(&left, |c: &regex::Captures| {
                                format!(
                                    r#"<span class="highlight">{}<<span><sup>{}<<sup>"#,
                                    c.get(0).unwrap().as_str(),
                                    count.to_string()
                                )
                            });
                            let highlighted_replaced_left = regex
                                .replace(
                                    &left,
                                    format!(
                                        r#"<span class="highlight">{}<<span><sup>{}<<sup>"#,
                                        replacement,
                                        count.to_string()
                                    ),
                                )
                                .to_string();

                            replaced_parts.push(replaced_left.to_string());
                            highlighted_parts.push(highlighted_left.to_string());
                            highlighted_replaced_parts.push(highlighted_replaced_left);

                            remaining_text = right;
                            count += 1;
                            if end == 0 || end == text.len() {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    if !remaining_text.is_empty() {
                        highlighted_parts.push(remaining_text.to_string());
                        replaced_parts.push(remaining_text.to_string());
                        highlighted_replaced_parts.push(remaining_text.to_string());
                    }

                    let highlighted = highlighted_parts.concat();
                    let replaced = replaced_parts.concat();
                    let highlighted_replaced = highlighted_replaced_parts.concat();

                    Ok((highlighted, replaced, highlighted_replaced))
                }
                mut n if n > 0 => {
                    let mut highlighted_parts = Vec::new();
                    let mut replaced_parts = Vec::new();
                    let mut highlighted_replaced_parts = Vec::new();
                    let mut remaining_text = text;
                    let mut count = 1;

                    while n > 0 {
                        if let Some(match_) = regex.find(&remaining_text) {
                            let end = match_.end();
                            let (left, right) = remaining_text.split_at(end);
                            let replaced_left = regex.replace(&left, replacement);
                            let highlighted_left = regex.replace(&left, |c: &regex::Captures| {
                                format!(
                                    r#"<span class="highlight">{}<<span><sup>{}<<sup>"#,
                                    c.get(0).unwrap().as_str(),
                                    count.to_string()
                                )
                            });
                            let highlighted_replaced_left = regex
                                .replace(
                                    &left,
                                    format!(
                                        r#"<span class="highlight">{}<<span><sup>{}<<sup>"#,
                                        replacement,
                                        count.to_string()
                                    ),
                                )
                                .to_string();

                            replaced_parts.push(replaced_left.to_string());
                            highlighted_parts.push(highlighted_left.to_string());
                            highlighted_replaced_parts.push(highlighted_replaced_left);

                            remaining_text = right;
                            n -= 1;
                            count += 1;
                            if end == 0 || end == text.len() {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    if !remaining_text.is_empty() {
                        highlighted_parts.push(remaining_text.to_string());
                        replaced_parts.push(remaining_text.to_string());
                        highlighted_replaced_parts.push(remaining_text.to_string());
                    }

                    let highlighted = highlighted_parts.concat();
                    let replaced = replaced_parts.concat();
                    let highlighted_replaced = highlighted_replaced_parts.concat();

                    Ok((highlighted, replaced, highlighted_replaced))
                }
                mut n => {
                    let mut highlighted_parts = Vec::new();
                    let mut replaced_parts = Vec::new();
                    let mut highlighted_replaced_parts = Vec::new();
                    let mut remaining_text = text;
                    let mut count = 1;

                    while n < 0 {
                        if let Some(match_) = regex.find_iter(&remaining_text).last() {
                            let start = match_.start();
                            let (left, right) = remaining_text.split_at(start);
                            let replaced_right = regex.replace(&right, replacement);
                            // let highlighted_wrapper = regex.replace(&right, |c: &regex::Captures| {
                            //     format!(r#"<span class="highlight">{}<<span><sup>{}<<sup>"#, c.get(0).unwrap().as_str(), count.to_string())
                            // });
                            // let highlighted_right = regex.replace(&right, highlighted_wrapper);
                            // let highlighted_replaced_right = regex
                            //     .replace(&right, format!(r#"<span class="highlight">{}<<span><sup>{}<<sup>"#, replacement, count.to_string())).to_string();
                            let highlighted_right = regex.replace(&right, |c: &regex::Captures| {
                                format!(
                                    r#"<span class="highlight">{}<<span><sup>{}<<sup>"#,
                                    c.get(0).unwrap().as_str(),
                                    count.to_string()
                                )
                            });
                            let highlighted_replaced_right = regex
                                .replace(
                                    &right,
                                    format!(
                                        r#"<span class="highlight">{}<<span><sup>{}<<sup>"#,
                                        replacement,
                                        count.to_string()
                                    ),
                                )
                                .to_string();

                            highlighted_parts.push(highlighted_right.to_string());
                            replaced_parts.push(replaced_right.to_string());
                            highlighted_replaced_parts.push(highlighted_replaced_right);

                            remaining_text = left;
                            n += 1;
                            count += 1;
                            if start == 0 || start == text.len() {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    if !remaining_text.is_empty() {
                        highlighted_parts.push(remaining_text.to_string());
                        replaced_parts.push(remaining_text.to_string());
                        highlighted_replaced_parts.push(remaining_text.to_string());
                    }

                    highlighted_parts.reverse();
                    replaced_parts.reverse();
                    highlighted_replaced_parts.reverse();
                    let highlighted = highlighted_parts.concat();
                    let replaced = replaced_parts.concat();
                    let highlighted_replaced = highlighted_replaced_parts.concat();

                    Ok((highlighted, replaced, highlighted_replaced))
                }
            }
        } else {
            Ok((text.to_string(), text.to_string(), text.to_string()))
        }
    } else {
        if pattern.is_empty() {
            return Ok((text.to_string(), text.to_string(), text.to_string()))
        }
        match count {
            0 => {
                let mut highlighted_parts = Vec::new();
                let mut replaced_parts = Vec::new();
                let mut highlighted_replaced_parts: Vec<String> = Vec::new();
                let mut remaining_text = text;
                let mut count = 1;

                loop {
                    if let Some(start) = remaining_text.find(pattern) {
                        let end = start + pattern.len();
                        let (left, right) = remaining_text.split_at(end);
                        let replaced_left = left.replace(pattern, replacement);
                        let highlighted_pattern = format!(
                            r#"<span class="highlight">{}<<span><sup>{}<<sup>"#,
                            pattern,
                            count.to_string()
                        );
                        let highlighted_left = left.replace(pattern, highlighted_pattern.as_str());
                        let highlighted_replacement = format!(
                            r#"<span class="highlight">{}<<span><sup>{}<<sup>"#,
                            replacement,
                            count.to_string()
                        );
                        let highlighted_replaced_left = left
                            .replace(&pattern, highlighted_replacement.as_str())
                            .to_string();

                        replaced_parts.push(replaced_left);
                        highlighted_parts.push(highlighted_left);
                        highlighted_replaced_parts.push(highlighted_replaced_left);

                        remaining_text = right;
                        count += 1;
                    } else {
                        break;
                    }
                }

                if !remaining_text.is_empty() {
                    highlighted_parts.push(remaining_text.to_string());
                    replaced_parts.push(remaining_text.to_string());
                    highlighted_replaced_parts.push(remaining_text.to_string());
                }

                let highlighted = highlighted_parts.concat();
                let replaced = replaced_parts.concat();
                let highlighted_replaced = highlighted_replaced_parts.concat();

                Ok((highlighted, replaced, highlighted_replaced))
            }
            mut n if n > 0 => {
                let mut highlighted_parts = Vec::new();
                let mut replaced_parts = Vec::new();
                let mut highlighted_replaced_parts: Vec<String> = Vec::new();
                let mut remaining_text = text;
                let mut count = 1;

                while n > 0 {
                    if let Some(start) = remaining_text.find(pattern) {
                        let end = start + pattern.len();
                        let (left, right) = remaining_text.split_at(end);
                        let replaced_left = left.replace(pattern, replacement);
                        let highlighted_pattern = format!(
                            r#"<span class="highlight">{}<<span><sup>{}<<sup>"#,
                            pattern,
                            count.to_string()
                        );
                        let highlighted_left = left.replace(pattern, highlighted_pattern.as_str());
                        let highlight_replacement = format!(
                            r#"<span class="highlight">{}<<span><sup>{}<<sup>"#,
                            replacement,
                            count.to_string()
                        );
                        let highlighted_replaced_left = left
                            .replace(&pattern, highlight_replacement.as_str())
                            .to_string();

                        replaced_parts.push(replaced_left);
                        highlighted_parts.push(highlighted_left);
                        highlighted_replaced_parts.push(highlighted_replaced_left);

                        remaining_text = right;
                        n -= 1;
                        count += 1;
                    } else {
                        break;
                    }
                }

                if !remaining_text.is_empty() {
                    highlighted_parts.push(remaining_text.to_string());
                    replaced_parts.push(remaining_text.to_string());
                    highlighted_replaced_parts.push(remaining_text.to_string());
                }

                let highlighted = highlighted_parts.concat();
                let replaced = replaced_parts.concat();
                let highlighted_replaced = highlighted_replaced_parts.concat();

                Ok((highlighted, replaced, highlighted_replaced))
            }
            mut n => {
                let mut highlighted_parts = Vec::new();
                let mut replaced_parts = Vec::new();
                let mut highlighted_replaced_parts = Vec::new();
                let mut remaining_text = text;
                let mut count = 1;

                while n < 0 {
                    if let Some(pos) = remaining_text.rfind(pattern) {
                        let (left, right) = remaining_text.split_at(pos);
                        let replaced_right = right.replace(pattern, replacement);
                        let highlighted_pattern = format!(
                            r#"<span class="highlight">{}<<span><sup>{}<<sup>"#,
                            pattern,
                            count.to_string()
                        );
                        let highlighted_right =
                            right.replace(pattern, highlighted_pattern.as_str());
                        let highlighted_replacement = format!(
                            r#"<span class="highlight">{}<<span><sup>{}<<sup>"#,
                            replacement,
                            count.to_string()
                        );
                        let highlighted_replaced_right = right
                            .replace(&pattern, &highlighted_replacement)
                            .to_string();

                        highlighted_parts.push(highlighted_right);
                        replaced_parts.push(replaced_right);
                        highlighted_replaced_parts.push(highlighted_replaced_right);

                        remaining_text = left;
                        n += 1;
                        count += 1;
                    } else {
                        break;
                    }
                }

                if !remaining_text.is_empty() {
                    highlighted_parts.push(remaining_text.to_string());
                    replaced_parts.push(remaining_text.to_string());
                    highlighted_replaced_parts.push(remaining_text.to_string());
                }

                highlighted_parts.reverse();
                replaced_parts.reverse();
                highlighted_replaced_parts.reverse();
                let highlighted = highlighted_parts.concat();
                let replaced = replaced_parts.concat();
                let highlighted_replaced = highlighted_replaced_parts.concat();

                Ok((highlighted, replaced, highlighted_replaced))
            }
        }
    }
}

fn with_stem(path: &Path, new_stem: &str) -> Result<PathBuf, String> {
    let mut new_path = path.to_path_buf();
    if let Some(parent) = path.parent() {
        new_path = parent.to_path_buf();
    }
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        new_path = new_path.join(format!("{}.{}", new_stem, ext));
    } else {
        new_path = new_path.join(new_stem);
    }
    Ok(new_path)
}

fn foresight(
    path: &Path,
    pattern: &str,
    replacement: &str,
    use_regex: bool,
    target: RenameTarget,
    count: i8,
) -> Result<(String, String, String, String), String> {
    let default_result = (
        path.to_str().unwrap_or("").to_string(),
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string(),
        path.to_str().unwrap_or("").to_string(),
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string(),
    );
    if !path.exists() {
        return Ok(default_result);
    }

    let (original_path, original_name, target_path, target_name) = match target {
        RenameTarget::NAME => {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                let (highlighted, replaced, highlighted_replaced) =
                    replace_with_count(use_regex, name, pattern, replacement, count)?;
                (
                    path.to_str().unwrap_or("").to_string(),
                    highlighted.to_string().replace("<<", "</"),
                    path.with_file_name(&replaced)
                        .to_str()
                        .unwrap_or("")
                        .to_string(),
                    highlighted_replaced.to_string().replace("<<", "</"),
                )
            } else {
                return Ok(default_result);
            }
        }
        RenameTarget::STEM => {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                let (highlighted, replaced, highlighted_replaced) =
                    replace_with_count(use_regex, stem, pattern, replacement, count as i8)?;
                (
                    path.to_str().unwrap_or("").to_string(),
                    with_stem(path, &highlighted)?
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string()
                        .replace("<<", "</"),
                    with_stem(path, &replaced)?
                        .to_str()
                        .unwrap_or("")
                        .to_string(),
                    with_stem(path, &highlighted_replaced)?
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string()
                        .replace("<<", "</"),
                )
            } else {
                return Ok(default_result);
            }
        }
        RenameTarget::SUFFIX => {
            if let Some(suffix) = path.extension().and_then(|s| s.to_str()) {
                let (highlighted, replaced, highlighted_replaced) =
                    replace_with_count(use_regex, suffix, pattern, replacement, count as i8)?;
                (
                    path.to_str().unwrap_or("").to_string(),
                    path.with_extension(highlighted)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string()
                        .replace("<<", "</"),
                    path.with_extension(&replaced)
                        .to_str()
                        .unwrap_or("")
                        .to_string(),
                    path.with_extension(&highlighted_replaced)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string()
                        .replace("<<", "</"),
                )
            } else {
                return Ok(default_result);
            }
        }
    };

    let new_path = Path::new(&target_path);
    if new_path == path {
        return Ok(default_result);
    }

    if new_path.exists() {
        return Err("Target path already exists".to_string());
    }

    Ok((original_path, original_name, target_path, target_name))
}

fn walk(root: PathBuf, depth: usize, file_filter: &str) -> Result<Vec<PathBuf>, String> {
    let mut builder = WalkDir::new(root);
    if depth > 0 {
        builder = builder.max_depth(depth);
    }
    let entries = builder
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(move|entry| {
            match &file_filter[..] {
                "*" => true,
                "?" => entry.file_type().is_file(),
                "/" => entry.file_type().is_dir(),
                mut ext => {
                    if ext.starts_with('.') {
                        ext = &ext[1..]
                    }
                    if let Some(file_ext) = entry.path().extension() {
                        file_ext.to_string_lossy().to_lowercase() == ext.to_lowercase()
                    } else {
                        false
                    }
                }
            }
        })
        .map(|entry| entry.path().to_path_buf())
        .collect::<Vec<PathBuf>>();
    Ok(entries)
}

#[tauri::command]
async fn foresights(
    app_handle: AppHandle,
    root: PathBuf,
    depth: usize,
    file_filter: &str,
    pattern: String,
    replacement: &str,
    use_regex: bool,
    target: &str,
    count: i8,
) -> Result<(), String> {
    let mut serial_number = 1;
    let paths = walk(root, depth, file_filter)?;

    let mut batch = Vec::new();
    for path in paths {
        let new_replacement = replacement_handler(replacement, serial_number)?;

        let (original_path, original_name, target_path, target_name) = foresight(
            &path,
            pattern.as_str(),
            &new_replacement,
            use_regex,
            target.parse().unwrap_or(RenameTarget::NAME),
            count,
        )?;
        if original_path != target_path {
            serial_number += 1;
        }
        batch.push((original_path, original_name, target_path, target_name));

        if batch.len() == 100 {
            let event = json!(batch);
            app_handle.emit("foresights_event", event).unwrap();
            batch.clear();
        }
    }

    if !batch.is_empty() {
        let event = json!(batch);
        app_handle.emit("foresights_event", event).unwrap();
    }

    Ok(())
}

#[tauri::command]
async fn validate_pattern(pattern: String) -> bool {
    Regex::new(pattern.as_str()).is_ok()
}

#[tauri::command]
async fn renames(pathmap: Vec<(String, String)>) -> Result<Vec<(String, String)>, String> {
    let mut results: Vec<(String, String)> = Vec::new();
    for (original, target) in pathmap {
        if let Ok(canonicalized_path) = fs::canonicalize(&original) {
            if let Ok(canonicalized_new_path) = fs::canonicalize(&target) {
                if canonicalized_path != canonicalized_new_path {
                    if let Err(_) = fs::rename(&original, &target) {
                        results.push((original, target));
                    }
                }
            }
        }
    }
    Ok(results)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let _ = app.get_webview_window("main").expect("No main window").set_focus();
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![foresights, validate_pattern, renames])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
