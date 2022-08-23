use std::sync::Arc;
use std::time::{Duration, Instant};
use reqwest::Client;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{Decoder, Encoder, Framed};
use url::Url;
use crate::goose::{GOOSE_REQUEST_TIMEOUT, GooseRequestCadence, GooseUserData};
use crate::{GooseConfiguration, GooseError, GooseMetric, GooseUser};
use crate::logger::GooseLog;

pub struct CodecGooseUser<S, Item, U>
    where S: AsyncRead + AsyncWrite + Sized, U: Decoder<Item=Item> + Encoder<Item> {
    /// The Instant when this `GooseUser` client started.
    pub started: Instant,
    /// How many iterations of the scenario this GooseUser has run.
    pub(crate) iterations: usize,
    /// An index into the internal [`GooseAttack`](../struct.GooseAttack.html)`.scenarios`
    /// vector, indicating which [`Scenario`](./struct.Scenario.html) is running.
    pub scenarios_index: usize,
    /// Framed used to make requests, managing sessions and cookies.
    pub framed: Arc<Framed<S, U>>,
    /// timeout
    pub timeout: u64,
    /// The base URL to prepend to all relative paths.
    pub base_url: Url,
    /// A local copy of the global [`GooseConfiguration`](../struct.GooseConfiguration.html).
    pub config: GooseConfiguration,
    /// Channel to logger.
    pub logger: Option<flume::Sender<Option<GooseLog>>>,
    /// Channel to throttle.
    pub throttle: Option<flume::Sender<bool>>,
    /// Normal transactions are optionally throttled,
    /// [`test_start`](../struct.GooseAttack.html#method.test_start) and
    /// [`test_stop`](../struct.GooseAttack.html#method.test_stop) transactions are not.
    pub is_throttled: bool,
    /// Channel for sending metrics to the parent for aggregation.
    pub metrics_channel: Option<flume::Sender<GooseMetric>>,
    /// Channel for notifying the parent when thread shuts down.
    pub shutdown_channel: Option<flume::Sender<usize>>,
    /// An index into the internal [`GooseAttack`](../struct.GooseAttack.html)`.weighted_users`
    /// vector, indicating which weighted `GooseUser` is running.
    pub weighted_users_index: usize,
    /// Load test hash.
    pub load_test_hash: u64,
    /// Tracks the cadence that this user is looping through all Transactions, used by Coordinated
    /// Omission Mitigation.
    request_cadence: GooseRequestCadence,
    /// Tracks how much time is spent sleeping during a loop through all transactions.
    pub(crate) slept: u64,
    /// Current transaction name.
    pub(crate) transaction_name: Option<String>,
    /// Optional per-user session data of a generic type implementing the
    /// [`GooseUserData`] trait.
    session_data: Option<Box<dyn GooseUserData>>,
}

impl<S, Item, U> CodecGooseUser<S, Item, U>
    where S: AsyncRead + AsyncWrite + Sized, U: Decoder<Item=Item> + Encoder<Item>
{
    /// Create a new codec user state.
    pub fn new(
        scenarios_index: usize,
        base_url: Url,
        configuration: &GooseConfiguration,
        load_test_hash: u64,
        inner: S,
        codec: U,
    ) -> Result<Self, GooseError> {
        trace!("new GooseUser");

        // Either use manually configured timeout, or default.
        let timeout = if configuration.timeout.is_some() {
            match crate::util::get_float_from_string(configuration.timeout.clone()) {
                Some(f) => f as u64 * 1_000,
                None => GOOSE_REQUEST_TIMEOUT,
            }
        } else {
            GOOSE_REQUEST_TIMEOUT
        };

        let framed = Arc::new(Framed::new(inner, codec));

        Ok(CodecGooseUser {
            started: Instant::now(),
            iterations: 0,
            scenarios_index,
            framed,
            timeout,
            base_url,
            config: configuration.clone(),
            logger: None,
            throttle: None,
            is_throttled: true,
            metrics_channel: None,
            shutdown_channel: None,
            // A value of max_value() indicates this user isn't fully initialized yet.
            weighted_users_index: usize::max_value(),
            load_test_hash,
            request_cadence: GooseRequestCadence::new(),
            slept: 0,
            transaction_name: None,
            session_data: None,
        })
    }
}