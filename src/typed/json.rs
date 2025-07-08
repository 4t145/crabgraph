use std::marker::PhantomData;

use serde::de::DeserializeOwned;

use crate::state::View;

pub struct TypedState<T>(PhantomData<fn() -> T>);

impl<T> TypedState<T> {
    pub const fn new() -> Self {
        TypedState(PhantomData)
    }
}

impl<T> Default for TypedState<T> {
    fn default() -> Self {
        TypedState::new()
    }
}

impl<T> View<crate::JsonObject> for TypedState<T>
where
    T: DeserializeOwned,
{
    type Data = Result<T, crate::Error>;
    fn view(self, target: &crate::JsonObject) -> Self::Data {
        serde_json::from_value(serde_json::Value::Object(target.clone()))
            .map_err(crate::Error::SerdeError)
    }
}

// impl<S, T: DeserializeOwned> FromRequest<S> for Json<T> {
//     fn from_request(request: &Request<S>) -> Result<Self, crate::Error> {
//         let state = request.state.as_ref().clone().0;
//         let value = serde_json::from_value(serde_json::Value::Object(state))?;
//         Ok(Json(value))
//     }
// }

// impl<T: Serialize> IntoStateModification for Json<T> {
//     fn into_state(self) -> Result<crate::state::State, crate::Error> {
//         let value = serde_json::to_value(self.0)?;
//         let map = value.as_object().cloned().unwrap_or_default();
//         Ok(crate::state::State(map))
//     }
// }
