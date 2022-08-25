use http::header;
use crate::metrics::GooseRequestMetric;
use crate::prelude::{TransactionError, TransactionResult};

pub trait Goose {
    fn send_request_metric_to_parent(&self, request_metric: GooseRequestMetric) -> TransactionResult;

    fn set_failure(&self, tag: &str, request: &mut GooseRequestMetric, headers: Option<&header::HeaderMap>, body: Option<&str>) -> TransactionResult;

    fn set_success(&self, request: &mut GooseRequestMetric) -> TransactionResult;

    fn log_debug(
        &self,
        tag: &str,
        request: Option<&GooseRequestMetric>,
        headers: Option<&header::HeaderMap>,
        body: Option<&str>,
    ) -> TransactionResult;
}