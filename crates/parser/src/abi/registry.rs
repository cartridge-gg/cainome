use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    tokens::{
        constants, genericity, utils::normalize_type_path, ArrayContainer, Constructor,
        NonZeroContainer, OptionContainer, ResultContainer, Token, TupleContainer, TypePath,
    },
    CainomeResult, Error,
};

#[derive(Debug, Clone)]
pub struct TypeRegistry {
    store: HashMap<String, Rc<RefCell<Token>>>,
    // TODO: add base generic types here?
}

pub(super) fn get_generic_inner_types(type_path: &str) -> CainomeResult<Vec<String>> {
    let type_path = &normalize_type_path(type_path);

    if ArrayContainer::test_path(type_path) {
        let inner_type_path = ArrayContainer::get_inner(type_path)?;
        return get_generic_inner_types(&inner_type_path);
    }

    if NonZeroContainer::test_path(type_path) {
        let inner_type_path = NonZeroContainer::get_inner(type_path)?;
        return get_generic_inner_types(&inner_type_path);
    }

    if OptionContainer::test_path(type_path) {
        let inner_type_path = OptionContainer::get_inner(type_path)?;
        return get_generic_inner_types(&inner_type_path);
    }

    if ResultContainer::test_path(type_path) {
        let inner_type_path = ResultContainer::get_inner(type_path)?;

        let mut inners = vec![];
        inners.append(&mut get_generic_inner_types(&inner_type_path.inner)?);
        inners.append(&mut get_generic_inner_types(&inner_type_path.error)?);

        return Ok(inners);
    }

    if TupleContainer::test_path(type_path) {
        let inner_type_paths = TupleContainer::get_inner(type_path)?;

        let mut inners = vec![];

        for inner_type_path in inner_type_paths.iter() {
            let mut tuple_elements = get_generic_inner_types(inner_type_path)?;
            inners.append(&mut tuple_elements);
        }

        return Ok(inners);
    }

    let mut paths = genericity::extract_generics_args(type_path)?
        .into_iter()
        .map(|it| it.1)
        .collect::<Vec<_>>();

    paths.push(genericity::type_path_no_generic(type_path));

    Ok(paths)
}

/// This function returns all inner generic types even nested ones inside other generics.
pub(super) fn get_all_generic_inner_types(type_path: &str) -> CainomeResult<Vec<String>> {
    let mut paths: Vec<String> = vec![type_path.to_string()];
    let mut seen_paths_count = 0;
    let mut new_seen_paths_count = 1;

    while new_seen_paths_count > seen_paths_count {
        seen_paths_count = new_seen_paths_count;
        let nested_types = paths
            .into_iter()
            .map(|p| get_generic_inner_types(&p))
            .collect::<Result<Vec<_>, _>>()?;

        paths = nested_types.concat();
        new_seen_paths_count = paths.len();
    }

    let normalised_paths = paths
        .iter()
        .map(|p| normalize_type_path(p))
        .collect::<Vec<_>>();

    Ok(normalised_paths)
}

fn wrap_generic_containers(
    type_path: &str,
    registry: &TypeRegistry,
) -> Result<Rc<RefCell<Token>>, Error> {
    // TODO(baitcode): It's a crotch. Need to use syn::Type everywhere.
    let type_path = type_path.replace(" ", "");

    if ArrayContainer::test_path(&type_path) {
        let inner_type_path = ArrayContainer::get_inner(&type_path)?;
        let inner_type = wrap_generic_containers(&inner_type_path, registry)?;
        let token = Token::Array(ArrayContainer::new(&type_path, &inner_type));
        return Ok(Rc::new(RefCell::new(token)));
    }

    if NonZeroContainer::test_path(&type_path) {
        let inner_type_path = NonZeroContainer::get_inner(&type_path)?;
        let inner_type = wrap_generic_containers(&inner_type_path, registry)?;
        let token = Token::NonZero(NonZeroContainer::new(&type_path, &inner_type));
        return Ok(Rc::new(RefCell::new(token)));
    }

    if OptionContainer::test_path(&type_path) {
        let inner_type_path = OptionContainer::get_inner(&type_path)?;

        let inner_type = wrap_generic_containers(&inner_type_path, registry)?;

        let token = Token::Option(OptionContainer::new(&type_path, &inner_type));
        return Ok(Rc::new(RefCell::new(token)));
    }

    if ResultContainer::test_path(&type_path) {
        let inner_type_path = ResultContainer::get_inner(&type_path)?;

        let inner_type = wrap_generic_containers(&inner_type_path.inner, registry)?;
        let error_type = wrap_generic_containers(&inner_type_path.error, registry)?;

        let token = Token::Result(ResultContainer::new(&type_path, &inner_type, &error_type));

        return Ok(Rc::new(RefCell::new(token)));
    }

    if TupleContainer::test_path(&type_path) {
        let inner_type_paths = TupleContainer::get_inner(&type_path)?;

        let mut inners = vec![];

        for inner_type_path in inner_type_paths.iter() {
            inners.push(wrap_generic_containers(inner_type_path, registry)?);
        }

        let token = Token::Tuple(TupleContainer::new(&type_path, inners));

        return Ok(Rc::new(RefCell::new(token)));
    }

    let type_path = normalize_type_path(&type_path);
    tracing::debug!("Looking for type in registry. Normalised: {}", &type_path);
    if let Some(token) = registry.store.get(&type_path) {
        return Ok(Rc::clone(token));
    }

    let type_path = genericity::type_path_no_generic(&type_path);
    tracing::debug!("Looking for type in registry. Non generic: {}", &type_path);
    if let Some(token) = registry.store.get(&type_path) {
        return Ok(Rc::clone(token));
    }

    Err(Error::ParsingFailed(format!(
        "Could not match '{type_path}' against generic container or existent types",
    )))
}

impl Default for TypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            store: HashMap::new(),
        };

        // Register basic types by default
        registry.set("()", Token::Basic(TypePath::new("()")));
        for val in constants::CAIRO_CORE_BASIC {
            registry.set(val, Token::Basic(TypePath::new(val)));
        }

        registry
    }

    pub fn is_known_type(&self, path: &str) -> Result<bool, Error> {
        let path = normalize_type_path(path);

        let inner_paths = get_all_generic_inner_types(&path)?;
        tracing::trace!("Checking inner paths: {:?}", inner_paths);
        for path in inner_paths.into_iter() {
            let path = normalize_type_path(&path);
            tracing::trace!("Checking inner type: {}", path);
            if !self.store.contains_key(&path) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    pub fn get(&self, path: &str) -> Result<Rc<RefCell<Token>>, Error> {
        let generic_token_chain = wrap_generic_containers(path, self)?;
        Ok(generic_token_chain)
    }

    pub fn set(&mut self, path: &str, token: Token) -> Rc<RefCell<Token>> {
        let type_path = normalize_type_path(path);

        if let Some(cell) = self.store.get(&type_path) {
            if token == Token::Placeholder {
                // Do not overwrite with placeholder.
                return Rc::clone(cell);
            }

            tracing::debug!("Replacing type in registry: {}", &type_path);

            let mut stored_value = cell.borrow_mut();

            match &*stored_value {
                Token::Skip(_) | Token::Substitute(_) => (),
                Token::Placeholder => {
                    // Replace placeholder with real token
                    *stored_value = token;
                }
                _ => *stored_value = token,
            };

            Rc::clone(cell)
        } else {
            tracing::debug!("Saving type in registry: {}", &type_path);
            let reference = Rc::new(RefCell::new(token));
            let to_return = Rc::clone(&reference);
            self.store.insert(type_path, reference);
            to_return
        }
    }

    pub fn remove(&mut self, path: &str) -> Option<Rc<RefCell<Token>>> {
        let type_path = normalize_type_path(path);
        self.store.remove(&type_path)
    }

    pub fn values(self) -> Vec<Rc<RefCell<Token>>> {
        self.store.into_values().collect()
    }

    pub fn get_unresolvable_generics(&self) -> Vec<Rc<RefCell<Token>>> {
        vec![]
    }

    pub fn get_uninitialised_placeholders(&self) -> Vec<String> {
        let mut unresolved_placeholders = vec![];

        for (path, val) in self.store.iter() {
            if Token::Placeholder == *val.borrow() {
                unresolved_placeholders.push(path.clone());
            }
        }

        unresolved_placeholders
    }

    pub fn get_structs(&self) -> impl Iterator<Item = Rc<RefCell<Token>>> + '_ {
        self.store
            .values()
            .filter(|token| matches!(&*token.borrow(), Token::Struct(_)))
            .cloned()
    }

    pub fn get_enums(&self) -> impl Iterator<Item = Rc<RefCell<Token>>> + '_ {
        self.store
            .values()
            .filter(|token| matches!(&*token.borrow(), Token::Enum(_)))
            .cloned()
    }

    pub fn get_events(&self) -> impl Iterator<Item = Rc<RefCell<Token>>> + '_ {
        self.store
            .values()
            .filter(|token| matches!(&*token.borrow(), Token::Event(_)))
            .cloned()
    }

    ///
    /// Returns all registered functions from Registry
    ///
    /// NOTE: This includes functions inside interfaces as well.
    pub fn get_functions(&self) -> impl Iterator<Item = Rc<RefCell<Token>>> + '_ {
        let plain_functions = self
            .store
            .values()
            .filter(|token| matches!(&*token.borrow(), Token::Function(_)))
            .cloned();

        // TODO: move out
        let interfaces = self
            .store
            .values()
            .filter(|token| matches!(&*token.borrow(), Token::Interface(_)));

        let interface_functions = interfaces.flat_map(|token| {
            let Token::Interface(interface) = &*token.borrow() else {
                unreachable!()
            };
            interface.functions.clone()
        });

        interface_functions.chain(plain_functions)
    }

    pub fn get_constructor(&self) -> Option<Constructor> {
        for token in self.store.values() {
            if let Token::Constructor(c) = &*token.borrow() {
                return Some(c.clone());
            }
        }

        None
    }

    pub fn apply_substitutions<K>(&mut self, substitutions: &HashMap<K, String>)
    where
        K: AsRef<str>,
    {
        for (target, external_path) in substitutions.iter() {
            if let Some(val) = self.store.get(target.as_ref()) {
                let mut mul = val.borrow_mut();
                *mul = Token::Substitute(TypePath::new(external_path));
            } else {
                self.set(
                    target.as_ref(),
                    Token::Substitute(TypePath::new(external_path)),
                );
            }
        }
    }

    pub fn get_keys(&self) -> Vec<String> {
        self.store.keys().cloned().collect()
    }
}
