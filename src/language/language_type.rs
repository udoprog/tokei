use std::borrow::Cow;
use std::fmt;
use std::path::Path;
use std::io;
use std::str::FromStr;

use self::LanguageType::*;
use stats::Stats;

use super::syntax::SyntaxCounter;
use utils::bytes::{self, Bytes};
use FileAccess;

include!(concat!(env!("OUT_DIR"), "/language_type.rs"));

impl LanguageType {
    /// Build a language type and statistics from the given file.
    pub fn parse<'a, F>(
        file_access: F,
        types: Option<&[LanguageType]>,
    ) -> io::Result<Option<(LanguageType, Stats)>>
        where F: FileAccess<'a>
    {
        use std::io::Read;

        let is_supported = |language: &LanguageType| {
            types.map(|t| t.contains(language)).unwrap_or(true)
        };

        // language determined from metadata.
        if let Some(language) = LanguageType::from_file_access(file_access) {
            if !is_supported(&language) {
                return Ok(None);
            }

            let mut text = Vec::new();
            file_access.open()?.read_to_end(&mut text)?;
            let stats = language.parse_from_bytes(file_access.name(), &text)?;
            return Ok(Some((language, stats)));
        }

        // need to read a bit of content, read the first 8000 bytes to check if binary.
        let mut text = Vec::new();
        let mut reader = file_access.open()?;
        (&mut reader).take(8000).read_to_end(&mut text)?;

        // ignore binary files.
        if bytes::is_binary(&text) {
            return Ok(None);
        }

        reader.read_to_end(&mut text)?;

        if let Some(language) = LanguageType::from_content(&text) {
            let text = bytes::decode(&text).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            let stats = language.parse_from_bytes_checked(file_access.name(), Bytes::new(&text));
            return Ok(Some((language, stats)));
        }

        Ok(None)
    }

    /// Parses the text provided. Returning `Stats` on success.
    pub fn parse_from_str<'a>(self, name: Cow<'a, str>, text: &str) -> Stats {
        self.parse_from_bytes_checked(name, Bytes::new(text.as_bytes()))
    }

    /// Parses the text provided. Returning `Stats` on success.
    pub fn parse_from_bytes<'a>(self, name: Cow<'a, str>, text: &[u8]) -> Result<Stats, io::Error> {
        if bytes::is_binary(&text) {
            return Err(io::Error::new(io::ErrorKind::Other, "binary file"));
        }

        let text = bytes::decode(text).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(self.parse_from_bytes_checked(name, Bytes::new(&text)))
    }

    /// Parse from a known good (UTF-8) sequence of bytes.
    fn parse_from_bytes_checked<'a>(self, name: Cow<'a, str>, text: Bytes) -> Stats {
        let lines = text.lines();
        let mut stats = Stats::new(name.to_string());

        if self.is_blank() {
            let count = lines.count();
            stats.lines = count;
            stats.code = count;
            stats
        } else {
            self.parse_lines(lines, stats)
        }
    }

    /// Attempts to parse the line as simply as possible if there are no multi
    /// line comments or quotes. Returns `bool` indicating whether it was
    /// successful or not.
    #[inline]
    fn parse_basic(self, syntax: &SyntaxCounter, line: Bytes, stats: &mut Stats)
        -> bool
    {
        if syntax.quote.is_some() ||
           !syntax.stack.is_empty() ||
           syntax.important_syntax().any(|s| line.contains(s.as_bytes()))
        {
            return false;
        }

        if syntax.line_comments.into_iter()
                               .any(|s| line.as_bytes()
                                            .starts_with(s.as_bytes()))
        {
            stats.comments += 1;
            trace!("Comment No.{}", stats.comments);
        } else {
            stats.code += 1;
            trace!("Code No.{}", stats.code);
        }

        trace!("{}", line);
        trace!("^ Skippable.");

        true
    }

    #[inline]
    fn parse_lines<'a>(
        self,
        lines: impl IntoIterator<Item=Bytes<'a>>,
        mut stats: Stats
    ) -> Stats
    {
        let mut syntax = SyntaxCounter::new(self);

        for line in lines {

            if line.utf8_chars_lossy().all(char::is_whitespace) {
                stats.blanks += 1;
                trace!("Blank No.{}", stats.blanks);
                continue;
            }

            // FORTRAN has a rule where it only counts as a comment if it's the
            // first character in the column, so removing starting whitespace
            // could cause a miscount.
            let line = if syntax.is_fortran { line } else { line.trim() };
            let mut ended_with_comments = false;
            let mut had_multi_line = !syntax.stack.is_empty();
            let mut skip = 0;
            macro_rules! skip {
                ($skip:expr) => {{
                    skip = $skip - 1;
                }}
            }

            if self.parse_basic(&syntax, line, &mut stats) {
                continue;
            }


            'window: for i in 0..line.len() {
                if skip != 0 {
                    skip -= 1;
                    continue;
                }

                ended_with_comments = false;
                let line = line.as_bytes();
                let window = &line[i..];

                let is_end_of_quote_or_multi_line =
                    syntax.parse_end_of_quote(window)
                    .or_else(|| syntax.parse_end_of_multi_line(window));

                if let Some(skip_amount) = is_end_of_quote_or_multi_line {
                    ended_with_comments = true;
                    skip!(skip_amount);
                    continue;
                }

                let is_quote_or_multi_line = syntax.parse_quote(window)
                    .or_else(|| syntax.parse_multi_line_comment(window));

                if let Some(skip_amount) = is_quote_or_multi_line {
                    skip!(skip_amount);
                    continue;
                }

                if syntax.parse_line_comment(window) {
                    break 'window;
                }

            }

            trace!("{}", line);

            if ((!syntax.stack.is_empty() || ended_with_comments) && had_multi_line) ||
                (syntax.start_of_comments().any(|comment| line.starts_with(comment.as_bytes())) &&
                 syntax.quote.is_none())
            {
                stats.comments += 1;
                trace!("Comment No.{}", stats.comments);
                trace!("Was the Comment stack empty?: {}", !had_multi_line);
            } else {
                stats.code += 1;
                trace!("Code No.{}", stats.code);
            }
        }

        stats.lines = stats.blanks + stats.code + stats.comments;
        stats
    }
}

