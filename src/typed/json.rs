use serde::{Serialize, de::DeserializeOwned};

use crate::{
    request::{FromRequest, Request},
    state::IntoStateModification,
};

// pub struct Json<T>(pub T);

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
