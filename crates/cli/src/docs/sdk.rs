//! The SDK page: every documented export of `crates/script/sdk/*.ts`, with its signature, its doc comment and its
//! runnable examples.
//!
//! An example is a doc comment line `@example` followed by a complete film script (imports, `comp(...)`, layers,
//! `export default film`), up to the next `@tag` or the end of the comment. It is written to the page as a
//! ```` ```ts example ```` block, and `tests/sdk_examples.rs` builds and checks every such block, so a snippet that stops
//! working fails the build. Prose in a comment may still mention code in backticks; only `@example` blocks are run.

use std::fmt::Write as _;

use super::page as heading;

/// A documented declaration found in an SDK source file.
#[derive(Debug, PartialEq)]
struct Item {
    signature: String,
    doc: String,
    examples: Vec<String>,
}

/// One comment line without its comment syntax: `/** text`, ` * text`, ` */` all become `text` (indentation inside is kept).
fn clean(line: &str) -> String {
    let t = line.trim();
    let t = t.strip_suffix("*/").unwrap_or(t);
    let t = t.strip_prefix("/**").or_else(|| t.strip_prefix('*')).unwrap_or(t);
    t.strip_prefix(' ').unwrap_or(t).trim_end().to_string()
}

/// Split a comment into its prose (one line) and its `@example` blocks (lines kept as written).
fn split_comment(lines: &[String]) -> (String, Vec<String>) {
    let (mut prose, mut examples): (Vec<&str>, Vec<Vec<&str>>) = (vec![], vec![]);
    let mut in_example = false;
    for line in lines {
        let t = line.trim();
        if t == "@example" {
            in_example = true;
            examples.push(vec![]);
        } else if in_example && !t.starts_with('@') {
            examples.last_mut().expect("an example is open").push(line);
        } else {
            in_example = false;
            if !t.is_empty() {
                prose.push(t);
            }
        }
    }
    let examples = examples
        .into_iter()
        .map(|block| block.join("\n").trim_matches('\n').to_string())
        .filter(|code| !code.trim().is_empty())
        .collect();
    (prose.join(" "), examples)
}

/// The leading `//` comment block of a module, then each `export` preceded by a `/** */` comment.
fn read_module(src: &str) -> (String, Vec<Item>) {
    let head: Vec<_> = src.lines().take_while(|l| l.starts_with("//")).map(|l| l.trim_start_matches('/').trim()).collect();
    let (mut items, mut comment, mut open) = (vec![], Vec::<String>::new(), false);
    for line in src.lines() {
        let t = line.trim();
        if t.starts_with("/**") {
            comment = vec![clean(t)];
            open = !t.ends_with("*/");
        } else if open {
            open = !t.ends_with("*/");
            comment.push(clean(t));
        } else if line.starts_with("export ") && !line.starts_with("export {") && !line.starts_with("export *") && !line.starts_with("export type {") {
            if !comment.is_empty() {
                let line = line.split(" => ").next().unwrap_or(line);
                let signature = line.trim_end().trim_end_matches('{').trim_end().to_string();
                let (doc, examples) = split_comment(&std::mem::take(&mut comment));
                items.push(Item { signature, doc, examples });
            }
        } else if !t.is_empty() {
            comment.clear();
        }
    }
    (head.join(" ").trim().to_string(), items)
}

pub fn page() -> String {
    let mut s = heading(
        "TypeScript SDK",
        "The functions a film imports from `\"turbomotion\"`, taken from the doc comments on the exports in `crates/script/sdk`. Only documented exports are listed. Blocks marked `ts example` are whole film scripts: save one as `film.ts` and `turbomotion sdk build film.ts` builds it (a test builds and checks every one).",
    );
    let mut sources = turbomotion_script::sdk_sources().to_vec();
    sources.retain(|(name, _)| *name != "index");
    sources.sort_by_key(|(name, _)| *name);
    for (name, src) in sources {
        let (head, items) = read_module(src);
        if items.is_empty() {
            continue;
        }
        let _ = writeln!(s, "## `{name}`\n");
        if !head.is_empty() {
            let _ = writeln!(s, "{head}\n");
        }
        for i in items {
            let _ = writeln!(s, "```ts\n{}\n```\n{}\n", i.signature, i.doc);
            for code in &i.examples {
                let _ = writeln!(s, "```ts example\n{code}\n```\n");
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_one_line_comment_is_its_prose() {
        let (_, items) = read_module("// head\n\n/** Says hi. */\nexport const hi = (): string => \"hi\";\n");
        assert_eq!(items, [Item { signature: "export const hi = (): string".into(), doc: "Says hi.".into(), examples: vec![] }]);
    }

    #[test]
    fn a_multi_line_comment_joins_its_prose_and_keeps_each_example_as_written() {
        let src = "/**\n * First line.\n * Second line.\n * @example\n * import { a } from \"x\";\n * if (a) {\n *   run();\n * }\n * @example\n * second();\n */\nexport function f(): void {\n";
        let (_, items) = read_module(src);
        assert_eq!(items[0].doc, "First line. Second line.");
        assert_eq!(items[0].examples, ["import { a } from \"x\";\nif (a) {\n  run();\n}", "second();"]);
    }

    #[test]
    fn a_tag_after_an_example_ends_it_and_a_stray_comment_is_not_attached_to_the_next_export() {
        let (_, items) = read_module("/**\n * Doc.\n * @example\n * code();\n * @deprecated use g\n */\nexport function f(): void {\n\n/** orphan */\nconst x = 1;\nexport function g(): void {\n");
        assert_eq!(items.len(), 1);
        assert_eq!((items[0].doc.as_str(), items[0].examples.as_slice()), ("Doc. @deprecated use g", ["code();".to_string()].as_slice()));
    }

    #[test]
    fn the_page_prints_an_example_as_a_ts_example_block() {
        let text = page();
        assert!(text.contains("```ts example\n"), "the SDK documents no runnable example");
        assert!(!text.contains("@example"), "the tag itself is not printed");
    }
}
