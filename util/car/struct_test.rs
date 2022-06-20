use crate::repeated_field::RepeatedField;
use crate::Serializable;

struct MyMessageOwned {
    name: String,
    age: u32,
    grades: Vec<u8>,
}

enum MyMessage<'a> {
    Encoded(RepeatedField<'a>),
    Decoded(MyMessageOwned),
}

impl<'a> MyMessage<'a> {
    fn to_owned(&mut self) {
        match self {
            Self::Encoded(rf) => {
                // decode into owned message
                *self = Self::Decoded(MyMessageOwned {
                    name: self.get_name().to_owned(),
                    age: self.get_age(),
                    grades: Vec::new(),
                })
            }
            _ => (),
        }
    }

    fn set_name(&mut self, name: String) {
        self.to_owned();
        match self {
            Self::Decoded(ref mut msg) => {
                msg.name = name;
            }
            _ => (),
        }
    }

    fn get_name(&self) -> &str {
        match self {
            Self::Encoded(rf) => {
                if let Some(Ok(x)) = rf.get(0) {
                    x
                } else {
                    Serializable::zero()
                }
            }
            Self::Decoded(msg) => &msg.name,
        }
    }

    fn get_age(&self) -> u32 {
        match self {
            Self::Encoded(rf) => {
                if let Some(Ok(x)) = rf.get(1) {
                    x
                } else {
                    Serializable::zero()
                }
            }
            Self::Decoded(msg) => msg.age,
        }
    }
}
