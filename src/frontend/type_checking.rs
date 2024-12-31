use super::super::{
    error::{ThrushError, ThrushErrorKind},
    frontend::lexer::{DataTypes, TokenKind},
};

/*

BINARY INSTRUCTION

--------------------
A OPERATOR B
--------------------
*/

#[inline]
fn check_binary_instr_add(a: &DataTypes, b: &DataTypes, line: usize) -> Result<(), ThrushError> {
    match (a, b) {
        (DataTypes::String, DataTypes::String) => Ok(()),
        (
            DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64,
            DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64,
        ) => Ok(()),
        (DataTypes::String, DataTypes::Char) => Ok(()),
        (DataTypes::F32 | DataTypes::F64, DataTypes::F32 | DataTypes::F64) => Ok(()),

        _ => Err(ThrushError::Parse(
            ThrushErrorKind::SyntaxError,
            String::from("Type Checking"),
            format!(
                "Arithmatic addition ({} + {}) is impossible. Check your operands and types.",
                a, b
            ),
            line,
        )),
    }
}

#[inline]
fn check_binary_instr_sub(a: &DataTypes, b: &DataTypes, line: usize) -> Result<(), ThrushError> {
    if let (
        DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64,
        DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64,
    ) = (a, b)
    {
        return Ok(());
    } else if let (DataTypes::F32 | DataTypes::F64, DataTypes::F32 | DataTypes::F64) = (a, b) {
        return Ok(());
    }

    Err(ThrushError::Parse(
        ThrushErrorKind::SyntaxError,
        String::from("Type Checking"),
        format!(
            "Arithmatic subtraction ({} - {}) is impossible. Check your operands and types.",
            a, b
        ),
        line,
    ))
}

#[inline]
fn check_binary_instr_div(a: &DataTypes, b: &DataTypes, line: usize) -> Result<(), ThrushError> {
    if let (
        DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64,
        DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64,
    ) = (a, b)
    {
        return Ok(());
    } else if let (DataTypes::F32 | DataTypes::F64, DataTypes::F32 | DataTypes::F64) = (a, b) {
        return Ok(());
    }

    Err(ThrushError::Parse(
        ThrushErrorKind::SyntaxError,
        String::from("Type Checking"),
        format!(
            "Arithmatic division ({} / {}) is impossible. Check your operands and types.",
            a, b
        ),
        line,
    ))
}

#[inline]
fn check_binary_instr_mul(a: &DataTypes, b: &DataTypes, line: usize) -> Result<(), ThrushError> {
    if let (
        DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64,
        DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64,
    ) = (a, b)
    {
        return Ok(());
    } else if let (DataTypes::F32 | DataTypes::F64, DataTypes::F32 | DataTypes::F64) = (a, b) {
        return Ok(());
    }

    Err(ThrushError::Parse(
        ThrushErrorKind::SyntaxError,
        String::from("Type Checking"),
        format!(
            "Arithmatic multiplication ({} * {}) is impossible. Check your operands and types.",
            a, b
        ),
        line,
    ))
}

#[inline]
fn check_binary_instr_eqeq(a: &DataTypes, b: &DataTypes, line: usize) -> Result<(), ThrushError> {
    if let (
        DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64,
        DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64,
    ) = (a, b)
    {
        return Ok(());
    } else if let (DataTypes::F32 | DataTypes::F64, DataTypes::F32 | DataTypes::F64) = (a, b) {
        return Ok(());
    } else if let (DataTypes::String, DataTypes::String) = (a, b) {
        return Ok(());
    } else if let (DataTypes::Bool, DataTypes::Bool) = (a, b) {
        return Ok(());
    } else if let (DataTypes::Char, DataTypes::Char) = (a, b) {
        return Ok(());
    }

    Err(ThrushError::Parse(
        ThrushErrorKind::SyntaxError,
        String::from("Type Checking"),
        format!(
            "Logical operation ({} == {}) is impossible. Check your operands and types.",
            a, b
        ),
        line,
    ))
}

#[inline]
fn check_binary_instr_bangeq(a: &DataTypes, b: &DataTypes, line: usize) -> Result<(), ThrushError> {
    if let (
        DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64,
        DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64,
    ) = (a, b)
    {
        return Ok(());
    } else if let (DataTypes::F32 | DataTypes::F64, DataTypes::F32 | DataTypes::F64) = (a, b) {
        return Ok(());
    } else if let (DataTypes::String, DataTypes::String) = (a, b) {
        return Ok(());
    } else if let (DataTypes::Bool, DataTypes::Bool) = (a, b) {
        return Ok(());
    } else if let (DataTypes::Char, DataTypes::Char) = (a, b) {
        return Ok(());
    }

    Err(ThrushError::Parse(
        ThrushErrorKind::SyntaxError,
        String::from("Type Checking"),
        format!(
            "Logical operation ({} != {}) is impossible. Check your operands and types.",
            a, b
        ),
        line,
    ))
}

#[inline]
fn check_binary_instr_greater(
    a: &DataTypes,
    b: &DataTypes,
    line: usize,
) -> Result<(), ThrushError> {
    if let (
        DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64 | DataTypes::Bool,
        DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64 | DataTypes::Bool,
    ) = (a, b)
    {
        return Ok(());
    } else if let (DataTypes::F32 | DataTypes::F64, DataTypes::F32 | DataTypes::F64) = (a, b) {
        return Ok(());
    }

    Err(ThrushError::Parse(
        ThrushErrorKind::SyntaxError,
        String::from("Type Checking"),
        format!(
            "Logical operation ({} > {}) is impossible. Check your operands and types.",
            a, b
        ),
        line,
    ))
}

#[inline]
fn check_binary_instr_greatereq(
    a: &DataTypes,
    b: &DataTypes,
    line: usize,
) -> Result<(), ThrushError> {
    if let (
        DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64,
        DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64,
    ) = (a, b)
    {
        return Ok(());
    } else if let (DataTypes::F32 | DataTypes::F64, DataTypes::F32 | DataTypes::F64) = (a, b) {
        return Ok(());
    }

    Err(ThrushError::Parse(
        ThrushErrorKind::SyntaxError,
        String::from("Type Checking"),
        format!(
            "Logical operation ({} >= {}) is impossible. Check your operands and types.",
            a, b
        ),
        line,
    ))
}

#[inline]
fn check_binary_instr_less(a: &DataTypes, b: &DataTypes, line: usize) -> Result<(), ThrushError> {
    if let (
        DataTypes::Bool | DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64,
        DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64 | DataTypes::Bool,
    ) = (a, b)
    {
        return Ok(());
    } else if let (DataTypes::F32 | DataTypes::F64, DataTypes::F32 | DataTypes::F64) = (a, b) {
        return Ok(());
    }

    Err(ThrushError::Parse(
        ThrushErrorKind::SyntaxError,
        String::from("Type Checking"),
        format!(
            "Logical operation ({} < {}) is impossible. Check your operands and types.",
            a, b
        ),
        line,
    ))
}

#[inline]
fn check_binary_instr_lesseq(a: &DataTypes, b: &DataTypes, line: usize) -> Result<(), ThrushError> {
    if let (
        DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64,
        DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64,
    ) = (a, b)
    {
        return Ok(());
    } else if let (DataTypes::F32 | DataTypes::F64, DataTypes::F32 | DataTypes::F64) = (a, b) {
        return Ok(());
    }

    Err(ThrushError::Parse(
        ThrushErrorKind::SyntaxError,
        String::from("Type Checking"),
        format!(
            "Logical operation ({} <= {}) is impossible. Check your operands and types.",
            a, b
        ),
        line,
    ))
}

#[inline]
fn check_binary_instr_and(a: &DataTypes, b: &DataTypes, line: usize) -> Result<(), ThrushError> {
    if let (DataTypes::Bool, DataTypes::Bool) = (a, b) {
        return Ok(());
    }

    Err(ThrushError::Parse(
        ThrushErrorKind::SyntaxError,
        String::from("Type Checking"),
        format!(
            "Logical operation ({} && {}) is impossible. Check your operands and types.",
            a, b
        ),
        line,
    ))
}

#[inline]
fn check_binary_instr_or(a: &DataTypes, b: &DataTypes, line: usize) -> Result<(), ThrushError> {
    if let (
        DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64,
        DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64,
    ) = (a, b)
    {
        return Ok(());
    } else if let (DataTypes::F32 | DataTypes::F64, DataTypes::F32 | DataTypes::F64) = (a, b) {
        return Ok(());
    } else if let (DataTypes::Bool, DataTypes::Bool) = (a, b) {
        return Ok(());
    }

    Err(ThrushError::Parse(
        ThrushErrorKind::SyntaxError,
        String::from("Type Checking"),
        format!(
            "Logical operation ({} || {}) is impossible. Check your operands and types.",
            a, b
        ),
        line,
    ))
}

#[inline]
pub fn check_binary_instr(
    op: &TokenKind,
    a: &DataTypes,
    b: &DataTypes,
    line: usize,
) -> Result<(), ThrushError> {
    match op {
        TokenKind::Plus => check_binary_instr_add(a, b, line),
        TokenKind::Minus => check_binary_instr_sub(a, b, line),
        TokenKind::Slash => check_binary_instr_div(a, b, line),
        TokenKind::Star => check_binary_instr_mul(a, b, line),
        TokenKind::EqEq => check_binary_instr_eqeq(a, b, line),
        TokenKind::BangEq => check_binary_instr_bangeq(a, b, line),
        TokenKind::Greater => check_binary_instr_greater(a, b, line),
        TokenKind::GreaterEq => check_binary_instr_greatereq(a, b, line),
        TokenKind::Less => check_binary_instr_less(a, b, line),
        TokenKind::LessEq => check_binary_instr_lesseq(a, b, line),
        TokenKind::And => check_binary_instr_and(a, b, line),
        TokenKind::Or => check_binary_instr_or(a, b, line),
        _ => Ok(()),
    }
}

/*

UNARY INSTRUCTION

--------------------
OPERATOR B OPERATOR
--------------------
*/

#[inline]
fn check_unary_instr_negate(a: &DataTypes, line: usize) -> Result<(), ThrushError> {
    if let DataTypes::I8
    | DataTypes::I16
    | DataTypes::I32
    | DataTypes::I64
    | DataTypes::F32
    | DataTypes::F64 = a
    {
        return Ok(());
    }

    Err(ThrushError::Parse(
        ThrushErrorKind::SyntaxError,
        String::from("Type Checking"),
        format!(
            "Negative operation (-{}) is impossible. Check your operand and type.",
            a
        ),
        line,
    ))
}

#[inline]
fn check_unary_instr_minusminus(a: &DataTypes, line: usize) -> Result<(), ThrushError> {
    if let DataTypes::I8
    | DataTypes::I16
    | DataTypes::I32
    | DataTypes::I64
    | DataTypes::F32
    | DataTypes::F64 = a
    {
        return Ok(());
    }

    Err(ThrushError::Parse(
        ThrushErrorKind::SyntaxError,
        String::from("Type Checking"),
        format!(
            "Substractive operation (--{} or {}--) is impossible. Check your operand and type.",
            a, a
        ),
        line,
    ))
}

#[inline]
fn check_unary_instr_plusplus(a: &DataTypes, line: usize) -> Result<(), ThrushError> {
    if let DataTypes::I8
    | DataTypes::I16
    | DataTypes::I32
    | DataTypes::I64
    | DataTypes::F32
    | DataTypes::F64 = a
    {
        return Ok(());
    }

    Err(ThrushError::Parse(
        ThrushErrorKind::SyntaxError,
        String::from("Type Checking"),
        format!(
            "Additive operation (++{} or {}++) is impossible. Check your operand and type.",
            a, a
        ),
        line,
    ))
}

#[inline]
fn check_unary_instr_bang(a: &DataTypes, line: usize) -> Result<(), ThrushError> {
    if let DataTypes::Bool = a {
        return Ok(());
    }

    Err(ThrushError::Parse(
        ThrushErrorKind::SyntaxError,
        String::from("Type Checking"),
        format!(
            "Logical operation (!{}) is impossible. Check your operand and type.",
            a
        ),
        line,
    ))
}

#[inline]
pub fn check_unary_instr(op: &TokenKind, a: &DataTypes, line: usize) -> Result<(), ThrushError> {
    match op {
        TokenKind::PlusPlus => check_unary_instr_plusplus(a, line),
        TokenKind::MinusMinus => check_unary_instr_minusminus(a, line),
        TokenKind::Minus => check_unary_instr_negate(a, line),
        TokenKind::Bang => check_unary_instr_bang(a, line),
        _ => Ok(()),
    }
}

#[inline]
pub fn check_type(
    kind: DataTypes,
    target: DataTypes,
    line: usize,
    title: String,
    desc: String,
) -> Result<(), ThrushError> {
    match (kind, target) {
        (DataTypes::Char, DataTypes::Char) => Ok(()),
        (DataTypes::String, DataTypes::String) => Ok(()),
        (DataTypes::Bool, DataTypes::Bool) => Ok(()),
        (DataTypes::I8, DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64) => Ok(()),
        (DataTypes::I16, DataTypes::I16 | DataTypes::I32 | DataTypes::I64) => Ok(()),
        (DataTypes::I32, DataTypes::I32 | DataTypes::I64) => Ok(()),
        (DataTypes::I64, DataTypes::I64) => Ok(()),
        (DataTypes::F32, DataTypes::F32 | DataTypes::F64) => Ok(()),
        (DataTypes::F64, DataTypes::F64) => Ok(()),
        _ => Err(ThrushError::Parse(
            ThrushErrorKind::SyntaxError,
            title,
            desc,
            line,
        )),
    }
}
