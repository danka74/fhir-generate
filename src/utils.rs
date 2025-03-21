use std::fs::File;
use std::io::BufReader;
use serde_json::Value;

pub fn load_json_from_file(path: &String) -> Result<Value, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let value = serde_json::from_reader(reader)?;
    Ok(value)
}

pub fn get_slice_after_last_occurrence(s: &String, c: char) -> Option<String> {
    if let Some(last_index) = s.rfind(c) {
        Some(s[last_index + c.len_utf8()..].to_string())
    } else {
        None
    }
}

pub fn count_char_occurrences(s: &String, c: char) -> usize {
    s.chars().filter(|&ch| ch == c).count()
}

pub fn camel_to_spaced_pascal(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c.is_uppercase() && !result.is_empty() {
            result.push(' ');
        }
        result.push(c);
    }

    result
        .split_whitespace()
        .map(|word| {
            let mut c = word.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn reduce_datatypes(datatypes: &Vec<String>) -> String {
    let mut result = String::new();
    let mut first = true;
    for d in datatypes.iter() {
        if !first {
            result.push_str(", ");
        };
        result.push_str(d);
        first = false;
    }
    result
}