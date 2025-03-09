pub trait Indexed {
    fn index_definitions() -> Vec<IndexDef> {
        vec![]
    }
}

pub struct IndexDef {
    pub name: String,
    pub columns: Vec<String>,
}
