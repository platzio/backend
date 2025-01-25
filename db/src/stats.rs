use crate::DbPool;
use prometheus::{register_int_gauge, IntGauge};
use std::time::Duration;
use tokio::time;

lazy_static::lazy_static! {
    static ref CONNECTIONS: IntGauge = register_int_gauge!(
        "platz_db_pool_connections",
        "The number of connections currently being managed by the pool"
    )
    .unwrap();
    static ref IDLE_CONNECTIONS: IntGauge = register_int_gauge!(
        "platz_db_pool_idle_connections",
        "The number of idle connections"
    )
    .unwrap();
    static ref GET_DIRECT: IntGauge = register_int_gauge!(
        "platz_db_pool_get_direct",
        "Total gets performed that did not have to wait for a connection"
    )
    .unwrap();
    static ref GET_WAITED: IntGauge = register_int_gauge!(
        "platz_db_pool_get_waited",
        "Total gets performed that had to wait for a connection available"
    )
    .unwrap();
    static ref GET_TIMED_OUT: IntGauge = register_int_gauge!(
        "platz_db_pool_get_timed_out",
        "Total gets performed that timed out while waiting for a connection"
    )
    .unwrap();
    static ref GET_WAIT_TIME: IntGauge = register_int_gauge!(
        "platz_db_pool_get_wait_time_ms",
        "Total time accumulated waiting for a connection (in milliseconds)"
    )
    .unwrap();
    static ref CONNECTIONS_CREATED: IntGauge = register_int_gauge!(
        "platz_db_pool_connections_created",
        "Total connections created"
    )
    .unwrap();
    static ref CONNECTIONS_CLOSED_BROKEN: IntGauge = register_int_gauge!(
        "platz_db_pool_connections_closed_broken",
        "Total connections that were closed due to be in broken state"
    )
    .unwrap();
    static ref CONNECTIONS_CLOSED_INVALID: IntGauge = register_int_gauge!(
        "platz_db_pool_connections_closed_invalid",
        "Total connections that were closed due to be considered invalid"
    )
    .unwrap();
    static ref CONNECTIONS_CLOSED_MAX_LIFETIME: IntGauge = register_int_gauge!(
        "platz_db_pool_connections_closed_max_lifetime",
        "Total connections that were closed because they reached the max lifetime"
    )
    .unwrap();
    static ref CONNECTIONS_CLOSED_IDLE_TIMEOUT: IntGauge = register_int_gauge!(
        "platz_db_pool_connections_closed_idle_timeout",
        "Total connections that were closed because they reached the max idle timeout"
    )
    .unwrap();
}

pub(crate) async fn start(pool: DbPool) {
    let mut interval = time::interval(Duration::from_secs(15));
    loop {
        interval.tick().await;
        let state = pool.state();
        CONNECTIONS.set(state.connections.into());
        IDLE_CONNECTIONS.set(state.idle_connections.into());
        GET_DIRECT.set(state.statistics.get_direct.try_into().unwrap());
        GET_WAITED.set(state.statistics.get_waited.try_into().unwrap());
        GET_TIMED_OUT.set(state.statistics.get_timed_out.try_into().unwrap());
        GET_WAIT_TIME.set(
            state
                .statistics
                .get_wait_time
                .as_millis()
                .try_into()
                .unwrap(),
        );
        CONNECTIONS_CREATED.set(state.statistics.connections_created.try_into().unwrap());
        CONNECTIONS_CLOSED_BROKEN.set(
            state
                .statistics
                .connections_closed_broken
                .try_into()
                .unwrap(),
        );
        CONNECTIONS_CLOSED_INVALID.set(
            state
                .statistics
                .connections_closed_invalid
                .try_into()
                .unwrap(),
        );
        CONNECTIONS_CLOSED_MAX_LIFETIME.set(
            state
                .statistics
                .connections_closed_max_lifetime
                .try_into()
                .unwrap(),
        );
        CONNECTIONS_CLOSED_IDLE_TIMEOUT.set(
            state
                .statistics
                .connections_closed_idle_timeout
                .try_into()
                .unwrap(),
        );
    }
}
