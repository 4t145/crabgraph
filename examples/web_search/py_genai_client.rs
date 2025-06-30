use std::sync::Arc;

use crabgraph::request::FromRequest;
use pyo3::{
    Bound, Py, PyAny, PyResult, Python,
    types::{PyDict, PyDictMethods},
};

use crate::App;
#[derive(Debug, Clone)]
pub struct PyGenaiClient {
    pub client: Arc<Py<PyAny>>,
}

impl FromRequest<App> for PyGenaiClient {
    fn from_request(request: &crabgraph::request::Request<App>) -> Result<Self, crabgraph::Error> {
        Ok(request.context.state.py_llm.clone())
    }
}
impl PyGenaiClient {
    pub async fn generate_content(
        &self,
        model: &str,
        contents: String,
        config: impl for<'py> FnOnce(Python<'py>) -> PyResult<Bound<'py, PyDict>>,
    ) -> PyResult<Py<PyAny>> {
        let response = pyo3::Python::with_gil(|py| {
            let models = self.client.getattr(py, "models")?;
            let args = PyDict::new(py);
            args.set_item("model", model)?;
            args.set_item("contents", contents)?;
            args.set_item("config", config(py)?)?;
            let response = models.call_method(py, "generate_content", (), Some(&args))?;
            PyResult::Ok(response)
        })?;

        Ok(response)
    }
}
