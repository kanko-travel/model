use crate::{Error, FieldDefinition, ModelDef};
use crate::{FieldDefinitionMap, FieldValue, Filter, Model};

use super::parser::ExprParser;

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Var(Var),
    Val(FieldValue),
    Comp(Box<Expr>, CompOp, Box<Expr>),
    Neg(LogicOp, Box<Expr>),
    Conj(Box<Expr>, LogicOp, Box<Expr>),
    Disj(Box<Expr>, LogicOp, Box<Expr>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Var {
    Leaf(String),
    Node((String, Box<Var>)),
}

#[derive(Clone, Debug, PartialEq)]
pub enum LogicOp {
    Not,
    And,
    Or,
}

impl Var {
    fn to_sql(&self) -> String {
        self.to_string()
    }

    pub fn resolve_definition(&self, model_def: &ModelDef) -> Result<FieldDefinition, Error> {
        match self {
            Var::Leaf(name) => {
                let FieldDefinitionMap(field_defs) = (model_def.field_definitions)().into();
                let def = field_defs
                    .get(name)
                    .ok_or_else(|| Error::bad_request("undefined field"))?;

                Ok(def.clone())
            }
            Var::Node((name, var)) => {
                let relation_defs = (model_def.relation_definitions)();
                let relation_def = relation_defs
                    .iter()
                    .find(|def| &def.name == name)
                    .ok_or_else(|| Error::bad_request("undefined field"))?;

                let model_def = &relation_def.model_definition;

                var.resolve_definition(model_def)
            }
        }
    }
}

impl ToString for Var {
    fn to_string(&self) -> String {
        match self {
            Self::Leaf(val) => val.into(),
            Self::Node((name, var)) => {
                if let Self::Leaf(_) = var.as_ref() {
                    format!("{}.{}", name, var.to_sql())
                } else {
                    format!("{}_{}", name, var.to_sql())
                }
            }
        }
    }
}

impl From<&str> for Var {
    fn from(value: &str) -> Self {
        Self::Leaf(value.into())
    }
}

impl LogicOp {
    fn to_sql(&self) -> String {
        match self {
            Self::Not => "NOT",
            Self::And => "AND",
            Self::Or => "OR",
        }
        .into()
    }
}

impl ToString for LogicOp {
    fn to_string(&self) -> String {
        match self {
            Self::Not => "!",
            Self::And => "&&",
            Self::Or => "||",
        }
        .into()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CompOp {
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    Like,
    Ilike,
}

impl CompOp {
    fn to_sql(&self) -> String {
        match self {
            Self::Eq => "=",
            Self::Neq => "<>",
            Self::Gt => ">",
            Self::Gte => ">=",
            Self::Lt => "<",
            Self::Lte => "<=",
            Self::Like => "LIKE",
            Self::Ilike => "ILIKE",
        }
        .into()
    }
}

impl ToString for CompOp {
    fn to_string(&self) -> String {
        match self {
            Self::Eq => "=",
            Self::Neq => "!=",
            Self::Gt => ">",
            Self::Gte => ">=",
            Self::Lt => "<",
            Self::Lte => "<=",
            Self::Like => "LIKE",
            Self::Ilike => "ILIKE",
        }
        .into()
    }
}

impl Expr {
    pub fn from_str<T: Model>(input: &str) -> Result<Self, Error> {
        let model_def = T::definition();

        let boxed = ExprParser::new()
            .parse(&model_def, input)
            .map_err(|err| Error::bad_request(&format!("invalid filter: {:?}", err)))?;

        Ok(*boxed)
    }

    pub fn to_sql(&self, var_binding_idx_offset: usize) -> (String, Vec<Var>, Vec<FieldValue>) {
        match self {
            Expr::Var(var) => (var.to_sql(), vec![var.clone()], vec![]),
            Expr::Val(val) => {
                let sql = format!("${}", var_binding_idx_offset + 1);
                (sql, vec![], vec![val.clone()])
            }
            Expr::Comp(a_expr, op, b_expr) => {
                let (a_sql, mut a_vars, mut a_bindings) = a_expr.to_sql(var_binding_idx_offset);
                let (b_sql, b_vars, b_bindings) =
                    b_expr.to_sql(var_binding_idx_offset + a_bindings.len());
                let op_sql = op.to_sql();

                let sql = format!("{} {} {}", a_sql, op_sql, b_sql);
                a_vars.extend(b_vars);
                a_bindings.extend(b_bindings);

                (sql, a_vars, a_bindings)
            }
            Expr::Neg(op, expr) => {
                let (expr_sql, vars, expr_bindings) = expr.to_sql(var_binding_idx_offset);
                let op_sql = op.to_sql();

                let sql = format!("({} ({}))", op_sql, expr_sql);

                (sql, vars, expr_bindings)
            }
            Expr::Conj(a_expr, op, b_expr) => {
                let (a_sql, mut a_vars, mut a_bindings) = a_expr.to_sql(var_binding_idx_offset);
                let (b_sql, b_vars, b_bindings) =
                    b_expr.to_sql(var_binding_idx_offset + a_bindings.len());
                let op_sql = op.to_sql();

                let sql = format!("({} {} {})", a_sql, op_sql, b_sql);
                a_vars.extend(b_vars);
                a_bindings.extend(b_bindings);

                (sql, a_vars, a_bindings)
            }
            Expr::Disj(a_expr, op, b_expr) => {
                let (a_sql, mut a_vars, mut a_bindings) = a_expr.to_sql(var_binding_idx_offset);
                let (b_sql, b_vars, b_bindings) =
                    b_expr.to_sql(var_binding_idx_offset + a_bindings.len());
                let op_sql = op.to_sql();

                let sql = format!("({} {} {})", a_sql, op_sql, b_sql);
                a_vars.extend(b_vars);
                a_bindings.extend(b_bindings);

                (sql, a_vars, a_bindings)
            }
        }
    }
}

impl TryInto<Filter> for Expr {
    type Error = Error;

    fn try_into(self) -> Result<Filter, Error> {
        let filter = match self {
            Expr::Comp(var, op, val) => {
                match *var {
                    Expr::Var(var) => match *val {
                        Expr::Val(val) => match op {
                            CompOp::Eq => Filter::new().field(&var.to_string()).eq(val),
                            CompOp::Neq => Filter::new().field(&var.to_string()).neq(val),
                            CompOp::Gt => Filter::new().field(&var.to_string()).gt(val),
                            CompOp::Gte => Filter::new().field(&var.to_string()).gte(val),
                            CompOp::Lt => Filter::new().field(&var.to_string()).lt(val),
                            CompOp::Lte => Filter::new().field(&var.to_string()).lte(val),
                            CompOp::Like => Filter::new().field(&var.to_string()).like(val),
                            CompOp::Ilike => Filter::new().field(&var.to_string()).ilike(val),
                        },
                        _ => return Err(Error::internal("invalid filter expression: this should not happen as any errors should have been caught during parsing of the expression")) 
                    },
                    _ => return Err(Error::internal("invalid filter expression: this should not happen as any errors should have been caught during parsing of the expression"))
                }
            },
            Expr::Neg(_, exp) => {
                Filter::new().not().group((*exp).try_into()?)
            },
            Expr::Conj(left, _, right) => {
                Filter::new().group((*left).try_into()?).and().group((*right).try_into()?)
            }
            Expr::Disj(left, _, right) => {
                Filter::new().group((*left).try_into()?).or().group((*right).try_into()?)
            }
            _ => return Err(Error::internal("invalid filter expression: this should not happen as any errors should have been caught during parsing of the expression"))
        };

        Ok(filter)
    }
}
