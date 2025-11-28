use acir::{AcirField, FieldElement};
use noirc_abi::{input_parser::InputValue, InputMap};
use std::collections::BTreeMap;
use std::convert::Infallible;
use thiserror::Error;

//------------------------ Input Error Definition -----------------------
#[derive(Clone, Debug, Error)]
pub enum InputError {
    #[error("Invalid Field Element representation: {reason}")]
    InvalidFieldRepresentation { reason: String },
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
        }
    }

    pub fn reason(&self) -> &str {
        match self {
            InputError::InvalidFieldRepresentation { reason } => reason.as_str(),
        }
    }
}

//------------------------ Inputs - Wrapper around InputMap -----------------------

#[derive(Debug, Default)]
pub struct Inputs {
    inputs: InputMap,
}

impl Inputs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_field(mut self, name: impl AsRef<str>, value: impl Into<FieldInput>) -> Self {
        let value = InputValue::Field(value.into().0);
        let name = String::from(name.as_ref());
        self.inputs.insert(name, value);
        self
    }

    pub fn as_input_map(&self) -> &InputMap {
        &self.inputs
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
}
