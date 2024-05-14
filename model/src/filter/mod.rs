use lalrpop_util::lalrpop_mod;

pub(crate) mod ast;
pub mod builder;
pub(crate) mod util;
lalrpop_mod!(pub(crate) parser, "/filter/grammar.rs");

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use super::{
        ast::{CompOp, Expr, LogicOp},
        parser::ExprParser,
    };
    use crate::{FieldDefinition, FieldDefinitionMap, FieldType, FieldValue};
    use chrono::NaiveDate;

    fn field_definitions() -> HashMap<String, FieldDefinition> {
        let FieldDefinitionMap(field_defs) = vec![
            FieldDefinition {
                name: "org_id".into(),
                type_: FieldType::Int,
                immutable: false,
                primary_key: false,
                unique: false,
                nullable: false,
            },
            FieldDefinition {
                name: "start_date".into(),
                type_: FieldType::Date,
                immutable: false,
                primary_key: false,
                unique: false,
                nullable: false,
            },
            FieldDefinition {
                name: "property_id".into(),
                type_: FieldType::Int,
                immutable: false,
                primary_key: false,
                unique: false,
                nullable: false,
            },
            FieldDefinition {
                name: "max_occupancy".into(),
                type_: FieldType::Int,
                immutable: false,
                primary_key: false,
                unique: false,
                nullable: false,
            },
            FieldDefinition {
                name: "name".into(),
                type_: FieldType::String,
                immutable: false,
                primary_key: false,
                unique: false,
                nullable: false,
            },
            FieldDefinition {
                name: "closed".into(),
                type_: FieldType::Bool,
                immutable: false,
                primary_key: false,
                unique: false,
                nullable: false,
            },
        ]
        .into();

        field_defs
    }

    #[test]
    fn test_explicit_precedence() {
        let field_defs = field_definitions();
        let query = r#"(org_id = "123" || start_date > "2021-01-01") && (property_id = "444" || max_occupancy >= "4")"#;

        // lhs of OR
        let org_id_var = Box::new(Expr::Var("org_id".into()));
        let org_id_val = Box::new(Expr::Val(FieldValue::Int(123.into())));
        let org_id_comp = Box::new(Expr::Comp(org_id_var, CompOp::Eq, org_id_val));

        let start_date_var = Box::new(Expr::Var("start_date".into()));
        let start_date_val = Box::new(Expr::Val(FieldValue::Date(
            NaiveDate::parse_from_str("2021-01-01", "%Y-%m-%d")
                .unwrap()
                .into(),
        )));
        let start_date_comp = Box::new(Expr::Comp(start_date_var, CompOp::Gt, start_date_val));

        let first_disj = Box::new(Expr::Disj(org_id_comp, LogicOp::Or, start_date_comp));

        // rhs of OR
        let property_id_var = Box::new(Expr::Var("property_id".into()));
        let property_id_val = Box::new(Expr::Val(FieldValue::Int(444.into())));
        let property_id_comp = Box::new(Expr::Comp(property_id_var, CompOp::Eq, property_id_val));

        let max_occupancy_var = Box::new(Expr::Var("max_occupancy".into()));
        let max_occupancy_val = Box::new(Expr::Val(FieldValue::Int(4.into())));
        let max_occupancy_comp = Box::new(Expr::Comp(
            max_occupancy_var,
            CompOp::Gte,
            max_occupancy_val,
        ));
        let second_disj = Box::new(Expr::Disj(
            property_id_comp,
            LogicOp::Or,
            max_occupancy_comp,
        ));

        let expected = Box::new(Expr::Conj(first_disj, LogicOp::And, second_disj));
        let generated = ExprParser::new().parse(&field_defs, query).unwrap();

        println!("Expected:");
        println!("{:?}\n", expected);

        println!("Generated:");
        println!("{:?}\n", generated);

        assert_eq!(generated, expected);
    }

    #[test]
    fn test_implicit_precedence() {
        let field_defs = field_definitions();
        let query = r#"org_id = "123" || start_date > "2021-01-01" && property_id = "444" || max_occupancy >= "4""#;

        // lhs of OR
        let org_id_var = Box::new(Expr::Var("org_id".into()));
        let org_id_val = Box::new(Expr::Val(FieldValue::Int(123.into())));
        let org_id_comp = Box::new(Expr::Comp(org_id_var, CompOp::Eq, org_id_val));

        let start_date_var = Box::new(Expr::Var("start_date".into()));
        let start_date_val = Box::new(Expr::Val(FieldValue::Date(
            NaiveDate::parse_from_str("2021-01-01", "%Y-%m-%d")
                .unwrap()
                .into(),
        )));
        let start_date_comp = Box::new(Expr::Comp(start_date_var, CompOp::Gt, start_date_val));

        // rhs of OR
        let property_id_var = Box::new(Expr::Var("property_id".into()));
        let property_id_val = Box::new(Expr::Val(FieldValue::Int(444.into())));
        let property_id_comp = Box::new(Expr::Comp(property_id_var, CompOp::Eq, property_id_val));

        let max_occupancy_var = Box::new(Expr::Var("max_occupancy".into()));
        let max_occupancy_val = Box::new(Expr::Val(FieldValue::Int(4.into())));
        let max_occupancy_comp = Box::new(Expr::Comp(
            max_occupancy_var,
            CompOp::Gte,
            max_occupancy_val,
        ));

        let first_conj = Expr::Conj(start_date_comp, LogicOp::And, property_id_comp);
        let first_disj = Expr::Disj(org_id_comp, LogicOp::Or, first_conj.into());

        let expected = Box::new(Expr::Disj(
            first_disj.into(),
            LogicOp::Or,
            max_occupancy_comp,
        ));
        let generated = ExprParser::new().parse(&field_defs, query).unwrap();

        println!("Expected:");
        println!("{:?}\n", expected);

        println!("Generated:");
        println!("{:?}\n", generated);

        assert_eq!(generated, expected);
    }

    #[test]
    fn test_negation() {
        let field_defs = field_definitions();
        let query = r#"!org_id="678""#;

        let org_id_var = Expr::Var("org_id".into());
        let org_id_val = Expr::Val(FieldValue::Int(678.into()));

        let cond = Expr::Comp(org_id_var.into(), CompOp::Eq, org_id_val.into());

        let expected = Box::new(Expr::Neg(LogicOp::Not, cond.into()));
        let generated = ExprParser::new().parse(&field_defs, query).unwrap();

        println!("Expected:");
        println!("{:?}\n", expected);

        println!("Generated:");
        println!("{:?}\n", generated);

        assert_eq!(generated, expected);
    }

    #[test]
    fn test_string_parsing() {
        let field_defs = field_definitions();

        let query = r#"!(name != "cant\"ona")"#;

        let name_var = Expr::Var("name".into());
        let name_val = Expr::Val("cant\"ona".to_string().into());

        let expected = Box::new(Expr::Neg(
            LogicOp::Not,
            Expr::Comp(name_var.into(), CompOp::Neq, name_val.into()).into(),
        ));
        let generated = ExprParser::new().parse(&field_defs, query).unwrap();

        println!("Expected:");
        println!("{:?}\n", expected);

        println!("Generated:");
        println!("{:?}\n", generated);

        assert_eq!(generated, expected);
    }

    #[test]
    fn test_null_values() {
        let field_defs = field_definitions();
        let query = r#"(org_id = null || start_date > null) && !(property_id = null || max_occupancy >= null)"#;

        ExprParser::new().parse(&field_defs, query).unwrap();
    }

    #[test]
    fn test_boolean_values() {
        let field_defs = field_definitions();
        let query = r#"(org_id = null || start_date > null) && !(property_id = null || max_occupancy >= null) && closed = true"#;

        ExprParser::new().parse(&field_defs, query).unwrap();
    }

    #[test]
    fn test_sql_generation() {
        let field_defs = field_definitions();
        let query = r#"(org_id = "123" || start_date > "2021-01-01") && !(property_id = "444" || max_occupancy >= "4")"#;

        let expr = ExprParser::new().parse(&field_defs, query).unwrap();
        let (sql, bindings) = expr.to_sql(0);

        println!("{:?}", sql);
        println!("{:?}", bindings);
    }
}
