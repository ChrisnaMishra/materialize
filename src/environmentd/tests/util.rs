// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

// BEGIN LINT CONFIG
// DO NOT EDIT. Automatically generated by bin/gen-lints.
// Have complaints about the noise? See the note in misc/python/materialize/cli/gen-lints.py first.
#![allow(clippy::style)]
#![allow(clippy::complexity)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::mutable_key_type)]
#![allow(clippy::stable_sort_primitive)]
#![allow(clippy::map_entry)]
#![allow(clippy::box_default)]
#![warn(clippy::bool_comparison)]
#![warn(clippy::clone_on_ref_ptr)]
#![warn(clippy::no_effect)]
#![warn(clippy::unnecessary_unwrap)]
#![warn(clippy::dbg_macro)]
#![warn(clippy::todo)]
#![warn(clippy::wildcard_dependencies)]
#![warn(clippy::zero_prefixed_literal)]
#![warn(clippy::borrowed_box)]
#![warn(clippy::deref_addrof)]
#![warn(clippy::double_must_use)]
#![warn(clippy::double_parens)]
#![warn(clippy::extra_unused_lifetimes)]
#![warn(clippy::needless_borrow)]
#![warn(clippy::needless_question_mark)]
#![warn(clippy::needless_return)]
#![warn(clippy::redundant_pattern)]
#![warn(clippy::redundant_slicing)]
#![warn(clippy::redundant_static_lifetimes)]
#![warn(clippy::single_component_path_imports)]
#![warn(clippy::unnecessary_cast)]
#![warn(clippy::useless_asref)]
#![warn(clippy::useless_conversion)]
#![warn(clippy::builtin_type_shadow)]
#![warn(clippy::duplicate_underscore_argument)]
#![warn(clippy::double_neg)]
#![warn(clippy::unnecessary_mut_passed)]
#![warn(clippy::wildcard_in_or_patterns)]
#![warn(clippy::crosspointer_transmute)]
#![warn(clippy::excessive_precision)]
#![warn(clippy::overflow_check_conditional)]
#![warn(clippy::as_conversions)]
#![warn(clippy::match_overlapping_arm)]
#![warn(clippy::zero_divided_by_zero)]
#![warn(clippy::must_use_unit)]
#![warn(clippy::suspicious_assignment_formatting)]
#![warn(clippy::suspicious_else_formatting)]
#![warn(clippy::suspicious_unary_op_formatting)]
#![warn(clippy::mut_mutex_lock)]
#![warn(clippy::print_literal)]
#![warn(clippy::same_item_push)]
#![warn(clippy::useless_format)]
#![warn(clippy::write_literal)]
#![warn(clippy::redundant_closure)]
#![warn(clippy::redundant_closure_call)]
#![warn(clippy::unnecessary_lazy_evaluations)]
#![warn(clippy::partialeq_ne_impl)]
#![warn(clippy::redundant_field_names)]
#![warn(clippy::transmutes_expressible_as_ptr_casts)]
#![warn(clippy::unused_async)]
#![warn(clippy::disallowed_methods)]
#![warn(clippy::disallowed_macros)]
#![warn(clippy::disallowed_types)]
#![warn(clippy::from_over_into)]
// END LINT CONFIG

use std::collections::BTreeMap;
use std::error::Error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::{env, thread};

use anyhow::anyhow;
use mz_controller::ControllerConfig;
use mz_environmentd::{WebSocketAuth, WebSocketResponse};
use mz_frontegg_auth::Authentication as FronteggAuthentication;
use mz_orchestrator_process::{ProcessOrchestrator, ProcessOrchestratorConfig};
use mz_ore::metrics::MetricsRegistry;
use mz_ore::now::{EpochMillis, NowFn, SYSTEM_TIME};
use mz_ore::retry::Retry;
use mz_ore::task;
use mz_ore::tracing::{
    OpenTelemetryConfig, StderrLogConfig, StderrLogFormat, TracingConfig, TracingGuard,
    TracingHandle,
};
use mz_persist_client::cache::PersistClientCache;
use mz_persist_client::cfg::{PersistConfig, PersistParameters};
use mz_persist_client::rpc::PersistGrpcPubSubServer;
use mz_persist_client::PersistLocation;
use mz_secrets::SecretsController;
use mz_sql::catalog::EnvironmentId;
use mz_stash::StashFactory;
use mz_storage_client::types::connections::ConnectionContext;
use once_cell::sync::Lazy;
use postgres::error::DbError;
use postgres::tls::{MakeTlsConnect, TlsConnect};
use postgres::types::{FromSql, Type};
use postgres::{NoTls, Socket};
use regex::Regex;
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio::runtime::Runtime;
use tokio_postgres::config::Host;
use tokio_postgres::Client;
use tokio_stream::wrappers::TcpListenerStream;
use tower_http::cors::AllowOrigin;
use tracing::Level;
use tracing_subscriber::EnvFilter;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};
use url::Url;

pub static KAFKA_ADDRS: Lazy<String> =
    Lazy::new(|| env::var("KAFKA_ADDRS").unwrap_or_else(|_| "localhost:9092".into()));

#[derive(Clone)]
pub struct Config {
    data_directory: Option<PathBuf>,
    tls: Option<mz_environmentd::TlsConfig>,
    frontegg: Option<FronteggAuthentication>,
    unsafe_mode: bool,
    workers: usize,
    now: NowFn,
    seed: u32,
    storage_usage_collection_interval: Duration,
    storage_usage_retention_period: Option<Duration>,
    default_cluster_replica_size: String,
    builtin_cluster_replica_size: String,
    propagate_crashes: bool,
    enable_tracing: bool,
    bootstrap_role: Option<String>,
    deploy_generation: Option<u64>,
    system_parameter_defaults: BTreeMap<String, String>,
    concurrent_webhook_req_count: Option<usize>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            data_directory: None,
            tls: None,
            frontegg: None,
            unsafe_mode: false,
            workers: 1,
            now: SYSTEM_TIME.clone(),
            seed: rand::random(),
            storage_usage_collection_interval: Duration::from_secs(3600),
            storage_usage_retention_period: None,
            default_cluster_replica_size: "1".to_string(),
            builtin_cluster_replica_size: "1".to_string(),
            propagate_crashes: false,
            enable_tracing: false,
            bootstrap_role: Some("materialize".into()),
            deploy_generation: None,
            system_parameter_defaults: BTreeMap::new(),
            concurrent_webhook_req_count: None,
        }
    }
}

impl Config {
    pub fn data_directory(mut self, data_directory: impl Into<PathBuf>) -> Self {
        self.data_directory = Some(data_directory.into());
        self
    }

    pub fn with_tls(mut self, cert_path: impl Into<PathBuf>, key_path: impl Into<PathBuf>) -> Self {
        self.tls = Some(mz_environmentd::TlsConfig {
            cert: cert_path.into(),
            key: key_path.into(),
        });
        self
    }

    pub fn unsafe_mode(mut self) -> Self {
        self.unsafe_mode = true;
        self
    }

    pub fn workers(mut self, workers: usize) -> Self {
        self.workers = workers;
        self
    }

    pub fn with_frontegg(mut self, frontegg: &FronteggAuthentication) -> Self {
        self.frontegg = Some(frontegg.clone());
        self
    }

    pub fn with_now(mut self, now: NowFn) -> Self {
        self.now = now;
        self
    }

    pub fn with_storage_usage_collection_interval(
        mut self,
        storage_usage_collection_interval: Duration,
    ) -> Self {
        self.storage_usage_collection_interval = storage_usage_collection_interval;
        self
    }

    pub fn with_storage_usage_retention_period(
        mut self,
        storage_usage_retention_period: Duration,
    ) -> Self {
        self.storage_usage_retention_period = Some(storage_usage_retention_period);
        self
    }

    pub fn with_default_cluster_replica_size(
        mut self,
        default_cluster_replica_size: String,
    ) -> Self {
        self.default_cluster_replica_size = default_cluster_replica_size;
        self
    }

    pub fn with_builtin_cluster_replica_size(
        mut self,
        builtin_cluster_replica_size: String,
    ) -> Self {
        self.builtin_cluster_replica_size = builtin_cluster_replica_size;
        self
    }

    pub fn with_propagate_crashes(mut self, propagate_crashes: bool) -> Self {
        self.propagate_crashes = propagate_crashes;
        self
    }

    pub fn with_enable_tracing(mut self, enable_tracing: bool) -> Self {
        self.enable_tracing = enable_tracing;
        self
    }

    pub fn with_bootstrap_role(mut self, bootstrap_role: Option<String>) -> Self {
        self.bootstrap_role = bootstrap_role;
        self
    }

    pub fn with_deploy_generation(mut self, deploy_generation: Option<u64>) -> Self {
        self.deploy_generation = deploy_generation;
        self
    }

    pub fn with_system_parameter_default(mut self, param: String, value: String) -> Self {
        self.system_parameter_defaults.insert(param, value);
        self
    }

    pub fn with_concurrent_webhook_req_count(mut self, limit: usize) -> Self {
        self.concurrent_webhook_req_count = Some(limit);
        self
    }
}

pub struct Listeners {
    pub runtime: Arc<Runtime>,
    pub inner: mz_environmentd::Listeners,
}

impl Listeners {
    pub fn new() -> Result<Listeners, anyhow::Error> {
        let runtime = Arc::new(Runtime::new()?);
        let inner =
            runtime.block_on(async { mz_environmentd::Listeners::bind_any_local().await })?;
        Ok(Listeners { runtime, inner })
    }

    pub fn serve(self, config: Config) -> Result<Server, anyhow::Error> {
        let environment_id = EnvironmentId::for_tests();
        let (data_directory, temp_dir) = match config.data_directory {
            None => {
                // If no data directory is provided, we create a temporary
                // directory. The temporary directory is cleaned up when the
                // `TempDir` is dropped, so we keep it alive until the `Server` is
                // dropped.
                let temp_dir = tempfile::tempdir()?;
                (temp_dir.path().to_path_buf(), Some(temp_dir))
            }
            Some(data_directory) => (data_directory, None),
        };
        let scratch_dir = tempfile::tempdir()?;
        let (consensus_uri, adapter_stash_url, storage_stash_url) = {
            let seed = config.seed;
            let cockroach_url = env::var("COCKROACH_URL")
                .map_err(|_| anyhow!("COCKROACH_URL environment variable is not set"))?;
            let mut conn = postgres::Client::connect(&cockroach_url, NoTls)?;
            conn.batch_execute(&format!(
                "CREATE SCHEMA IF NOT EXISTS consensus_{seed};
                 CREATE SCHEMA IF NOT EXISTS adapter_{seed};
                 CREATE SCHEMA IF NOT EXISTS storage_{seed};",
            ))?;
            (
                format!("{cockroach_url}?options=--search_path=consensus_{seed}"),
                format!("{cockroach_url}?options=--search_path=adapter_{seed}"),
                format!("{cockroach_url}?options=--search_path=storage_{seed}"),
            )
        };
        let metrics_registry = MetricsRegistry::new();
        let orchestrator = Arc::new(
            self.runtime
                .block_on(ProcessOrchestrator::new(ProcessOrchestratorConfig {
                    image_dir: env::current_exe()?
                        .parent()
                        .unwrap()
                        .parent()
                        .unwrap()
                        .to_path_buf(),
                    suppress_output: false,
                    environment_id: environment_id.to_string(),
                    secrets_dir: data_directory.join("secrets"),
                    command_wrapper: vec![],
                    propagate_crashes: config.propagate_crashes,
                    tcp_proxy: None,
                    scratch_directory: scratch_dir.path().to_path_buf(),
                }))?,
        );
        // Messing with the clock causes persist to expire leases, causing hangs and
        // panics. Is it possible/desirable to put this back somehow?
        let persist_now = SYSTEM_TIME.clone();
        let mut persist_cfg = PersistConfig::new(&mz_environmentd::BUILD_INFO, persist_now);
        // Tune down the number of connections to make this all work a little easier
        // with local postgres.
        persist_cfg.consensus_connection_pool_max_size = 1;
        // Stress persist more by writing rollups frequently
        let mut persist_parameters = PersistParameters::default();
        persist_parameters.rollup_threshold = Some(5);
        persist_parameters.apply(&persist_cfg);

        let persist_pubsub_server = PersistGrpcPubSubServer::new(&persist_cfg, &metrics_registry);
        let persist_pubsub_client = persist_pubsub_server.new_same_process_connection();
        let persist_pubsub_tcp_listener = self
            .runtime
            .block_on(TcpListener::bind(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::LOCALHOST),
                0,
            )))
            .expect("pubsub addr binding");
        let persist_pubsub_server_port = persist_pubsub_tcp_listener
            .local_addr()
            .expect("pubsub addr has local addr")
            .port();
        let _persist_pubsub_server = {
            let _tokio_guard = self.runtime.enter();
            mz_ore::task::spawn(|| "persist_pubsub_server", async move {
                persist_pubsub_server
                    .serve_with_stream(TcpListenerStream::new(persist_pubsub_tcp_listener))
                    .await
                    .expect("success")
            });
        };
        let persist_clients = {
            let _tokio_guard = self.runtime.enter();
            PersistClientCache::new(persist_cfg, &metrics_registry, |_, _| persist_pubsub_client)
        };
        let persist_clients = Arc::new(persist_clients);
        let postgres_factory = StashFactory::new(&metrics_registry);
        let secrets_controller = Arc::clone(&orchestrator);
        let connection_context = ConnectionContext::for_tests(orchestrator.reader());
        let (tracing_handle, tracing_guard) = if config.enable_tracing {
            let config = TracingConfig::<fn(&tracing::Metadata) -> sentry_tracing::EventFilter> {
                service_name: "environmentd",
                stderr_log: StderrLogConfig {
                    format: StderrLogFormat::Json,
                    filter: EnvFilter::default(),
                },
                opentelemetry: Some(OpenTelemetryConfig {
                    endpoint: "http://fake_address_for_testing:8080".to_string(),
                    headers: http::HeaderMap::new(),
                    filter: EnvFilter::default().add_directive(Level::DEBUG.into()),
                    resource: opentelemetry::sdk::resource::Resource::default(),
                }),
                #[cfg(feature = "tokio-console")]
                tokio_console: None,
                sentry: None,
                build_version: mz_environmentd::BUILD_INFO.version,
                build_sha: mz_environmentd::BUILD_INFO.sha,
                build_time: mz_environmentd::BUILD_INFO.time,
                registry: metrics_registry.clone(),
            };
            let (tracing_handle, tracing_guard) =
                self.runtime.block_on(mz_ore::tracing::configure(config))?;
            (tracing_handle, Some(tracing_guard))
        } else {
            (TracingHandle::disabled(), None)
        };

        let inner = self.runtime.block_on(async {
            self.inner
                .serve(mz_environmentd::Config {
                    adapter_stash_url,
                    controller: ControllerConfig {
                        build_info: &mz_environmentd::BUILD_INFO,
                        orchestrator,
                        clusterd_image: "clusterd".into(),
                        init_container_image: None,
                        persist_location: PersistLocation {
                            blob_uri: format!("file://{}/persist/blob", data_directory.display()),
                            consensus_uri,
                        },
                        persist_clients,
                        storage_stash_url,
                        now: SYSTEM_TIME.clone(),
                        postgres_factory,
                        metrics_registry: metrics_registry.clone(),
                        persist_pubsub_url: format!(
                            "http://localhost:{}",
                            persist_pubsub_server_port
                        ),
                        secrets_args: mz_service::secrets::SecretsReaderCliArgs {
                            secrets_reader: mz_service::secrets::SecretsControllerKind::LocalFile,
                            secrets_reader_local_file_dir: Some(data_directory.join("secrets")),
                            secrets_reader_kubernetes_context: None,
                            secrets_reader_aws_region: None,
                            secrets_reader_aws_prefix: None,
                        },
                    },
                    secrets_controller,
                    cloud_resource_controller: None,
                    tls: config.tls,
                    frontegg: config.frontegg,
                    concurrent_webhook_req_count: config.concurrent_webhook_req_count,
                    unsafe_mode: config.unsafe_mode,
                    all_features: false,
                    metrics_registry: metrics_registry.clone(),
                    now: config.now,
                    environment_id,
                    cors_allowed_origin: AllowOrigin::list([]),
                    cluster_replica_sizes: Default::default(),
                    default_storage_cluster_size: None,
                    bootstrap_default_cluster_replica_size: config.default_cluster_replica_size,
                    bootstrap_builtin_cluster_replica_size: config.builtin_cluster_replica_size,
                    system_parameter_defaults: config.system_parameter_defaults,
                    availability_zones: Default::default(),
                    connection_context,
                    tracing_handle,
                    storage_usage_collection_interval: config.storage_usage_collection_interval,
                    storage_usage_retention_period: config.storage_usage_retention_period,
                    segment_api_key: None,
                    egress_ips: vec![],
                    aws_account_id: None,
                    aws_privatelink_availability_zones: None,
                    launchdarkly_sdk_key: None,
                    launchdarkly_key_map: Default::default(),
                    config_sync_loop_interval: None,
                    bootstrap_role: config.bootstrap_role,
                    deploy_generation: config.deploy_generation,
                })
                .await
        })?;
        let server = Server {
            inner,
            runtime: self.runtime,
            metrics_registry,
            _temp_dir: temp_dir,
            _tracing_guard: tracing_guard,
        };
        Ok(server)
    }
}

pub fn start_server(config: Config) -> Result<Server, anyhow::Error> {
    let listeners = Listeners::new()?;
    listeners.serve(config)
}

pub struct Server {
    pub inner: mz_environmentd::Server,
    pub runtime: Arc<Runtime>,
    pub metrics_registry: MetricsRegistry,
    _temp_dir: Option<TempDir>,
    _tracing_guard: Option<TracingGuard>,
}

impl Server {
    pub fn pg_config(&self) -> postgres::Config {
        let local_addr = self.inner.sql_local_addr();
        let mut config = postgres::Config::new();
        config
            .host(&Ipv4Addr::LOCALHOST.to_string())
            .port(local_addr.port())
            .user("materialize");
        config
    }

    pub fn pg_config_internal(&self) -> postgres::Config {
        let local_addr = self.inner.internal_sql_local_addr();
        let mut config = postgres::Config::new();
        config
            .host(&Ipv4Addr::LOCALHOST.to_string())
            .port(local_addr.port())
            .user("mz_system");
        config
    }

    pub fn pg_config_async(&self) -> tokio_postgres::Config {
        let local_addr = self.inner.sql_local_addr();
        let mut config = tokio_postgres::Config::new();
        config
            .host(&Ipv4Addr::LOCALHOST.to_string())
            .port(local_addr.port())
            .user("materialize");
        config
    }

    pub fn enable_feature_flags(&self, flags: &[&'static str]) {
        let mut internal_client = self.connect_internal(postgres::NoTls).unwrap();

        for flag in flags {
            internal_client
                .batch_execute(&format!("ALTER SYSTEM SET {} = true;", flag))
                .unwrap();
        }
    }

    pub fn connect<T>(&self, tls: T) -> Result<postgres::Client, postgres::Error>
    where
        T: MakeTlsConnect<Socket> + Send + 'static,
        T::TlsConnect: Send,
        T::Stream: Send,
        <T::TlsConnect as TlsConnect<Socket>>::Future: Send,
    {
        self.pg_config().connect(tls)
    }

    pub fn connect_internal<T>(&self, tls: T) -> Result<postgres::Client, anyhow::Error>
    where
        T: MakeTlsConnect<Socket> + Send + 'static,
        T::TlsConnect: Send,
        T::Stream: Send,
        <T::TlsConnect as TlsConnect<Socket>>::Future: Send,
    {
        Ok(self.pg_config_internal().connect(tls)?)
    }

    pub async fn connect_async<T>(
        &self,
        tls: T,
    ) -> Result<(tokio_postgres::Client, tokio::task::JoinHandle<()>), postgres::Error>
    where
        T: MakeTlsConnect<Socket> + Send + 'static,
        T::TlsConnect: Send,
        T::Stream: Send,
        <T::TlsConnect as TlsConnect<Socket>>::Future: Send,
    {
        let (client, conn) = self.pg_config_async().connect(tls).await?;
        let handle = task::spawn(|| "connect_async", async move {
            if let Err(err) = conn.await {
                panic!("connection error: {}", err);
            }
        });
        Ok((client, handle))
    }

    pub fn ws_addr(&self) -> Url {
        Url::parse(&format!(
            "ws://{}/api/experimental/sql",
            self.inner.http_local_addr()
        ))
        .unwrap()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct MzTimestamp(pub u64);

impl<'a> FromSql<'a> for MzTimestamp {
    fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<MzTimestamp, Box<dyn Error + Sync + Send>> {
        let n = mz_pgrepr::Numeric::from_sql(ty, raw)?;
        Ok(MzTimestamp(u64::try_from(n.0 .0)?))
    }

    fn accepts(ty: &Type) -> bool {
        mz_pgrepr::Numeric::accepts(ty)
    }
}

pub trait PostgresErrorExt {
    fn unwrap_db_error(self) -> DbError;
}

impl PostgresErrorExt for postgres::Error {
    fn unwrap_db_error(self) -> DbError {
        match self.source().and_then(|e| e.downcast_ref::<DbError>()) {
            Some(e) => e.clone(),
            None => panic!("expected DbError, but got: {:?}", self),
        }
    }
}

impl<T, E> PostgresErrorExt for Result<T, E>
where
    E: PostgresErrorExt,
{
    fn unwrap_db_error(self) -> DbError {
        match self {
            Ok(_) => panic!("expected Err(DbError), but got Ok(_)"),
            Err(e) => e.unwrap_db_error(),
        }
    }
}

/// Group commit will block writes until the current time has advanced. This can make
/// performing inserts while using deterministic time difficult. This is a helper
/// method to perform writes and advance the current time.
pub fn insert_with_deterministic_timestamps(
    table: &'static str,
    values: &'static str,
    server: &Server,
    now: Arc<std::sync::Mutex<EpochMillis>>,
) -> Result<(), Box<dyn Error>> {
    let mut client_write = server.connect(postgres::NoTls)?;
    let mut client_read = server.connect(postgres::NoTls)?;

    let mut current_ts = get_explain_timestamp(table, &mut client_read);
    let write_thread = thread::spawn(move || {
        client_write
            .execute(&format!("INSERT INTO {table} VALUES {values}"), &[])
            .unwrap();
    });
    while !write_thread.is_finished() {
        // Keep increasing `now` until the write has executed succeed. Table advancements may
        // have increased the global timestamp by an unknown amount.
        current_ts += 1;
        *now.lock().expect("lock poisoned") = current_ts;
        thread::sleep(Duration::from_millis(1));
    }
    write_thread.join().unwrap();
    Ok(())
}

pub fn get_explain_timestamp(table: &str, client: &mut postgres::Client) -> EpochMillis {
    let row = client
        .query_one(&format!("EXPLAIN TIMESTAMP FOR SELECT * FROM {table}"), &[])
        .unwrap();
    let explain: String = row.get(0);
    let timestamp_re = Regex::new(r"^\s+query timestamp:\s*(\d+)").unwrap();
    let timestamp_caps = timestamp_re.captures(&explain).unwrap();
    timestamp_caps.get(1).unwrap().as_str().parse().unwrap()
}

/// Helper function to create a Postgres source.
///
/// IMPORTANT: Make sure to call closure that is returned at
/// the end of the test to clean up Postgres state.
///
/// WARNING: If multiple tests use this, and the tests are run
/// in parallel, then make sure the test use different postgres
/// tables.
pub fn create_postgres_source_with_table(
    runtime: &Arc<Runtime>,
    mz_client: &mut postgres::Client,
    table_name: &str,
    table_schema: &str,
    source_name: &str,
) -> Result<
    (
        Client,
        impl FnOnce(&mut postgres::Client, &mut Client, &Arc<Runtime>) -> Result<(), Box<dyn Error>>,
    ),
    Box<dyn Error>,
> {
    let postgres_url = env::var("POSTGRES_URL")
        .map_err(|_| anyhow!("POSTGRES_URL environment variable is not set"))?;

    let (pg_client, connection) =
        runtime.block_on(tokio_postgres::connect(&postgres_url, postgres::NoTls))?;

    let pg_config: tokio_postgres::Config = postgres_url.parse().unwrap();
    let user = pg_config.get_user().unwrap_or("postgres");
    let db_name = pg_config.get_dbname().unwrap_or(user);
    let ports = pg_config.get_ports();
    let port = if ports.is_empty() { 5432 } else { ports[0] };
    let hosts = pg_config.get_hosts();
    let host = if hosts.is_empty() {
        "localhost".to_string()
    } else {
        match &hosts[0] {
            Host::Tcp(host) => host.to_string(),
            Host::Unix(host) => host.to_str().unwrap().to_string(),
        }
    };
    let password = pg_config.get_password();

    let pg_runtime = Arc::<tokio::runtime::Runtime>::clone(runtime);
    thread::spawn(move || {
        if let Err(e) = pg_runtime.block_on(connection) {
            panic!("connection error: {}", e);
        }
    });

    // Create table in Postgres with publication.
    let _ =
        runtime.block_on(pg_client.execute(&format!("DROP TABLE IF EXISTS {table_name};"), &[]))?;
    let _ = runtime
        .block_on(pg_client.execute(&format!("DROP PUBLICATION IF EXISTS {source_name};"), &[]))?;
    let _ = runtime
        .block_on(pg_client.execute(&format!("CREATE TABLE {table_name} {table_schema};"), &[]))?;
    let _ = runtime.block_on(pg_client.execute(
        &format!("ALTER TABLE {table_name} REPLICA IDENTITY FULL;"),
        &[],
    ))?;
    let _ = runtime.block_on(pg_client.execute(
        &format!("CREATE PUBLICATION {source_name} FOR TABLE {table_name};"),
        &[],
    ))?;

    // Create postgres source in Materialize.
    let mut connection_str = format!("HOST '{host}', PORT {port}, USER {user}, DATABASE {db_name}");
    if let Some(password) = password {
        let password = std::str::from_utf8(password).unwrap();
        mz_client.batch_execute(&format!("CREATE SECRET s AS '{password}'"))?;
        connection_str = format!("{connection_str}, PASSWORD SECRET s");
    }
    mz_client.batch_execute(&format!(
        "CREATE CONNECTION pgconn TO POSTGRES ({connection_str})"
    ))?;
    mz_client.batch_execute(&format!(
        "CREATE SOURCE {source_name}
            FROM POSTGRES
            CONNECTION pgconn
            (PUBLICATION '{source_name}')
            FOR TABLES ({table_name});"
    ))?;

    let table_name = table_name.to_string();
    let source_name = source_name.to_string();
    Ok((
        pg_client,
        move |mz_client: &mut postgres::Client, pg_client: &mut Client, runtime: &Arc<Runtime>| {
            mz_client.batch_execute(&format!("DROP SOURCE {source_name};"))?;
            mz_client.batch_execute("DROP CONNECTION pgconn;")?;

            let _ = runtime
                .block_on(pg_client.execute(&format!("DROP PUBLICATION {source_name};"), &[]))?;
            let _ =
                runtime.block_on(pg_client.execute(&format!("DROP TABLE {table_name};"), &[]))?;
            Ok(())
        },
    ))
}

pub fn wait_for_view_population(
    mz_client: &mut postgres::Client,
    view_name: &str,
    source_rows: i64,
) -> Result<(), Box<dyn Error>> {
    let current_isolation = mz_client
        .query_one("SHOW transaction_isolation", &[])?
        .get::<_, String>(0);
    mz_client.batch_execute("SET transaction_isolation = SERIALIZABLE")?;
    Retry::default()
        .retry(|_| {
            let rows = mz_client
                .query_one(&format!("SELECT COUNT(*) FROM {view_name};"), &[])
                .unwrap()
                .get::<_, i64>(0);
            if rows == source_rows {
                Ok(())
            } else {
                Err(format!(
                    "Waiting for {source_rows} row to be ingested. Currently at {rows}."
                ))
            }
        })
        .unwrap();
    mz_client.batch_execute(&format!(
        "SET transaction_isolation = '{current_isolation}'"
    ))?;
    Ok(())
}

// Initializes a websocket connection. Returns the init messages before the initial ReadyForQuery.
pub fn auth_with_ws(
    ws: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    options: BTreeMap<String, String>,
) -> Result<Vec<WebSocketResponse>, anyhow::Error> {
    ws.write_message(Message::Text(
        serde_json::to_string(&WebSocketAuth::Basic {
            user: "materialize".into(),
            password: "".into(),
            options,
        })
        .unwrap(),
    ))?;
    // Wait for initial ready response.
    let mut msgs = Vec::new();
    loop {
        let resp = ws.read_message()?;
        match resp {
            Message::Text(msg) => {
                let msg: WebSocketResponse = serde_json::from_str(&msg).unwrap();
                match msg {
                    WebSocketResponse::ReadyForQuery(_) => break,
                    msg => {
                        msgs.push(msg);
                    }
                }
            }
            Message::Ping(_) => continue,
            Message::Close(None) => return Err(anyhow!("ws closed after auth")),
            Message::Close(Some(close_frame)) => {
                return Err(anyhow!("ws closed after auth").context(close_frame))
            }
            _ => panic!("unexpected response: {:?}", resp),
        }
    }
    Ok(msgs)
}
