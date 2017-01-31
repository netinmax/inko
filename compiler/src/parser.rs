//! LL(1) recursive-descent parser for Inko source code.

use lexer::{Lexer, Token, TokenType};

macro_rules! binary_op {
    ($rec: expr, $lhs: expr, $child: ident, $ntype: ident) => ({
        let start = $rec.lexer.skip_and_next().unwrap();
        let rhs = $rec.$child(start)?;

        Node::$ntype(Box::new($lhs), Box::new(rhs))
    })
}

pub struct Parser<'a> {
    lexer: Lexer<'a>,
}

#[derive(Debug)]
pub enum Node {
    None, // TODO: remove
    Expressions(Vec<Node>),
    And(Box<Node>, Box<Node>),
    Or(Box<Node>, Box<Node>),
    Equal(Box<Node>, Box<Node>),
    NotEqual(Box<Node>, Box<Node>),
    String(String, usize, usize),
}

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

pub type ParseResult = Result<Node, ParseError>;

impl<'a> Parser<'a> {
    pub fn new(input: &str) -> Self {
        Parser { lexer: Lexer::new(input.chars().collect()) }
    }

    /// Parses the input and returns an AST.
    pub fn parse(&mut self) -> ParseResult {
        let mut children = Vec::new();

        while let Some(token) = self.lexer.next() {
            children.push(self.expression(token)?);
        }

        Ok(Node::Expressions(children))
    }

    /// Parses a single expression.
    fn expression(&mut self, start: Token) -> ParseResult {
        self.or_expression(start)
    }

    /// Parses a binary OR expression.
    fn or_expression(&mut self, start: Token) -> ParseResult {
        let mut node = self.and_expression(start)?;

        loop {
            if self.lexer.next_type_is(TokenType::Or) {
                node = binary_op!(self, node, and_expression, Or);
            } else {
                break;
            }
        }

        Ok(node)
    }

    /// Parses a binary AND expression.
    fn and_expression(&mut self, start: Token) -> ParseResult {
        let mut node = self.eq_expression(start)?;

        loop {
            if self.lexer.next_type_is(TokenType::And) {
                node = binary_op!(self, node, eq_expression, And);
            } else {
                break;
            }
        }

        Ok(node)
    }

    /// Parses a binary equality expression.
    fn eq_expression(&mut self, start: Token) -> ParseResult {
        let mut node = self.compare_expression(start)?;

        //loop {
        //match self.lexer.peek() {
        //Some(token) if token.token_type == TokenType::Equal => {
        //let start = self.lexer.skip_and_next().unwrap();
        //let rhs = self.compare_expression(start)?;

        //node = Node::Equal(Box::new(node), Box::new(rhs));
        //}
        //Some(token) if token.token_type == TokenType::NotEqual => {
        //let start = self.lexer.skip_and_next().unwrap();
        //let rhs = self.compare_expression(start)?;

        //node = Node::NotEqual(Box::new(node), Box::new(rhs));
        //}
        //_ => break,
        //}
        //}

        Ok(node)
    }

    fn compare_expression(&mut self, start: Token) -> ParseResult {
        self.string(start)
    }

    /// Parses a string.
    fn string(&mut self, start: Token) -> ParseResult {
        Ok(Node::String(start.value, start.line, start.column))
    }
}
