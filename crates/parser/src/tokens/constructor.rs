use crate::{tokens::NamedToken, CainomeResult};

#[derive(Debug, Clone, PartialEq)]
pub struct Constructor {
    pub type_path: String,
    pub inputs: Vec<NamedToken>,
}

impl Constructor {
    pub fn new(type_path: &str) -> CainomeResult<Self> {
        Ok(Self {
            type_path: type_path.to_string(),
            inputs: vec![],
        })
    }
}
