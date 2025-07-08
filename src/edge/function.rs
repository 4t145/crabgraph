use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    marker::PhantomData,
    sync::Arc,
};

use futures::future::{BoxFuture, ready};

use crate::{
    edge::{Edge, IntoEdge},
    node::NodeKey,
    request::{FromRequest, Request},
    utils::TryIntoSet,
};

pub struct EdgeFunction<F, R> {
    pub f: F,
    pub router: HashMap<R, NodeKey>,
}

impl<S, F, R> Edge<S> for EdgeFunction<F, R>
where
    F: Fn(&Request<S>) -> Result<HashSet<R>, crate::Error> + Send + Sync + 'static,
    R: Hash + Eq + Clone + Send + Sync + std::fmt::Debug + 'static,
    S: Send + Sync + 'static,
{
    fn next_nodes(
        &self,
        request: &Request<S>,
    ) -> BoxFuture<Result<HashSet<NodeKey>, crate::Error>> {
        let sync_result = (|| {
            let key = (self.f)(request)?;
            let mut result = HashSet::new();
            for r in key {
                if let Some(node_key) = self.router.get(&r) {
                    result.insert(node_key.clone());
                } else {
                    return Err(crate::GraphError::UndefinedRoute(format!("{r:?}")).into());
                }
            }
            Ok(result)
        })();
        Box::pin(ready(sync_result))
    }
    fn neighbours(&self) -> HashSet<NodeKey> {
        self.router.values().cloned().collect()
    }
    fn description(&self) -> String {
        format!("Function Edge to [{:?}]", self.router)
    }
}

pub struct AsyncEdgeFunction<F, R> {
    pub f: F,
    pub router: Arc<HashMap<R, NodeKey>>,
}

impl<S, F, R> Edge<S> for AsyncEdgeFunction<F, R>
where
    F: Fn(&Request<S>) -> BoxFuture<'static, Result<HashSet<R>, crate::Error>>
        + Send
        + Sync
        + 'static
        + Clone,
    R: Hash + Eq + Clone + Send + Sync + std::fmt::Debug + 'static,
    S: Send + Sync + 'static,
{
    fn next_nodes(
        &self,
        request: &Request<S>,
    ) -> BoxFuture<Result<HashSet<NodeKey>, crate::Error>> {
        let router = self.router.clone();
        let key_fut = (self.f)(request);
        Box::pin(async move {
            let key = key_fut.await?;
            let mut result = HashSet::new();
            for r in key {
                if let Some(node_key) = router.get(&r) {
                    result.insert(node_key.clone());
                } else {
                    return Err(crate::GraphError::UndefinedRoute(format!("{r:?}")).into());
                }
            }
            Ok(result)
        })
    }
    fn neighbours(&self) -> HashSet<NodeKey> {
        self.router.values().cloned().collect()
    }
    fn description(&self) -> String {
        format!("Function Edge to [{:?}]", self.router)
    }
}

pub struct FunctionAdapter<Args, Output, OutputAdapter>(
    PhantomData<(fn(Args) -> Output, fn() -> OutputAdapter)>,
);

pub struct AsyncFunctionAdapter<Args, Output, Fut, OutputAdapter>(
    PhantomData<(fn(Args) -> Output, fn() -> Fut, fn() -> OutputAdapter)>,
);

macro_rules! impl_for {
    ($($T: ident)*) => {
        impl_for!(@unfold [] [$($T)*]);
    };
    (@impl $($T: ident)*) => {
        impl<$( $T, )* Output, S, F, R, OA> IntoEdge<S, FunctionAdapter<($($T,)*), Output, OA>> for (F, HashMap<R, NodeKey>)
        where F: Fn($($T,)*) -> Output + Clone + Send + Sync + 'static,
        Output: TryIntoSet<R, OA> + Send + 'static,
        R: Hash + Eq + Clone + Send + Sync + std::fmt::Debug + 'static + Send,
        S: Send + Sync + 'static,
        $( $T: FromRequest<S> + Send + 'static, )*
        {
            #[allow(unused_variables, non_snake_case)]
            fn into_edge(self) -> std::sync::Arc<dyn Edge<S>> {
                let f = self.0;
                let router = self.1;
                std::sync::Arc::new(EdgeFunction::<_, R> {
                    f: move |request: &Request<S>| {
                        let f = f.clone();
                        $(
                        let $T = $T::from_request(request)?;
                        )*
                        let output = f($($T,)*);
                        let result: Result<HashSet<R>, crate::Error> = output.try_into_set();
                        result
                    },
                    router,
                }) as std::sync::Arc<dyn Edge<S>>
            }
        }
        impl<$( $T, )* Output, S, F, R, Fut, OA> IntoEdge<S, AsyncFunctionAdapter<($($T,)*), Output, Fut,OA>> for (F, HashMap<R, NodeKey>)
        where F: Fn($($T,)*) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = Output> + Send + 'static,
        Output: TryIntoSet<R, OA> + Send + 'static,
        R: Hash + Eq + Clone + Send + Sync + std::fmt::Debug + 'static + Send,
        S: Clone + Send + Sync + 'static,
        $( $T: FromRequest<S> + Send + 'static, )*
        {
            #[allow(unused_variables, non_snake_case)]
            fn into_edge(self) -> std::sync::Arc<dyn Edge<S>> {
                let f = self.0;
                let router = self.1;
                std::sync::Arc::new(AsyncEdgeFunction::<_, R> {
                    f: move |request: &Request<S>| {
                        let request = request.clone();
                        let extract = move || {
                            $(
                            let $T = $T::from_request(&request)?;
                            )*
                            <Result<($($T,)*), crate::Error>>::Ok(($($T,)*))
                        };
                        let f = f.clone();
                        Box::pin(async move {
                            let ($($T,)*) = extract()?;
                            let output = f($($T,)*).await;
                            let result: Result<HashSet<R>, crate::Error> = output.try_into_set();
                            result
                        }) as BoxFuture<'static, Result<HashSet<R>, crate::Error>>
                    },
                    router: Arc::new(router),
                }) as std::sync::Arc<dyn Edge<S>>
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
