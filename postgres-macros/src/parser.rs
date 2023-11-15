use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::prelude::*;
use chumsky::text::ident;

pub fn parse(input: &str) -> Result<Vec<Token<'_>>, Rich<'_, char>> {
    match parser().parse(input).into_result() {
        Ok(tokens) => Ok(tokens),
        Err(errors) => {
            let err = errors.into_iter().next().unwrap();
            Report::build(ReportKind::Error, (), err.span().start)
                .with_message(err.to_string())
                .with_label(
                    Label::new(err.span().into_range())
                        .with_message(err.reason().to_string())
                        .with_color(Color::Red),
                )
                .finish()
                .eprint(Source::from(input))
                .unwrap();
            Err(err)
        }
    }
}

fn parser<'a>() -> impl Parser<'a, &'a str, Vec<Token<'a>>, extra::Err<Rich<'a, char>>> {
    token_parser().repeated().collect().then_ignore(end())
}

pub enum Token<'a> {
    EscapedCurlyStart,
    EscapedCurlyEnd,
    Text(&'a str),
    Argument(Argument<'a>),
}

fn token_parser<'a>() -> impl Parser<'a, &'a str, Token<'a>, extra::Err<Rich<'a, char>>> {
    choice((
        // escaped `{` (via `{{``)
        just("{{").map(|_| Token::EscapedCurlyStart),
        // escaped `}` (via `}}`)
        just("}}").map(|_| Token::EscapedCurlyEnd),
        // reject $ (accidental direct use of positional parameters)
        just("$").validate(|text: &str, e, emitter| {
            emitter.emit(Rich::custom(
                e.span(),
                "use {} instead of $x for positional parameters",
            ));
            Token::Text(text)
        }),
        // text chunks outside of {}
        none_of("{}$")
            .repeated()
            .at_least(1)
            .to_slice()
            .map(Token::Text),
        // arguments: {}, {0}, {name}
        just("{")
            .ignore_then(argument_parser())
            .then_ignore(just("}"))
            .map(Token::Argument),
    ))
}

pub enum Argument<'a> {
    Positional(usize),
    Next,
    Named(&'a str),
}

fn argument_parser<'a>() -> impl Parser<'a, &'a str, Argument<'a>, extra::Err<Rich<'a, char>>> {
    // none_of("{}").repeated().then(
    choice((
        text::int(10)
            .from_str()
            .unwrapped()
            .map(Argument::Positional),
        ident().map(Argument::Named),
        empty().map(|_| Argument::Next),
    ))
}
