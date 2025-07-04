use std::ops::{Deref, DerefMut};

use modify::SendDynModification;
use serde::{Deserialize, Serialize};

use crate::JsonValue;

#[derive(Debug, Default)]
pub struct State(pub tokio::sync::RwLock<serde_json::value::Map<String, JsonValue>>);
impl State {
    // pub fn merge(&mut self, other: &State) {
    //     for (k, v) in &other.0 {
    //         self.0.insert(k.clone(), v.clone());
    //     }
    // }
    // pub fn from_json_value(value: JsonValue) -> State {
    //     match value {
    //         JsonValue::Object(map) => State(map),
    //         _ => State::default(),
    //     }
    // }
    // pub fn from_typed<T>(value: T) -> Result<State, crate::Error>
    // where
    //     T: Serialize,
    // {
    //     let json_value = serde_json::to_value(value).map_err(crate::Error::from)?;
    //     Ok(State::from_json_value(json_value))
    // }
}
// impl Deref for State {
//     type Target = serde_json::value::Map<String, JsonValue>;

//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }

// impl DerefMut for State {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.0
//     }
// }

pub trait IntoStateModification {
    fn into_state(self) -> Result<SendDynModification<State>, crate::Error>;
}

impl<T, E> IntoStateModification for Result<T, E>
where
    T: IntoStateModification,
    crate::Error: From<E>,
{
    fn into_state(self) -> Result<SendDynModification<State>, crate::Error> {
        self.map_err(|e| e.into()).and_then(|s| s.into_state())
    }
}

// impl IntoStateModification for State {
//     fn into_state(self) -> Result<SendDynModification<State>, crate::Error> {
//         Ok(self)
//     }
// }

pub struct Annotated<T, M> {
    value: T,
    merge: M,
}

pub trait Merger<T> {
    fn merge(prev: T, input: T) -> T;
}
pub trait Merge {
    fn merge(prev: Self, input: Self) -> Self;
}

pub struct Replace;
impl<T> Merger<T> for Replace {
    fn merge(_prev: T, input: T) -> T {
        input
    }
}

impl<T, M> Merge for Annotated<T, M>
where
    M: Merger<T>,
{
    fn merge(prev: Self, input: Self) -> Self {
        Annotated {
            value: M::merge(prev.value, input.value),
            merge: prev.merge,
        }
    }
}
