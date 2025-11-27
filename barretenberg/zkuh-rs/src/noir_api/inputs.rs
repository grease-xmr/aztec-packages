use acir::FieldElement;
use noirc_abi::{input_parser::InputValue, InputMap};

#[derive(Debug, Default)]
pub struct Inputs {
    inputs: InputMap,
}

impl Inputs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_field(mut self, name: impl AsRef<str>, value: impl Into<FieldElement>) -> Self {
        let value = InputValue::Field(value.into());
        let name = String::from(name.as_ref());
        self.inputs.insert(name, value);
        self
    }

    pub fn as_input_map(&self) -> &InputMap {
        &self.inputs
    }
}
