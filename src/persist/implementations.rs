use super::Serializable;

impl Serializable for u8 {
    const SIZE: usize = 1;
    fn serialize(self) -> Vec<u8> {
        vec![self]
    }

    fn deserialize(bytes: &[u8]) -> Self {
        bytes[0]
    }
}

impl Serializable for u16 {
    const SIZE: usize = 2;
    fn serialize(self) -> Vec<u8> {
        self.to_le_bytes().into()
    }

    fn deserialize(bytes: &[u8]) -> Self {
        u16::from_le_bytes([bytes[0], bytes[1]])
    }
}

impl Serializable for u32 {
    const SIZE: usize = 4;
    fn serialize(self) -> Vec<u8> {
        self.to_le_bytes().into()
    }

    fn deserialize(bytes: &[u8]) -> Self {
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    }
}