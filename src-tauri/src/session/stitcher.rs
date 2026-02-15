use super::AudioSegment;

pub struct Stitcher;

impl Stitcher {
    pub fn stitch_transcripts(segments: &[AudioSegment]) -> Result<String, StitchError> {
        if segments.is_empty() {
            return Ok(String::new());
        }

        let mut full_text = String::new();
        let mut previous_words: Vec<String> = Vec::new();

        for segment in segments {
            let transcript = segment
                .transcript
                .as_ref()
                .ok_or_else(|| StitchError::SegmentNotTranscribed(segment.id.clone()))?;

            let mut words: Vec<String> = transcript
                .text
                .split_whitespace()
                .map(|s: &str| s.to_string())
                .collect();

            if !previous_words.is_empty() && !words.is_empty() {
                let overlap_size = Self::detect_overlap(&previous_words, &words);
                if overlap_size > 0 {
                    tracing::debug!(
                        "Detected overlap of {} words, removing from segment {}",
                        overlap_size,
                        segment.sequence_number
                    );
                    words.drain(0..overlap_size);
                }
            }

            if !full_text.is_empty() && !words.is_empty() {
                full_text.push(' ');
            }
            if !words.is_empty() {
                full_text.push_str(&words.join(" "));
            }

            if !words.is_empty() {
                previous_words = words
                    .iter()
                    .rev()
                    .take(3)
                    .rev()
                    .cloned()
                    .collect();
            }
        }

        let normalized = Self::normalize_text(&full_text);
        Ok(normalized)
    }

    fn detect_overlap(previous: &[String], current: &[String]) -> usize {
        let max_check = std::cmp::min(3, std::cmp::min(previous.len(), current.len()));

        for n in (1..=max_check).rev() {
            let prev_tail: Vec<_> = previous
                .iter()
                .rev()
                .take(n)
                .rev()
                .map(|s| s.to_lowercase())
                .collect();

            let curr_head: Vec<_> = current
                .iter()
                .take(n)
                .map(|s| s.to_lowercase())
                .collect();

            if prev_tail == curr_head {
                return n;
            }
        }

        0
    }

    fn normalize_text(text: &str) -> String {
        let collapsed = collapse_spaces(text);
        let spaced = ensure_space_after_punct(&collapsed);
        let cleaned = remove_space_before_punct(&spaced);
        let capitalized = capitalize_sentences(&cleaned);
        collapse_spaces(&capitalized).trim().to_string()
    }
}

#[derive(Debug)]
pub enum StitchError {
    SegmentNotTranscribed(String),
}

fn is_punct(c: char) -> bool {
    matches!(c, '.' | '!' | '?' | ',')
}

fn collapse_spaces(text: &str) -> String {
    let mut out = String::new();
    let mut in_space = false;

    for ch in text.chars() {
        if ch.is_whitespace() {
            if !in_space {
                out.push(' ');
                in_space = true;
            }
            continue;
        }

        in_space = false;
        out.push(ch);
    }

    out
}

fn ensure_space_after_punct(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        out.push(ch);

        if matches!(ch, '.' | '!' | '?' | ',') {
            if let Some(next) = chars.peek() {
                if !next.is_whitespace() {
                    out.push(' ');
                }
            }
        }
    }

    out
}

fn remove_space_before_punct(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch.is_whitespace() {
            if let Some(next) = chars.peek() {
                if is_punct(*next) {
                    continue;
                }
            }
        }

        out.push(ch);
    }

    out
}

fn capitalize_sentences(text: &str) -> String {
    let mut out = String::new();
    let mut capitalize_next = true;

    for ch in text.chars() {
        if capitalize_next && ch.is_alphabetic() {
            for up in ch.to_uppercase() {
                out.push(up);
            }
            capitalize_next = false;
            continue;
        }

        out.push(ch);

        if matches!(ch, '.' | '!' | '?') {
            capitalize_next = true;
        }
    }

    out
}
