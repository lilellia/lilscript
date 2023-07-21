use chrono::NaiveDateTime;
use num_format::{Locale, ToFormattedString};
use regex::Regex;
use std::{
    collections::HashMap,
    fmt::{self, Display},
    ops::Add,
};

/// A representation of a word count for a script
#[derive(Debug, PartialEq)]
pub struct WordCount {
    /// The number of spoken words.
    pub spoken: usize,

    /// The number of unspoken words.
    pub unspoken: usize,
}

impl fmt::Display for WordCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let locale = Locale::en;
        let decimals = f.precision().unwrap_or(2);

        let density = match self.speech_density() {
            x if x.is_nan() => "———%".to_string(),
            x => format!("{:.decimals$}%", 100. * x, decimals = decimals),
        };

        let spoken = self.spoken.to_formatted_string(&locale);
        let unspoken = self.unspoken.to_formatted_string(&locale);
        let total = self.total().to_formatted_string(&locale);
        write!(
            f,
            "{} spoken + {} unspoken -> {} total (ρ = {})",
            spoken, unspoken, total, density
        )
    }
}

impl WordCount {
    /// Construct a new WordCount initialised to zero.
    pub fn zero() -> Self {
        Self::new(0, 0)
    }

    /// Construct a new WordCount initialised with the given values.
    pub fn new(spoken: usize, unspoken: usize) -> Self {
        Self { spoken, unspoken }
    }

    /**
    Return the total number of words.

    # Return

    * `usize` - the total number of words

    # Examples:
    ```
    # use lilscript::script::WordCount;
    let wordcount = WordCount::new(100, 200);
    assert_eq!(wordcount.total(), 300);
    ```
    */
    pub fn total(&self) -> usize {
        self.spoken + self.unspoken
    }

    /// Return the speech density in the given script.
    ///
    /// # Return
    ///
    /// * `f64` - the proportion of words in the script that are spoken.
    ///
    /// # Notes
    ///
    /// The result will be NaN if there are no words counted.
    fn speech_density(&self) -> f64 {
        (self.spoken as f64) / (self.total() as f64)
    }
}

impl Add for WordCount {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self {
            spoken: self.spoken + other.spoken,
            unspoken: self.unspoken + other.unspoken,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum SpanKind {
    /// just some normal text
    Normal,

    /// emphasised text
    Emphasis,

    /// inline direction
    InlineDirection,
}

#[derive(Debug, PartialEq)]
pub struct TextSpan {
    /// The kind of span this represents.
    pub kind: SpanKind,

    /// The text within the span.
    pub contents: String,
}

impl TextSpan {
    pub fn new(kind: SpanKind, contents: &str) -> Self {
        Self {
            kind,
            contents: contents.to_string(),
        }
    }

    /// Construct a new span with kind Normal
    pub fn normal(contents: &str) -> Self {
        Self::new(SpanKind::Normal, &contents)
    }

    /// Construct a new span with kind Emphasis
    pub fn emphasis(contents: &str) -> Self {
        Self::new(SpanKind::Emphasis, &contents)
    }

    /// Construct a new span with kind InlineDirection
    pub fn inline(contents: &str) -> Self {
        Self::new(SpanKind::InlineDirection, &contents)
    }

    /// Convert this TextSpan to a different variant
    pub fn as_variant(&self, variant: SpanKind) -> Self {
        Self {
            kind: variant,
            contents: self.contents.clone(),
        }
    }
}

/// A representation of the type of text container.
#[derive(Debug, PartialEq)]
pub enum ContainerKind {
    /// a container for spoken text
    Spoken,

    /// a container for stage directions
    StageDir,

    /// a container for sound effects
    Sfx,

    /// a container for listener dialogue
    ListenerDialogue,

    /// a container for untagged text
    PlainText,
}

/// A representation of a container of text.
/// Used for a "line" of a script.
#[derive(Debug, PartialEq)]
pub struct TextContainer {
    /// the type of container this is
    pub kind: ContainerKind,

    /// a vector over the text spans
    pub spans: Vec<TextSpan>,
}

impl TextContainer {
    /**
    Create a new, empty text container.

    ```
    # use lilscript::script::{ContainerKind, TextContainer};
    let container = TextContainer::new();
    assert_eq!(container.kind, ContainerKind::PlainText);
    assert_eq!(container.spans, vec![]);
    ```
    */
    pub fn new() -> Self {
        Self {
            kind: ContainerKind::PlainText,
            spans: vec![],
        }
    }

    /// Return the number of spans in the container.
    pub fn len(&self) -> usize {
        self.spans.len()
    }
}

#[derive(Debug, Default, PartialEq)]
/// A representation of the series a script belongs to, including its part index.
pub struct SeriesEntry {
    /// The title of the series.
    pub title: Option<String>,
    /// The part index for the script.
    pub part: Option<usize>,
}

impl From<&str> for SeriesEntry {
    fn from(value: &str) -> Self {
        match value {
            "" | "—" | "\\textemdash" => Self {
                title: None,
                part: None,
            },
            value => {
                let re = Regex::new(r"^(.*?) \(Part (\d+)\)$").unwrap();
                let captures = re.captures(&value);

                if captures.is_none() {
                    return Self {
                        title: None,
                        part: None,
                    };
                }

                let captures = captures.unwrap();
                let title = captures.get(1).unwrap().as_str().to_owned();
                let part = captures.get(2).unwrap().as_str();
                let part: usize = part.parse().unwrap_or(0);

                Self {
                    title: Some(title),
                    part: Some(part),
                }
            }
        }
    }
}

impl fmt::Display for SeriesEntry {
    /**
    ```
    # use lilscript::script::SeriesEntry;
    let s = SeriesEntry::new("A Very Cool Series", 7);
    assert_eq!(format!("{}", s), "A Very Cool Series (Part 7)");
    ```
    */
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let (Some(title), Some(part)) = (self.title.clone(), self.part) {
            write!(f, "{} (Part {})", title, part)
        } else {
            write!(f, "")
        }
    }
}

impl SeriesEntry {
    /// Construct a SeriesEntry with the given title and part index.
    pub fn new(title: &str, part: usize) -> Self {
        Self {
            title: Some(title.to_owned()),
            part: Some(part),
        }
    }
}

#[derive(Debug, Default)]
/// A representation of a script.
pub struct Script {
    /// The name of the author. Even with multiple authors, it is only one string.
    pub author: String,

    /// The title of the script.
    pub title: String,

    /// The series (if any) that the script belongs to, as well as its part index.
    pub series: SeriesEntry,

    /// Any tags attributed to the script. Note that they do not include any brackets.
    pub tags: Vec<String>,

    /// The date of the script.
    pub date: Option<NaiveDateTime>,

    /// The summary of the script.
    pub summary: String,

    /// Information about the characters - {name => description}
    pub characters: HashMap<String, String>,

    /// The actual text of the script.
    pub paragraphs: Vec<TextContainer>,
}

impl Script {
    /**
    Construct a new Script with the given author and title.

    # Arguments

    * `author` - a string slice representing the author of the script
    * `title` - a string slice representing the title of the script

    All other values are set to an empty default.

    # Example

    ```
    # use lilscript::script::Script;
    let script = Script::new("lilellia", "A Very Cool Script");
    ```
    */
    pub fn new(author: &str, title: &str) -> Self {
        Self {
            author: author.to_owned(),
            title: title.to_owned(),
            ..Default::default()
        }
    }
}

impl Display for Script {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut lines: Vec<String> = vec![
            format!("Title: {}", self.title),
            format!("Author: {}", self.author),
            format!("Series: {}", self.series),
            format!(
                "Tags: {}",
                self.tags
                    .iter()
                    .map(|t| format!("[{}]", t))
                    .collect::<Vec<String>>()
                    .join(" ")
            ),
            format!("Date: {}", ""),  // TODO: Implement Date
            format!("Summary: {}", self.summary),
            format!("Characters: {}", ""),  // TODO: Implement Characters
            String::new(),
        ];

        for container in &self.paragraphs {
            for (i, span) in container.spans.iter().enumerate() {
                let prefix = if i == 0 {
                    format!("{:?}", container.kind)
                } else {
                    String::from("_")
                };
                let line = format!("{}::{:?}", prefix, span);
                lines.push(line)
            }
        }

        let s = lines.join("\n");
        write!(f, "{}", s)
    }
}

#[cfg(test)]
mod test {
    // use super::*;
}
