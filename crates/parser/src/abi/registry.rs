use std::{cell::RefCell, collections::HashMap, fmt::format, rc::Rc};

use crate::{
    abi::extensions::TryTokenConvertable,
    tokens::{
        constants, ArrayContainer, CoreBasic, NonZeroContainer, OptionContainer, ResultContainer,
        Token, TupleContainer,
    },
    CainomeResult, Error,
};

pub struct TypeRegistry {
    store: HashMap<String, Rc<RefCell<Token>>>,
}

// TODO: memoise maybe? set?
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

fn wrap_generic_containers(
    type_path: &str,
    registry: &TypeRegistry,
) -> Result<Rc<RefCell<Token>>, Error> {
    if ArrayContainer::test_path(&type_path) {
        let inner_type_path = ArrayContainer::get_inner(&type_path)?;
        let inner_type = wrap_generic_containers(&inner_type_path, registry)?;
        let token = ArrayContainer::new_token(&type_path, &inner_type);
        return Ok(Rc::new(RefCell::new(token)));
    }

    if NonZeroContainer::test_path(&type_path) {
        let inner_type_path = NonZeroContainer::get_inner(&type_path)?;
        let inner_type = wrap_generic_containers(&inner_type_path, registry)?;
        let token = NonZeroContainer::new_token(&type_path, &inner_type);
        return Ok(Rc::new(RefCell::new(token)));
    }

    if OptionContainer::test_path(&type_path) {
        let inner_type_path = OptionContainer::get_inner(&type_path)?;

        let inner_type = wrap_generic_containers(&inner_type_path, registry)?;

        let token = OptionContainer::new_token(&type_path, &inner_type);
        return Ok(Rc::new(RefCell::new(token)));
    }

    if ResultContainer::test_path(&type_path) {
        let inner_type_path = ResultContainer::get_inner(&type_path)?;

        let inner_type = wrap_generic_containers(&inner_type_path.inner, registry)?;
        let error_type = wrap_generic_containers(&inner_type_path.error, registry)?;

        let token = ResultContainer::new_token(&type_path, &inner_type, &error_type);

        return Ok(Rc::new(RefCell::new(token)));
    }

    // TODO: Tuple should not be here
    if TupleContainer::test_path(&type_path) {
        let inner_type_paths = TupleContainer::get_inner(&type_path)?;

        let mut inners = vec![];

        for inner_type_path in inner_type_paths.iter() {
            inners.push(wrap_generic_containers(&inner_type_path, registry)?);
        }

        let token = TupleContainer::new_token(type_path, inners);
        return Ok(Rc::new(RefCell::new(token)));
    }

    if let Some(token) = registry.store.get(type_path) {
        return Ok(Rc::clone(token));
    }

    Err(Error::ParsingFailed(format!(
        "Could not match '{}' against generic container or existent types",
        type_path,
    )))
}

impl TypeRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            store: HashMap::new(),
        };

        // Register basic types by default
        registry.set("()", Token::Basic(CoreBasic::new("()")));
        for val in constants::CAIRO_CORE_BASIC {
            registry.set(val, Token::Basic(CoreBasic::new(val)));
        }

        registry
    }

    pub fn is_known_type(&self, path: &str) -> Result<bool, Error> {
        let inner_paths = get_generic_inner_types(path)?;

        for path in inner_paths.into_iter() {
            if !self.store.contains_key(&path) {
                return Ok(false);
            }
        }

        return Ok(true);
    }

    pub fn get(&self, path: &str) -> Result<Rc<RefCell<Token>>, Error> {
        let generic_token_chain = wrap_generic_containers(path, &self)?;
        return Ok(generic_token_chain);
    }

    pub fn set(&mut self, path: &str, token: Token) {
        if let Some(cell) = self.store.get(path) {
            if token == Token::Placeholder {
                // Do not overwrite with placeholder.
                return;
            }

            let mut cell = cell.as_ref().borrow_mut();
            *cell = token;
        } else {
            let reference = Rc::new(RefCell::new(token));
            self.store.insert(path.to_string(), reference);
        }
    }

    pub fn remove(&mut self, path: &str) -> Option<Rc<RefCell<Token>>> {
        self.store.remove(path)
    }

    pub fn values(self) -> Vec<Rc<RefCell<Token>>> {
        self.store.into_values().collect()
    }

    pub fn get_uninitialised_placeholders(&self) -> Vec<String> {
        let mut unresolved_placeholders = vec![];

        for (path, val) in self.store.iter() {
            if Token::Placeholder == *val.borrow() {
                unresolved_placeholders.push(path.clone());
            }
        }

        return unresolved_placeholders;
    }
}
