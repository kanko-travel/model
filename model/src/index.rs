pub trait Indexed {
    fn index_definitions() -> Vec<IndexDef> {
        vec![]
    }
}

pub enum IndexType {
    BTree,
    Fulltext,
    FulltextEnglish,
}

pub struct IndexDef {
    pub name: String,
    pub columns: Vec<String>,
    pub type_: IndexType,
}

impl IndexDef {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            columns: vec![],
            type_: IndexType::BTree,
        }
    }

    pub fn column(mut self, column: &str) -> Self {
        self.columns.push(column.to_string());
        self
    }

    pub fn fulltext(mut self) -> Self {
        self.type_ = IndexType::Fulltext;
        self
    }

    pub fn fulltext_english(mut self) -> Self {
        self.type_ = IndexType::FulltextEnglish;
        self
    }
}
