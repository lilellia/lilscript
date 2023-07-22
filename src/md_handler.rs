use crate::script::{ContainerKind, Script, SpanKind, TextContainer, TextSpan};
use log::warn;
use regex::Regex;
use std::fmt::Write;

pub trait ToMarkdown {
    /// Convert the object to a Markdown format.
    fn to_markdown(&self) -> String;
}

impl ToMarkdown for TextSpan {
    /// Convert the TextSpan to Markdown
    ///
    /// # Examples
    ///
    /// ```
    /// # use lilscript::{script::TextSpan, md_handler::ToMarkdown};
    /// let span = TextSpan::normal("Some normal text");
    /// assert_eq!(span.to_markdown(), "Some normal text");
    /// ```
    /// ```
    /// # use lilscript::{script::TextSpan, md_handler::ToMarkdown};
    /// let span = TextSpan::emphasis("impact");
    /// assert_eq!(span.to_markdown(), "/impact/");
    /// ```
    /// ```
    /// # use lilscript::{script::TextSpan, md_handler::ToMarkdown};
    /// let span = TextSpan::inline("an inline");
    /// assert_eq!(span.to_markdown(), "*(an inline)*");
    /// ```
    fn to_markdown(&self) -> String {
        let s = &self.contents;
        match self.kind {
            SpanKind::Normal => s.to_owned(),
            SpanKind::Emphasis => format!("/{}/", s),
            SpanKind::InlineDirection => format!("*({})*", s),
        }
    }
}

impl ToMarkdown for TextContainer {
    /// Convert the TextContainer to Markdown
    ///
    /// # Examples
    ///
    /// ```
    /// # use lilscript::{script::{ContainerKind, TextSpan, TextContainer}, md_handler::ToMarkdown};
    /// let kind = ContainerKind::PlainText;
    /// let spans = vec![
    ///     TextSpan::normal("some text"),
    ///     TextSpan::inline("loudly"),
    ///     TextSpan::emphasis("EMPHASIS")
    /// ];
    /// let container = TextContainer { kind, spans };
    /// let expected = "some text *(loudly)* /EMPHASIS/";
    /// assert_eq!(container.to_markdown(), expected);
    /// ```
    ///
    /// ```
    /// # use lilscript::{script::{ContainerKind, TextSpan, TextContainer}, md_handler::ToMarkdown};
    /// let kind = ContainerKind::StageDir;
    /// let spans = vec![
    ///     TextSpan::normal("some text"),
    ///     TextSpan::inline("loudly"),
    ///     TextSpan::emphasis("EMPHASIS")
    /// ];
    /// let container = TextContainer { kind, spans };
    ///
    /// // notice that the asterisks are suppressed around the inline
    /// let expected = "> *[some text (loudly) /EMPHASIS/]*";
    /// assert_eq!(container.to_markdown(), expected);
    /// ```
    ///
    /// ```
    /// # use lilscript::{script::{ContainerKind, TextSpan, TextContainer}, md_handler::ToMarkdown};
    /// let kind = ContainerKind::Sfx;
    /// let spans = vec![
    ///     TextSpan::normal("some text"),
    ///     TextSpan::inline("loudly"),
    ///     TextSpan::emphasis("EMPHASIS")
    /// ];
    /// let container = TextContainer { kind, spans };
    ///
    /// // notice that the asterisks are suppressed around the inline
    /// let expected = "> *[sfx: some text (loudly) /EMPHASIS/]*";
    /// assert_eq!(container.to_markdown(), expected);
    /// ```
    ///
    /// ```
    /// # use lilscript::{script::{ContainerKind, TextSpan, TextContainer}, md_handler::ToMarkdown};
    /// let kind = ContainerKind::ListenerDialogue;
    /// let spans = vec![
    ///     TextSpan::normal("some text"),
    ///     TextSpan::inline("loudly"),
    ///     TextSpan::emphasis("EMPHASIS")
    /// ];
    /// let container = TextContainer { kind, spans };
    ///
    /// // notice that the asterisks are suppressed around the inline
    /// let expected = "> *« some text (loudly) /EMPHASIS/ »*";
    /// assert_eq!(container.to_markdown(), expected);
    /// ```
    ///
    /// ```
    /// # use lilscript::{script::{ContainerKind, TextSpan, TextContainer}, md_handler::ToMarkdown};
    /// let kind = ContainerKind::Spoken;
    /// let spans = vec![
    ///     TextSpan::inline("quietly, slowly"),
    ///     TextSpan::normal("some text"),
    ///     TextSpan::inline("loudly"),
    ///     TextSpan::emphasis("EMPHASIS"),
    ///     TextSpan::normal("...hm?")
    /// ];
    /// let container = TextContainer { kind, spans };
    ///
    /// // notice that the asterisks are suppressed around the inline
    /// let expected = "*(quietly, slowly)* **some text** *(loudly)* **/EMPHASIS/** **...hm?**";
    /// assert_eq!(container.to_markdown(), expected);
    /// ```
    fn to_markdown(&self) -> String {
        // TODO: combine adjacent like-blocks after alterations (the spoken emphasis in example)
        let mut buf = String::new();

        for span in &self.spans {
            // handle the different contexts
            let text = match self.kind {
                // This one's nice and easy ^_^
                ContainerKind::PlainText => span.to_markdown(),

                ContainerKind::StageDir | ContainerKind::Sfx | ContainerKind::ListenerDialogue => {
                    match span.kind {
                        // asterisks on an inline should be suppressed:
                        // > *[this is text (and this could be an inline)]*
                        SpanKind::InlineDirection => {
                            span.to_markdown().trim_matches('*').to_string()
                        }
                        _ => span.to_markdown(),
                    }
                }

                ContainerKind::Spoken => match span.kind {
                    // spoken dialogue (which is wrapped in Normal) should be bold
                    SpanKind::Normal => format!("**{}**", span.to_markdown()),
                    SpanKind::Emphasis => {
                        let md = span.to_markdown();
                        let context = (&self.spans)
                            .into_iter()
                            .map(|s| s.contents.clone())
                            .collect::<Vec<String>>()
                            .join(" ");

                        warn!(
                            "The emphasised span \"{}\" occurs within the scope of a \
                            spoken line and has been rendered as spoken. However, it MAY occur \
                            within an inline direction, etc., but we do not know. \
                            Context: \"{}\"",
                            md, context
                        );
                        format!("**{}**", md)
                    }
                    _ => span.to_markdown(),
                },
            };

            write!(buf, " {} ", text).unwrap_or_else(|_| {
                warn!("Failed writing to buffer: {}", span.contents);
                ()
            });
        }

        // remove extraneous spaces
        let re = Regex::new(r"[[:space:]]+").unwrap();
        buf = re.replace_all(&buf, " ").trim().to_string();

        // handle the global formatting
        match self.kind {
            ContainerKind::PlainText | ContainerKind::Spoken => buf,
            ContainerKind::StageDir => format!("> *[{}]*", buf),
            ContainerKind::Sfx => format!("> *[sfx: {}]*", buf),
            ContainerKind::ListenerDialogue => format!("> *« {} »*", buf),
        }
    }
}

impl ToMarkdown for Script {
    fn to_markdown(&self) -> String {
        const DIVIDER: &str = "--8<--";

        let mut lines: Vec<String> = Vec::new();

        // NOTE: This does not include any script info header information

        // Character info
        lines.push(String::from("## Characters"));
        for character in &self.characters {
            lines.push(format!(
                "- **{}** ∼ {}",
                character.name, character.description
            ))
        }

        // Formatting guide
        lines.append(&mut vec![
            String::from("## Formatting guide"),

            TextContainer::new(ContainerKind::Spoken)
                .push(TextSpan::normal("spoken text"))
                .to_markdown(),
            
            TextContainer::new(ContainerKind::Spoken)
                .push(TextSpan::emphasis("emphasis"))
                .to_markdown(),

            TextContainer::new(ContainerKind::Spoken)
                .push(TextSpan::inline("tone cue, suggested"))
                .to_markdown(),

            TextContainer::new(ContainerKind::StageDir)
                .push(TextSpan::normal("stage direction and/or sfx"))
                .to_markdown(),

            TextContainer::new(ContainerKind::ListenerDialogue)
                .push(TextSpan::normal("example listener dialogue, not intended to be voiced"))
                .to_markdown(),

            TextContainer::new(ContainerKind::PlainText)
                .push(TextSpan::normal(DIVIDER))
                .to_markdown()
        ]);

        for container in &self.paragraphs {
            lines.push(container.to_markdown());
        }

        lines.join("\n\n")
    }
}
