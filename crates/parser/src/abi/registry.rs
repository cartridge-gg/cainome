use std::{collections::HashMap, rc::Rc};

use crate::{
    tokens::{
        constants, ArrayContainer, CoreBasic, NonZeroContainer, OptionContainer, ResultContainer,
        Token, TupleContainer,
    },
    CainomeResult, Error,
};

pub struct TypeRegistry {
    store: HashMap<String, Rc<Token>>,
}

// TODO: memoise maybe?
fn get_generic_inner_types(type_path: &str) -> CainomeResult<Vec<String>> {
    if ArrayContainer::test_path(&type_path) {
        let inner_type_path = ArrayContainer::get_inner(&type_path)?;
        return get_generic_inner_types(&inner_type_path);
    }

    if NonZeroContainer::test_path(&type_path) {
        let inner_type_path = NonZeroContainer::get_inner(&type_path)?;
        return get_generic_inner_types(&inner_type_path);
    }

    if OptionContainer::test_path(&type_path) {
        let inner_type_path = OptionContainer::get_inner(&type_path)?;
        return get_generic_inner_types(&inner_type_path);
    }

    if ResultContainer::test_path(&type_path) {
        let inner_type_path = ResultContainer::get_inner(&type_path)?;

        let mut inners = vec![];
        inners.append(&mut get_generic_inner_types(&inner_type_path.inner)?);
        inners.append(&mut get_generic_inner_types(&inner_type_path.error)?);

        return Ok(inners);
    }

    if TupleContainer::test_path(&type_path) {
        let inner_type_paths = TupleContainer::get_inner(&type_path)?;

        let mut inners = vec![];

        for inner_type_path in inner_type_paths.iter() {
            let mut tuple_elements = get_generic_inner_types(&inner_type_path)?;
            inners.append(&mut tuple_elements);
        }

        return Ok(inners);
    }

    // TODO: Struct?

    Ok(vec![type_path.to_string()])
}

fn wrap_generic_containers(type_path: &str, registry: &TypeRegistry) -> CainomeResult<Rc<Token>> {
    if ArrayContainer::test_path(&type_path) {
        let inner_type_path = ArrayContainer::get_inner(&type_path)?;

        let inner_type = wrap_generic_containers(&inner_type_path, registry)?;

        let token = Token::Array(ArrayContainer::new(&type_path, &inner_type));
        return Ok(Rc::new(token));
    }

    if NonZeroContainer::test_path(&type_path) {
        let inner_type_path = NonZeroContainer::get_inner(&type_path)?;

        let inner_type = wrap_generic_containers(&inner_type_path, registry)?;

        let token = Token::NonZero(NonZeroContainer::new(&type_path, &inner_type));
        return Ok(Rc::new(token));
    }

    if OptionContainer::test_path(&type_path) {
        let inner_type_path = OptionContainer::get_inner(&type_path)?;

        let inner_type = wrap_generic_containers(&inner_type_path, registry)?;

        let token = Token::Option(OptionContainer::new(&type_path, &inner_type));
        return Ok(Rc::new(token));
    }

    if ResultContainer::test_path(&type_path) {
        let inner_type_path = ResultContainer::get_inner(&type_path)?;

        let inner_type = wrap_generic_containers(&inner_type_path.inner, registry)?;
        let error_type = wrap_generic_containers(&inner_type_path.error, registry)?;

        let token = Token::Result(ResultContainer::new(&type_path, &inner_type, &error_type));

        return Ok(Rc::new(token));
    }

    // TODO: Tuple should not be here
    if TupleContainer::test_path(&type_path) {
        let inner_type_paths = TupleContainer::get_inner(&type_path)?;

        let mut inners = vec![];

        for inner_type_path in inner_type_paths.iter() {
            inners.push(wrap_generic_containers(&inner_type_path, registry)?);
        }

        let token = Token::Tuple(TupleContainer::new(type_path, inners));
        return Ok(Rc::new(token));
    }

    if let Some(token) = registry.store.get(type_path) {
        return Ok(Rc::clone(token));
    }

    // TODO: errors
    Err(Error::ParsingFailed("asdasd".to_string()))
}

impl TypeRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            store: HashMap::new(),
        };

        // Register basic types by default
        registry.set("()".to_string(), Token::CoreBasic(CoreBasic::new("()")));
        for val in constants::CAIRO_CORE_BASIC {
            registry.set(val.to_string(), Token::CoreBasic(CoreBasic::new(val)));
        }

        registry
    }

    pub fn is_known_type(&self, path: &str) -> CainomeResult<bool> {
        let inner_paths = get_generic_inner_types(path)?;

        let res = inner_paths.iter().all(|p| self.store.contains_key(p));

        return Ok(res);
    }

    pub fn get(&self, path: &str) -> CainomeResult<Rc<Token>> {
        let inner_path = wrap_generic_containers(path, &self)?;
        return Ok(inner_path);
    }

    pub fn set(&mut self, path: String, token: Token) -> Rc<Token> {
        let reference = Rc::new(token);
        let cloned = Rc::clone(&reference);
        self.store.insert(path, reference);
        cloned
    }

    pub fn values(self) -> Vec<Rc<Token>> {
        self.store.into_values().collect::<Vec<_>>()
    }
}

#[cfg(test)]
mod tests {}
