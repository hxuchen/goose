use std::hash::Hash;
use std::time::Instant;
use http::header;
use url::Url;
use crate::{GooseConfiguration, GooseError, GooseLoggerTx, GooseMetric};
use crate::logger::GooseLog;
use crate::metrics::GooseRequestMetric;
use crate::prelude::{TransactionError, TransactionResult};

pub trait Goose: 'static + Hash + Sized + Send + Sync {
    /// The function type of a goose transaction function.
    type TransactionFunction;

    fn add_slept(&mut self, duration: u64);

    fn add_iterations(&mut self, num: usize);

    fn iterations(&self) -> usize;

    fn update_request_cadence(&mut self, thread_number: usize);

    fn started(&self) -> Instant;

    fn scenarios_index(&self) -> usize;

    fn set_config(&mut self, config: GooseConfiguration);

    fn config(&self) -> &GooseConfiguration;

    fn set_shutdown_channel(&mut self, shutdown_channel: Option<flume::Sender<usize>>);

    fn shutdown_channel(&self) -> Option<flume::Sender<usize>>;

    fn set_metrics_channel(&mut self, metrics_channel: Option<flume::Sender<GooseMetric>>);

    fn metrics_channel(&self) -> Option<flume::Sender<GooseMetric>>;

    fn set_logger(&mut self, logger: GooseLoggerTx);

    fn logger(&self) -> GooseLoggerTx;

    fn set_throttle(&mut self, logger: Option<flume::Sender<bool>>);

    fn set_weighted_users_index(&mut self, total_users: usize);

    fn weighted_users_index(&self) -> usize;

    fn set_transaction_name(&mut self, transaction_name: String);

    fn take_transaction_name(&mut self) -> Option<String>;

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

    fn single(base_url: Url, configuration: &GooseConfiguration) -> Result<Self, GooseError>;
}