use std::{marker::PhantomData, pin::Pin};

use futures::future::BoxFuture;

use crate::{
    node::{IntoNode, Node},
    request::{FromRequest, Request},
};

pub struct NodeFunction<F>(pub F);

impl<F, S> Node<S> for NodeFunction<F>
where
    F: Fn(Request<S>) -> BoxFuture<'static, Result<(), crate::Error>> + Send + Sync + 'static,
{
    fn call(
        self: std::sync::Arc<Self>,
        request: Request<S>,
    ) -> BoxFuture<'static, Result<(), crate::Error>> {
        (self.0)(request)
    }
}

pub struct AsyncFunctionAdapter<Args, Fut, Output>(
    PhantomData<fn(Args) -> Pin<Box<Fut>>>,
    pub fn() -> Output,
);

macro_rules! impl_for {
    ($($T: ident)*) => {
        impl_for!(@unfold [] [$($T)*]);
    };
    (@impl $($T: ident)*) => {
        impl<$( $T, )* Fut, F, S> IntoNode<S, AsyncFunctionAdapter<($($T,)*), Fut, Result<(), crate::Error>>> for F
        where F: Fn($($T,)*) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = Result<(), crate::Error>> + Send + 'static,
        S: Send + Sync + Clone + 'static,
        $( $T: FromRequest<S> + Send + 'static, )*
        {
            #[allow(unused_variables, non_snake_case)]
            fn into_node(self) -> std::sync::Arc<dyn Node<S>> {
                std::sync::Arc::new(NodeFunction(move |request: Request<S>| {
                    let f = self.clone();
                    Box::pin(async move {
                        $(
                            let $T = $T::from_request(&request)?;
                        )*
                        let fut = f($($T,)*);
                        fut.await?;
                        Ok(())
                    }) as BoxFuture<'static, Result<(), crate::Error>>
                })) as std::sync::Arc<dyn Node<S>>
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
