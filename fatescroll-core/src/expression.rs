// ABOUTME: Parser and type checker for result value expressions.
// ABOUTME: Tokenizes, parses, and checks integer/boolean/string expressions.

use std::collections::BTreeMap;

pub(crate) const MAX_SOURCE_BYTES: usize = 4096;
pub(crate) const MAX_AST_DEPTH: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ValueType {
    Integer,
    Boolean,
    Text,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ExpressionError {
    pub(crate) offset: usize,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Expression {
    root: ExprNode,
}

#[derive(Debug, Clone, PartialEq)]
struct ExprNode {
    kind: ExprKind,
    start: usize,
    end: usize,
}

#[derive(Debug, Clone, PartialEq)]
enum ExprKind {
    Integer(i64),
    Boolean(bool),
    Text(String),
    Variable(String),
    Unary {
        op: UnaryOp,
        operand: Box<ExprNode>,
    },
    Dice {
        literal: String,
        expr: diceman::Expr,
    },
    Binary {
        op: BinaryOp,
        left: Box<ExprNode>,
        right: Box<ExprNode>,
    },
    Conditional {
        cond: Box<ExprNode>,
        then_branch: Box<ExprNode>,
        else_branch: Box<ExprNode>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinaryOp {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

pub(crate) type TypeScope = BTreeMap<String, ValueType>;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Value {
    Integer(i64),
    Boolean(bool),
    Text(String),
}

pub(crate) type ValueScope = BTreeMap<String, Value>;

pub(crate) fn evaluate(
    expr: &Expression,
    scope: &ValueScope,
    rng: &mut impl diceman::Rng,
) -> Result<Value, ExpressionError> {
    eval_node(&expr.root, scope, rng)
}

fn eval_node(
    node: &ExprNode,
    scope: &ValueScope,
    rng: &mut impl diceman::Rng,
) -> Result<Value, ExpressionError> {
    match &node.kind {
        ExprKind::Integer(value) => Ok(Value::Integer(*value)),
        ExprKind::Boolean(value) => Ok(Value::Boolean(*value)),
        ExprKind::Text(value) => Ok(Value::Text(value.clone())),
        ExprKind::Variable(name) => scope
            .get(name)
            .cloned()
            .ok_or_else(|| error(node.start, format!("unknown name `{name}`"))),
        ExprKind::Dice { literal, expr } => {
            // The caller's RNG is the only randomness source: evaluate the
            // checked dice plan diceman parsed, never creating or reseeding.
            let rolled = diceman::roller::evaluate_with_rng(expr, rng)
                .map_err(|reason| error(node.start, format!("invalid dice roll: {reason}")))?;
            rolled
                .outcome
                .as_numeric()
                .map(Value::Integer)
                .ok_or_else(|| {
                    error(
                        node.start,
                        format!("dice literal `{literal}` produced a non-numeric outcome"),
                    )
                })
        }
        ExprKind::Unary { op, operand } => {
            let operand_value = eval_node(operand, scope, rng)?;
            match (op, operand_value) {
                (UnaryOp::Neg, Value::Integer(a)) => a
                    .checked_neg()
                    .map(Value::Integer)
                    .ok_or_else(|| error(node.start, "integer negation overflows")),
                (UnaryOp::Not, Value::Boolean(a)) => Ok(Value::Boolean(!a)),
                (UnaryOp::Neg, _) => Err(error(node.start, "unary `-` requires an integer")),
                (UnaryOp::Not, _) => Err(error(node.start, "unary `!` requires a boolean")),
            }
        }
        ExprKind::Conditional {
            cond,
            then_branch,
            else_branch,
        } => {
            let cond_value = eval_node(cond, scope, rng)?;
            match cond_value {
                Value::Boolean(true) => eval_node(then_branch, scope, rng),
                Value::Boolean(false) => eval_node(else_branch, scope, rng),
                _ => Err(error(
                    cond.start,
                    "`if` condition must be boolean".to_string(),
                )),
            }
        }
        ExprKind::Binary { op, left, right } => match op {
            // `&&` and `||` decide from the left side first and evaluate the
            // right side only when it can change the result.
            BinaryOp::And => match eval_node(left, scope, rng)? {
                Value::Boolean(false) => Ok(Value::Boolean(false)),
                Value::Boolean(true) => match eval_node(right, scope, rng)? {
                    Value::Boolean(b) => Ok(Value::Boolean(b)),
                    _ => Err(error(right.start, "boolean operators require booleans")),
                },
                _ => Err(error(left.start, "boolean operators require booleans")),
            },
            BinaryOp::Or => match eval_node(left, scope, rng)? {
                Value::Boolean(true) => Ok(Value::Boolean(true)),
                Value::Boolean(false) => match eval_node(right, scope, rng)? {
                    Value::Boolean(b) => Ok(Value::Boolean(b)),
                    _ => Err(error(right.start, "boolean operators require booleans")),
                },
                _ => Err(error(left.start, "boolean operators require booleans")),
            },
            _ => {
                let left_value = eval_node(left, scope, rng)?;
                let right_value = eval_node(right, scope, rng)?;
                eval_eager_binary(node.start, *op, left_value, right_value)
            }
        },
    }
}

/// Evaluate a binary operator whose operands are already evaluated.
/// Callers handle lazy operators (`&&`, `||`) before reaching this helper.
fn eval_eager_binary(
    offset: usize,
    op: BinaryOp,
    left: Value,
    right: Value,
) -> Result<Value, ExpressionError> {
    match (op, left, right) {
        (BinaryOp::Add, Value::Integer(a), Value::Integer(b)) => a
            .checked_add(b)
            .map(Value::Integer)
            .ok_or_else(|| error(offset, "integer addition overflows")),
        (BinaryOp::Mul, Value::Integer(a), Value::Integer(b)) => a
            .checked_mul(b)
            .map(Value::Integer)
            .ok_or_else(|| error(offset, "integer multiplication overflows")),
        (BinaryOp::Sub, Value::Integer(a), Value::Integer(b)) => a
            .checked_sub(b)
            .map(Value::Integer)
            .ok_or_else(|| error(offset, "integer subtraction overflows")),
        (BinaryOp::Div, Value::Integer(a), Value::Integer(b)) => a
            .checked_div(b)
            .map(Value::Integer)
            .ok_or_else(|| error(offset, "integer division overflows or divides by zero")),
        (BinaryOp::Rem, Value::Integer(a), Value::Integer(b)) => a
            .checked_rem(b)
            .map(Value::Integer)
            .ok_or_else(|| error(offset, "integer remainder overflows or divides by zero")),
        (BinaryOp::Equal, Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a == b)),
        (BinaryOp::Equal, Value::Boolean(a), Value::Boolean(b)) => Ok(Value::Boolean(a == b)),
        (BinaryOp::Equal, Value::Text(a), Value::Text(b)) => Ok(Value::Boolean(a == b)),
        (BinaryOp::NotEqual, Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a != b)),
        (BinaryOp::NotEqual, Value::Boolean(a), Value::Boolean(b)) => Ok(Value::Boolean(a != b)),
        (BinaryOp::NotEqual, Value::Text(a), Value::Text(b)) => Ok(Value::Boolean(a != b)),
        (BinaryOp::Less, Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a < b)),
        (BinaryOp::LessEqual, Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a <= b)),
        (BinaryOp::Greater, Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a > b)),
        (BinaryOp::GreaterEqual, Value::Integer(a), Value::Integer(b)) => {
            Ok(Value::Boolean(a >= b))
        }
        (BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem, _, _) => {
            Err(error(offset, "arithmetic requires integers"))
        }
        (
            BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual,
            _,
            _,
        ) => Err(error(offset, "ordering requires integers")),
        (BinaryOp::Equal | BinaryOp::NotEqual, _, _) => {
            Err(error(offset, "equality requires matching operand types"))
        }
        _ => Err(error(offset, "unsupported operation")),
    }
}

pub(crate) fn parse(source: &str) -> Result<Expression, ExpressionError> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(error(
            MAX_SOURCE_BYTES,
            "expression source exceeds size limit",
        ));
    }
    let tokens = tokenize(source)?;
    let mut parser = Parser {
        tokens,
        pos: 0,
        source_len: source.len(),
        nesting: 1,
    };
    let root = parser.parse_expression()?;
    if parser.pos < parser.tokens.len() {
        let offset = parser.tokens[parser.pos].start;
        return Err(error(offset, "unexpected trailing input"));
    }
    Ok(Expression { root })
}

pub(crate) fn check(
    expr: &Expression,
    scope: &TypeScope,
    allow_roll: bool,
) -> Result<ValueType, ExpressionError> {
    check_node(&expr.root, scope, allow_roll)
}

fn check_node(
    node: &ExprNode,
    scope: &TypeScope,
    allow_roll: bool,
) -> Result<ValueType, ExpressionError> {
    match &node.kind {
        ExprKind::Integer(_) => Ok(ValueType::Integer),
        ExprKind::Boolean(_) => Ok(ValueType::Boolean),
        ExprKind::Text(_) => Ok(ValueType::Text),
        ExprKind::Variable(name) => scope
            .get(name)
            .copied()
            .ok_or_else(|| error(node.start, format!("unknown name `{name}`"))),
        ExprKind::Unary { op, operand } => {
            let operand_type = check_node(operand, scope, allow_roll)?;
            match op {
                UnaryOp::Neg => {
                    if operand_type != ValueType::Integer {
                        return Err(error(
                            node.start,
                            format!("unary `-` requires an integer, found {operand_type:?}"),
                        ));
                    }
                    Ok(ValueType::Integer)
                }
                UnaryOp::Not => {
                    if operand_type != ValueType::Boolean {
                        return Err(error(
                            node.start,
                            format!("unary `!` requires a boolean, found {operand_type:?}"),
                        ));
                    }
                    Ok(ValueType::Boolean)
                }
            }
        }
        ExprKind::Binary { op, left, right } => {
            let left_type = check_node(left, scope, allow_roll)?;
            let right_type = check_node(right, scope, allow_roll)?;
            match op {
                BinaryOp::Equal | BinaryOp::NotEqual => {
                    if left_type != right_type {
                        return Err(error(
                            node.start,
                            format!(
                                "equality requires matching operand types, found {left_type:?} and {right_type:?}"
                            ),
                        ));
                    }
                    Ok(ValueType::Boolean)
                }
                BinaryOp::Less
                | BinaryOp::LessEqual
                | BinaryOp::Greater
                | BinaryOp::GreaterEqual => {
                    if left_type != ValueType::Integer || right_type != ValueType::Integer {
                        return Err(error(
                            node.start,
                            format!(
                                "ordering requires integers, found {left_type:?} and {right_type:?}"
                            ),
                        ));
                    }
                    Ok(ValueType::Boolean)
                }
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => {
                    if left_type != ValueType::Integer || right_type != ValueType::Integer {
                        return Err(error(
                            node.start,
                            format!(
                                "arithmetic requires integers, found {left_type:?} and {right_type:?}"
                            ),
                        ));
                    }
                    Ok(ValueType::Integer)
                }
                BinaryOp::And | BinaryOp::Or => {
                    if left_type != ValueType::Boolean || right_type != ValueType::Boolean {
                        return Err(error(
                            node.start,
                            format!(
                                "boolean operators require booleans, found {left_type:?} and {right_type:?}"
                            ),
                        ));
                    }
                    Ok(ValueType::Boolean)
                }
            }
        }
        ExprKind::Conditional {
            cond,
            then_branch,
            else_branch,
        } => {
            let cond_type = check_node(cond, scope, allow_roll)?;
            if cond_type != ValueType::Boolean {
                return Err(error(
                    cond.start,
                    "`if` condition must be boolean".to_string(),
                ));
            }
            let then_type = check_node(then_branch, scope, allow_roll)?;
            let else_type = check_node(else_branch, scope, allow_roll)?;
            if then_type != else_type {
                return Err(error(
                    node.start,
                    format!(
                        "`if` branches must have the same type, found {then_type:?} and {else_type:?}"
                    ),
                ));
            }
            Ok(then_type)
        }
        ExprKind::Dice { .. } => {
            if !allow_roll {
                return Err(error(node.start, "dice rolls are not allowed here"));
            }
            Ok(ValueType::Integer)
        }
    }
}

/// Depth of a constructed syntax tree, with leaves at depth 1.
fn ast_depth(node: &ExprNode) -> usize {
    match &node.kind {
        ExprKind::Integer(_)
        | ExprKind::Boolean(_)
        | ExprKind::Text(_)
        | ExprKind::Variable(_)
        | ExprKind::Dice { .. } => 1,
        ExprKind::Unary { operand, .. } => 1 + ast_depth(operand),
        ExprKind::Binary { left, right, .. } => 1 + ast_depth(left).max(ast_depth(right)),
        ExprKind::Conditional {
            cond,
            then_branch,
            else_branch,
        } => 1 + ast_depth(cond).max(ast_depth(then_branch).max(ast_depth(else_branch))),
    }
}

/// Attach a binary node only when the resulting tree stays within budget.
fn join_binary(op: BinaryOp, left: ExprNode, right: ExprNode) -> Result<ExprNode, ExpressionError> {
    let (start, end) = (left.start, right.end);
    let depth = 1 + ast_depth(&left).max(ast_depth(&right));
    if depth > MAX_AST_DEPTH {
        return Err(error(start, "expression tree exceeds depth limit"));
    }
    Ok(ExprNode {
        kind: ExprKind::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        },
        start,
        end,
    })
}

fn error(offset: usize, reason: impl Into<String>) -> ExpressionError {
    ExpressionError {
        offset,
        reason: reason.into(),
    }
}

/// Check a `roll("...")` literal against the bounded `[N]dS` subset and
/// delegate to diceman for the parsed dice plan. Returns the checked dice
/// expression; the caller's original literal is stored on the AST node.
fn check_dice_literal(literal: &str, offset: usize) -> Result<diceman::Expr, ExpressionError> {
    let fail = |reason: &str| {
        error(
            offset,
            format!("invalid dice literal `{literal}`: {reason}"),
        )
    };
    if literal.bytes().any(|b| b.is_ascii_whitespace()) {
        return Err(fail("no internal whitespace allowed"));
    }
    let Some(d) = literal.find('d') else {
        return Err(fail("expected `[N]dS` form"));
    };
    if literal[d + 1..].contains('d') || literal.contains('D') {
        return Err(fail("expected `[N]dS` form"));
    }
    let (count_text, sides_text) = (&literal[..d], &literal[d + 1..]);
    if !sides_text.bytes().all(|b| b.is_ascii_digit()) || sides_text.is_empty() {
        return Err(fail("sides must be ASCII decimal digits"));
    }
    if !count_text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(fail("count must be ASCII decimal digits"));
    }
    let count: u32 = if count_text.is_empty() {
        1
    } else {
        count_text
            .parse()
            .map_err(|_| fail("count must be ASCII decimal digits"))?
    };
    let sides: u32 = sides_text
        .parse()
        .map_err(|_| fail("sides must be ASCII decimal digits"))?;
    if !(1..=1000).contains(&count) {
        return Err(fail("count must be 1-1000"));
    }
    if !(1..=1_000_000).contains(&sides) {
        return Err(fail("sides must be 1-1000000"));
    }
    let normalized = format!("{count}d{sides}");
    diceman::parse(&normalized).map_err(|_| fail("diceman rejected the dice literal"))
}

#[derive(Debug, Clone, PartialEq)]
struct Token {
    kind: TokenKind,
    start: usize,
    end: usize,
}

#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    Integer(i64),
    Text(String),
    Identifier(String),
    If,
    Then,
    Else,
    True,
    False,
    Roll,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Bang,
    AmpAmp,
    PipePipe,
    LeftParen,
    RightParen,
}

fn tokenize(source: &str) -> Result<Vec<Token>, ExpressionError> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = source[i..].chars().next().unwrap();
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        if c.is_ascii_digit() {
            let mut j = i;
            while j < bytes.len() && (bytes[j] as char).is_ascii_digit() {
                j += 1;
            }
            let text = &source[start..j];
            match text.parse::<i64>() {
                Ok(value) => tokens.push(Token {
                    kind: TokenKind::Integer(value),
                    start,
                    end: j,
                }),
                Err(_) => return Err(error(start, "integer literal out of range")),
            }
            i = j;
            continue;
        }
        if c == '"' {
            let mut value = String::new();
            let mut j = i + 1;
            let mut closed = false;
            while j < source.len() {
                let rest = &source[j..];
                let ch = rest.chars().next().unwrap();
                if ch == '"' {
                    closed = true;
                    j += 1;
                    break;
                }
                if ch == '\\' {
                    let escape_at = j;
                    let next = rest[ch.len_utf8()..].chars().next();
                    match next {
                        Some('"') => value.push('"'),
                        Some('\\') => value.push('\\'),
                        Some('n') => value.push('\n'),
                        Some('r') => value.push('\r'),
                        Some('t') => value.push('\t'),
                        _ => return Err(error(escape_at, "unknown string escape")),
                    }
                    j += ch.len_utf8() + 1;
                    continue;
                }
                if ch.is_control() {
                    return Err(error(j, "raw control character in string literal"));
                }
                value.push(ch);
                j += ch.len_utf8();
            }
            if !closed {
                return Err(error(start, "unterminated string literal"));
            } else {
                tokens.push(Token {
                    kind: TokenKind::Text(value),
                    start,
                    end: j,
                });
            }
            i = j;
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            let mut j = i;
            while j < bytes.len()
                && ((bytes[j] as char).is_ascii_alphanumeric() || bytes[j] == b'_')
            {
                j += 1;
            }
            let word = &source[start..j];
            let kind = match word {
                "if" => TokenKind::If,
                "then" => TokenKind::Then,
                "else" => TokenKind::Else,
                "true" => TokenKind::True,
                "false" => TokenKind::False,
                "roll" => TokenKind::Roll,
                _ => TokenKind::Identifier(word.to_string()),
            };
            tokens.push(Token {
                kind,
                start,
                end: j,
            });
            i = j;
            continue;
        }
        let two = if i + 1 < bytes.len() {
            source.get(i..i + 2)
        } else {
            None
        };
        let kind = match two {
            Some("==") => Some(TokenKind::Equal),
            Some("!=") => Some(TokenKind::NotEqual),
            Some("<=") => Some(TokenKind::LessEqual),
            Some(">=") => Some(TokenKind::GreaterEqual),
            Some("&&") => Some(TokenKind::AmpAmp),
            Some("||") => Some(TokenKind::PipePipe),
            _ => None,
        };
        if let Some(kind) = kind {
            tokens.push(Token {
                kind,
                start,
                end: start + 2,
            });
            i += 2;
            continue;
        }
        let kind = match c {
            '<' => TokenKind::Less,
            '>' => TokenKind::Greater,
            '+' => TokenKind::Plus,
            '-' => TokenKind::Minus,
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '%' => TokenKind::Percent,
            '!' => TokenKind::Bang,
            '(' => TokenKind::LeftParen,
            ')' => TokenKind::RightParen,
            _ => {
                return Err(error(start, format!("unexpected character `{c}`")));
            }
        };
        tokens.push(Token {
            kind,
            start,
            end: start + 1,
        });
        i += 1;
    }
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    source_len: usize,
    nesting: usize,
}

impl Parser {
    fn descend<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T, ExpressionError>,
    ) -> Result<T, ExpressionError> {
        let next = self.nesting + 1;
        if next > MAX_AST_DEPTH {
            let offset = self
                .tokens
                .get(self.pos)
                .map(|token| token.start)
                .unwrap_or(self.source_len);
            return Err(error(offset, "expression nesting exceeds depth limit"));
        }
        let saved = self.nesting;
        self.nesting = next;
        let result = parse(&mut *self);
        self.nesting = saved;
        result
    }

    fn parse_expression(&mut self) -> Result<ExprNode, ExpressionError> {
        self.parse_conditional()
    }

    fn parse_conditional(&mut self) -> Result<ExprNode, ExpressionError> {
        if self.eat(TokenKind::If) {
            let if_start = self.tokens[self.pos - 1].start;
            let cond = self.descend(Self::parse_expression)?;
            self.expect(TokenKind::Then, "`then`")?;
            let then_branch = self.descend(Self::parse_expression)?;
            self.expect(TokenKind::Else, "`else`")?;
            let else_branch = self.descend(Self::parse_expression)?;
            let end = else_branch.end;
            let depth =
                1 + ast_depth(&cond).max(ast_depth(&then_branch).max(ast_depth(&else_branch)));
            if depth > MAX_AST_DEPTH {
                return Err(error(if_start, "expression tree exceeds depth limit"));
            }
            return Ok(ExprNode {
                kind: ExprKind::Conditional {
                    cond: Box::new(cond),
                    then_branch: Box::new(then_branch),
                    else_branch: Box::new(else_branch),
                },
                start: if_start,
                end,
            });
        }
        self.parse_disjunction()
    }

    fn parse_disjunction(&mut self) -> Result<ExprNode, ExpressionError> {
        let mut left = self.parse_conjunction()?;
        while self.peek_kind() == Some(&TokenKind::PipePipe) {
            self.pos += 1;
            let right = self.parse_conjunction()?;
            left = join_binary(BinaryOp::Or, left, right)?;
        }
        Ok(left)
    }

    fn parse_conjunction(&mut self) -> Result<ExprNode, ExpressionError> {
        let mut left = self.parse_equality()?;
        while self.peek_kind() == Some(&TokenKind::AmpAmp) {
            self.pos += 1;
            let right = self.parse_equality()?;
            left = join_binary(BinaryOp::And, left, right)?;
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<ExprNode, ExpressionError> {
        let left = self.parse_comparison()?;
        let op = match self.peek_kind() {
            Some(TokenKind::Equal) => BinaryOp::Equal,
            Some(TokenKind::NotEqual) => BinaryOp::NotEqual,
            _ => return Ok(left),
        };
        self.pos += 1;
        let right = self.parse_comparison()?;
        if matches!(
            self.peek_kind(),
            Some(TokenKind::Equal | TokenKind::NotEqual)
        ) {
            let offset = self.tokens[self.pos].start;
            return Err(error(offset, "repeated equality comparison"));
        }
        join_binary(op, left, right)
    }

    fn parse_comparison(&mut self) -> Result<ExprNode, ExpressionError> {
        let left = self.parse_sum()?;
        let op = match self.peek_kind() {
            Some(TokenKind::Less) => BinaryOp::Less,
            Some(TokenKind::LessEqual) => BinaryOp::LessEqual,
            Some(TokenKind::Greater) => BinaryOp::Greater,
            Some(TokenKind::GreaterEqual) => BinaryOp::GreaterEqual,
            _ => return Ok(left),
        };
        self.pos += 1;
        let right = self.parse_sum()?;
        if matches!(
            self.peek_kind(),
            Some(
                TokenKind::Less
                    | TokenKind::LessEqual
                    | TokenKind::Greater
                    | TokenKind::GreaterEqual
            )
        ) {
            let offset = self.tokens[self.pos].start;
            return Err(error(offset, "repeated ordering comparison"));
        }
        join_binary(op, left, right)
    }

    fn parse_sum(&mut self) -> Result<ExprNode, ExpressionError> {
        let mut left = self.parse_product()?;
        loop {
            let op = match self.peek_kind() {
                Some(TokenKind::Plus) => BinaryOp::Add,
                Some(TokenKind::Minus) => BinaryOp::Sub,
                _ => return Ok(left),
            };
            self.pos += 1;
            let right = self.parse_product()?;
            left = join_binary(op, left, right)?;
        }
    }

    fn parse_product(&mut self) -> Result<ExprNode, ExpressionError> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek_kind() {
                Some(TokenKind::Star) => BinaryOp::Mul,
                Some(TokenKind::Slash) => BinaryOp::Div,
                Some(TokenKind::Percent) => BinaryOp::Rem,
                _ => return Ok(left),
            };
            self.pos += 1;
            let right = self.parse_unary()?;
            left = join_binary(op, left, right)?;
        }
    }

    fn parse_unary(&mut self) -> Result<ExprNode, ExpressionError> {
        let op = match self.peek_kind() {
            Some(TokenKind::Minus) => UnaryOp::Neg,
            Some(TokenKind::Bang) => UnaryOp::Not,
            _ => return self.parse_primary(),
        };
        let start = self.tokens[self.pos].start;
        self.pos += 1;
        let operand = self.descend(Self::parse_unary)?;
        let end = operand.end;
        if 1 + ast_depth(&operand) > MAX_AST_DEPTH {
            return Err(error(start, "expression tree exceeds depth limit"));
        }
        Ok(ExprNode {
            kind: ExprKind::Unary {
                op,
                operand: Box::new(operand),
            },
            start,
            end,
        })
    }

    fn parse_primary(&mut self) -> Result<ExprNode, ExpressionError> {
        let token = self.next_token()?;
        match token.kind {
            TokenKind::Integer(value) => Ok(ExprNode {
                kind: ExprKind::Integer(value),
                start: token.start,
                end: token.end,
            }),
            TokenKind::Text(value) => Ok(ExprNode {
                kind: ExprKind::Text(value),
                start: token.start,
                end: token.end,
            }),
            TokenKind::True => Ok(ExprNode {
                kind: ExprKind::Boolean(true),
                start: token.start,
                end: token.end,
            }),
            TokenKind::False => Ok(ExprNode {
                kind: ExprKind::Boolean(false),
                start: token.start,
                end: token.end,
            }),
            TokenKind::Identifier(name) => Ok(ExprNode {
                kind: ExprKind::Variable(name),
                start: token.start,
                end: token.end,
            }),
            TokenKind::LeftParen => {
                let inner = self.descend(Self::parse_expression)?;
                self.expect(TokenKind::RightParen, "`)`")?;
                Ok(inner)
            }
            TokenKind::Roll => {
                let start = token.start;
                self.expect(TokenKind::LeftParen, "`(` after `roll`")?;
                let (literal, literal_offset) = self.expect_string()?;
                let end_token = self.expect(TokenKind::RightParen, "`)`")?;
                let dice = check_dice_literal(&literal, literal_offset)?;
                Ok(ExprNode {
                    kind: ExprKind::Dice {
                        literal,
                        expr: dice,
                    },
                    start,
                    end: end_token.end,
                })
            }
            _ => Err(error(token.start, "expected an expression")),
        }
    }

    fn expect_string(&mut self) -> Result<(String, usize), ExpressionError> {
        match self.tokens.get(self.pos).cloned() {
            Some(token) => match token.kind {
                TokenKind::Text(value) => {
                    self.pos += 1;
                    Ok((value, token.start))
                }
                _ => Err(error(token.start, "`roll()` requires a string literal")),
            },
            None => Err(error(self.source_len, "`roll()` requires a string literal")),
        }
    }

    fn peek_kind(&self) -> Option<&TokenKind> {
        self.tokens.get(self.pos).map(|token| &token.kind)
    }

    fn eat(&mut self, kind: TokenKind) -> bool {
        if self.peek_kind() == Some(&kind) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: TokenKind, what: &str) -> Result<Token, ExpressionError> {
        match self.tokens.get(self.pos).cloned() {
            Some(token) if token.kind == kind => {
                self.pos += 1;
                Ok(token)
            }
            Some(token) => Err(error(token.start, format!("expected {what}"))),
            None => Err(error(self.source_len, format!("expected {what}"))),
        }
    }

    fn next_token(&mut self) -> Result<Token, ExpressionError> {
        match self.tokens.get(self.pos).cloned() {
            Some(token) => {
                self.pos += 1;
                Ok(token)
            }
            None => Err(error(self.source_len, "expected an expression")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expression_checks_conditional_types() {
        let scope = TypeScope::from([("count".into(), ValueType::Integer)]);
        let expr = parse(r#"if count == 1 then "gem" else "gems""#).unwrap();
        assert_eq!(check(&expr, &scope, false).unwrap(), ValueType::Text);
        let invalid = parse(r#"if true then 1 else "gems""#).unwrap();
        assert!(check(&invalid, &scope, false).is_err());
    }

    #[test]
    fn expression_treats_keywords_as_whole_tokens() {
        let scope = TypeScope::new();
        let expr = parse("ifx == 1").unwrap();
        let error = check(&expr, &scope, false).unwrap_err();
        assert!(error.reason.contains("ifx"), "got: {}", error.reason);
        let expr = parse("truex").unwrap();
        let error = check(&expr, &scope, false).unwrap_err();
        assert!(error.reason.contains("truex"), "got: {}", error.reason);
    }

    #[test]
    fn expression_requires_parentheses_around_conditional_operands() {
        assert!(parse("1 + if true then 2 else 3").is_err());
        let scope = TypeScope::new();
        let expr = parse("(if true then 2 else 3) + 1").unwrap();
        assert_eq!(check(&expr, &scope, false).unwrap(), ValueType::Integer);
    }

    #[test]
    fn expression_pins_nesting_depth_against_ast_depth() {
        let allowed = format!("{}1", "-".repeat(63));
        assert!(parse(&allowed).is_ok());
        let nested = format!("{}1", "-".repeat(64));
        assert!(parse(&nested).is_err());
        let allowed_if = format!("{}1{}", "if true then ".repeat(63), " else 1".repeat(63));
        assert!(parse(&allowed_if).is_ok());
        let nested_if = format!("{}1{}", "if true then ".repeat(64), " else 1".repeat(64));
        assert!(parse(&nested_if).is_err());
        // Discarded group parentheses cost nesting budget but leave a
        // depth-1 AST that checks as a plain integer.
        let scope = TypeScope::new();
        let grouped = format!("{}1{}", "(".repeat(63), ")".repeat(63));
        let expr = parse(&grouped).unwrap();
        assert_eq!(check(&expr, &scope, false).unwrap(), ValueType::Integer);
    }

    #[test]
    fn expression_bounds_constructed_ast_depth() {
        let chain = (0..65).map(|_| "1").collect::<Vec<_>>().join(" + ");
        assert!(parse(&chain).is_err());
        let allowed = (0..64).map(|_| "1").collect::<Vec<_>>().join(" + ");
        assert!(parse(&allowed).is_ok());
    }

    #[test]
    fn expression_rejects_dice_when_rolls_are_disallowed() {
        let scope = TypeScope::new();
        let expr = parse("roll(\"1d6\") + 1").unwrap();
        assert!(check(&expr, &scope, false).is_err());
        assert_eq!(check(&expr, &scope, true).unwrap(), ValueType::Integer);
        let hidden = parse("if false then roll(\"1d6\") else 1").unwrap();
        assert!(check(&hidden, &scope, false).is_err());
    }

    #[test]
    fn expression_rejects_prohibited_dice_syntax() {
        for source in [
            "roll(\"2d6x10\")",
            "roll(\"4d6kh3\")",
            "roll(\"1d6!\")",
            "roll(\"2D6\")",
            "roll(\"2d 6\")",
            "roll(\" 2d6\")",
            "roll(\"\")",
            "roll(\"abc\")",
            "roll(\"D66\")",
            "roll(\"1d6+2\")",
            "roll(6)",
            "roll(count)",
            "roll(\"1d6\", \"2d6\")",
            "roll()",
            "bounce(\"1d6\")",
        ] {
            assert!(
                parse(source).is_err(),
                "expected dice rejection for `{source}`"
            );
        }
    }

    #[test]
    fn expression_rejects_out_of_bounds_dice_in_dead_branches() {
        assert!(parse("roll(\"0d6\")").is_err());
        assert!(parse("roll(\"1001d6\")").is_err());
        assert!(parse("roll(\"1d0\")").is_err());
        assert!(parse("roll(\"1d1000001\")").is_err());
        assert!(parse("if false then roll(\"0d6\") else 1").is_err());
        assert!(parse("if false then 1 / 0 else 1").is_ok());
    }

    #[test]
    fn expression_accepts_bounded_dice_literals() {
        let scope = TypeScope::new();
        for source in ["roll(\"1d6\")", "roll(\"d20\")", "roll(\"1000d1000000\")"] {
            let expr = parse(source).unwrap();
            assert_eq!(check(&expr, &scope, true).unwrap(), ValueType::Integer);
        }
    }

    #[test]
    fn expression_enforces_source_and_nesting_limits() {
        let oversized = format!("1{}", " ".repeat(MAX_SOURCE_BYTES));
        assert!(oversized.len() > MAX_SOURCE_BYTES);
        assert!(parse(&oversized).is_err());
        let nested = format!("{}1{}", "(".repeat(64), ")".repeat(64));
        assert!(parse(&nested).is_err());
        // The root counts as depth 1, so 63 enclosing groups fit the
        // depth-64 budget while the 64th exceeds it.
        let allowed = format!("{}1{}", "(".repeat(63), ")".repeat(63));
        assert!(parse(&allowed).is_ok());
    }

    #[test]
    fn expression_rejects_malformed_or_out_of_range_integers() {
        let scope = TypeScope::new();
        let error = parse("9999999999999999999999").unwrap_err();
        assert_eq!(error.offset, 0);
        let error = parse("-9223372036854775808").unwrap_err();
        assert_eq!(error.offset, 1);
        let expr = parse("9223372036854775807").unwrap();
        assert_eq!(check(&expr, &scope, false).unwrap(), ValueType::Integer);
    }

    #[test]
    fn expression_checks_both_conditional_branches() {
        let scope = TypeScope::new();
        let expr = parse("if false then \"a\" else 1").unwrap();
        assert!(check(&expr, &scope, false).is_err());
        let expr = parse("if false then 1 else 2").unwrap();
        assert_eq!(check(&expr, &scope, false).unwrap(), ValueType::Integer);
    }

    #[test]
    fn expression_rejects_operator_type_mismatches() {
        let scope = TypeScope::new();
        for source in [
            "1 + true",
            "true && 1",
            "1 || false",
            "\"a\" < \"b\"",
            "1 == \"a\"",
            "-true",
            "!1",
        ] {
            let expr = parse(source).unwrap();
            assert!(
                check(&expr, &scope, false).is_err(),
                "expected a type error for `{source}`"
            );
        }
        for (source, expected) in [
            ("1 + 2", ValueType::Integer),
            ("true && false", ValueType::Boolean),
            ("true || false", ValueType::Boolean),
            ("!true", ValueType::Boolean),
            ("-5", ValueType::Integer),
            ("1 == 2", ValueType::Boolean),
            ("\"a\" == \"b\"", ValueType::Boolean),
        ] {
            let expr = parse(source).unwrap();
            assert_eq!(check(&expr, &scope, false).unwrap(), expected);
        }
    }

    #[test]
    fn expression_rejects_unknown_names() {
        let scope = TypeScope::from([("count".into(), ValueType::Integer)]);
        let expr = parse("total + 1").unwrap();
        let error = check(&expr, &scope, false).unwrap_err();
        assert!(error.reason.contains("total"), "got: {}", error.reason);
        assert_eq!(error.offset, 0);
    }

    #[test]
    fn expression_handles_string_escapes() {
        let scope = TypeScope::new();
        for source in [
            r#""a\"b""#,
            r#""a\\b""#,
            r#""a\nb""#,
            r#""a\rb""#,
            r#""a\tb""#,
        ] {
            let combined = format!("{source} == {source}");
            let expr = parse(&combined).unwrap();
            assert_eq!(check(&expr, &scope, false).unwrap(), ValueType::Boolean);
        }
        assert!(parse(r#""\q""#).is_err());
        assert!(parse("\"\\u0041\"").is_err());
        assert!(parse("\"a\nb\"").is_err());
        assert!(parse("\"abc").is_err());
    }

    #[test]
    fn expression_supports_unicode_strings_with_byte_offsets() {
        let scope = TypeScope::new();
        let expr = parse(r#"if "héllo" == "héllo" then 1 else 2"#).unwrap();
        assert_eq!(check(&expr, &scope, false).unwrap(), ValueType::Integer);
        let error = parse("\"héllo\" \"x\"").unwrap_err();
        assert_eq!(error.offset, 9);
    }

    #[test]
    fn expression_rejects_trailing_input() {
        for source in ["1 2", "1 +", "(1", "1)", "if true then 1"] {
            assert!(
                parse(source).is_err(),
                "expected error for trailing input in `{source}`"
            );
        }
        assert!(parse("").is_err());
    }

    #[test]
    fn expression_rejects_nonboolean_conditions_in_dead_branches() {
        let scope = TypeScope::new();
        let expr = parse("if true then 1 else if 1 then 2 else 3").unwrap();
        let error = check(&expr, &scope, false).unwrap_err();
        assert!(error.reason.contains("boolean"), "got: {}", error.reason);
    }

    #[test]
    fn expression_accepts_mixed_precedence_comparisons() {
        let scope = TypeScope::from([("value".into(), ValueType::Integer)]);
        let expr = parse("value < 3 == true").unwrap();
        assert_eq!(check(&expr, &scope, false).unwrap(), ValueType::Boolean);
    }

    #[test]
    fn expression_rejects_repeated_same_level_comparisons() {
        let first = parse("value < 3 < 5").unwrap_err();
        assert!(first.reason.contains("repeated"), "got: {}", first.reason);
        let second = parse("value == 3 == true").unwrap_err();
        assert!(second.reason.contains("repeated"), "got: {}", second.reason);
    }

    #[test]
    fn expression_checks_comparison_types() {
        let scope = TypeScope::from([("value".into(), ValueType::Integer)]);
        for source in ["value < 3", "value <= 3", "value > 3", "value >= 3"] {
            let expr = parse(source).unwrap();
            assert_eq!(check(&expr, &scope, false).unwrap(), ValueType::Boolean);
        }
        let expr = parse("value != 3").unwrap();
        assert_eq!(check(&expr, &scope, false).unwrap(), ValueType::Boolean);
    }

    #[test]
    fn expression_rejects_non_ascii_outside_strings_without_panicking() {
        let error = parse("€").unwrap_err();
        assert_eq!(error.offset, 0);
        assert!(error.reason.contains('€'), "got: {}", error.reason);
        let error = parse("a€b").unwrap_err();
        assert_eq!(error.offset, 1);
        assert!(error.reason.contains('€'), "got: {}", error.reason);
        let error = parse("héllo").unwrap_err();
        assert_eq!(error.offset, 1);
        assert!(error.reason.contains('é'), "got: {}", error.reason);
    }

    #[test]
    fn expression_skips_unselected_arithmetic() {
        let mut rng = diceman::FastRng::with_seed(19);
        let expr = parse("if true then 7 else 1 / 0").unwrap();
        assert_eq!(
            evaluate(&expr, &ValueScope::new(), &mut rng).unwrap(),
            Value::Integer(7)
        );
        let bad = parse("9223372036854775807 + 1").unwrap();
        assert!(evaluate(&bad, &ValueScope::new(), &mut rng).is_err());
    }

    #[test]
    fn expression_evaluates_dice_with_shared_rng() {
        let scope = ValueScope::new();
        // One RNG drives evaluate(); an identically seeded RNG manually rolls
        // the same diceman plan. Both the outcome and the consumed RNG state
        // (via checkpoint) must agree.
        let mut eval_rng = diceman::FastRng::with_seed(7);
        let mut manual_rng = diceman::FastRng::with_seed(7);
        let expr = parse(r#"roll("1d6")"#).unwrap();
        let evaluated = evaluate(&expr, &scope, &mut eval_rng).unwrap();
        let manual =
            diceman::roller::evaluate_with_rng(&diceman::parse("1d6").unwrap(), &mut manual_rng)
                .unwrap()
                .outcome
                .as_numeric()
                .unwrap();
        assert_eq!(evaluated, Value::Integer(manual));
        assert_eq!(eval_rng.checkpoint(), manual_rng.checkpoint());
    }

    #[test]
    fn expression_rolls_dice_left_to_right() {
        // A seed whose first two 1d6 rolls differ, so operand order is
        // observable: `roll + roll * 100` distinguishes left-to-right
        // (a + b * 100) from right-to-left (b + a * 100).
        let seed = (0..1000u64)
            .find(|seed| {
                let mut rng = diceman::FastRng::with_seed(*seed);
                let first =
                    diceman::roller::evaluate_with_rng(&diceman::parse("1d6").unwrap(), &mut rng)
                        .unwrap()
                        .outcome
                        .as_numeric()
                        .unwrap();
                let second =
                    diceman::roller::evaluate_with_rng(&diceman::parse("1d6").unwrap(), &mut rng)
                        .unwrap()
                        .outcome
                        .as_numeric()
                        .unwrap();
                first != second
            })
            .expect("a discriminating seed exists");
        let mut eval_rng = diceman::FastRng::with_seed(seed);
        let mut manual_rng = diceman::FastRng::with_seed(seed);
        let scope = ValueScope::new();
        let expr = parse(r#"roll("1d6") + roll("1d6") * 100"#).unwrap();
        let evaluated = evaluate(&expr, &scope, &mut eval_rng).unwrap();
        let first =
            diceman::roller::evaluate_with_rng(&diceman::parse("1d6").unwrap(), &mut manual_rng)
                .unwrap()
                .outcome
                .as_numeric()
                .unwrap();
        let second =
            diceman::roller::evaluate_with_rng(&diceman::parse("1d6").unwrap(), &mut manual_rng)
                .unwrap()
                .outcome
                .as_numeric()
                .unwrap();
        assert_eq!(evaluated, Value::Integer(first + second * 100));
        assert_eq!(eval_rng.checkpoint(), manual_rng.checkpoint());
    }

    #[test]
    fn expression_skips_dice_in_unselected_branches() {
        let scope = ValueScope::new();
        // Lazy branches must not advance RNG state: the checkpoint before
        // and after evaluation is identical.
        for source in [
            r#"if false then roll("1d6") else 42"#,
            r#"if true then 42 else roll("1d6")"#,
            r#"false && (roll("1d6") == 0)"#,
            r#"true || (roll("1d6") == 0)"#,
        ] {
            let mut rng = diceman::FastRng::with_seed(7);
            let before = rng.checkpoint();
            let expr = parse(source).unwrap();
            let value = evaluate(&expr, &scope, &mut rng).unwrap();
            assert_eq!(rng.checkpoint(), before, "for `{source}`");
            let _ = value;
        }
        // Pure expressions leave the RNG untouched as well.
        let mut rng = diceman::FastRng::with_seed(7);
        let before = rng.checkpoint();
        let expr = parse("1 + 2 * 3").unwrap();
        assert_eq!(
            evaluate(&expr, &scope, &mut rng).unwrap(),
            Value::Integer(7)
        );
        assert_eq!(rng.checkpoint(), before);
    }

    #[test]
    fn expression_resolves_scope_variables() {
        let mut rng = diceman::FastRng::with_seed(19);
        let scope = ValueScope::from([
            ("count".into(), Value::Integer(3)),
            ("name".into(), Value::Text("gem".into())),
            ("flag".into(), Value::Boolean(true)),
        ]);
        let expr = parse("count * 25").unwrap();
        assert_eq!(
            evaluate(&expr, &scope, &mut rng).unwrap(),
            Value::Integer(75)
        );
        let words = parse(r#"if flag then name else "none""#).unwrap();
        assert_eq!(
            evaluate(&words, &scope, &mut rng).unwrap(),
            Value::Text("gem".into())
        );
        let missing = parse("total + 1").unwrap();
        let error = evaluate(&missing, &scope, &mut rng).unwrap_err();
        assert!(error.reason.contains("total"), "got: {}", error.reason);
        // A text value used where an integer is required fails at runtime,
        // mirroring the static type checker's rejection.
        let wrong_type = parse("name + 1").unwrap();
        assert!(evaluate(&wrong_type, &scope, &mut rng).is_err());
        let wrong_cond = parse("if count then 1 else 2").unwrap();
        assert!(evaluate(&wrong_cond, &scope, &mut rng).is_err());
    }

    #[test]
    fn expression_evaluates_text_and_equality_values() {
        let mut rng = diceman::FastRng::with_seed(19);
        let scope = ValueScope::new();
        // Supported escapes were accepted by the parser in Task 1 but their
        // decoded values were never pinned; pin them here.
        for (source, expected) in [
            (r#""a\nb""#, "a\nb"),
            (r#""a\rb""#, "a\rb"),
            (r#""a\tb""#, "a\tb"),
            (r#""a\"b""#, "a\"b"),
            (r#""a\\b""#, "a\\b"),
        ] {
            let expr = parse(source).unwrap();
            assert_eq!(
                evaluate(&expr, &scope, &mut rng).unwrap(),
                Value::Text(expected.to_string()),
                "for `{source}`"
            );
        }
        for (source, expected) in [
            (r#""a" == "a""#, true),
            (r#""a" != "b""#, true),
            (r#""a" == "b""#, false),
            ("true == true", true),
            ("true != false", true),
            ("false == false", true),
            ("1 == 1", true),
            ("1 != 2", true),
        ] {
            let expr = parse(source).unwrap();
            assert_eq!(
                evaluate(&expr, &scope, &mut rng).unwrap(),
                Value::Boolean(expected),
                "for `{source}`"
            );
        }
        let mismatched = parse(r#"1 == "a""#).unwrap();
        assert!(evaluate(&mismatched, &scope, &mut rng).is_err());
    }

    #[test]
    fn expression_evaluates_boolean_operators_lazily() {
        let mut rng = diceman::FastRng::with_seed(19);
        let scope = ValueScope::new();
        for (source, expected) in [
            ("!true", false),
            ("!false", true),
            ("true && false", false),
            ("true || false", true),
            ("1 < 2", true),
            ("2 <= 2", true),
            ("3 > 2", true),
            ("2 >= 3", false),
        ] {
            let expr = parse(source).unwrap();
            assert_eq!(
                evaluate(&expr, &scope, &mut rng).unwrap(),
                Value::Boolean(expected),
                "for `{source}`"
            );
        }
        // Short-circuiting must skip the failing division on the right.
        for source in ["false && (1 / 0 == 0)", "true || (1 / 0 == 0)"] {
            let expr = parse(source).unwrap();
            let expected = source.starts_with("false");
            assert_eq!(
                evaluate(&expr, &scope, &mut rng).unwrap(),
                Value::Boolean(!expected),
                "for `{source}`"
            );
        }
    }

    #[test]
    fn expression_evaluates_checked_negation() {
        let mut rng = diceman::FastRng::with_seed(19);
        let scope = ValueScope::new();
        let expr = parse("-5").unwrap();
        assert_eq!(
            evaluate(&expr, &scope, &mut rng).unwrap(),
            Value::Integer(-5)
        );
        let double = parse("- -5").unwrap();
        assert_eq!(
            evaluate(&double, &scope, &mut rng).unwrap(),
            Value::Integer(5)
        );
        let overflow = parse("-(0 - 9223372036854775807 - 1)").unwrap();
        assert!(evaluate(&overflow, &scope, &mut rng).is_err());
    }

    #[test]
    fn expression_evaluates_checked_remainder() {
        let mut rng = diceman::FastRng::with_seed(19);
        let scope = ValueScope::new();
        for (source, expected) in [("7 % 3", 1), ("(0 - 7) % 3", -1), ("7 % (0 - 3)", 1)] {
            let expr = parse(source).unwrap();
            assert_eq!(
                evaluate(&expr, &scope, &mut rng).unwrap(),
                Value::Integer(expected),
                "for `{source}`"
            );
        }
        for source in [
            "1 % 0",
            "1 % (2 - 2)",
            "(0 - 9223372036854775807 - 1) % (0 - 1)",
        ] {
            let expr = parse(source).unwrap();
            assert!(
                evaluate(&expr, &scope, &mut rng).is_err(),
                "expected an error for `{source}`"
            );
        }
    }

    #[test]
    fn expression_evaluates_checked_division() {
        let mut rng = diceman::FastRng::with_seed(19);
        let scope = ValueScope::new();
        for (source, expected) in [
            ("7 / 2", 3),
            ("(0 - 7) / 2", -3),
            ("7 / (0 - 2)", -3),
            ("(0 - 7) / (0 - 2)", 3),
        ] {
            let expr = parse(source).unwrap();
            assert_eq!(
                evaluate(&expr, &scope, &mut rng).unwrap(),
                Value::Integer(expected),
                "for `{source}`"
            );
        }
        for source in [
            "1 / 0",
            "1 / (2 - 2)",
            "(0 - 9223372036854775807 - 1) / (0 - 1)",
        ] {
            let expr = parse(source).unwrap();
            assert!(
                evaluate(&expr, &scope, &mut rng).is_err(),
                "expected an error for `{source}`"
            );
        }
    }

    #[test]
    fn expression_evaluates_checked_subtraction() {
        let mut rng = diceman::FastRng::with_seed(19);
        let scope = ValueScope::new();
        let expr = parse("10 - 3 - 2").unwrap();
        assert_eq!(
            evaluate(&expr, &scope, &mut rng).unwrap(),
            Value::Integer(5)
        );
        let overflow = parse("0 - 9223372036854775807 - 2").unwrap();
        assert!(evaluate(&overflow, &scope, &mut rng).is_err());
    }

    #[test]
    fn expression_evaluates_arithmetic_precedence() {
        let mut rng = diceman::FastRng::with_seed(19);
        let scope = ValueScope::new();
        let plain = parse("1 + 2 * 3").unwrap();
        assert_eq!(
            evaluate(&plain, &scope, &mut rng).unwrap(),
            Value::Integer(7)
        );
        let grouped = parse("(1 + 2) * 3").unwrap();
        assert_eq!(
            evaluate(&grouped, &scope, &mut rng).unwrap(),
            Value::Integer(9)
        );
    }

    #[test]
    fn expression_applies_arithmetic_precedence() {
        let scope = TypeScope::new();
        let expr = parse("1 + 2 * 3 == 7").unwrap();
        assert_eq!(check(&expr, &scope, false).unwrap(), ValueType::Boolean);
        let grouped = parse("(1 + 2) * 3 == 9").unwrap();
        assert_eq!(check(&grouped, &scope, false).unwrap(), ValueType::Boolean);
    }
}
