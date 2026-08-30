//! Parsed Rust-shader-subset syntax. A parsed file is NOT yet semantically valid
//! shader code -- the subset gate and lowering decide that.

/// A parsed source file: a flat list of top-level items.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedModule {
    /// Every top-level item found in the source.
    pub items: Vec<Item>,
}

/// One top-level item.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// A function definition.
    Function(Function),
    /// A struct definition.
    Struct(StructDecl),
    /// A const definition.
    Const(ConstDecl),
    /// A construct the parser recognized but the subset does not support.
    Unsupported {
        /// What kind of construct this was (diagnostic text).
        kind: String,
    },
}

/// A parsed function.
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    /// The function's name.
    pub name: String,
    /// Attributes attached to the function (`#[vertex]`, `#[compute(8,8,1)]`, ...).
    pub attributes: Vec<Attribute>,
    /// Parameters.
    pub params: Vec<Param>,
    /// Return type, if any.
    pub return_ty: Option<TypeRef>,
    /// Function body statements.
    pub body: Vec<Stmt>,
    /// True when the function declares generic parameters — rejected by the subset gate.
    pub has_generics: bool,
}

/// A parsed struct definition.
#[derive(Debug, Clone, PartialEq)]
pub struct StructDecl {
    /// The struct's name.
    pub name: String,
    /// The struct's fields.
    pub fields: Vec<Param>,
}
/// A parsed const definition.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstDecl {
    /// The const's name.
    pub name: String,
    /// The const's declared type.
    pub ty: TypeRef,
    /// The const's value expression.
    pub value: Expr,
}
/// A parameter or struct field.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    /// The parameter/field name.
    pub name: String,
    /// The parameter/field type.
    pub ty: TypeRef,
    /// Attributes attached to this parameter (`@builtin`, `@location`, ...).
    pub attributes: Vec<Attribute>,
}
/// A parsed attribute, e.g. `#[compute(8, 8, 1)]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    /// The attribute's name.
    pub name: String,
    /// The attribute's arguments, as raw text.
    pub args: Vec<String>,
}

/// A referenced type in shader-subset source.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeRef {
    /// `bool`.
    Bool,
    /// `i32`.
    I32,
    /// `u32`.
    U32,
    /// `f32`.
    F32,
    /// A 2-component float vector.
    Vec2F,
    /// A 3-component unsigned vector.
    Vec3U,
    /// A 4-component float vector.
    Vec4F,
    /// A named struct type.
    Custom(String),
}
/// A parsed statement.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// `return <expr>?;`
    Return(Option<Expr>),
    /// `let <name>: <ty>? = <expr>;`
    Let {
        /// The bound name.
        name: String,
        /// The declared type, if any.
        ty: Option<TypeRef>,
        /// The initializer expression.
        expr: Expr,
    },
    /// A bare expression statement.
    Expr(Expr),
}
/// A parsed expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A bare identifier reference.
    Ident(String),
    /// A float literal.
    Float(f32),
    /// An unsigned integer literal.
    UInt(u32),
    /// A function/method call.
    Call {
        /// The callee name.
        callee: String,
        /// Call arguments.
        args: Vec<Expr>,
    },
}
