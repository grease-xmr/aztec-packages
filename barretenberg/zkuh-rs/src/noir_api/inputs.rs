use crate::{bytes_to_uint256, Uint256};
use acir::{AcirField, FieldElement};
use noirc_abi::{input_parser::json::JsonTypes, input_parser::InputValue, Abi, AbiType, AbiVisibility, InputMap};
use std::collections::BTreeMap;
use std::convert::Infallible;
use thiserror::Error;

//------------------------ Input Error Definition -----------------------
#[derive(Clone, Debug, Error)]
pub enum InputError {
    #[error("Invalid Field Element representation: {reason}")]
    InvalidFieldRepresentation { reason: String },
    #[error("JSON parsing error: {0}")]
    JsonParseError(String),
    #[error("Expected JSON object for 'inputs' field")]
    ExpectedInputsObject,
    #[error("Parameter '{0}' not found in ABI")]
    ParameterNotInAbi(String),
}

impl InputError {
    /// Creates a new error of the same variant as self, combining the reasons from both errors.
    pub fn combine_reasons(&self, other: &Self) -> Self {
        match self {
            InputError::InvalidFieldRepresentation { reason } => {
                InputError::InvalidFieldRepresentation {
                    reason: format!("{reason} and {}", other.reason()),
                }
            }
            InputError::JsonParseError(msg) => {
                InputError::JsonParseError(format!("{msg} and {}", other.reason()))
            }
            InputError::ExpectedInputsObject => InputError::ExpectedInputsObject,
            InputError::ParameterNotInAbi(name) => InputError::ParameterNotInAbi(name.clone()),
        }
    }

    pub fn reason(&self) -> &str {
        match self {
            InputError::InvalidFieldRepresentation { reason } => reason.as_str(),
            InputError::JsonParseError(msg) => msg.as_str(),
            InputError::ExpectedInputsObject => "Expected JSON object for 'inputs' field",
            InputError::ParameterNotInAbi(name) => name.as_str(),
        }
    }
}

impl From<Infallible> for InputError {
    fn from(x: Infallible) -> Self {
        match x {}
    }
}

#[derive(Clone, Debug, Error)]
#[error("Public input {0} was not specified.")]
pub struct PublicInputError(pub String);

//------------------------ Inputs - Wrapper around InputMap -----------------------

#[derive(Debug, Default, Clone)]
pub struct Inputs {
    inputs: InputMap,
    return_value: Option<InputValue>,
}

impl Inputs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn try_add_field<T>(mut self, name: &str, value: T) -> Result<Self, (Self, InputError)>
    where
        T: TryInto<FieldInput, Error = InputError>,
    {
        match value.try_into() {
            Ok(field_input) => {
                let value: InputValue = field_input.into();
                let name = String::from(name);
                self.inputs.insert(name, value);
                Ok(self)
            }
            Err(e) => Err((self, e.into())),
        }
    }

    pub fn add_field<T: Into<FieldInput>>(mut self, name: &str, value: T) -> Self {
        let v = value.into();
        self.inputs.insert(String::from(name), v.into());
        self
    }

    pub fn add<T>(mut self, name: impl AsRef<str>, value: T) -> Result<Self, (Self, InputError)>
    where
        T: ToInputValue,
        T::Error: Into<InputError>,
    {
        match value.to_input_value() {
            Ok(v) => {
                let name = String::from(name.as_ref());
                self.inputs.insert(name, v);
                Ok(self)
            }
            Err(e) => Err((self, e.into())),
        }
    }

    pub fn add_point<T>(mut self, name: &str, x: T, y: T) -> Result<Self, (Self, InputError)>
    where
        T: TryInto<FieldInput>,
        T::Error: Into<InputError>,
    {
        match PointInput::new(x, y) {
            Ok(point) => {
                let value: InputValue = point.into();
                self.inputs.insert(String::from(name), value);
                Ok(self)
            }
            Err(e) => Err((self, e.into())),
        }
    }

    pub fn return_value<T: Into<FieldInput>>(mut self, value: T) -> Self {
        let input = value.into();
        self.return_value = Some(input.into());
        self
    }

    pub fn as_input_map(&self) -> &InputMap {
        &self.inputs
    }

    /// Parses a JSON string into `Inputs`.
    ///
    /// The JSON format must be an object with the following structure:
    /// ```json
    /// {
    ///   "inputs": { ... },
    ///   "return_value": ...  // optional, can be null or omitted
    /// }
    /// ```
    ///
    /// The `inputs` field must be an object where keys are parameter names and values
    /// can be:
    /// - Strings (interpreted as field elements - hex with "0x" prefix or decimal)
    /// - Integers (converted to field elements)
    /// - Booleans (converted to field elements: true=1, false=0)
    /// - Arrays (converted to Vec of InputValues)
    /// - Objects (converted to Struct InputValues)
    ///
    /// If an `abi` is provided, the ABI types are used for parsing, which enables
    /// proper handling of signed integers and type validation. If `abi` is `None`,
    /// types are inferred from the JSON structure.
    ///
    /// # Errors
    ///
    /// Returns `InputError` if:
    /// - The JSON is malformed
    /// - The root is not an object
    /// - The `inputs` field is missing or not an object
    /// - Any value cannot be converted to an InputValue
    /// - A parameter in the JSON is not found in the ABI (when ABI is provided)
    pub fn parse_json(json_str: &str, abi: Option<&Abi>) -> Result<Self, InputError> {
        #[derive(serde::Deserialize)]
        struct RawInputs {
            inputs: JsonTypes,
            return_value: Option<JsonTypes>,
        }

        let raw: RawInputs =
            serde_json::from_str(json_str).map_err(|e| InputError::JsonParseError(e.to_string()))?;

        // Build ABI type map if ABI is provided
        let abi_types = abi.map(|a| a.to_btree_map());

        // Parse the inputs field - must be an object/table
        let inputs = match raw.inputs {
            JsonTypes::Table(table) => {
                let mut input_map = InputMap::new();
                for (key, value) in table {
                    let abi_type = match &abi_types {
                        Some(types) => types
                            .get(&key)
                            .ok_or_else(|| InputError::ParameterNotInAbi(key.clone()))?,
                        None => &infer_abi_type(&value),
                    };
                    let input_value = InputValue::try_from_json(value, abi_type, &key)
                        .map_err(|e| InputError::JsonParseError(e.to_string()))?;
                    input_map.insert(key, input_value);
                }
                input_map
            }
            _ => return Err(InputError::ExpectedInputsObject),
        };

        // Parse optional return_value
        let return_value = match raw.return_value {
            Some(json_val) => {
                let inferred_type = infer_abi_type(&json_val);
                let abi_type = match abi {
                    Some(a) => a
                        .return_type
                        .as_ref()
                        .map(|rt| &rt.abi_type)
                        .unwrap_or(&inferred_type),
                    None => &inferred_type,
                };
                Some(
                    InputValue::try_from_json(json_val, abi_type, "return_value")
                        .map_err(|e| InputError::JsonParseError(e.to_string()))?,
                )
            }
            None => None,
        };

        Ok(Self {
            inputs,
            return_value,
        })
    }

    /// Extracts public inputs as a vector of Uint256 in the order defined by the ABI.
    ///
    /// If a public return value is specified in the ABI, it must be provided in the Inputs via [`Self::return_value`].
    ///
    /// # Errors
    ///
    /// Returns `PublicInputError` if
    /// * any required public input is missing.
    /// * a public return value is required but not provided.
    /// * a return value is provided when none is expected.
    pub fn public_inputs(&self, abi: &Abi) -> Result<Vec<Uint256>, PublicInputError> {
        let map = self.as_input_map();
        let mut public_inputs = abi
            .parameters
            .iter()
            .filter(|param| param.is_public())
            .map(|p| {
                let name = &p.name;
                let input = map
                    .get(name)
                    .ok_or_else(|| PublicInputError(name.clone()))?;
                let encoded = Self::encode_value(input.clone(), &p.typ)
                    .map_err(|e| PublicInputError(e.to_string()))?;
                Ok::<Vec<Uint256>, PublicInputError>(encoded)
            })
            .collect::<Result<Vec<_>, _>>()?;
        match (&abi.return_type, &self.return_value) {
            (Some(t), Some(val)) if t.visibility == AbiVisibility::Public => {
                let encoded = Self::encode_value(val.clone(), &t.abi_type).map_err(|e| {
                    PublicInputError(format!("Could not convert return value: {e}"))
                })?;
                public_inputs.push(encoded);
            }
            (Some(t), None) if t.visibility == AbiVisibility::Public => {
                return Err(PublicInputError(
                    "You must specify a return value".to_string(),
                ));
            }
            (None, Some(_)) => {
                return Err(PublicInputError(
                    "You must not specify a return value".to_string(),
                ))
            }
            _ => {}
        }
        let public_inputs = public_inputs.into_iter().flatten().collect();
        Ok(public_inputs)
    }

    fn encode_value(value: InputValue, abi_type: &AbiType) -> Result<Vec<Uint256>, InputError> {
        let mut encoded_value: Vec<Uint256> = Vec::new();
        match (value, abi_type) {
            (InputValue::Field(elem), _) => {
                let val =
                    fe_to_uint256(&elem).map_err(|e| InputError::InvalidFieldRepresentation {
                        reason: e.to_string(),
                    })?;
                encoded_value.push(val);
            }

            (InputValue::Vec(vec_elements), AbiType::Array { typ, .. }) => {
                for elem in vec_elements {
                    encoded_value.extend(Self::encode_value(elem, typ)?);
                }
            }

            (InputValue::String(string), _) => {
                let str_as_fields = string.bytes().map(|byte| Uint256::from(byte));
                encoded_value.extend(str_as_fields);
            }

            (InputValue::Struct(object), AbiType::Struct { fields, .. }) => {
                for (field, typ) in fields {
                    encoded_value.extend(Self::encode_value(object[field].clone(), typ)?);
                }
            }
            (InputValue::Vec(vec_elements), AbiType::Tuple { fields }) => {
                for (value, typ) in vec_elements.into_iter().zip(fields) {
                    encoded_value.extend(Self::encode_value(value, typ)?);
                }
            }
            _ => unreachable!("value should have already been checked to match abi type"),
        }
        Ok(encoded_value)
    }
}

fn fe_to_uint256(fe: &FieldElement) -> Result<Uint256, &'static str> {
    let repr = fe.to_be_bytes();
    let arr = bytes_to_uint256(&repr)?;
    if arr.len() != 1 {
        return Err("FieldElement did not convert to single Uint256");
    }
    Ok(arr[0])
}

/// Infers an `AbiType` from the structure of a `JsonTypes` value.
///
/// String types are treated as field elements. In the future, this could be extended to differentiate strings.
fn infer_abi_type(value: &JsonTypes) -> AbiType {
    match value {
        JsonTypes::String(_) | JsonTypes::Integer(_) => AbiType::Field,
        JsonTypes::Bool(_) => AbiType::Boolean,
        JsonTypes::Array(arr) => {
            let elem_type = arr.first().map(infer_abi_type).unwrap_or(AbiType::Field);
            AbiType::Array {
                length: arr.len() as u32,
                typ: Box::new(elem_type),
            }
        }
        JsonTypes::Table(table) => {
            let fields: Vec<(String, AbiType)> = table
                .iter()
                .map(|(k, v)| (k.clone(), infer_abi_type(v)))
                .collect();
            AbiType::Struct {
                path: String::new(),
                fields,
            }
        }
    }
}

//------------------------ ToInputValue - Helper trait -----------------------
pub trait ToInputValue {
    type Error;
    fn to_input_value(self) -> Result<InputValue, Self::Error>;
}

//------------------------ FieldInput - Wrapper around FieldElement -----------------------

#[derive(Clone, Copy, Debug)]
pub struct FieldInput(FieldElement);

impl FieldInput {
    pub fn from_hex(hex_str: &str) -> Result<Self, InputError> {
        if !hex_str.starts_with("0x") {
            return Err(InputError::InvalidFieldRepresentation {
                reason: "Hex string must start with '0x'".to_string(),
            });
        }
        if hex_str.len() != 64 + 2 {
            return Err(InputError::InvalidFieldRepresentation {
                reason: format!(
                    "Hex string must be 66 characters long including '0x', got {}",
                    hex_str.len()
                ),
            });
        }
        let bytes =
            hex::decode(&hex_str[2..]).map_err(|e| InputError::InvalidFieldRepresentation {
                reason: format!("Failed to decode hex string: {e}"),
            })?;

        // Audit -- is this secure? xxx_reduce applies a modulus operation, which may bias the result
        // Should we not just throw an error if the value is not a canonical field element?
        let val = FieldElement::from_be_bytes_reduce(&bytes);
        Ok(FieldInput(val))
    }

    pub fn from_decimal_str(dec_str: &str) -> Result<Self, InputError> {
        // Hack to prevent reparsing it as hex
        if dec_str.contains('x') {
            return Err(InputError::InvalidFieldRepresentation {
                reason: format!("Invalid decimal number: {dec_str}"),
            });
        }
        let val = FieldElement::try_from_str(dec_str).ok_or_else(|| {
            InputError::InvalidFieldRepresentation {
                reason: format!("Invalid decimal number: {dec_str}"),
            }
        })?;
        Ok(FieldInput(val))
    }
}

impl Into<InputValue> for FieldInput {
    fn into(self) -> InputValue {
        InputValue::Field(self.0)
    }
}

impl TryFrom<&str> for FieldInput {
    type Error = InputError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_hex(value)
            .or_else(|e| Self::from_decimal_str(value).map_err(|e2| e.combine_reasons(&e2)))
    }
}

macro_rules! fieldinput_from_int {
    ($t:ty) => {
        impl From<$t> for FieldInput {
            fn from(value: $t) -> Self {
                FieldInput(FieldElement::from(value as u64))
            }
        }
    };
}

impl<T: TryInto<FieldInput>> ToInputValue for T {
    type Error = T::Error;

    fn to_input_value(self) -> Result<InputValue, Self::Error> {
        let field = self.try_into()?;
        Ok(InputValue::Field(field.0))
    }
}

// Use the macro for common unsigned integer types
fieldinput_from_int!(u8);
fieldinput_from_int!(u16);
fieldinput_from_int!(u32);
fieldinput_from_int!(u64);
fieldinput_from_int!(usize);

impl From<[u8; 32]> for FieldInput {
    fn from(value: [u8; 32]) -> Self {
        let val = FieldElement::from_be_bytes_reduce(&value);
        FieldInput(val)
    }
}

//------------------------ PointInput - Wrapper around InputValue(Struct) -----------------------
pub struct PointInput {
    pub x: FieldElement,
    pub y: FieldElement,
}

impl PointInput {
    pub fn new<T: TryInto<FieldInput>>(x: T, y: T) -> Result<Self, T::Error> {
        Ok(PointInput {
            x: x.try_into()?.0,
            y: y.try_into()?.0,
        })
    }
}

impl Into<InputValue> for PointInput {
    fn into(self) -> InputValue {
        let values = [
            ("x".to_string(), InputValue::Field(self.x)),
            ("y".to_string(), InputValue::Field(self.y)),
        ]
        .into_iter()
        .collect::<BTreeMap<String, InputValue>>();
        InputValue::Struct(values)
    }
}

impl ToInputValue for PointInput {
    type Error = Infallible;
    fn to_input_value(self) -> Result<InputValue, Self::Error> {
        Ok(self.into())
    }
}

//------------------------ VecInput - Wrapper around InputMap for vectors -----------------------
pub struct VecInput<T> {
    pub elements: Vec<T>,
}

impl<T> VecInput<T> {
    pub fn new(data: Vec<T>) -> Self {
        VecInput { elements: data }
    }
}

impl<T: ToInputValue> ToInputValue for VecInput<T> {
    type Error = T::Error;
    fn to_input_value(self) -> Result<InputValue, Self::Error> {
        let vec = self
            .elements
            .into_iter()
            .map(|e| e.to_input_value())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(InputValue::Vec(vec))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use noirc_abi::input_parser::InputValue;

    #[test]
    fn field_inputs_from_types() {
        let field_string =
            "6766328158903275796830164114166065706728391996142987446961316929502416783667";
        let field_hex = "0x0ef59b243ee8819f82a6da86c875508d0e786c7453ef791beae4fcf0ae88c933";
        let field_u8: u8 = 210;
        let field_u16 = 1_234u16;
        let field_u32 = 1_234_567u32;
        let field_u64 = 1_234_567_890u64;
        let field_arr: [u8; 32] = [
            14, 245, 155, 36, 62, 232, 129, 159, 130, 166, 218, 134, 200, 117, 80, 141, 14, 120,
            108, 116, 83, 239, 121, 27, 234, 228, 252, 240, 174, 136, 201, 51,
        ];

        let f = FieldInput::try_from(field_string).expect("Failed to parse field string");
        assert_eq!(f.0.to_string(), field_string);
        let f = FieldInput::try_from(field_hex).expect("Failed to parse field hex");
        assert_eq!(f.0.to_string(), field_string);
        let f = FieldInput::try_from(field_u8).unwrap();
        assert_eq!(f.0.to_string(), "210");
        let f = FieldInput::from(field_u16);
        assert_eq!(f.0.to_string(), "1234");
        let f = FieldInput::from(field_u32);
        assert_eq!(f.0.to_string(), "1234567");
        let f = FieldInput::from(field_u64);
        assert_eq!(f.0.to_string(), "1234567890");
        let f = FieldInput::from(field_arr);
        assert_eq!(f.0.to_string(), field_string);
    }
    #[test]
    fn invalid_field_inputs() {
        let invalid_hex = "0xZZZ59b243ee8819f82a6da86c875508d0e786c7453ef791beae4fcf0ae88c933";
        let short_hex = "0x0ef59b243ee8819f82a6da86c875508d0e786c7453ef791beae4fcf0ae88c9"; // 64 chars instead of 66
        let invalid_decimal =
            "67663281589032757968301641141660657067283916.195024167836678901234567890"; // too large
        let hex_without_prefix = "0ef59b243ee8819f82a6da86c875508d0e786c7453ef791beae4fcf0ae88c933";

        let err = FieldInput::try_from(invalid_hex).unwrap_err();
        assert!(
            matches!(&err, InputError::InvalidFieldRepresentation { reason }
                if reason.contains("Invalid character 'Z' at position 0 and Invalid decimal number")
            ),
            "{err}"
        );

        let err = FieldInput::try_from(short_hex).unwrap_err();
        assert!(
            matches!(&err, InputError::InvalidFieldRepresentation { reason }
                if reason.contains("Hex string must be 66 characters long including '0x'")
            ),
            "{err}"
        );

        let err = FieldInput::try_from(invalid_decimal).unwrap_err();
        assert!(
            matches!(&err, InputError::InvalidFieldRepresentation { reason }
                if reason.contains("Invalid decimal number")
            ),
            "{err}"
        );

        let err = FieldInput::try_from(hex_without_prefix).unwrap_err();
        assert!(
            matches!(&err, InputError::InvalidFieldRepresentation { reason }
                if reason.contains("Invalid decimal number")
            ),
            "{err}"
        );
    }

    #[test]
    fn array_inputs() {
        let data = vec![
            "0x0ef59b243ee8819f82a6da86c875508d0e786c7453ef791beae4fcf0ae88c933",
            "6766328158903275796830164114166065706728391996142987446961316929502416783667",
            "0x2a8a23239d91f7c2ff94c2b094bb91ff6751c03b76fd69a8770186628753ad4f",
            "19241207056750953839054933711683019584791293159572660626677985726834175880527",
        ];

        let input = VecInput::new(data);

        let val = input.to_input_value().expect("Failed to parse input");
        assert!(
            matches!(val, InputValue::Vec(v) if v.len() == 4 && matches!(&v[0], InputValue::Field(_)))
        );
    }

    #[test]
    fn point_inputs() {
        let x_hex = "0x0ef59b243ee8819f82a6da86c875508d0e786c7453ef791beae4fcf0ae88c933";
        let y_hex = "0x2a8a23239d91f7c2ff94c2b094bb91ff6751c03b76fd69a8770186628753ad4f";
        let p1 = PointInput::new(x_hex, y_hex).expect("Failed to create point");

        let x_bin = "6766328158903275796830164114166065706728391996142987446961316929502416783667"; // convert hex above to decimal
        let y_bin = "19241207056750953839054933711683019584791293159572660626677985726834175880527";
        let p2 = PointInput::new(x_bin, y_bin).expect("Failed to create point 2");

        assert_eq!(p1.x, p2.x);
        assert_eq!(p1.y, p2.y);

        let val: InputValue = p1.into();
        assert!(matches!(val, InputValue::Struct(_)));
    }

    #[test]
    fn input_map() {
        let inputs = Inputs::new()
            .try_add_field(
                "a_1",
                "70143195093839929636068986763442859911856008756585124285077086015668936144",
            )
            .expect("Failed to add decimal field")
            .add_field(
                "challenge",
                [
                    210u8, 156, 128, 245, 232, 124, 124, 171, 13, 76, 166, 149, 132, 86, 239, 144,
                    111, 194, 164, 150, 102, 99, 216, 211, 170, 244, 216, 145, 101, 64, 210, 37,
                ],
            )
            .add_point(
                "T0",
                "0x0ef59b243ee8819f82a6da86c875508d0e786c7453ef791beae4fcf0ae88c933",
                "0x2a8a23239d91f7c2ff94c2b094bb91ff6751c03b76fd69a8770186628753ad4f",
            )
            .expect("Failed to create input map");
        let input_map = inputs.as_input_map();
        assert_eq!(input_map.len(), 3);
    }

    fn expect_value(map: &InputMap, var_name: &str, expected: u128) {
        match map.get(var_name) {
            Some(InputValue::Field(fe)) => {
                let val = fe.to_u128();
                assert_eq!(val, expected, "Value for '{}' does not match", var_name);
            }
            _ => panic!("Expected field input for '{}'", var_name),
        }
    }

    fn expect_return_value(inputs: &Inputs, expected: u128) {
        match &inputs.return_value {
            Some(InputValue::Field(fe)) => {
                let val = fe.to_u128();
                assert_eq!(val, expected, "Return value does not match");
            }
            _ => panic!("Expected field input for return_value"),
        }
    }

    #[test]
    fn parse_json_basic() {
        let json = r#"{
            "inputs": {
                "x": "0x01",
                "y": 42,
                "flag": true
            }
        }"#;

        let inputs = Inputs::parse_json(json, None).expect("Failed to parse JSON");
        let map = inputs.as_input_map();
        assert_eq!(map.len(), 3);
        expect_value(map, "x", 1);
        expect_value(map, "y", 42);
        expect_value(map, "flag", 1); // true = 1
    }

    #[test]
    fn parse_json_with_return_value() {
        let json = r#"{
            "inputs": {
                "x": "0x01"
            },
            "return_value": 123
        }"#;

        let inputs = Inputs::parse_json(json, None).expect("Failed to parse JSON");
        assert_eq!(inputs.as_input_map().len(), 1);
        expect_return_value(&inputs, 123);
    }

    #[test]
    fn parse_json_with_null_return_value() {
        let json = r#"{
            "inputs": {
                "x": "0x01"
            },
            "return_value": null
        }"#;

        let inputs = Inputs::parse_json(json, None).expect("Failed to parse JSON");
        assert_eq!(inputs.as_input_map().len(), 1);
        assert!(inputs.return_value.is_none());
    }

    #[test]
    fn parse_json_with_array() {
        let json = r#"{
            "inputs": {
                "arr": ["0x01", "0x02", "0x03"]
            }
        }"#;

        let inputs = Inputs::parse_json(json, None).expect("Failed to parse JSON");
        let map = inputs.as_input_map();
        assert!(matches!(map.get("arr"), Some(InputValue::Vec(v)) if v.len() == 3));
    }

    #[test]
    fn parse_json_with_struct() {
        let json = r#"{
            "inputs": {
                "point": {
                    "x": "0x01",
                    "y": "0x02"
                }
            }
        }"#;

        let inputs = Inputs::parse_json(json, None).expect("Failed to parse JSON");
        let map = inputs.as_input_map();
        assert!(matches!(map.get("point"), Some(InputValue::Struct(_))));
    }

    #[test]
    fn parse_json_invalid_inputs_not_object() {
        let json = r#"{
            "inputs": [1, 2, 3]
        }"#;

        let err = Inputs::parse_json(json, None).unwrap_err();
        assert!(matches!(err, InputError::ExpectedInputsObject));
    }

    #[test]
    fn parse_json_missing_inputs() {
        let json = r#"{
            "return_value": 42
        }"#;

        let err = Inputs::parse_json(json, None).unwrap_err();
        assert!(matches!(err, InputError::JsonParseError(_)));
    }

    #[test]
    fn parse_json_with_abi() {
        use crate::noir_api::artifacts::load_artifact;

        // hello_world has parameters: x (Field, private), y (Field, public)
        let artifact = load_artifact("test_vectors/hello_world.json").expect("Load artifact");

        let json = r#"{
            "inputs": {
                "x": 1,
                "y": 2
            }
        }"#;

        let inputs = Inputs::parse_json(json, Some(&artifact.abi)).expect("Failed to parse JSON");
        let map = inputs.as_input_map();
        assert_eq!(map.len(), 2);
        expect_value(map, "x", 1);
        expect_value(map, "y", 2);
    }

    #[test]
    fn parse_json_with_abi_unknown_param() {
        use crate::noir_api::artifacts::load_artifact;

        let artifact = load_artifact("test_vectors/hello_world.json").expect("Load artifact");

        let json = r#"{
            "inputs": {
                "x": 1,
                "unknown": 2
            }
        }"#;

        let err = Inputs::parse_json(json, Some(&artifact.abi)).unwrap_err();
        assert!(matches!(err, InputError::ParameterNotInAbi(name) if name == "unknown"));
    }

    #[test]
    fn parse_json_with_abi_return_value() {
        use crate::noir_api::artifacts::load_artifact;

        // public_outputs has:
        //   inputs: [u64; 4] (public), index: u32 (private), offset: u64 (public), factor: u64 (private)
        //   return_type: u64 (public)
        let artifact =
            load_artifact("test_vectors/public_outputs.json").expect("Load artifact");

        let json = r#"{
            "inputs": {
                "inputs": [1, 2, 3, 4],
                "index": 0,
                "offset": 100,
                "factor": 2
            },
            "return_value": 42
        }"#;

        let inputs =
            Inputs::parse_json(json, Some(&artifact.abi)).expect("Failed to parse JSON");
        let map = inputs.as_input_map();
        assert_eq!(map.len(), 4);

        // Verify array was parsed correctly
        assert!(matches!(map.get("inputs"), Some(InputValue::Vec(v)) if v.len() == 4));

        // Verify scalar inputs
        expect_value(map, "index", 0);
        expect_value(map, "offset", 100);
        expect_value(map, "factor", 2);

        // Verify return value
        expect_return_value(&inputs, 42);
    }
}
