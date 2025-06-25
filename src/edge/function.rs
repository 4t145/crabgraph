use std::{collections::HashSet, marker::PhantomData};

use crate::{
    edge::{Edge, IntoEdge},
    node::{NodeKey, TryIntoNodeKeySet},
    request::{FromRequest, Request},
};

pub struct EdgeFunction<F>(pub F);

impl<S, F> Edge<S> for EdgeFunction<F>
where
    F: Fn(&Request<S>) -> Result<HashSet<crate::node::NodeKey>, crate::Error>
        + Send
        + Sync
        + 'static,
{
    fn next_nodes(&self, request: &Request<S>) -> Result<HashSet<NodeKey>, crate::Error> {
        (self.0)(request)
    }
}

pub struct FunctionAdapter<Args, Output>(PhantomData<fn(Args) -> Output>);

macro_rules! impl_for {
    ($($T: ident)*) => {
        impl_for!(@unfold [] [$($T)*]);
    };
    (@impl $($T: ident)*) => {
        impl<$( $T, )* Output, F, S> IntoEdge<S, FunctionAdapter<($($T,)*), Output>> for F
        where F: Fn($($T,)*) -> Output + Clone + Send + Sync + 'static,
        Output: TryIntoNodeKeySet + Send + 'static,
        $( $T: FromRequest<S> + Send + 'static, )*
        {
            #[allow(unused_variables, non_snake_case)]
            fn into_edge(self) -> std::sync::Arc<dyn Edge<S>> {
                std::sync::Arc::new(EdgeFunction(move |request: &Request<S>| {
                    let f = self.clone();

                        $(
                            let $T = $T::from_request(request)?;
                        )*
                        let output = f($($T,)*);
                        let result: Result<HashSet<NodeKey>, crate::Error> = output.try_into_node_key_set();
                        result
                })) as std::sync::Arc<dyn Edge<S>>
            }
        }
    };
    (@unfold [$($T: ident)*] []) => {
        impl_for!(@impl $($T)*);
    };
    (@unfold [$($T: ident)*] [$TN: ident $($TRest: ident)*]) => {
        impl_for!(@impl $($T)* );
        impl_for!(@unfold [$($T)* $TN] [$($TRest)*]);
    };
}

impl_for!(T0 T1 T2 T3 T4 T5 T6 T7 T8 T9 T10 T11 T12 T13 T14 T15);
