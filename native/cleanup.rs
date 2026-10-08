//! Text-only interface; current rules never translate, infer words or edit code.
pub trait TextCleaner {
    fn clean(&self, original: &str) -> String;
}
pub struct Conservative;
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
                && (next.is_some_and(|c| [',', '.', ';', ':', '!', '?'].contains(&c))
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
        punctuation(&out.join(" "))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mechanical_cleanup_and_unambiguous_acoustic_fragments() {
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
            assert_eq!(Conservative.clean(before), after);
        }
    }
    #[test]
    fn preserves_meaning_names_dates_negation_english_and_code() {
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
        ] {
            assert_eq!(Conservative.clean(original), original);
        }
    }
}
