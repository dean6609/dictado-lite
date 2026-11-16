//! Text-only interface; current rules never translate, infer words or edit code.
use std::path::Path;

pub trait TextCleaner {
    fn clean(&self, original: &str) -> String;
}
/// Mechanical cleanup plus the user's own word list (`replacements.txt`).
#[derive(Default)]
pub struct Conservative {
    replacements: Vec<(String, String)>,
}
impl Conservative {
    /// Rules with the optional word list beside the settings; a missing or
    /// unreadable file simply means no replacements.
    pub fn load() -> Self {
        crate::paths::data_dir()
            .ok()
            .map(|dir| Self::from_file(&dir.join(REPLACEMENTS_FILE)))
            .unwrap_or_default()
    }
    pub fn from_file(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .map(|text| Self::parse(&text))
            .unwrap_or_default()
    }
    /// One `heard = written` pair per line; `#` starts a comment.
    pub fn parse(text: &str) -> Self {
        let replacements = text
            .lines()
            .filter(|line| !line.trim_start().starts_with('#'))
            .filter_map(|line| {
                let (from, to) = line.split_once('=')?;
                let from = from.trim().to_lowercase();
                let to = to.trim().to_string();
                (!from.is_empty() && !to.is_empty()).then_some((from, to))
            })
            .collect();
        Self { replacements }
    }
}
pub const REPLACEMENTS_FILE: &str = "replacements.txt";

fn code_like(text: &str) -> bool {
    text.contains([
        '\n', '\r', '\t', '`', '{', '}', '[', ']', '\\', '_', '@', '$', '=', '<', '>', '*',
    ]) || ["::", "=>", "==", "!=", "->", "://", " = ", ".NET"]
        .iter()
        .any(|marker| text.contains(marker))
        || text
            .split_whitespace()
            .any(|word| word.contains('/') && !word.chars().all(|c| c.is_ascii_digit() || c == '/'))
}
fn filler(token: &str) -> bool {
    let Some(sound) = token.strip_suffix(',') else {
        return false;
    };
    [('u', 'h'), ('u', 'm'), ('e', 'h'), ('e', 'm')]
        .into_iter()
        .any(|(first, repeat)| {
            let mut chars = sound.chars();
            chars.next() == Some(first) && sound.len() >= 4 && chars.all(|c| c == repeat)
        })
}
const MARKS: [char; 6] = [',', '.', ';', ':', '!', '?'];
fn punctuation(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    for (index, &ch) in chars.iter().enumerate() {
        if ch == ' ' {
            let prev = out.chars().next_back();
            let next = chars.get(index + 1).copied();
            let numeric = prev.is_some_and(|c| c.is_ascii_digit())
                && next.is_some_and(|c| ['.', ',', ':', '/', '-'].contains(&c))
                && chars[index + 2..]
                    .iter()
                    .find(|c| **c != ' ')
                    .is_some_and(|c| c.is_ascii_digit());
            if !numeric
                && (next.is_some_and(|c| MARKS.contains(&c))
                    || prev.is_some_and(|c| ['¿', '¡'].contains(&c)))
            {
                continue;
            }
        }
        out.push(ch);
        // Comma/semicolon spacing only between letters, never digit separators.
        if [',', ';'].contains(&ch)
            && chars
                .get(index.wrapping_sub(1))
                .is_some_and(|c| c.is_alphabetic())
            && chars.get(index + 1).is_some_and(|c| c.is_alphabetic())
        {
            out.push(' ');
        }
    }
    out
}
/// Pauses while dictating leave runs of sentence marks (`..`, `. .`, `.,`)
/// and marks before the first word. Keep one mark per run; a period followed
/// by a lowercase word is a pause inside a sentence, not its end.
fn pauses(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        if MARKS.contains(&ch) {
            // Gather the whole run of marks and spaces; keep the strongest mark.
            let mut run_end = index;
            while run_end < chars.len()
                && (MARKS.contains(&chars[run_end]) || chars[run_end] == ' ')
            {
                run_end += 1;
            }
            let run: Vec<char> = chars[index..run_end]
                .iter()
                .copied()
                .filter(|c| MARKS.contains(c))
                .collect();
            let count = run.len();
            let weak = run.iter().copied().find(|c| [',', ';', ':'].contains(c));
            let mut last =
                if let Some(strong) = run.iter().copied().find(|c| ['!', '?'].contains(c)) {
                    strong
                } else if run.contains(&'.') {
                    '.'
                } else {
                    run[run.len() - 1]
                };
            let next = chars[run_end..].iter().find(|c| **c != ' ').copied();
            if last == '.' && next.is_some_and(char::is_lowercase) {
                if let Some(weak) = weak {
                    last = weak;
                }
            }
            let is_decimal = last == '.'
                && count == 1
                && out.chars().next_back().is_some_and(|c| c.is_ascii_digit())
                && chars.get(index + 1).is_some_and(|c| c.is_ascii_digit());
            if out.chars().all(char::is_whitespace) {
                // Marks before the first word carry no information.
            } else if is_decimal {
                out.push('.');
                index += 1;
                continue;
            } else if last == '.' && next.is_some_and(char::is_lowercase) {
                // Mid-sentence pause: join the halves without a false stop.
            } else {
                out.push(last);
            }
            if run_end < chars.len() && !out.is_empty() && !out.ends_with(' ') {
                out.push(' ');
            }
            index = run_end;
            continue;
        }
        out.push(ch);
        index += 1;
    }
    out.trim().to_string()
}
/// Words that continue a clause and almost never open a dictated sentence.
/// A pause before one of them makes the recognizer close the sentence and
/// capitalize; rejoining keeps the speaker's own phrase.
const CONTINUATIONS: [&str; 44] = [
    "y", "e", "o", "u", "ni", "pero", "sino", "porque", "que", "aunque", "para", "con", "de",
    "del", "en", "por", "sin", "sobre", "hasta", "desde", "entre", "hacia", "según", "mientras",
    "and", "or", "nor", "but", "because", "that", "which", "with", "of", "for", "to", "from",
    "into", "than", "whereas", "while", "unless", "until", "although", "whether",
];
fn continuations(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(found) = rest.find(". ") {
        let after = &rest[found + 2..];
        let word: String = after.chars().take_while(|c| c.is_alphabetic()).collect();
        let lower = word.to_lowercase();
        let joins = !out.is_empty() || found > 0;
        if joins
            && word.chars().next().is_some_and(char::is_uppercase)
            && word.chars().skip(1).all(char::is_lowercase)
            && CONTINUATIONS.contains(&lower.as_str())
        {
            out.push_str(&rest[..found]);
            out.push(' ');
            out.push_str(&lower);
            rest = &after[word.len()..];
        } else {
            out.push_str(&rest[..found + 2]);
            rest = after;
        }
    }
    out.push_str(rest);
    out
}
fn replace_words(text: &str, replacements: &[(String, String)]) -> String {
    let mut text = text.to_string();
    for (from, to) in replacements {
        let lower = text.to_lowercase();
        if lower.len() != text.len() {
            // Case folding changed byte offsets; stay safe and skip.
            continue;
        }
        let mut out = String::with_capacity(text.len());
        let mut cursor = 0;
        while let Some(found) = lower[cursor..].find(from.as_str()) {
            let start = cursor + found;
            let end = start + from.len();
            let bounded = lower[..start]
                .chars()
                .next_back()
                .is_none_or(|c| !c.is_alphanumeric())
                && lower[end..]
                    .chars()
                    .next()
                    .is_none_or(|c| !c.is_alphanumeric());
            out.push_str(&text[cursor..start]);
            if bounded {
                let heard = &text[start..end];
                let mut written = to.clone();
                if heard.chars().next().is_some_and(char::is_uppercase) {
                    let mut chars = written.chars();
                    if let Some(first) = chars.next() {
                        written = first.to_uppercase().chain(chars).collect();
                    }
                }
                out.push_str(&written);
            } else {
                out.push_str(heard_slice(&text, start, end));
            }
            cursor = end;
        }
        out.push_str(&text[cursor..]);
        text = out;
    }
    text
}
fn heard_slice(text: &str, start: usize, end: usize) -> &str {
    &text[start..end]
}
impl TextCleaner for Conservative {
    fn clean(&self, original: &str) -> String {
        if code_like(original) {
            return original.into();
        }
        let tokens: Vec<_> = original.split_whitespace().collect();
        let mut acoustic = Vec::with_capacity(tokens.len());
        for (i, &token) in tokens.iter().enumerate() {
            let boundary = i == 0 || tokens[i - 1].ends_with([',', '.', '!', '?']);
            if boundary && filler(token) && i + 1 < tokens.len() {
                continue;
            }
            acoustic.push(token);
        }
        let mut out = Vec::with_capacity(acoustic.len());
        let mut i = 0;
        while i < acoustic.len() {
            let max = 3.min((acoustic.len() - i) / 2);
            let repeat = (3..=max).rev().find(|&len| {
                let part = &acoustic[i..i + len];
                part == &acoustic[i + len..i + 2 * len]
                    && matches!(
                        part,
                        ["voy", "a", "revisar"]
                            | ["voy", "a", "comprobar"]
                            | ["vamos", "a", "revisar"]
                            | ["let", "me", "check"]
                            | ["let", "me", "see"]
                            | ["i", "will", "check"]
                            | ["i", "will", "review"]
                    )
            });
            if let Some(len) = repeat {
                let part = &acoustic[i..i + len];
                out.extend_from_slice(part);
                i += len;
                while i + len <= acoustic.len() && &acoustic[i..i + len] == part {
                    i += len;
                }
            } else {
                out.push(acoustic[i]);
                i += 1;
            }
        }
        let text = continuations(&pauses(&punctuation(&out.join(" "))));
        let text = replace_words(&text, &self.replacements);
        if text.trim().is_empty() {
            original.trim().into()
        } else {
            text
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mechanical_cleanup_and_unambiguous_acoustic_fragments() {
        let cleaner = Conservative::default();
        for (before, after) in [
            ("  Hola  , mundo !  ", "Hola, mundo!"),
            ("Hello,world ; again !", "Hello, world; again!"),
            ("¿ Qué pasa ?", "¿Qué pasa?"),
            ("uhhh, let me check the file", "let me check the file"),
            ("Hola, ehhhh, necesito revisar", "Hola, necesito revisar"),
            (
                "voy a revisar voy a revisar el documento",
                "voy a revisar el documento",
            ),
            (
                "let me check let me check the file",
                "let me check the file",
            ),
        ] {
            assert_eq!(cleaner.clean(before), after);
        }
    }
    #[test]
    fn pauses_leave_one_mark_and_no_false_stops() {
        let cleaner = Conservative::default();
        for (before, after) in [
            ("Hola. . Qué tal.", "Hola. Qué tal."),
            ("Hola... Qué tal...", "Hola. Qué tal."),
            ("Hola.. . Qué tal", "Hola. Qué tal"),
            (". Hola, mundo.", "Hola, mundo."),
            (", . Hola", "Hola"),
            (
                "Quería comentarte. que mañana. no voy.",
                "Quería comentarte que mañana no voy.",
            ),
            (
                "Primero. compramos el pan. Luego. vamos al parque.",
                "Primero compramos el pan. Luego vamos al parque.",
            ),
            ("¿Vienes?. Sí.", "¿Vienes? Sí."),
            ("Vale,. seguimos", "Vale, seguimos"),
            ("Bueno. , lo vemos", "Bueno, lo vemos"),
            ("Son las 5. 30 de la tarde", "Son las 5. 30 de la tarde"),
            ("3.14 y 2.5", "3.14 y 2.5"),
            ("Hola. . .", "Hola."),
            (
                "No voy a poder ir a la reunión. Porque tengo una cita. Luego te llamo.",
                "No voy a poder ir a la reunión porque tengo una cita. Luego te llamo.",
            ),
            (
                "Compramos el pan. Y luego. Vamos al parque.",
                "Compramos el pan y luego. Vamos al parque.",
            ),
            (
                "I will not make it. Because I have an appointment. Talk to you later.",
                "I will not make it because I have an appointment. Talk to you later.",
            ),
            ("Para eso. Necesito tiempo.", "Para eso. Necesito tiempo."),
            ("Es de. De Madrid.", "Es de de Madrid."),
            ("...", "..."),
        ] {
            assert_eq!(cleaner.clean(before), after, "{before:?}");
        }
    }
    #[test]
    fn word_list_replaces_whole_words_case_insensitively() {
        let cleaner = Conservative::parse(
            "# heard = written\nguit jab = GitHub\npul request = pull request\ndoker=Docker\n=bad\nbad=\n",
        );
        assert_eq!(cleaner.replacements.len(), 3);
        assert_eq!(
            cleaner.clean("Subo el commit a guit jab y abro un pul request. Doker falla."),
            "Subo el commit a GitHub y abro un pull request. Docker falla."
        );
        assert_eq!(cleaner.clean("Guit jab es útil"), "GitHub es útil");
        assert_eq!(cleaner.clean("dokers no cambia"), "dokers no cambia");
        assert_eq!(
            Conservative::from_file(Path::new("missing-replacements.txt")).clean("sin cambios"),
            "sin cambios"
        );
    }
    #[test]
    fn preserves_meaning_names_dates_negation_english_and_code() {
        let cleaner = Conservative::default();
        for original in [
            "Bueno, como siempre, pues lo revisamos.",
            "eh, mm, ha, um, uh, son sonidos ambiguos.",
            "Laura García revisa el API y README de GitHub.",
            "no voy mañana no voy mañana",
            "very very very very very very",
            "vamos el martes vamos el martes",
            "15 de octubre 15 de octubre",
            "3 . 14 y 1 , 000",
            "Don't translate the deployment failed message.",
            "fn main() { let count = 1; }",
            "C:\\Users\\Laura\\file.txt",
            "https://example.com?q=hello",
            "first_line  =  value",
            "if (x != 1) return x;",
            "line one\n  line two",
            "Uhhh, es el nombre de un personaje.",
            "Hola, ¿qué tal? Bien, gracias.",
        ] {
            assert_eq!(cleaner.clean(original), original);
        }
    }
}
