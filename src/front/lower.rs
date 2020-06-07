use std::collections::HashMap;

use crate::front::ast;
use crate::mid::ir;

type Result<T> = std::result::Result<T, &'static str>;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Type {
    Bool,
    Int,
}

impl Type {
    fn as_ir_type(&self) -> ir::TypeInfo {
        match self {
            Type::Bool => ir::TypeInfo::Integer { bits: 1 },
            Type::Int => ir::TypeInfo::Integer { bits: 32 },
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct Const {
    ty: Type,
    value: i32,
}

fn parse_type(ty: &ast::Type) -> Result<Type> {
    match ty.string.as_ref() {
        "int" => Ok(Type::Int),
        "bool" => Ok(Type::Bool),
        _ => Err("invalid return type"),
    }
}

fn parse_literal(lit: &str, ty: Option<Type>) -> Result<Const> {
    match ty {
        None => {
            match lit {
                "true" => Ok(Const { ty: Type::Bool, value: true as i32 }),
                "false" => Ok(Const { ty: Type::Bool, value: false as i32 }),
                _ => Err("cannot infer type for literal"),
            }
        }
        Some(Type::Bool) => Ok(Const {
            ty: Type::Bool,
            value: lit.parse::<bool>().map_err(|_| "failed to parse bool")? as i32,
        }),
        Some(Type::Int) => Ok(Const {
            ty: Type::Int,
            value: lit.parse::<i32>().map_err(|_| "failed to parse int")?,
        }),
    }
}

pub fn lower(root: &ast::Function) -> Result<ir::Program> {
    if &root.id.string != "main" { return Err("function should be called main"); };
    let mut ir_program = ir::Program::new();

    let ret_type = parse_type(&root.ret_type)?;
    ir_program.get_func_mut(ir_program.entry).ret_type = ir_program.define_type(ret_type.as_ir_type());

    // (x, true) -> the function should return x
    // (x, false) -> the result of this expression is x
    fn eval(
        value: &ast::Expression,
        expect_ty: Option<Type>,
        ret_type: Type,
        variables: &HashMap<String, Const>,
    ) -> Result<(Const, bool)> {
        match &value.kind {
            ast::ExpressionKind::Literal { value } => {
                parse_literal(&value, expect_ty)
                    .map(|v| (v, false))
            }
            ast::ExpressionKind::Identifier { id } => {
                variables.get(&id.string)
                    .ok_or("undeclared variable")
                    .and_then(|&cst| {
                        if expect_ty.map(|et| et == cst.ty).unwrap_or(true) {
                            Ok((cst, false))
                        } else {
                            Err("type mismatch")
                        }
                    })
            }
            ast::ExpressionKind::Return { value } => {
                eval(value, Some(ret_type), ret_type, variables)
                    .map(|(v, _)| (v, true))
            }
        }
    }

    // for now we just eagerly evaluate everything
    let mut variables: HashMap<String, Const> = Default::default();
    let mut return_value = None;

    for stmt in &root.body.statements {
        match &stmt.kind {
            ast::StatementKind::Declaration(decl) => {
                assert!(!decl.mutable, "mutable variables not supported");
                let init = decl.init.as_ref().ok_or("variables must have initializers for now")?;
                let ty = decl.ty.as_ref().map(parse_type).transpose()?;

                let (value, should_ret) = eval(&init, ty, ret_type, &variables)?;
                if should_ret && return_value.is_none() {
                    return_value = Some(value);
                };
                //the value stored if should_ret doesn't matter but it still needs to exist to allow
                //the (dead) code after the return to compile
                let value = if should_ret {
                    if let Some(ty) = ty {
                        Const { ty, value: -1 }
                    } else {
                        return Err("cannot infer type for variable");
                    }
                } else {
                    value
                };

                if variables.insert(decl.id.string.clone(), value).is_some() {
                    return Err("variable declared twice");
                }
            }
            ast::StatementKind::Expression(expr) => {
                let (value, should_ret) = eval(expr, None, ret_type, &variables)?;
                if should_ret && return_value.is_none() {
                    return_value = Some(value)
                }
            }
        }
    }

    match return_value {
        None => return Err("missing return statement"),
        Some(cst) => {
            let ty = ir_program.define_type(cst.ty.as_ir_type());
            let value = ir::Value::Const(ir::Const { ty, value: cst.value });
            let ret = ir_program.define_term(ir::TerminatorInfo::Return { value });
            ir_program.get_block_mut(ir_program.get_func(ir_program.entry).entry).terminator = ret;
        }
    }

    println!("Variables: {:?}", variables);

    Ok(ir_program)
}