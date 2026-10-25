#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    Ref(Option<usize>),
    Null,
}

impl Default for Value {
    fn default() -> Self {
        Value::Int(0)
    }
}

impl Value {
    pub fn as_int(&self) -> i32 {
        match self {
            Value::Int(v) => *v,
            Value::Long(v) => *v as i32,
            Value::Float(v) => *v as i32,
            Value::Double(v) => *v as i32,
            Value::Ref(Some(idx)) => *idx as i32,
            Value::Ref(None) | Value::Null => 0,
        }
    }

    pub fn as_long(&self) -> i64 {
        match self {
            Value::Int(v) => *v as i64,
            Value::Long(v) => *v,
            Value::Float(v) => *v as i64,
            Value::Double(v) => *v as i64,
            Value::Ref(Some(idx)) => *idx as i64,
            Value::Ref(None) | Value::Null => 0,
        }
    }

    pub fn as_bool(&self) -> bool {
        self.as_int() != 0
    }
}
