# Conservative bilingual cleanup

`TextCleaner` accepts and returns text, leaving a narrow extension point for a
future measured alternative. The current `Conservative` implementation uses no
model, translation, vocabulary substitution or network. It runs after recognition
only when the native tray's basic-cleanup option is enabled (default on).

Collapse spaces and mechanically attach punctuation, preserving numeric
separators. Add comma/semicolon spacing only between letters. Code-like markers,
paths, URLs and multiline content bypass all rules. Long lowercase acoustic
fragments (`uhhh,`, `ummm,`, `ehhh,`, `emmm,` and longer) are removed only at a
sentence/comma boundary when followed by content. Short or capitalized sounds and
ordinary words such as bueno/como/pues remain intact.

Adjacent repeated fragments are limited to seven exact lowercase three-word
phrases: voy a revisar, voy a comprobar, vamos a revisar, let me check, let me see,
i will check, i will review. Arbitrary repeated nouns, names, dates, numbers,
negations, technical phrases and single-word emphasis are preserved. This small
allowlist deliberately misses many possible disfluencies. Repetition can be
intentional; users can turn cleanup off. This is a heuristic, never a claim to
recover the speaker's intent or repair recognition mistakes.

Unit cases exercise both useful edits and protected Spanish/English/code content.
Private recordings are checked locally against the accepted Parakeet baseline;
only numeric outcomes belong in benchmarks. The CLI's explicit `--cleanup` flag
uses the same interface and reports cleanup microseconds; default CLI output
remains unprocessed so raw recognition comparisons stay reproducible.
