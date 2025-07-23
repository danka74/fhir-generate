use assert_cmd::Command;
use std::fs;

#[test]
fn test_reduce_datatypes() {
    
}

#[test]
fn test_next_alphabet_char() {
    use next_alphabet_char;
    assert_eq!(next_alphabet_char('A'), 'B');
    assert_eq!(next_alphabet_char('Y'), 'Z');
    assert_eq!(next_alphabet_char('Z'), 'A');
    assert_eq!(next_alphabet_char('a'), 'b');
    assert_eq!(next_alphabet_char('y'), 'z');
    assert_eq!(next_alphabet_char('z'), 'a');
    assert_eq!(next_alphabet_char('1'), 'A');
    assert_eq!(next_alphabet_char('-'), 'A');
}
