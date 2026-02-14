use core::{
    fmt::{self, Formatter},
    marker::PhantomData,
};

use serde::{
    Deserialize, Deserializer,
    de::{self, DeserializeSeed, IntoDeserializer, SeqAccess},
};

use crate::{Parser, config::Config, serde::Error, source::Source, span::Span};

pub const TOKEN: &str = "$flexon::Span";

pub struct Builder<'a, 'de, S: Source, C: Config> {
    de: &'a mut Parser<'de, S, C>,
    state: State,
}

pub enum State {
    Start,
    Value,
    End,
}

impl<'a, 'de, S: Source, C: Config> Builder<'a, 'de, S, C> {
    #[inline]
    pub fn new(de: &'a mut Parser<'de, S, C>) -> Self {
        Self {
            de,
            state: State::Start,
        }
    }
}

impl<'a, 'de, S: Source, C: Config> SeqAccess<'de> for Builder<'a, 'de, S, C> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.state {
            State::Start => {
                self.state = State::Value;
                self.de.skip_whitespace();
                self.de.dec();
                seed.deserialize(self.de.idx().wrapping_add(1).into_deserializer())
            }
            State::Value => {
                self.state = State::End;
                seed.deserialize(&mut *self.de)
            }
            State::End => seed.deserialize(self.de.idx().into_deserializer()),
        }
        .map(Some)
    }
}

impl<'a, T: Deserialize<'a>> Deserialize<'a> for Span<T> {
    fn deserialize<D: Deserializer<'a>>(de: D) -> Result<Self, D::Error> {
        struct Visitor<T>(PhantomData<T>);

        impl<'a, T: Deserialize<'a>> de::Visitor<'a> for Visitor<T> {
            type Value = Span<T>;

            #[cold]
            fn expecting(&self, fmt: &mut Formatter) -> fmt::Result {
                fmt.write_str("a spanned value")
            }

            fn visit_seq<A: SeqAccess<'a>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let start = seq.next_element()?;
                let data = seq.next_element()?;
                let end = seq.next_element()?;

                match (start, data, end) {
                    (Some(start), Some(data), Some(end)) => Ok(Span::with(data, start, end)),
                    _ => Err(de::Error::custom("invalid spanned value")),
                }
            }
        }

        de.deserialize_newtype_struct(TOKEN, Visitor(PhantomData))
    }
}
