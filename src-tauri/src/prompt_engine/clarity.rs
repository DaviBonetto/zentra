// prompt_engine/clarity.rs — Rules-based PT-BR text cleanup

/// Apply rules-based clarity corrections without LLM
pub fn transform(text: &str) -> String {
    let mut result = text.to_string();

    // 1. Normalize whitespace: multiple spaces → single
    result = collapse_spaces(&result);

    // 2. Common PT-BR replacements
    result = fix_common_typos(&result);

    // 3. Fix punctuation spacing
    result = fix_punctuation(&result);

    // 4. Capitalize first letter of each sentence
    result = capitalize_sentences(&result);

    // 5. Trim
    result.trim().to_string()
}

fn collapse_spaces(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut prev_space = false;
    for ch in text.chars() {
        if ch == ' ' || ch == '\t' {
            if !prev_space {
                result.push(' ');
            }
            prev_space = true;
        } else {
            prev_space = false;
            result.push(ch);
        }
    }
    result
}

fn fix_common_typos(text: &str) -> String {
    let replacements = [
        ("nao ", "não "),
        ("nao,", "não,"),
        ("nao.", "não."),
        (" tb ", " também "),
        (" pq ", " porque "),
        (" vc ", " você "),
        (" eh ", " é "),
        (" q ", " que "),
        ("tah ", "tá "),
        (" oq ", " o que "),
        (" td ", " tudo "),
        (" mt ", " muito "),
        (" ngm ", " ninguém "),
        (" msm ", " mesmo "),
    ];

    let mut result = text.to_string();
    for (from, to) in &replacements {
        result = result.replace(from, to);
    }
    result
}

fn fix_punctuation(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();

    for i in 0..chars.len() {
        let ch = chars[i];

        // Remove space before punctuation
        if (ch == '.' || ch == ',' || ch == '!' || ch == '?' || ch == ':' || ch == ';')
            && !result.is_empty()
            && result.ends_with(' ')
        {
            result.pop(); // Remove trailing space
        }

        result.push(ch);

        // Ensure space after punctuation (if next char is letter)
        if (ch == '.' || ch == ',' || ch == '!' || ch == '?' || ch == ':' || ch == ';')
            && i + 1 < chars.len()
            && chars[i + 1].is_alphabetic()
        {
            result.push(' ');
        }
    }

    result
}

fn capitalize_sentences(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut capitalize_next = true;

    for ch in text.chars() {
        if capitalize_next && ch.is_alphabetic() {
            result.extend(ch.to_uppercase());
            capitalize_next = false;
        } else {
            result.push(ch);
        }

        if ch == '.' || ch == '!' || ch == '?' {
            capitalize_next = true;
        }
    }

    // Ensure text ends with period if it doesn't end with punctuation
    let trimmed = result.trim_end();
    if !trimmed.is_empty() {
        let last = trimmed.chars().last().unwrap();
        if last != '.' && last != '!' && last != '?' {
            result = trimmed.to_string();
            result.push('.');
        }
    }

    result
}
