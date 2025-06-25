use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};

use crate::JsonValue;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct State(pub serde_json::value::Map<String, JsonValue>);
impl State {
    pub fn merge(&mut self, other: &State) {
        for (k, v) in &other.0 {
            self.0.insert(k.clone(), v.clone());
        }
    }
}
impl Deref for State {
    type Target = serde_json::value::Map<String, JsonValue>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for State {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub trait IntoState {
    fn into_state(self) -> Result<State, crate::Error>;
}

impl<T> IntoState for Result<T, crate::Error>
where
    T: IntoState,
{
    fn into_state(self) -> Result<State, crate::Error> {
        self.and_then(|s| s.into_state())
    }
}

impl IntoState for State {
    fn into_state(self) -> Result<State, crate::Error> {
        Ok(self)
    }
}
