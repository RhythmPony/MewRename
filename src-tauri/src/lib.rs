use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::{io, fs};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use walkdir::WalkDir;
use tauri::{AppHandle, Emitter, Manager};
use serde_json::json;

const VEC_LENGTH_LIMIT:u8 = 100;

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

fn pad(s: &str, width: i8, fill_char: char) -> String {
    let abs_width = width.abs() as usize;
    if s.len() >= abs_width {
        return s.to_string();
    }
    let padding = abs_width - s.len();
    let mut padded = String::with_capacity(abs_width);
    match width {
        n if n >= 0 => {
            padded.push_str(&fill_char.to_string().repeat(padding));
            padded.push_str(s);
        }
        _ => {
            padded.push_str(s);
            padded.push_str(&fill_char.to_string().repeat(padding));
        }
    }
    padded
}

fn replacement_handler(replacement: &str, serial_number: i64) -> Result<String, String> {
    let mut new_replacement = replacement.to_string();
    let pattern = Regex::new(r"(@?)<([A-Za-z0-9_-]?)enum(-?[1-9]?)>")
        .map_err(|e| format!("Failed to compile regex: {}", e))?;

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
            ('0', 0)
        };

        let padded_serial = if at_sign {
            captured_str[1..].to_string()
        } else {
            pad(&serial_number.to_string(), number, padding)
        };

        new_replacement = pattern
            .replace_all(&new_replacement, padded_serial)
            .to_string();
    }

    Ok(new_replacement)
}

fn replace_with_captures(original: &str, replacement_caps: &regex::Captures) -> String {
    let re = Regex::new(r"(@?)<:(\d{1,2})>").unwrap();
    re.replace_all(original, |match_caps: &regex::Captures| {
        let at_sign = match_caps.get(1).map_or(false, |m| m.as_str() == "@");
        let num = match_caps.get(2).unwrap().as_str().parse::<usize>().unwrap();
        if at_sign {
            format!("<:{}>", match_caps.get(2).unwrap().as_str())
        } else {
            if num < replacement_caps.len() {
                replacement_caps.get(num).unwrap().as_str().to_string()
            } else {
                "<:invalid_index>".to_string()
            }
        }
    })
    .to_string()
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
            let mut highlighted_parts = Vec::new();
            let mut replaced_parts = Vec::new();
            let mut highlighted_replaced_parts = Vec::new();
            let mut remaining_text = text;

            let iter: Box<dyn Iterator<Item = i8>> = if count == 0 {
                Box::new(std::iter::repeat(0))
            } else {
                Box::new((0..count.abs()).into_iter())
            };

            for (i,_) in iter.enumerate().map(|(i,x)| {(i+1,x)}) {
                if let Some(match_) = if count >= 0 {
                    regex.find(&remaining_text)
                } else {
                    regex.find_iter(&remaining_text).last()
                } {
                    let break_position = if count >= 0 {
                        match_.end()
                    } else {
                        match_.start()
                    };
                    let (left, right) = remaining_text.split_at(break_position);
                    let text_to_be_replaced = if count >= 0 {
                        left
                    } else {
                        right
                    };
                    let replaced = regex.replace(&text_to_be_replaced, |caps: &regex::Captures|{
                        replace_with_captures(replacement, caps)
                    });
                    let highlighted = regex.replace(&text_to_be_replaced, |c: &regex::Captures| {
                        format!(
                            r#"<span class="highlight">{}<<span><sup>{}<<sup>"#,
                            c.get(0).unwrap().as_str(),
                            i.to_string()
                        )
                    });
                    let highlighted_replaced = regex
                        .replace(
                            &text_to_be_replaced,
                            |caps: &regex::Captures|{
                                format!(
                                    r#"<span class="highlight">{}<<span><sup>{}<<sup>"#,
                                    replace_with_captures(replacement, caps),
                                    i.to_string()
                                )
                            }
                        )
                        .to_string();

                    replaced_parts.push(replaced.to_string());
                    highlighted_parts.push(highlighted.to_string());
                    highlighted_replaced_parts.push(highlighted_replaced);

                    remaining_text = if count >= 0 {
                        right
                    } else {
                        left
                    };
                    if break_position == 0 || break_position == text.len() {
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

            if count < 0 {
                highlighted_parts.reverse();
                replaced_parts.reverse();
                highlighted_replaced_parts.reverse();
            }

            let highlighted_text = highlighted_parts.concat();
            let replaced_text = replaced_parts.concat();
            let highlighted_replaced_text = highlighted_replaced_parts.concat();

            Ok((highlighted_text, replaced_text, highlighted_replaced_text))
        } else {
            Ok((text.to_string(), text.to_string(), text.to_string()))
        }
    } else {
        if pattern.is_empty() {
            return Ok((text.to_string(), text.to_string(), text.to_string()))
        }
        let mut highlighted_parts = Vec::new();
        let mut replaced_parts = Vec::new();
        let mut highlighted_replaced_parts: Vec<String> = Vec::new();
        let mut remaining_text = text;

        let iter: Box<dyn Iterator<Item = i8>> = if count == 0 {
            Box::new(std::iter::repeat(0))
        } else {
            Box::new((0..count.abs()).into_iter())
        };

        for (i,_) in iter.enumerate().map(|(i,x)| {(i+1,x)}) {
            if let Some(break_position) = if count >= 0 {
                remaining_text.find(pattern).map(|pos| pos + pattern.len())
            } else {
                remaining_text.rfind(pattern)
            } {
                let (left, right) = remaining_text.split_at(break_position);
                let text_to_be_replaced = if count >= 0 {
                    left
                } else {
                    right
                };
                let replaced = text_to_be_replaced.replace(pattern, replacement);
                let highlighted_pattern = format!(
                    r#"<span class="highlight">{}<<span><sup>{}<<sup>"#,
                    pattern,
                    i.to_string()
                );
                let highlighted = text_to_be_replaced.replace(pattern, highlighted_pattern.as_str());
                let highlighted_replacement = format!(
                    r#"<span class="highlight">{}<<span><sup>{}<<sup>"#,
                    replacement,
                    i.to_string()
                );
                let highlighted_replaced = text_to_be_replaced
                    .replace(&pattern, highlighted_replacement.as_str())
                    .to_string();

                replaced_parts.push(replaced);
                highlighted_parts.push(highlighted);
                highlighted_replaced_parts.push(highlighted_replaced);

                remaining_text = if count >= 0 {
                    right
                } else {
                    left
                };
            } else {
                break;
            }
        }

        if !remaining_text.is_empty() {
            highlighted_parts.push(remaining_text.to_string());
            replaced_parts.push(remaining_text.to_string());
            highlighted_replaced_parts.push(remaining_text.to_string());
        }

        if count < 0 {
            highlighted_parts.reverse();
            replaced_parts.reverse();
            highlighted_replaced_parts.reverse();
        }

        let highlighted_text = highlighted_parts.concat();
        let replaced_text = replaced_parts.concat();
        let highlighted_replaced_text = highlighted_replaced_parts.concat();

        Ok((highlighted_text, replaced_text, highlighted_replaced_text))
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
fn foresight_with_serial(
    path: &Path,
    pattern: &str,
    replacement: &str,
    use_regex: bool,
    target: RenameTarget,
    count: i8,
    serial_number: i64,
) -> Result<(String, String, String, String), String> {
    let new_replacement = replacement_handler(replacement, serial_number)?;

    let (original_path, original_name, target_path, target_name) = foresight(
        &path,
        pattern,
        &new_replacement,
        use_regex,
        target,
        count,
    )?;
    Ok((original_path, original_name, target_path, target_name))
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

    let mut batch: Vec<(String, String, String, String)> = Vec::new();
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

        if batch.len() == VEC_LENGTH_LIMIT as usize {
            let event = json!(batch);
            app_handle.emit("foresights_event", event).unwrap();
            batch.clear();
        }
    }

    if !batch.is_empty() {
        let event = json!(batch);
        app_handle.emit("foresights_event", event).unwrap();
    }

    app_handle.emit("foresights_event", json!(None::<String>)).unwrap();

    Ok(())
}

#[tauri::command]
async fn validate_pattern(pattern: String) -> bool {
    Regex::new(pattern.as_str()).is_ok()
}

#[tauri::command]
async fn is_dir_exists(path: &str) -> Result<bool, String> {
    match fs::read_dir(path).await {
        Ok(_) => Ok(true),
        Err(e) => {
            if e.kind() == io::ErrorKind::NotFound {
                Ok(false)
            } else {
                Err(e.to_string())
            }
        }
    }
}


#[tauri::command]
async fn rename(original_path: String, target_path: String) -> Result<String, String> {
    let original_path = PathBuf::from(original_path);
    let target_path = PathBuf::from(target_path);

    if !original_path.exists() {
        return Err("Original path does not exist.".to_string());
    }

    if target_path.exists() {
        return Err("Target path already exists.".to_string());
    }

    match fs::rename(&original_path, &target_path).await {
        Ok(_) => Ok(target_path.to_string_lossy().to_string()),
        Err(e) => Err(format!("Failed to rename:{}", e)),
    }
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let _ = app.get_webview_window("main").expect("No main window").set_focus();
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![foresight_with_serial, foresights, validate_pattern, rename, is_dir_exists])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
