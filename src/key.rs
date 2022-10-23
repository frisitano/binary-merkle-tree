#[derive(Eq, PartialEq)]
pub struct Key<const N: usize>([u8; N]);

const BYTE_SIZE: u8 = 8;

impl<const N: usize> Key<N> {
    pub fn new(key: [u8; N]) -> Key<N> {
        Key(key)
    }

    pub const fn zero() -> Self {
        Self ([0u8; N])
    }

    pub fn is_zero(&self) -> bool {
        self == &Key::zero()
    }

    pub fn get_bit(&self, i: &u8) -> bool {
        let byte_pos = i / BYTE_SIZE;
        let bit_pos = i % BYTE_SIZE;
        let bit = self.0[byte_pos as usize] >> (7 - bit_pos) & 1;
        bit != 0
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0[..]
    }

    pub fn iter(&self) -> KeyIter<'_, N> {
        KeyIter { key: self, element: 0 }
    }
}

pub struct KeyIter<'a, const N: usize>{
    key: &'a Key<N>,
    element: u8
}

impl<'a, const N: usize> Iterator for KeyIter<'a, N> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        if self.element >= N as u8 * 8 {
            return None
        }

        let result = self.key.get_bit(&self.element);
        self.element += 1;

        Some(result)
    }
}
