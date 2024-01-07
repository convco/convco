use std::borrow::Cow;

use handlebars::{
    no_escape, Context, Handlebars, Helper, HelperDef, HelperResult, Output, RenderContext,
    Renderable, StringOutput,
};

fn word_wrap_acc<'a>(
    mut acc: Vec<Cow<'a, str>>,
    word: &'a str,
    line_length: usize,
) -> Vec<Cow<'a, str>> {
    let length = acc.len();
    if length != 0 {
        let last_line = acc.last().unwrap();
        if last_line.len() + word.len() < line_length {
            acc[length - 1] = format!("{} {}", last_line, word).into();
        } else {
            acc.push(word.into());
        }
    } else {
        acc.push(word.into());
    }
    acc
}

fn word_wrap(s: &str, line_length: usize) -> String {
    s.split(' ')
        .fold(Vec::new(), |acc, word| {
            word_wrap_acc(acc, word, line_length - 2)
        })
        .join("\n")
}

/// Helper for handlebars, does not wrap existing lines
///
/// ```hbs
/// {{#word-wrap}}
/// The quick brown fox jumps over the lazy dog
/// {{/word-wrap}}
/// ```
struct WordWrapBlock {
    max: usize,
    disabled: bool,
}

impl HelperDef for WordWrapBlock {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        r: &'reg Handlebars<'reg>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let mut unwrapped = StringOutput::new();
        h.template()
            .map(|t| t.render(r, ctx, rc, &mut unwrapped))
            .unwrap_or(Ok(()))?;
        let unwrapped = unwrapped.into_string()?;
        let unwrapped = unwrapped.as_str();

        if self.disabled {
            out.write(unwrapped)?;
        } else {
            let wrapped = word_wrap(unwrapped, self.max);
            out.write(&wrapped)?;
        }

        Ok(())
    }
}

pub fn new(max: usize, disabled: bool) -> Handlebars<'static> {
    let mut handlebars = Handlebars::new();
    handlebars.set_strict_mode(true);
    handlebars.register_escape_fn(no_escape);
    handlebars.register_helper("word-wrap", Box::new(WordWrapBlock { max, disabled }));
    handlebars
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_word_wrap_block() {
        let template =
            r#"{{#word-wrap max=8}}The quick brown fox jumps over the lazy dog{{/word-wrap}}"#;
        let mut handlebars = Handlebars::new();
        handlebars.register_helper(
            "word-wrap",
            Box::new(WordWrapBlock {
                max: 8,
                disabled: false,
            }),
        );
        let result = handlebars.render_template(template, &()).unwrap();
        assert_eq!(
            result,
            "The\nquick\nbrown\nfox\njumps\nover\nthe\nlazy\ndog"
        )
    }

    #[test]
    fn test_word_wrap() {
        let s = "The quick brown fox jumps over the lazy dog";
        assert_eq!(word_wrap(s, 80), s);
        assert_eq!(
            word_wrap(s, 8),
            "The\nquick\nbrown\nfox\njumps\nover\nthe\nlazy\ndog"
        );
    }
}
