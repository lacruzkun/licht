use std::fmt;

/// Types known to the Licht type checker.
///
/// Programmers can only write `int`, `float`, `bool` and `string` in
/// annotations. `Unit`, `Never`, `List` and `Func` are tracked internally
/// by the checker (collection element types are never written by the user).
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Bool,
    Str,
    /// "No value": the result of `print`, of a function with no return type, etc.
    Unit,
    /// The type of something that never produces a value (e.g. `return`).
    /// It is compatible with every other type.
    Never,
    List(Box<Type>),
    Func(Vec<Type>, Box<Type>),
}

impl Type {
    /// Maps a written annotation (`int`, `float`, `bool`, `string`) to a type.
    pub fn from_name(name: &str) -> Option<Type> {
        match name {
            "int" => Some(Type::Int),
            "float" => Some(Type::Float),
            "bool" => Some(Type::Bool),
            "string" => Some(Type::Str),
            _ => None,
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::Bool => write!(f, "bool"),
            Type::Str => write!(f, "string"),
            Type::Unit => write!(f, "unit"),
            Type::Never => write!(f, "never"),
            Type::List(inner) => write!(f, "list of {}", inner),
            Type::Func(params, ret) => {
                let parts: Vec<String> = params.iter().map(|p| p.to_string()).collect();
                write!(f, "fn({}) -> {}", parts.join(", "), ret)
            }
        }
    }
}

/// Can a value of type `actual` be used where `expected` is required?
pub fn compatible(expected: &Type, actual: &Type) -> bool {
    match (expected, actual) {
        (_, Type::Never) => true,
        (Type::List(e), Type::List(a)) => compatible(e, a),
        _ => expected == actual,
    }
}
