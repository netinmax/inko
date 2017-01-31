//! Lexer for tokenizing Inko source code.

use std::collections::HashMap;
use std::collections::HashSet;
use std::iter::FromIterator;

macro_rules! hash_map {
    { $($key: expr => $value: expr),+ } => ({
        let mut map = HashMap::new();

        $(map.insert($key, $value);)+

        map
    });
}

macro_rules! hash_set {
    [ $($value: expr),+ ] => ({
        let mut set = HashSet::new();

        $(set.insert($value);)+

        set
    });
}

pub struct Lexer<'a> {
    input: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
    identifiers: HashMap<&'a str, TokenType>,
    specials: HashSet<char>,
    peeked: Option<Token>,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum TokenType {
    Add,
    AddAssign,
    And,
    AndAssign,
    Arrow,
    Assign,
    Attribute,
    BitwiseAnd,
    BitwiseAndAssign,
    BitwiseOr,
    BitwiseOrAssign,
    BitwiseXor,
    BitwiseXorAssign,
    BracketClose,
    BracketOpen,
    Colon,
    ColonColon,
    Comment,
    Constant,
    CurlyClose,
    CurlyOpen,
    Div,
    DivAssign,
    Enum,
    Equal,
    Float,
    Greater,
    Identifier,
    Impl,
    Import,
    Integer,
    Let,
    Lower,
    Member,
    Modulo,
    ModuloAssign,
    Mul,
    MulAssign,
    Not,
    NotEqual,
    Object,
    Or,
    OrAssign,
    ParenClose,
    ParenOpen,
    Pow,
    PowAssign,
    Return,
    Self_,
    ShiftLeft,
    ShiftRight,
    String,
    Sub,
    SubAssign,
    Trait,
    Var,
}

#[derive(Debug)]
pub struct Token {
    pub token_type: TokenType,
    pub value: String,
    pub line: usize,
    pub column: usize,
}

pub enum LexerError {
    InvalidUtf8,
}

impl<'a> Lexer<'a> {
    pub fn new(input: Vec<char>) -> Self {
        Lexer {
            input: input,
            position: 0,
            line: 1,
            column: 1,
            peeked: None,
            identifiers: hash_map! {
                "let" => TokenType::Let,
                "var" => TokenType::Var,
                "object" => TokenType::Object,
                "trait" => TokenType::Trait,
                "impl" => TokenType::Impl,
                "enum" => TokenType::Enum,
                "member" => TokenType::Member,
                "import" => TokenType::Import,
                "return" => TokenType::Return,
                "self" => TokenType::Self_
            },
            specials: hash_set!['!', '@', '#', '$', '%', '^', '&', '*', '(',
                                ')', '-', '_', '+', '=', '\\', ':', ';', '"',
                                '\'', '<', '>', '?', '/', ',', '.', ' ', '\r',
                                '\n', '|'],
        }
    }

    /// Returns the next available token, if any.
    ///
    /// This method will consume any previously peeked tokens before consuming
    /// more input.
    pub fn next(&mut self) -> Option<Token> {
        if self.peeked.is_some() {
            self.peeked.take()
        } else {
            self.next_raw()
        }
    }

    /// Returns a reference to the next token without advancing.
    pub fn peek(&mut self) -> Option<&Token> {
        if self.peeked.is_none() {
            self.peeked = self.next_raw();
        }

        self.peeked.as_ref()
    }

    /// Skips the current token and returns the next one.
    pub fn skip_and_next(&mut self) -> Option<Token> {
        self.next();
        self.next()
    }

    /// Returns true if the next token is of the given type.
    pub fn next_type_is(&mut self, token_type: TokenType) -> bool {
        if let Some(token) = self.peek() {
            token.token_type == token_type
        } else {
            false
        }
    }

    fn next_raw(&mut self) -> Option<Token> {
        loop {
            match self.input.get(self.position) {
                Some(&'@') => return self.attribute(),
                Some(&'#') => return self.comment(),
                Some(&'0'...'9') => return self.number(),
                Some(&'{') => return self.curly_open(),
                Some(&'}') => return self.curly_close(),
                Some(&'(') => return self.paren_open(),
                Some(&')') => return self.paren_close(),
                Some(&'\'') => return self.single_string(),
                Some(&'"') => return self.double_string(),
                Some(&':') => return self.colons(),
                Some(&'/') => return self.div(),
                Some(&'%') => return self.modulo(),
                Some(&'^') => return self.bitwise_xor(),
                Some(&'&') => return self.bitwise_and_or_boolean_and(),
                Some(&'|') => return self.bitwise_or_or_boolean_or(),
                Some(&'*') => return self.mul_or_pow(),
                Some(&'-') => return self.sub_or_arrow(),
                Some(&'+') => return self.add(),
                Some(&'=') => return self.assign_or_equal(),
                Some(&'<') => return self.lower_or_shift_left(),
                Some(&'>') => return self.greater_or_shift_right(),
                Some(&'[') => return self.bracket_open(),
                Some(&']') => return self.bracket_close(),
                Some(&'!') => return self.not_or_not_equal(),
                Some(&'\r') => {
                    self.advance_line();

                    // If we're followed by a \n we'll just consume it so we
                    // don't advance the line twice.
                    let advance = if let Some(curr) = self.input
                        .get(self.position) {
                        curr == &'\n'
                    } else {
                        false
                    };

                    if advance {
                        self.advance_one();
                    }
                }
                Some(&'\n') => self.advance_line(),
                Some(&' ') | Some(&'\t') => self.advance_one(),
                Some(&c) if c.is_lowercase() => {
                    return self.identifier_or_keyword()
                }
                Some(&c) if c.is_uppercase() => return self.constant(),
                _ => return None,
            }
        }
    }

    fn identifier_or_keyword(&mut self) -> Option<Token> {
        self.advance_until_special()
            .and_then(|(start, stop)| {
                let mut token = self.token(TokenType::Identifier, start, stop);

                if let Some(token_type) = self.identifiers
                    .get(&token.value.as_ref())
                    .cloned() {
                    token.token_type = token_type;
                }

                Some(token)
            })
    }

    fn constant(&mut self) -> Option<Token> {
        self.advance_until_special()
            .and_then(|(start, stop)| {
                Some(self.token(TokenType::Constant, start, stop))
            })
    }

    fn attribute(&mut self) -> Option<Token> {
        // Skip the "@" sign.
        self.position += 1;

        self.advance_until_special()
            .and_then(|(start, stop)| {
                let token = self.token(TokenType::Attribute, start, stop);

                self.advance_column(1);

                Some(token)
            })
    }

    fn comment(&mut self) -> Option<Token> {
        // Skip the "#" sign
        self.position += 1;

        let mut start = self.position;
        let mut position = self.position;

        // Skip any whitespace immediately following the # sign.
        while let Some(current) = self.input.get(position) {
            if current == &' ' || current == &'\t' {
                start += 1;
                position += 1;
            } else {
                break;
            }
        }

        loop {
            match self.input.get(position) {
                Some(&'\r') | Some(&'\n') | None => break,
                _ => position += 1,
            }
        }

        let token = self.token(TokenType::Comment, start, position);

        self.advance_column(1);
        self.position = position;

        Some(token)
    }

    fn number(&mut self) -> Option<Token> {
        let start = self.position;
        let mut position = self.position;
        let mut token_type = TokenType::Integer;

        loop {
            if let Some(current) = self.input.get(position) {
                match current {
                    &'.' => {
                        match token_type {
                            TokenType::Integer => {
                                token_type = TokenType::Float;

                                position += 1;
                            }
                            _ => return None,
                        }
                    }
                    &'0'...'9' | &'_' => position += 1,
                    _ => break,
                }
            } else {
                break;
            }
        }

        let mut token = self.token(token_type, start, position);
        token.value = token.value.replace("_", "");

        self.position = position;

        Some(token)
    }

    fn curly_open(&mut self) -> Option<Token> {
        let position = self.position;
        let token = self.token(TokenType::CurlyOpen, position, position + 1);

        self.position += 1;

        Some(token)
    }

    fn curly_close(&mut self) -> Option<Token> {
        let position = self.position;
        let token = self.token(TokenType::CurlyClose, position, position + 1);

        self.position += 1;

        Some(token)
    }

    fn paren_open(&mut self) -> Option<Token> {
        let position = self.position;
        let token = self.token(TokenType::ParenOpen, position, position + 1);

        self.position += 1;

        Some(token)
    }

    fn paren_close(&mut self) -> Option<Token> {
        let position = self.position;
        let token = self.token(TokenType::ParenClose, position, position + 1);

        self.position += 1;

        Some(token)
    }

    fn single_string(&mut self) -> Option<Token> {
        self.string_with_quote(&'\'', "\\'", "'")
    }

    fn double_string(&mut self) -> Option<Token> {
        self.string_with_quote(&'"', "\\\"", "\"")
    }

    fn colons(&mut self) -> Option<Token> {
        let start = self.position;
        let mut position = self.position;

        position += 1;

        let colon_colon = if let Some(current) = self.input.get(position) {
            current == &':'
        } else {
            false
        };

        let token_type = if colon_colon {
            position += 1;

            TokenType::ColonColon
        } else {
            TokenType::Colon
        };

        self.position = position;

        Some(self.token(token_type, start, position))
    }

    fn div(&mut self) -> Option<Token> {
        self.operator(1, TokenType::Div, TokenType::DivAssign)
    }

    fn modulo(&mut self) -> Option<Token> {
        self.operator(1, TokenType::Modulo, TokenType::ModuloAssign)
    }

    fn bitwise_xor(&mut self) -> Option<Token> {
        self.operator(1, TokenType::BitwiseXor, TokenType::BitwiseXorAssign)
    }

    fn bitwise_and_or_boolean_and(&mut self) -> Option<Token> {
        let is_and = if let Some(current) = self.input.get(self.position + 1) {
            current == &'&'
        } else {
            false
        };

        if is_and {
            self.operator(2, TokenType::And, TokenType::AndAssign)
        } else {
            self.operator(1, TokenType::BitwiseAnd, TokenType::BitwiseAndAssign)
        }
    }

    fn bitwise_or_or_boolean_or(&mut self) -> Option<Token> {
        let is_or = if let Some(current) = self.input.get(self.position + 1) {
            current == &'|'
        } else {
            false
        };

        if is_or {
            self.operator(2, TokenType::Or, TokenType::OrAssign)
        } else {
            self.operator(1, TokenType::BitwiseOr, TokenType::BitwiseOrAssign)
        }
    }

    fn mul_or_pow(&mut self) -> Option<Token> {
        let is_pow = if let Some(current) = self.input.get(self.position + 1) {
            current == &'*'
        } else {
            false
        };

        if is_pow {
            self.operator(2, TokenType::Pow, TokenType::PowAssign)
        } else {
            self.operator(1, TokenType::Mul, TokenType::MulAssign)
        }
    }

    fn sub_or_arrow(&mut self) -> Option<Token> {
        let is_arrow = if let Some(current) = self.input.get(self.position + 1) {
            current == &'>'
        } else {
            false
        };

        if is_arrow {
            self.arrow()
        } else {
            self.operator(1, TokenType::Sub, TokenType::SubAssign)
        }
    }

    fn arrow(&mut self) -> Option<Token> {
        let start = self.position;
        let mut position = self.position;

        position += 2;
        self.position = position;

        Some(self.token(TokenType::Arrow, start, position))
    }

    fn add(&mut self) -> Option<Token> {
        self.operator(1, TokenType::Add, TokenType::AddAssign)
    }

    fn assign_or_equal(&mut self) -> Option<Token> {
        self.operator(1, TokenType::Assign, TokenType::Equal)
    }

    fn not_or_not_equal(&mut self) -> Option<Token> {
        self.operator(1, TokenType::Not, TokenType::NotEqual)
    }

    fn lower_or_shift_left(&mut self) -> Option<Token> {
        let is_shift = if let Some(current) = self.input.get(self.position + 1) {
            current == &'<'
        } else {
            false
        };

        let (token_type, advance) = if is_shift {
            (TokenType::ShiftLeft, 2)
        } else {
            (TokenType::Lower, 1)
        };

        let start = self.position;
        self.position = start + advance;

        Some(self.token(token_type, start, start + advance))
    }

    fn greater_or_shift_right(&mut self) -> Option<Token> {
        let is_shift = if let Some(current) = self.input.get(self.position + 1) {
            current == &'>'
        } else {
            false
        };

        let (token_type, advance) = if is_shift {
            (TokenType::ShiftRight, 2)
        } else {
            (TokenType::Greater, 1)
        };

        let start = self.position;
        self.position = start + advance;

        Some(self.token(token_type, start, start + advance))
    }

    fn bracket_open(&mut self) -> Option<Token> {
        let position = self.position;
        let token = self.token(TokenType::BracketOpen, position, position + 1);

        self.position += 1;

        Some(token)
    }

    fn bracket_close(&mut self) -> Option<Token> {
        let position = self.position;
        let token = self.token(TokenType::BracketClose, position, position + 1);

        self.position += 1;

        Some(token)
    }

    fn operator(&mut self,
                advance: usize,
                mut token_type: TokenType,
                assign_type: TokenType)
                -> Option<Token> {
        let start = self.position;
        let mut position = self.position;

        position += advance;

        if let Some(current) = self.input.get(position) {
            if current == &'=' {
                position += 1;
                token_type = assign_type;
            }
        }

        self.position = position;

        Some(self.token(token_type, start, position))
    }

    fn advance_one(&mut self) {
        self.position += 1;
        self.column += 1;
    }

    fn advance_line(&mut self) {
        self.position += 1;
        self.line += 1;
        self.column = 1;
    }

    fn advance_column(&mut self, amount: usize) {
        self.column += amount;
    }

    fn advance_column_from_token(&mut self, token: &Token) {
        self.advance_column(token.value.chars().count());
    }

    fn slice(&self, start: usize, stop: usize) -> String {
        String::from_iter(self.input[start..stop].to_vec())
    }

    fn token(&mut self,
             token_type: TokenType,
             start: usize,
             stop: usize)
             -> Token {
        let token = Token {
            token_type: token_type,
            value: self.slice(start, stop),
            line: self.line,
            column: self.column,
        };

        self.advance_column_from_token(&token);

        token
    }

    // Advances the cursor until we hit a special character.
    //
    // The returned value is an Option containing the start and stop position.
    // None is returned if we reached the end of the input before consuming at
    // least a single character.
    fn advance_until_special(&mut self) -> Option<(usize, usize)> {
        let start = self.position;
        let mut position = self.position;

        loop {
            if let Some(current) = self.input.get(position) {
                if self.specials.contains(current) {
                    break;
                } else {
                    position += 1;
                }
            } else {
                // We need to consume at least 1 character.
                if position - start == 0 {
                    return None;
                } else {
                    break;
                }
            }
        }

        self.position = position;

        Some((start, self.position))
    }

    fn string_with_quote(&mut self,
                         escaped: &char,
                         find: &str,
                         replace: &str)
                         -> Option<Token> {
        // Skip the opening quote
        self.position += 1;

        let start = self.position;
        let mut position = self.position;
        let mut has_escape = false;

        loop {
            if let Some(current) = self.input.get(position) {
                position += 1;

                if current == escaped {
                    if let Some(prev) = self.input.get(position - 2) {
                        // If the quote is escaped we should continue
                        // processing.
                        if prev == &'\\' {
                            has_escape = true;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    };
                }
            } else {
                break;
            }
        }

        let mut token = self.token(TokenType::String, start, position - 1);

        if has_escape {
            token.value = token.value.replace(find, replace);
        }

        self.advance_column(2);
        self.position = position;

        Some(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod lexer {
        use super::*;

        macro_rules! test {
            ($test_name: ident, $func: ident, $variant: ident, $op: expr) => (
                #[test]
                fn $test_name() {
                    let mut lexer = Lexer::new($op.chars().collect());
                    let token_opt = lexer.$func();

                    assert!(token_opt.is_some());

                    let token = token_opt.unwrap();

                    assert_eq!(token.token_type, TokenType::$variant);
                    assert_eq!(token.value, $op);
                    assert_eq!(token.line, 1);
                    assert_eq!(token.column, 1);
                }
            )
        }

        #[test]
        fn test_new() {
            let lexer = Lexer::new("a".chars().collect());

            assert_eq!(lexer.position, 0);
            assert_eq!(lexer.line, 1);
            assert_eq!(lexer.column, 1);
        }

        #[test]
        fn test_next() {
            let mut lexer = Lexer::new("a".chars().collect());

            assert!(lexer.next().is_some());
            assert!(lexer.next().is_none());
        }

        #[test]
        fn test_peek() {
            let mut lexer = Lexer::new("a".chars().collect());

            assert!(lexer.peek().is_some());
            assert!(lexer.peek().is_some());
        }

        #[test]
        fn test_skip_and_next() {
            let mut lexer = Lexer::new("a b".chars().collect());

            assert!(lexer.peek().is_some());
            assert!(lexer.skip_and_next().is_some());
            assert!(lexer.next().is_none());
        }

        #[test]
        fn test_next_type_is() {
            let mut lexer = Lexer::new("a".chars().collect());

            assert!(lexer.next_type_is(TokenType::Identifier));

            lexer.next();

            assert_eq!(lexer.next_type_is(TokenType::Identifier), false);
        }

        #[test]
        fn test_peek_with_next() {
            let mut lexer = Lexer::new("a".chars().collect());

            assert!(lexer.peek().is_some());
            assert!(lexer.next().is_some());

            assert!(lexer.peek().is_none());
            assert!(lexer.next().is_none());
        }

        #[test]
        fn test_identifier_or_keyword_with_identifier() {
            let mut lexer = Lexer::new("foo".chars().collect());
            let token_opt = lexer.identifier_or_keyword();

            assert!(token_opt.is_some());

            let token = token_opt.unwrap();

            assert_eq!(token.token_type, TokenType::Identifier);
            assert_eq!(token.value, "foo".to_string());
            assert_eq!(token.line, 1);
            assert_eq!(token.column, 1);
        }

        #[test]
        fn test_constant() {
            let mut lexer = Lexer::new("Foo".chars().collect());
            let token_opt = lexer.constant();

            assert!(token_opt.is_some());

            let token = token_opt.unwrap();

            assert_eq!(token.token_type, TokenType::Constant);
            assert_eq!(token.value, "Foo".to_string());
            assert_eq!(token.line, 1);
            assert_eq!(token.column, 1);
        }

        #[test]
        fn test_attribute() {
            let mut lexer = Lexer::new("@foo".chars().collect());
            let token_opt = lexer.attribute();

            assert!(token_opt.is_some());

            let token = token_opt.unwrap();

            assert_eq!(token.token_type, TokenType::Attribute);
            assert_eq!(token.value, "foo".to_string());
            assert_eq!(token.line, 1);
            assert_eq!(token.column, 1);
        }

        #[test]
        fn test_comment() {
            let mut lexer = Lexer::new("# foo".chars().collect());
            let token_opt = lexer.comment();

            assert!(token_opt.is_some());

            let token = token_opt.unwrap();

            assert_eq!(token.token_type, TokenType::Comment);
            assert_eq!(token.value, "foo".to_string());
            assert_eq!(token.line, 1);
            assert_eq!(token.column, 1);
        }

        #[test]
        fn test_number_with_integer() {
            let mut lexer = Lexer::new("123".chars().collect());
            let token_opt = lexer.number();

            assert!(token_opt.is_some());

            let token = token_opt.unwrap();

            assert_eq!(token.token_type, TokenType::Integer);
            assert_eq!(token.value, "123".to_string());
            assert_eq!(token.line, 1);
            assert_eq!(token.column, 1);
        }

        #[test]
        fn test_number_with_integer_with_underscore() {
            let mut lexer = Lexer::new("123_4".chars().collect());
            let token_opt = lexer.number();

            assert!(token_opt.is_some());

            let token = token_opt.unwrap();

            assert_eq!(token.token_type, TokenType::Integer);
            assert_eq!(token.value, "1234".to_string());
            assert_eq!(token.line, 1);
            assert_eq!(token.column, 1);
        }

        #[test]
        fn test_number_with_float() {
            let mut lexer = Lexer::new("12.34".chars().collect());
            let token_opt = lexer.number();

            assert!(token_opt.is_some());

            let token = token_opt.unwrap();

            assert_eq!(token.token_type, TokenType::Float);
            assert_eq!(token.value, "12.34".to_string());
            assert_eq!(token.line, 1);
            assert_eq!(token.column, 1);
        }

        #[test]
        fn test_number_with_float_with_underscore() {
            let mut lexer = Lexer::new("12_3.34".chars().collect());
            let token_opt = lexer.number();

            assert!(token_opt.is_some());

            let token = token_opt.unwrap();

            assert_eq!(token.token_type, TokenType::Float);
            assert_eq!(token.value, "123.34".to_string());
            assert_eq!(token.line, 1);
            assert_eq!(token.column, 1);
        }

        #[test]
        fn test_single_string() {
            let mut lexer = Lexer::new("'foo'".chars().collect());
            let token_opt = lexer.single_string();

            assert!(token_opt.is_some());

            let token = token_opt.unwrap();

            assert_eq!(token.token_type, TokenType::String);
            assert_eq!(token.value, "foo".to_string());
            assert_eq!(token.line, 1);
            assert_eq!(token.column, 1);
        }

        #[test]
        fn test_single_string_with_escape() {
            let mut lexer = Lexer::new("'foo\\'bar'".chars().collect());
            let token_opt = lexer.single_string();

            assert!(token_opt.is_some());

            let token = token_opt.unwrap();

            assert_eq!(token.token_type, TokenType::String);
            assert_eq!(token.value, "foo'bar".to_string());
            assert_eq!(token.line, 1);
            assert_eq!(token.column, 1);
        }

        #[test]
        fn test_double_string() {
            let mut lexer = Lexer::new("\"foo\"".chars().collect());
            let token_opt = lexer.double_string();

            assert!(token_opt.is_some());

            let token = token_opt.unwrap();

            assert_eq!(token.token_type, TokenType::String);
            assert_eq!(token.value, "foo".to_string());
            assert_eq!(token.line, 1);
            assert_eq!(token.column, 1);
        }

        #[test]
        fn test_double_string_with_escape() {
            let mut lexer = Lexer::new("\"foo\\\"bar\"".chars().collect());
            let token_opt = lexer.double_string();

            assert!(token_opt.is_some());

            let token = token_opt.unwrap();

            assert_eq!(token.token_type, TokenType::String);
            assert_eq!(token.value, "foo\"bar".to_string());
            assert_eq!(token.line, 1);
            assert_eq!(token.column, 1);
        }

        test!(test_ident, identifier_or_keyword, Identifier, "foo");
        test!(test_let, identifier_or_keyword, Let, "let");
        test!(test_var, identifier_or_keyword, Var, "var");
        test!(test_object, identifier_or_keyword, Object, "object");
        test!(test_trait, identifier_or_keyword, Trait, "trait");
        test!(test_impl, identifier_or_keyword, Impl, "impl");
        test!(test_enum, identifier_or_keyword, Enum, "enum");
        test!(test_member, identifier_or_keyword, Member, "member");
        test!(test_import, identifier_or_keyword, Import, "import");
        test!(test_return, identifier_or_keyword, Return, "return");
        test!(test_self, identifier_or_keyword, Self_, "self");

        test!(test_bracket_open, bracket_open, BracketOpen, "[");
        test!(test_bracket_close, bracket_close, BracketClose, "]");

        test!(test_curly_open, curly_open, CurlyOpen, "{");
        test!(test_curly_close, curly_close, CurlyClose, "}");

        test!(test_paren_open, paren_open, ParenOpen, "(");
        test!(test_paren_close, paren_close, ParenClose, ")");

        test!(test_colons_single_colon, colons, Colon, ":");
        test!(test_colons_colon_colon, colons, ColonColon, "::");

        test!(test_div, div, Div, "/");
        test!(test_div_assign, div, DivAssign, "/=");

        test!(test_modulo, modulo, Modulo, "%");
        test!(test_module_assign, modulo, ModuloAssign, "%=");

        test!(test_bitwise_xor, bitwise_xor, BitwiseXor, "^");
        test!(test_bitwise_xor_assign, bitwise_xor, BitwiseXorAssign, "^=");

        test!(test_bitwise_and,
              bitwise_and_or_boolean_and,
              BitwiseAnd,
              "&");

        test!(test_bitwise_and_assign,
              bitwise_and_or_boolean_and,
              BitwiseAndAssign,
              "&=");

        test!(test_boolean_and, bitwise_and_or_boolean_and, And, "&&");

        test!(test_boolean_and_assign,
              bitwise_and_or_boolean_and,
              AndAssign,
              "&&=");

        test!(test_bitwise_or, bitwise_or_or_boolean_or, BitwiseOr, "|");

        test!(test_bitwise_or_assign,
              bitwise_or_or_boolean_or,
              BitwiseOrAssign,
              "|=");

        test!(test_boolean_or, bitwise_or_or_boolean_or, Or, "||");

        test!(test_boolean_or_assign,
              bitwise_or_or_boolean_or,
              OrAssign,
              "||=");

        test!(test_mul, mul_or_pow, Mul, "*");
        test!(test_mul_assign, mul_or_pow, MulAssign, "*=");

        test!(test_pow, mul_or_pow, Pow, "**");
        test!(test_pow_assign, mul_or_pow, PowAssign, "**=");

        test!(test_sub, sub_or_arrow, Sub, "-");
        test!(test_sub_assign, sub_or_arrow, SubAssign, "-=");

        test!(test_arrow, sub_or_arrow, Arrow, "->");

        test!(test_add, add, Add, "+");
        test!(test_add_assign, add, AddAssign, "+=");

        test!(test_assign_or_equal_assign, assign_or_equal, Assign, "=");
        test!(test_assign_or_equal_equal, assign_or_equal, Equal, "==");

        test!(test_not, not_or_not_equal, Not, "!");
        test!(test_not_equal, not_or_not_equal, NotEqual, "!=");

        test!(test_lower, lower_or_shift_left, Lower, "<");
        test!(test_shift_left, lower_or_shift_left, ShiftLeft, "<<");

        test!(test_greater, greater_or_shift_right, Greater, ">");
        test!(test_shift_right, greater_or_shift_right, ShiftRight, ">>");
    }
}
