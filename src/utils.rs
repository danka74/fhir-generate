use serde_json::Value;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

pub fn load_json_from_file(path: &PathBuf) -> Result<Value, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let value = serde_json::from_reader(reader)?;
    Ok(value)
}

pub fn get_slice_after_last_occurrence(s: &str, c: char) -> Option<String> {
    s.rfind(c)
        .map(|last_index| s[last_index + c.len_utf8()..].to_string())
}

pub fn get_slice_after_first_occurrence(s: &str, c: char) -> Option<String> {
    s.find(c)
        .map(|first_index| s[first_index..].to_string())
}

pub fn count_char_occurrences(s: &str, c: char) -> usize {
    s.chars().filter(|&ch| ch == c).count()
}

pub fn camel_to_spaced_pascal(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        let next = chars.peek();
        if c.is_uppercase() && !result.is_empty() && next.is_some_and(|x| x.is_lowercase()) {
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

pub fn reduce_datatypes(datatypes: &[String]) -> String {
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

// Function to convert an integer to its corresponding alphabetical code.
// The integer 'n' is 0-indexed, meaning 0 corresponds to "A", 1 to "B", 25 to "Z", 26 to "AA", and so on.
pub fn generate_code(mut n: usize) -> String {
    // If n is 0, the code is "A". This is a special case because of how the modulo and division work.
    if n == 0 {
        return "A".to_string();
    }

    let mut result = String::new(); // Initialize an empty string to build the code.

    // Loop until n becomes 0.
    // The logic is similar to converting a number to a different base (base 26 in this case).
    while n > 0 {
        // Decrement n by 1. This is crucial because our alphabet is 0-indexed (A=0, B=1, ..., Z=25),
        // but the modulo operator typically works with 1-indexed remainders for base conversion.
        n -= 1;

        // Calculate the remainder when n is divided by 26.
        // This remainder gives us the current character (0 for 'A', 1 for 'B', etc.).
        let remainder = n % 26;

        // Convert the remainder to a character.
        // 'A' as a u8 (byte) plus the remainder gives the ASCII value of the character.
        // For example, if remainder is 0, 'A' + 0 = 'A'. If remainder is 25, 'A' + 25 = 'Z'.
        let char_code = (b'A' + remainder as u8) as char;

        // Insert the character at the beginning of the result string.
        // We insert at the beginning because we're extracting the characters from right to left (least significant to most significant).
        result.insert(0, char_code);

        // Update n for the next iteration by dividing it by 26.
        // This is the "carry-over" mechanism, moving to the next "digit" in base 26.
        n /= 26;
    }

    result // Return the generated string.
}