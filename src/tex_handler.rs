// /// Convert from a string to a Script object.
// pub use crate::tex_handler::parse::to_script as parse;

use crate::script::{ContainerKind, Script, SeriesEntry, TextContainer, TextSpan};
use log::warn;
use regex::Regex;

/// A thin wrapper around a String, used to represent a .tex formatted string.
/// Also includes a few convenience methods for parsing/exporting.
pub struct Tex {
    pub text: String,
}

impl From<&str> for Tex {
    fn from(value: &str) -> Self {
        Self {
            text: value.to_string(),
        }
    }
}

impl From<String> for Tex {
    fn from(value: String) -> Self {
        Self { text: value }
    }
}

impl Tex {
    /** Remove the .tex idioms for normal text, such as \textellipsis ⟶ ...

    # Arguments

    * `s` - a string slice representing the .tex string to reparse

    # Return

    * `String` - a parsed version of the string that removes the tex-specific formatting

    Specifically, the following are converted:

    * `\ldots` and `\textellipsis` (or with {}) ⟶ `...` or `... `
    * TeX quotation marks are fixed: `"abc"`
    * un-escape characters: `\$\&\%` ⟶ `$&%`
    * `\kaosmile{}` ⟶ `^_^`
    * `\Tilde{}` ⟶ `∼`
    * TeX href `\href{URL}{TEXT}` ⟶ Markdown style `[TEXT](URL)`
    * after those substitutions, extraneous spaces are removed

    # Examples
    ```
    # use lilscript::tex_handler::Tex;
    let s = Tex::unescaped(r"This is some text\textellipsis{} and some more text.");
    assert_eq!(s, "This is some text... and some more text.");
    ```

    ```
    # use lilscript::tex_handler::Tex;
    let s = Tex::unescaped(r"This is\anotherCommand{3} some text\textellipsis{} and some more text.");
    assert_eq!(s, r"This is\anotherCommand{3} some text... and some more text.");
    ```
    */
    pub fn unescaped(s: &str) -> String {
        // handle ellipses, either with or without trailing space
        let s = s
            .replace(r"\ldots{}", "... ")
            .replace(r"\ldots", "...")
            .replace(r"\textellipsis{}", "... ")
            .replace(r"\textellipsis", "...");

        // handle quotation marks: ``abc'' -> "abc"
        let re = Regex::new(r"``(.*?)''").unwrap();
        let s = re.replace_all(&s, "\"$1\"");

        // handle the special single-characters
        let re = Regex::new(r"\\([%&$])").unwrap();
        let s = re.replace_all(&s, "$1");

        // handle a few custom commands
        let re = Regex::new(r"\\kaosmile(\{\})?").unwrap();
        let s = re.replace_all(&s, "^_^ ");

        let re = Regex::new(r"\\Tilde(\{\})?").unwrap();
        let s = re.replace_all(&s, "\u{223C}");

        // handle embedded link (convert to markdown format because...)
        let re = Regex::new(r"\\href\{(.*?)\}\{(.*?)\}").unwrap();
        let s = re.replace_all(&s, "[$2]($1)");

        // remove any unnecessarily duplicated spaces
        let re = Regex::new(r"[[:space:]]+").unwrap();
        let s = re.replace_all(&s, " ");

        // remove trailing space
        let s = s.trim();

        s.to_string()
    }

    /** The same as Tex::prettified, but done in-place.

    # Examples:

    ```
    # use lilscript::tex_handler::Tex;
    let mut tex = Tex::from(r"This is some text\textellipsis{} and some more text.");
    tex.unescape();
    assert_eq!(tex.text, "This is some text... and some more text.");
    ```
    */
    pub fn unescape(&mut self) {
        self.text = Tex::unescaped(self.text.as_str());
    }
}

impl TryFrom<&Tex> for TextContainer {
    type Error = String;

    fn try_from(value: &Tex) -> Result<Self, Self::Error> {
        let text = Tex::unescaped(&value.text);
        let re = Regex::new(r"^\\(.*?)\{(.*)\}$").unwrap();
        let captures = re
            .captures(&text)
            .ok_or(format!("Invalid tex line: {}", &text))?;
        let command = captures.get(1).unwrap().as_str();
        let remainder = captures.get(2).unwrap().as_str();

        let kind = match command {
            "spoken" => ContainerKind::Spoken,
            "stagedir" => ContainerKind::StageDir,
            "listener" => ContainerKind::ListenerDialogue,
            "sfx" => ContainerKind::Sfx,
            _ => {
                warn!("Could not identify container kind for command: {}", command);
                ContainerKind::PlainText
            }
        };

        // remainder will have one of the two forms:
        // form 1: "This is some text."
        // form 2: "This is some text \direct{a direction} and more text."
        // We need to split out these inline directions (or anything else) that occur in the middle.
        let re = Regex::new(r"\\.+?\{.*?\}").unwrap();

        let mut spans: Vec<TextSpan> = Vec::new();
        for s in regex_partition(re, &remainder) {
            if s.is_empty() {
                // ignore empty spans
                continue;
            }

            let t = Tex::from(s);
            if let Ok(span) = TextSpan::try_from(&t) {
                spans.push(span);
            } else {
                return Err(format!(
                    "[TextContainer::try_from<&Tex>] Could not parse span {}",
                    t.text
                ));
            }
        }

        let container = Self { kind, spans };
        Ok(container)
    }
}

impl TryFrom<&Tex> for TextSpan {
    type Error = String;

    fn try_from(value: &Tex) -> Result<Self, Self::Error> {
        let text = Tex::unescaped(&value.text);
        let re = Regex::new(r"\\(.+)\{(.*)\}").unwrap();

        match re.captures(&text) {
            None => {
                // this is just a block of text!
                let value = Tex::unescaped(value.text.trim());
                Ok(TextSpan::normal(&value))
            }
            Some(cap) => {
                // this is a command
                let command = cap.get(1).unwrap().as_str();
                let arg = cap.get(2).unwrap().as_str().trim();
                let arg = Tex::unescaped(arg);

                match command {
                    "direct" => Ok(TextSpan::inline(&arg)),
                    "ul" => Ok(TextSpan::emphasis(&arg)),
                    _ => unreachable!(),
                }
            }
        }
    }
}

impl TryFrom<&Tex> for Script {
    type Error = String;

    /// Attempt to create a Script from the give .tex file.
    fn try_from(value: &Tex) -> Result<Self, Self::Error> {
        // try to process the header information
        let title = search_tex(r"renewcommand\{\\SceneName\}", &value.text)
            .ok_or("Could not parse title")?;
        let author = search_tex("scriptAuthor", &value.text).ok_or("Could not parse author")?;

        let series = search_tex("scriptSeries", &value.text).ok_or("Could not find series")?;
        let series = SeriesEntry::from(series);

        let tags = search_tex("scriptTags", &value.text).ok_or("Could not find tags")?;
        let tags: Vec<String> = Regex::new(r"\[(.*?)\]")
            .unwrap()
            .captures_iter(&tags)
            .map(|c| c.get(1).unwrap().as_str().to_owned())
            .collect();

        let summary = search_tex("summary", &value.text).ok_or("Could not find summary")?;

        let index = match Regex::new(r"\\clearpage").unwrap().find(&value.text) {
            None => 0,
            Some(m) => m.end(),
        };
        let text = &value.text[index..].replace(r"\end{document}", "");

        let mut paragraphs: Vec<TextContainer> = Vec::new();
        for line in text.split("\n").filter(|line| !line.is_empty()) {
            let tex = Tex::from(line);
            let container = TextContainer::try_from(&tex).map_err(|err| {
                format!(
                    "[Script::try_from<&Tex>] Could not parse line: \"{}\" — via: {}",
                    line, err
                )
            })?;

            paragraphs.push(container);
        }

        // TODO: Add parsing for date
        // TODO: Add parsing for characters
        let script = Script {
            author: author.to_owned(),
            title: title.to_owned(),
            series,
            tags,
            summary: summary.to_owned(),
            paragraphs,
            ..Default::default()
        };

        Ok(script)
    }
}

/** Partition the given string according to the given pattern.
Like Regex::split, except we preserve the delimiters.

# Arguments

* `delimit_re` - a `Regex` object that finds the partitioning blocks
* `to_partition` - a string silce that we will be partitioning

# Returns

* `Vec<&str>` - the individual components of the partition (lifetime matches with `to_partition`)

# Examples

```
# use regex::Regex;
# use lilscript::tex_handler::regex_partition;
let s = "ABCCQBCPCCC";
let re = Regex::new("C+").unwrap();
let v = regex_partition(re, &s);
assert_eq!(v, vec!["AB", "CC", "QB", "C", "P", "CCC"]);
```

If we instead used `Regex::split`:
```
# use regex::Regex;
# let s = "ABCCQBCPCCC";
# let re = Regex::new("C+").unwrap();
let w: Vec<&str> = re.split(&s).collect();
assert_eq!(w, vec!["AB", "QB", "P", ""]);  // we've lost the delimiters!
```
*/
pub fn regex_partition<'h>(delimit_re: Regex, to_partition: &'h str) -> Vec<&'h str> {
    let mut results: Vec<&'h str> = Vec::new();

    let mut i = 0;
    delimit_re.find_iter(&to_partition).for_each(|m| {
        let before = &to_partition[i..m.start()];
        let block = m.as_str();

        results.push(before);
        results.push(block);

        i = m.end();
    });

    // get anything after the last delimiter
    let tail = &to_partition[i..];
    if !tail.is_empty() {
        results.push(tail);
    }

    results
}

/** Search a string of .tex formatted text for the value corresponding to a particular function.

# Arguments

- `command_key` - a string slice of the command that we're searching for
- `string` - a string slice of the text to search

# Return

- `Some(value)` if the command was found
- `None` otherwise

# Examples
```
# use lilscript::tex_handler::search_tex;
let s = r"This is some text with a \randomCommand{6} and another \differentCommand{-3px}.";

let value = search_tex("randomCommand", s).unwrap();
assert_eq!(value, "6");

let value = search_tex("differentCommand", s).unwrap();
assert_eq!(value, "-3px");
```

```
# use lilscript::tex_handler::search_tex;
# let s = r"This is some text with a \randomCommand{6} and another \differentCommand{-3px}.";
let value = search_tex("nonexistentCommand", s);
assert!(value.is_none());
```
*/
pub fn search_tex<'a>(command_key: &str, string: &'a str) -> Option<&'a str> {
    let pattern = format!(r"\\{x}\{{(?P<value>.*?)\}}", x = command_key);
    let re = Regex::new(&pattern);

    if re.is_err() {
        // the regex failed to be initialised
        return None;
    }

    let re = re.unwrap();
    match re.captures(string) {
        Some(captures) => Some(captures.name("value").unwrap().as_str()),
        None => None,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_text_span_parse_normal() {
        let tex = Tex::from("This is some text");
        let span = TextSpan::try_from(&tex).unwrap();

        let expected = TextSpan::normal("This is some text");
        assert_eq!(span, expected);
    }

    #[test]
    fn test_text_span_parse_inline() {
        let tex = Tex::from("\\direct{an inline!}");
        let span = TextSpan::try_from(&tex).unwrap();

        let expected = TextSpan::inline("an inline!");
        assert_eq!(span, expected);
    }

    #[test]
    fn test_text_span_parse_emphasis() {
        let tex = Tex::from("\\ul{EMPHASIS}");
        let span = TextSpan::try_from(&tex).unwrap();

        let expected = TextSpan::emphasis("EMPHASIS");
        assert_eq!(span, expected);
    }

    #[test]
    fn test_text_container_parse_one_span() {
        let tex = Tex::from("\\spoken{This is some text.}");
        let container = TextContainer::try_from(&tex).unwrap();

        let spans = vec![TextSpan::normal("This is some text.")];
        let expected = TextContainer {
            kind: ContainerKind::Spoken,
            spans,
        };

        assert_eq!(container, expected);
    }

    #[test]
    fn test_text_container_parse_multiple_spans() {
        let tex = Tex::from(
            "\\spoken{This is some text. \\direct{an inline direction} And some more dialogue.}",
        );
        let container = TextContainer::try_from(&tex).unwrap();

        let spans = vec![
            TextSpan::normal("This is some text."),
            TextSpan::inline("an inline direction"),
            TextSpan::normal("And some more dialogue."),
        ];
        let expected = TextContainer {
            kind: ContainerKind::Spoken,
            spans,
        };

        assert_eq!(container, expected);
    }

    #[test]
    fn test_text_container_parse_multiple_spans_2() {
        let tex = Tex::from("\\listener{\\direct{slowly, quietly} some text?}");
        let container = TextContainer::try_from(&tex).unwrap();

        let spans = vec![
            TextSpan::inline("slowly, quietly"),
            TextSpan::normal("some text?"),
        ];
        let expected = TextContainer {
            kind: ContainerKind::ListenerDialogue,
            spans,
        };

        assert_eq!(container, expected);
    }

    #[test]
    fn test_regex_partition() {
        let s = "ABCCQBCPCCCS";
        let re = Regex::new("C+").unwrap();
        let v = regex_partition(re, &s);
        assert_eq!(v, vec!["AB", "CC", "QB", "C", "P", "CCC", "S"]);
    }

    #[test]
    fn test_regex_partition_trailing_delim() {
        let s = "ABCCQBCPCCC";
        let re = Regex::new("C+").unwrap();
        let v = regex_partition(re, &s);
        assert_eq!(v, vec!["AB", "CC", "QB", "C", "P", "CCC"]);
    }

    #[test]
    fn test_search_tex_success() {
        let contents = r"blah blah \randomCommand{7} and more blah.";
        let value = search_tex("randomCommand", &contents).unwrap();
        assert_eq!(value, "7");
    }

    #[test]
    fn test_search_tex_fail() {
        let contents = r"blah blah \randomCommand{7} and more blah.";
        let value = search_tex("differentCommand", &contents);
        assert!(value.is_none());
    }

    #[test]
    fn test_unescaped_ellipsis() {
        let s = r"This is some text\textellipsis{} and some\ldots{} more text\textellipsis?";
        let expected = "This is some text... and some... more text...?";

        assert_eq!(Tex::unescaped(s), expected);
    }

    #[test]
    fn test_unescaped_symbols() {
        let s = r"This is some text\$ with \& a few \%symbols thrown in.";
        let expected = "This is some text$ with & a few %symbols thrown in.";

        assert_eq!(Tex::unescaped(s), expected);
    }

    #[test]
    fn test_unescaped_dupe_spaces() {
        let s = r"This is some      normal text, except there is additional space in the middle";
        let expected = "This is some normal text, except there is additional space in the middle";
        assert_eq!(Tex::unescaped(s), expected);
    }

    #[test]
    fn test_unescaped_custom_commands() {
        let s = r"This is some text, with some curious stuff\Tilde \Tilde{} \kaosmile{}";
        let expected = "This is some text, with some curious stuff\u{223C} \u{223C} ^_^";
        assert_eq!(Tex::unescaped(s), expected);
    }

    #[test]
    fn test_unescaped_href() {
        let s = r"This is some text with a \href{https://google.com}{link} in it.";
        let expected = "This is some text with a [link](https://google.com) in it.";
        assert_eq!(Tex::unescaped(s), expected);
    }

    #[test]
    fn test_prettify_ellipsis() {
        let mut tex =
            Tex::from(r"This is some text\textellipsis{} and some\ldots{} more text\textellipsis?");
        tex.unescape();
        assert_eq!(tex.text, "This is some text... and some... more text...?")
    }

    #[test]
    fn test_prettify_symbols() {
        let mut tex = Tex::from(r"This is some text\$ with \& a few \%symbols thrown in.");
        tex.unescape();
        assert_eq!(
            tex.text,
            "This is some text$ with & a few %symbols thrown in."
        );
    }

    #[test]
    fn test_prettify_dupe_spaces() {
        let mut tex = Tex::from(
            r"This is some very      normal text, except there is additional space in the middle",
        );
        tex.unescape();
        assert_eq!(
            tex.text,
            "This is some very normal text, except there is additional space in the middle"
        )
    }

    #[test]
    fn test_prettify_custom_commands() {
        let mut tex = Tex::from(
            r"This is some text, but it ends with some curious stuff\Tilde \Tilde{} \kaosmile{}",
        );
        tex.unescape();
        assert_eq!(
            tex.text,
            "This is some text, but it ends with some curious stuff\u{223C} \u{223C} ^_^"
        );
    }

    #[test]
    fn test_prettify_href() {
        let mut tex = Tex::from(r"This is some text with a \href{https://google.com}{link} in it.");
        tex.unescape();
        assert_eq!(
            tex.text,
            "This is some text with a [link](https://google.com) in it."
        );
    }
}

// /// Handle the re-exporting of a script into .tex format
// mod export {
//     use crate::script::{Script, TextBlock};

//     /**
//     Format a value between braces.

//     # Arguments

//     * `value` - a string slice containing the value to put between braces

//     # Return

//     * `&str` - a string slice containing the given value between braces

//     # Examples

//     ```ignore
//     # use lilscript::tex_handler::export::embrace;
//     let values: Vec<&str> = vec!["a", "6", "this", "word", ""];
//     let actual = values.into_iter().map(embrace).collect::<Vec<String>>();
//     let expected = vec!["{a}", "{6}", "{this}", "{word}", "{}"];

//     assert_eq!(actual, expected);
//     ```
//     */
//     fn embrace(value: &str) -> String {
//         const OPEN_BRACE: char = '{';
//         const CLOSE_BRACE: char = '}';
//         format!("{}{}{}", OPEN_BRACE, value, CLOSE_BRACE)
//     }

//     /**
//     Return a string representation of the TextBlock in .tex format.

//     # Return

//     * `String` - the .tex representation of the block

//     # Example

//     ```ignore
//     # use lilscript::tex_handler::export::block_to_tex;
//     # use lilscript::script::TextBlock;
//     let s = "The characters do something.".to_string();
//     let block = TextBlock::StageDir(s);
//     assert_eq!(block_to_tex(&block), r"\stagedir{The characters do something.}");
//     ```
//     */
//     pub fn block_to_tex(block: &TextBlock) -> String {
//         match block {
//             TextBlock::Spoken(dialogue, None) => format!("\\spoken{}", embrace(dialogue)),
//             TextBlock::Spoken(dialogue, Some(speaker)) => {
//                 format!("\\spoken[{}]{}", speaker, embrace(dialogue))
//             }
//             TextBlock::InlineDirection(direction) => format!("\\direct{}", embrace(direction)),
//             TextBlock::SFX(sfx) => format!("\\sfx{}", embrace(sfx)),
//             TextBlock::StageDir(direction) => format!("\\stagedir{}", embrace(direction)),
//             TextBlock::ListenerDialogue(dialogue) => format!("\\listener{}", embrace(dialogue)),
//             TextBlock::Emphasis(em) => format!("\\ul{}", embrace(em)),
//             TextBlock::Separator => String::from("\n\n")
//         }
//     }

//     /// Render the given script in .tex format.
//     pub fn script_to_tex(script: &Script) -> String {
//         // TODO: output preamble
//         // TODO: output script header info
//         script
//             .paragraphs
//             .iter()
//             .map(|line| {
//                 line.iter()
//                     .map(block_to_tex)
//                     .collect::<Vec<String>>()
//                     .join(" ")
//             })
//             .collect::<Vec<String>>()
//             .join("\n\n")
//     }

//     #[cfg(test)]
//     mod test {
//         use super::*;

//         #[test]
//         fn test_embrace() {
//             let values: Vec<&str> = vec!["a", "6", "this", "word", ""];
//             let actual = values.into_iter().map(embrace).collect::<Vec<String>>();
//             let expected = vec!["{a}", "{6}", "{this}", "{word}", "{}"];

//             assert_eq!(actual, expected);
//         }

//         #[test]
//         fn test_textblock_to_tex_stagedir() {
//             let s = "The characters do something.".to_owned();
//             let block = TextBlock::StageDir(s);
//             let output = block_to_tex(&block);

//             assert_eq!(output, r"\stagedir{The characters do something.}");
//         }

//         #[test]
//         fn test_textblock_to_tex_spoken_with_speaker() {
//             let s = "I'm going to say something.".to_owned();
//             let a = Some("lilellia".to_owned());
//             let block = TextBlock::Spoken(s, a);
//             let output = block_to_tex(&block);

//             assert_eq!(output, r"\spoken[lilellia]{I'm going to say something.}");
//         }

//         #[test]
//         fn test_textblock_to_tex_spoken_without_speaker() {
//             let s = "I'm going to say something.".to_owned();
//             let block = TextBlock::Spoken(s, None);
//             let output = block_to_tex(&block);

//             assert_eq!(output, r"\spoken{I'm going to say something.}");
//         }

//         #[test]
//         fn test_textblock_to_tex_sfx() {
//             let s = "a sound!".to_owned();
//             let block = TextBlock::SFX(s);
//             let output = block_to_tex(&block);

//             assert_eq!(output, r"\sfx{a sound!}");
//         }

//         #[test]
//         fn test_textblock_to_tex_listener() {
//             let s = "Some secret words".to_owned();
//             let block = TextBlock::ListenerDialogue(s);
//             let output = block_to_tex(&block);

//             assert_eq!(output, r"\listener{Some secret words}");
//         }
//     }
// }
