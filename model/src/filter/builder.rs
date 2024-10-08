use crate::Error;
use crate::{
    filter::{ast::*, parser::ExprParser},
    FieldValue, Model,
};

#[derive(Debug, Clone)]
enum Token {
    CompOp(CompOp),
    Var(String),
    Val(FieldValue),
    Group(Filter),
    LogicOp(LogicOp),
}

#[derive(Clone, Debug)]
pub struct Filter {
    tokens: Vec<Token>,
}

impl Filter {
    pub fn new() -> Self {
        Self { tokens: vec![] }
    }

    pub fn field(mut self, name: &str) -> Self {
        self.tokens.push(Token::Var(name.into()));

        self
    }

    pub fn eq(mut self, val: impl Into<FieldValue>) -> Self {
        self.tokens.push(Token::CompOp(CompOp::Eq));
        self.tokens.push(Token::Val(val.into()));

        self
    }

    pub fn neq(mut self, val: impl Into<FieldValue>) -> Self {
        self.tokens.push(Token::CompOp(CompOp::Neq));
        self.tokens.push(Token::Val(val.into()));

        self
    }

    pub fn gt(mut self, val: impl Into<FieldValue>) -> Self {
        self.tokens.push(Token::CompOp(CompOp::Gt));
        self.tokens.push(Token::Val(val.into()));

        self
    }

    pub fn gte(mut self, val: impl Into<FieldValue>) -> Self {
        self.tokens.push(Token::CompOp(CompOp::Gte));
        self.tokens.push(Token::Val(val.into()));

        self
    }

    pub fn lt(mut self, val: impl Into<FieldValue>) -> Self {
        self.tokens.push(Token::CompOp(CompOp::Lt));
        self.tokens.push(Token::Val(val.into()));

        self
    }

    pub fn lte(mut self, val: impl Into<FieldValue>) -> Self {
        self.tokens.push(Token::CompOp(CompOp::Lte));
        self.tokens.push(Token::Val(val.into()));

        self
    }

    pub fn like(mut self, val: impl Into<FieldValue>) -> Self {
        self.tokens.push(Token::CompOp(CompOp::Like));
        self.tokens.push(Token::Val(val.into()));

        self
    }

    pub fn ilike(mut self, val: impl Into<FieldValue>) -> Self {
        self.tokens.push(Token::CompOp(CompOp::Ilike));
        self.tokens.push(Token::Val(val.into()));

        self
    }

    pub fn not(mut self) -> Self {
        self.tokens.push(Token::LogicOp(LogicOp::Not));

        self
    }

    pub fn and(mut self) -> Self {
        self.tokens.push(Token::LogicOp(LogicOp::And));

        self
    }

    pub fn or(mut self) -> Self {
        self.tokens.push(Token::LogicOp(LogicOp::Or));

        self
    }

    pub fn group(mut self, group: Filter) -> Self {
        self.tokens.push(Token::Group(group));

        self
    }

    pub fn build<T: Model>(self) -> Result<Expr, Error> {
        let model_def = T::definition();

        let input = tokens_to_string(self.tokens);

        let expr = ExprParser::new()
            .parse(&model_def, &input)
            .map_err(|err| Error::internal(&format!("{:?}", err)))?;

        Ok(*expr)
    }
}

fn tokens_to_string(tokens: Vec<Token>) -> String {
    let tokens = tokens
        .into_iter()
        .map(|tok| match tok {
            Token::Group(b) => format!("({})", tokens_to_string(b.tokens)),
            Token::Var(var) => var,
            Token::Val(val) => format!(r#""{}""#, val.to_string()),
            Token::CompOp(op) => op.to_string(),
            Token::LogicOp(op) => op.to_string(),
        })
        .collect::<Vec<String>>();

    tokens.join(" ")
}

#[cfg(test)]
mod test {
    use chrono::NaiveDate;
    use uuid::Uuid;

    use crate::{self as model, Related};

    use super::*;

    #[derive(Clone, Debug, Model)]
    #[model(table_name = "example", has_relations)]
    struct Example {
        #[model(id, primary_key)]
        id: Uuid,
        organization_id: Uuid,
        name: String,
        rate_plan_id: Uuid,
        room_type_id: Uuid,
        start_date: NaiveDate,
        end_date: NaiveDate,
    }

    #[derive(Clone, Debug, Model)]
    #[model(table_name = "organization")]
    struct Organization {
        #[model(id, primary_key)]
        id: Uuid,
        name: String,
    }

    impl Related for Example {
        fn relation_definitions() -> Vec<crate::relation::RelationDef> {
            vec![Self::belongs_to::<Organization>(
                "organization".into(),
                "organization_id".into(),
            )]
        }
    }

    #[test]
    fn test_simple_filter() {
        Filter::new()
            .field("organization_id")
            .eq(Uuid::new_v4())
            .build::<Example>()
            .unwrap();
    }

    #[test]
    fn test_invalid_field() {
        let filter = Filter::new()
            .field("non_existent_field")
            .eq(Uuid::new_v4())
            .build::<Example>();

        match filter {
            Err(Error::InternalError(_)) => (),
            _ => panic!("unexpected result"),
        }
    }

    #[test]
    fn test_nested_filter() {
        let id = Uuid::new_v4();
        let organization_id = Uuid::new_v4();

        let inner = Filter::new()
            .field("id")
            .eq(id.clone())
            .or()
            .not()
            .field("organization_id")
            .eq(organization_id.clone());

        let outer = Filter::new()
            .group(inner)
            .and()
            .field("name")
            .eq(String::from("some_name"))
            .build::<Example>()
            .unwrap();

        let expected = format!(
            r#"(id = "{}" || !(organization_id = "{}")) && name = "{}""#,
            id, organization_id, "some_name"
        );

        let expected = Expr::from_str::<Example>(&expected).unwrap();

        assert_eq!(expected, outer);
    }

    #[test]
    fn test_real_world_filter() {
        let rate_plan_id = Uuid::new_v4();
        let room_type_id = Uuid::new_v4();

        let d1 = NaiveDate::from_ymd_opt(2022, 2, 20).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2022, 2, 20).unwrap();

        let filter = Filter::new()
            .field("rate_plan_id")
            .eq(rate_plan_id)
            .and()
            .field("room_type_id")
            .eq(room_type_id)
            .and()
            .group(
                Filter::new()
                    .field("start_date")
                    .gte(d1.clone())
                    .and()
                    .field("start_date")
                    .lte(d2.clone())
                    .or()
                    .field("end_date")
                    .gte(d1.clone())
                    .and()
                    .field("end_date")
                    .lte(d2.clone())
                    .or()
                    .field("start_date")
                    .lte(d1.clone())
                    .and()
                    .field("end_date")
                    .gte(d2.clone()),
            );

        let generated = filter.build::<Example>().unwrap();

        let expected = format!(
            r#"rate_plan_id = "{}" && room_type_id = "{}" && (start_date >= "{}" && start_date <= "{}" || end_date >= "{}" && end_date <= "{}" || start_date <= "{}" && end_date >= "{}")"#,
            rate_plan_id,
            room_type_id,
            d1.clone(),
            d2.clone(),
            d1.clone(),
            d2.clone(),
            d1.clone(),
            d2.clone()
        );

        let expected = Expr::from_str::<Example>(&expected).unwrap();

        assert_eq!(expected, generated);
    }

    #[test]
    fn test_related_filter() {
        Filter::new()
            .field("organization.name")
            .ilike("acme".to_string())
            .build::<Example>()
            .unwrap();
    }
}
