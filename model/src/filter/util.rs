pub fn apply_string_escapes(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(char) = chars.next() {
        match char {
            '\\' => match chars.peek() {
                Some('\\') => {
                    chars.next();
                    output.push('\\');
                }
                Some('"') => {
                    chars.next();
                    output.push('"');
                }
                _ => {
                    output.push(char);
                }
            },
            _ => output.push(char),
        }
    }

    output
}
