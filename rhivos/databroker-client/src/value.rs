//! Typed value representation for Kuksa Databroker signals.

use crate::proto::kuksa::val::v1::{self as proto, datapoint};

/// A typed value for a VSS signal.
///
/// This enum maps to the subset of Kuksa Databroker value types needed
/// by the safety-partition services:
///
/// - `Bool` for lock state and door state signals
/// - `Float` for speed (Vehicle.Speed is VSS type `float`)
/// - `Double` for latitude/longitude (VSS type `double`)
/// - `String` for command/response JSON payloads
#[derive(Debug, Clone, PartialEq)]
pub enum DataValue {
    /// Boolean value (e.g., IsLocked, IsOpen).
    Bool(bool),
    /// 32-bit float value (e.g., Vehicle.Speed).
    Float(f32),
    /// 64-bit double value (e.g., Latitude, Longitude).
    Double(f64),
    /// String value (e.g., command JSON payloads).
    String(String),
    /// 32-bit signed integer value.
    Int32(i32),
    /// 64-bit signed integer value.
    Int64(i64),
    /// 32-bit unsigned integer value.
    Uint32(u32),
    /// 64-bit unsigned integer value.
    Uint64(u64),
}

impl DataValue {
    /// Convert from a Kuksa proto `Datapoint` to a `DataValue`.
    ///
    /// Returns `None` if the datapoint has no value set.
    pub fn from_datapoint(dp: &proto::Datapoint) -> Option<Self> {
        dp.value.as_ref().map(|v| match v {
            datapoint::Value::Bool(b) => DataValue::Bool(*b),
            datapoint::Value::Float(f) => DataValue::Float(*f),
            datapoint::Value::Double(d) => DataValue::Double(*d),
            datapoint::Value::String(s) => DataValue::String(s.clone()),
            datapoint::Value::Int32(i) => DataValue::Int32(*i),
            datapoint::Value::Int64(i) => DataValue::Int64(*i),
            datapoint::Value::Uint32(u) => DataValue::Uint32(*u),
            datapoint::Value::Uint64(u) => DataValue::Uint64(*u),
            // Array types are not needed for safety-partition; map to string representation
            _ => DataValue::String(format!("{v:?}")),
        })
    }

    /// Convert this `DataValue` into a Kuksa proto `Datapoint`.
    pub fn to_datapoint(&self) -> proto::Datapoint {
        let value = match self {
            DataValue::Bool(b) => datapoint::Value::Bool(*b),
            DataValue::Float(f) => datapoint::Value::Float(*f),
            DataValue::Double(d) => datapoint::Value::Double(*d),
            DataValue::String(s) => datapoint::Value::String(s.clone()),
            DataValue::Int32(i) => datapoint::Value::Int32(*i),
            DataValue::Int64(i) => datapoint::Value::Int64(*i),
            DataValue::Uint32(u) => datapoint::Value::Uint32(*u),
            DataValue::Uint64(u) => datapoint::Value::Uint64(*u),
        };
        proto::Datapoint {
            timestamp: None,
            value: Some(value),
        }
    }

    /// Returns the value as a `bool`, if it is one.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            DataValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns the value as an `f32`, if it is one.
    pub fn as_float(&self) -> Option<f32> {
        match self {
            DataValue::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Returns the value as an `f64`, if it is one.
    pub fn as_double(&self) -> Option<f64> {
        match self {
            DataValue::Double(d) => Some(*d),
            _ => None,
        }
    }

    /// Returns the value as a `&str`, if it is one.
    pub fn as_string(&self) -> Option<&str> {
        match self {
            DataValue::String(s) => Some(s),
            _ => None,
        }
    }
}

impl std::fmt::Display for DataValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataValue::Bool(b) => write!(f, "{b}"),
            DataValue::Float(v) => write!(f, "{v}"),
            DataValue::Double(v) => write!(f, "{v}"),
            DataValue::String(s) => write!(f, "{s}"),
            DataValue::Int32(v) => write!(f, "{v}"),
            DataValue::Int64(v) => write!(f, "{v}"),
            DataValue::Uint32(v) => write!(f, "{v}"),
            DataValue::Uint64(v) => write!(f, "{v}"),
        }
    }
}

impl From<bool> for DataValue {
    fn from(b: bool) -> Self {
        DataValue::Bool(b)
    }
}

impl From<f32> for DataValue {
    fn from(f: f32) -> Self {
        DataValue::Float(f)
    }
}

impl From<f64> for DataValue {
    fn from(d: f64) -> Self {
        DataValue::Double(d)
    }
}

impl From<String> for DataValue {
    fn from(s: String) -> Self {
        DataValue::String(s)
    }
}

impl From<&str> for DataValue {
    fn from(s: &str) -> Self {
        DataValue::String(s.to_string())
    }
}

impl From<i32> for DataValue {
    fn from(i: i32) -> Self {
        DataValue::Int32(i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datavalue_bool_roundtrip() {
        let val = DataValue::Bool(true);
        let dp = val.to_datapoint();
        let back = DataValue::from_datapoint(&dp).unwrap();
        assert_eq!(val, back);
    }

    #[test]
    fn test_datavalue_float_roundtrip() {
        let val = DataValue::Float(42.5);
        let dp = val.to_datapoint();
        let back = DataValue::from_datapoint(&dp).unwrap();
        assert_eq!(val, back);
    }

    #[test]
    fn test_datavalue_double_roundtrip() {
        let val = DataValue::Double(48.1351);
        let dp = val.to_datapoint();
        let back = DataValue::from_datapoint(&dp).unwrap();
        assert_eq!(val, back);
    }

    #[test]
    fn test_datavalue_string_roundtrip() {
        let val = DataValue::String("hello".into());
        let dp = val.to_datapoint();
        let back = DataValue::from_datapoint(&dp).unwrap();
        assert_eq!(val, back);
    }

    #[test]
    fn test_datavalue_int32_roundtrip() {
        let val = DataValue::Int32(-42);
        let dp = val.to_datapoint();
        let back = DataValue::from_datapoint(&dp).unwrap();
        assert_eq!(val, back);
    }

    #[test]
    fn test_datavalue_from_empty_datapoint() {
        let dp = proto::Datapoint {
            timestamp: None,
            value: None,
        };
        assert!(DataValue::from_datapoint(&dp).is_none());
    }

    #[test]
    fn test_datavalue_as_accessors() {
        assert_eq!(DataValue::Bool(true).as_bool(), Some(true));
        assert_eq!(DataValue::Float(1.0).as_float(), Some(1.0));
        assert_eq!(DataValue::Double(2.0).as_double(), Some(2.0));
        assert_eq!(DataValue::String("x".into()).as_string(), Some("x"));

        // Wrong type returns None
        assert_eq!(DataValue::Bool(true).as_float(), None);
        assert_eq!(DataValue::Float(1.0).as_bool(), None);
        assert_eq!(DataValue::Double(2.0).as_string(), None);
        assert_eq!(DataValue::String("x".into()).as_double(), None);
    }

    #[test]
    fn test_datavalue_display() {
        assert_eq!(DataValue::Bool(true).to_string(), "true");
        assert_eq!(DataValue::Float(42.5).to_string(), "42.5");
        assert_eq!(DataValue::Double(48.1351).to_string(), "48.1351");
        assert_eq!(DataValue::String("test".into()).to_string(), "test");
        assert_eq!(DataValue::Int32(-5).to_string(), "-5");
    }

    #[test]
    fn test_datavalue_from_impls() {
        let _: DataValue = true.into();
        let _: DataValue = 42.5f32.into();
        let _: DataValue = 48.1351f64.into();
        let _: DataValue = "hello".into();
        let _: DataValue = String::from("world").into();
        let _: DataValue = 42i32.into();
    }
}
