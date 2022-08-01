use super::Serializable;

impl Serializable for u8 {
    const SIZE: usize = 1;
    fn serialize(self) -> Vec<u8> {
        return vec![self];
    }

    fn deserialize(bytes: &[u8]) -> Self {
        return bytes[0];
    }
}

impl Serializable for u16 {
    const SIZE: usize = 2;
    fn serialize(self) -> Vec<u8> {
        return self.to_be_bytes().into();
    }

    fn deserialize(bytes: &[u8]) -> Self {
        u16::from_be_bytes([bytes[0], bytes[1]])
    }
}