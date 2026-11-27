//! Serde helpers for fixed-size arrays of any length.
//!
//! serde only implements `Serialize`/`Deserialize` for arrays of up to 32
//! elements, while the emulated memories are much larger (WRAM, VRAM, ARAM,
//! CGRAM, OAM, framebuffers...). These modules are meant for
//! `#[serde(with = "...")]`:
//!
//! - [`array`](mod@array) for `[T; N]` fields,
//! - [`boxed_array`] for `Box<[T; N]>` fields.
//!
//! Both serialize the array as a slice (a length followed by the elements) and
//! deserialize it through a `Vec`, so a large array never has to be built on
//! the stack. A length different from `N` is reported as a clear error instead
//! of a panic.
//!
//! ```
//! #[derive(serde::Serialize, serde::Deserialize)]
//! struct Memories {
//!     #[serde(with = "common::serde_arrays::array")]
//!     palette: [u16; 256],
//!     #[serde(with = "common::serde_arrays::boxed_array")]
//!     ram: Box<[u8; 0x10000]>,
//! }
//! ```

use std::fmt;

/// Describes the expected length in deserialization errors.
struct ExpectedLength(usize);

impl serde::de::Expected for ExpectedLength {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "an array of {} elements", self.0)
    }
}

/// `#[serde(with = "common::serde_arrays::array")]` for `[T; N]` fields.
pub mod array {
    use super::ExpectedLength;
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};

    /// Serializes the array as a slice.
    pub fn serialize<S, T, const N: usize>(value: &[T; N], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize,
    {
        value.as_slice().serialize(serializer)
    }

    /// Deserializes a slice of exactly `N` elements.
    pub fn deserialize<'de, D, T, const N: usize>(deserializer: D) -> Result<[T; N], D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        let elements = Vec::<T>::deserialize(deserializer)?;
        let length = elements.len();
        elements
            .try_into()
            .map_err(|_| D::Error::invalid_length(length, &ExpectedLength(N)))
    }
}

/// `#[serde(with = "common::serde_arrays::boxed_array")]` for `Box<[T; N]>` fields.
pub mod boxed_array {
    use super::ExpectedLength;
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};

    /// Serializes the array as a slice.
    ///
    /// Takes `&[T; N]`: serde passes a `&Box<[T; N]>`, which derefs to it.
    pub fn serialize<S, T, const N: usize>(value: &[T; N], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize,
    {
        value.as_slice().serialize(serializer)
    }

    /// Deserializes a slice of exactly `N` elements straight into the heap.
    pub fn deserialize<'de, D, T, const N: usize>(deserializer: D) -> Result<Box<[T; N]>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        let elements = Vec::<T>::deserialize(deserializer)?;
        let length = elements.len();
        elements
            .into_boxed_slice()
            .try_into()
            .map_err(|_| D::Error::invalid_length(length, &ExpectedLength(N)))
    }
}

#[cfg(test)]
mod tests {
    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct Arrays {
        #[serde(with = "super::array")]
        inline: [u16; 40],
        #[serde(with = "super::boxed_array")]
        boxed: Box<[u8; 0x20000]>,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    struct Small {
        #[serde(with = "super::array")]
        inline: [u16; 3],
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    struct SmallBoxed {
        #[serde(with = "super::boxed_array")]
        boxed: Box<[u16; 4]>,
    }

    #[test]
    fn round_trip() {
        let mut arrays = Arrays {
            inline: std::array::from_fn(|i| i as u16 * 1000),
            boxed: vec![0; 0x20000].into_boxed_slice().try_into().unwrap(),
        };
        arrays.boxed[0x1FFFF] = 0xAB;
        arrays.boxed[12] = 7;

        let bytes = postcard::to_stdvec(&arrays).unwrap();
        let decoded: Arrays = postcard::from_bytes(&bytes).unwrap();

        assert_eq!(decoded, arrays);
    }

    #[test]
    fn wrong_length_is_an_error() {
        let bytes = postcard::to_stdvec(&Small { inline: [1, 2, 3] }).unwrap();

        let result = postcard::from_bytes::<SmallBoxed>(&bytes);

        assert!(result.is_err(), "{result:?}");
    }

    #[test]
    fn wrong_length_message() {
        use serde::de::{IntoDeserializer, value::Error};

        let short: serde::de::value::SeqDeserializer<_, Error> = vec![1u8, 2].into_deserializer();
        let error = super::array::deserialize::<_, u8, 4>(short).unwrap_err();

        assert_eq!(
            error.to_string(),
            "invalid length 2, expected an array of 4 elements"
        );
    }
}
