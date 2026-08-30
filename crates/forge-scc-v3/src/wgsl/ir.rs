//! `ShaderCoreIR` -- a small typed shader IR, closer to WGSL than to Rust MIR.
//! Stores shader stage, address space, group/binding, IO location/builtin, and
//! scalar/vector types. The emitter is a pure IR -> text pass over this.

/// A lowered, typed shader module — the emitter's sole input.
#[derive(Debug, Clone, PartialEq)]
pub struct ShaderModule {
    /// The module's name.
    pub name: String,
    /// Every shader entry point in the module.
    pub entry_points: Vec<EntryPoint>,
    /// Non-entry-point functions (unused in v0's minimal lowering).
    pub functions: Vec<IrFunction>,
    /// Bound resources (uniforms, storage buffers, textures).
    pub resources: Vec<ResourceBinding>,
}

/// One shader entry point (a `@vertex`/`@fragment`/`@compute` function).
#[derive(Debug, Clone, PartialEq)]
pub struct EntryPoint {
    /// The entry point's function name.
    pub name: String,
    /// Which shader stage this entry point runs at.
    pub stage: ShaderStage,
    /// Workgroup size, required for `Compute`, absent otherwise.
    pub workgroup_size: Option<[u32; 3]>,
    /// Entry point parameters.
    pub params: Vec<IrParam>,
    /// Return type, if any.
    pub return_ty: Option<IrType>,
}

/// Which pipeline stage an entry point runs at.
#[derive(Debug, Clone, PartialEq)]
pub enum ShaderStage {
    /// The vertex stage.
    Vertex,
    /// The fragment stage.
    Fragment,
    /// The compute stage.
    Compute,
}
/// Where a resource or variable lives.
#[derive(Debug, Clone, PartialEq)]
pub enum AddressSpace {
    /// Local to a function invocation.
    Function,
    /// Private to a single invocation, persists across the function body.
    Private,
    /// Shared within a workgroup (compute only).
    Workgroup,
    /// A read-only bound uniform buffer.
    Uniform,
    /// A read/write bound storage buffer.
    Storage,
}
/// A shader-IR scalar or vector type.
#[derive(Debug, Clone, PartialEq)]
pub enum IrType {
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
    Struct(String),
}

/// A lowered entry-point parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct IrParam {
    /// The parameter name.
    pub name: String,
    /// The parameter type.
    pub ty: IrType,
    /// The WGSL builtin this parameter binds to, if any (`@builtin(...)`).
    pub builtin: Option<String>,
    /// The IO location this parameter binds to, if any (`@location(...)`).
    pub location: Option<u32>,
}

/// A lowered non-entry-point function. v0's lowering never populates these.
#[derive(Debug, Clone, PartialEq)]
pub struct IrFunction {
    /// The function's name.
    pub name: String,
}

/// A bound resource (`@group(g) @binding(b) var<address_space> name: ty`).
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceBinding {
    /// The resource's name.
    pub name: String,
    /// The bind group index.
    pub group: u32,
    /// The binding index within the group.
    pub binding: u32,
    /// Which address space the resource lives in.
    pub address_space: AddressSpace,
    /// The resource's type.
    pub ty: IrType,
}
