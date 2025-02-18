use crate::server::ephemeral_storage;
use actix_web::{HttpResponseBuilder, ResponseError};
use datastore::{self, deserialization, serialization};
use nix::unistd::Gid;
use snafu::Snafu;
use std::io;
use std::path::PathBuf;
use std::string::String;

// We want server (router/handler) and controller errors together so it's easy to define response
// error codes for all the high-level types of errors that could happen during a request.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(super)))]
pub enum Error {
    // Systemd Notification errors
    #[snafu(display("Systemd notify error: {}", source))]
    SystemdNotify { source: std::io::Error },

    #[snafu(display("Failed to send systemd status notification"))]
    SystemdNotifyStatus,

    // =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=

    // Set file metadata errors
    #[snafu(display(
        "Failed to set file permissions on the API socket to {:o}: {}",
        mode,
        source
    ))]
    SetPermissions { source: std::io::Error, mode: u32 },

    #[snafu(display("Failed to set group owner on the API socket to {}: {}", gid, source))]
    SetGroup { source: nix::Error, gid: Gid },

    // =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=

    // Server errors
    #[snafu(display("Missing required input '{}'", input))]
    MissingInput { input: String },

    #[snafu(display("Input '{}' cannot be empty", input))]
    EmptyInput { input: String },

    #[snafu(display("Another thread poisoned the data store lock by panicking"))]
    DataStoreLock,

    #[snafu(display("Unable to serialize response: {}", source))]
    ResponseSerialization { source: serde_json::Error },

    #[snafu(display("Unable to bind to {}: {}", path.display(), source))]
    BindSocket { path: PathBuf, source: io::Error },

    #[snafu(display("Unable to start server: {}", source))]
    ServerStart { source: io::Error },

    #[snafu(display("Tried to commit with no pending changes"))]
    CommitWithNoPending,

    #[snafu(display("Unable to get OS release data: {}", source))]
    ReleaseData { source: bottlerocket_release::Error },

    // =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=

    // Controller errors
    #[snafu(display("Found no '{}' in datastore", prefix))]
    MissingData { prefix: String },

    #[snafu(display("Found no '{}' in datastore", requested))]
    ListKeys { requested: String },

    #[snafu(display("Listed key '{}' not found on disk", key))]
    ListedKeyNotPresent { key: String },

    #[snafu(display("Data store error during {}: {}", op, source))]
    DataStore {
        op: String,
        #[snafu(source(from(datastore::Error, Box::new)))]
        source: Box<datastore::Error>,
    },

    #[snafu(display("Error deserializing {}: {} ", given, source))]
    Deserialization {
        given: String,
        source: deserialization::Error,
    },

    // This is an important error, it's shown when the user uses 'apiclient set' with the
    // key=value form and we don't have enough data to deserialize the value.  It's not the
    // user's fault and so we want to be very clear and give an alternative.
    #[snafu(display("Unable to match your input to the data model.  We may not have enough type information.  Please try the --json input form.  Cause: {}", source))]
    DeserializeMap {
        source: datastore::deserialization::Error,
    },

    #[snafu(display("Unable to serialize data: {}", source))]
    Serialize { source: serde_json::Error },

    #[snafu(display("Error serializing settings to JSON: {}", source))]
    SettingsToJson { source: serde_json::Error },

    #[snafu(display("Error serializing {}: {} ", given, source))]
    DataStoreSerialization {
        given: String,
        source: serialization::Error,
    },

    #[snafu(display("Unable to deserialize input JSON into model: {}", source))]
    DeserializeJson { source: serde_json::Error },

    #[snafu(display("Error serializing {}: {} ", given, source))]
    CommandSerialization {
        given: String,
        source: serde_json::Error,
    },

    #[snafu(display("Unable to initialize ephemeral storage: {}", source))]
    EphemeralInitialize {
        source: ephemeral_storage::error::Error,
    },

    #[snafu(display("Unable to bind ephemeral storage: {}", source))]
    EphemeralBind {
        source: ephemeral_storage::error::Error,
    },

    #[snafu(display("Unable to list ephemeral disks: {}", source))]
    EphemeralListDisks {
        source: ephemeral_storage::error::Error,
    },

    #[snafu(display("Unable to make {} key '{}': {}", key_type, name, source))]
    NewKey {
        key_type: String,
        name: String,
        #[snafu(source(from(datastore::Error, Box::new)))]
        source: Box<datastore::Error>,
    },

    #[snafu(display(
        "Metadata '{}' for key '{}' is not valid JSON: {}",
        key,
        data_key,
        source
    ))]
    InvalidMetadata {
        key: String,
        data_key: String,
        source: serde_json::Error,
    },

    #[snafu(display("Failed to split the string: {}", input))]
    InvalidKeyPair { input: String },

    #[snafu(display("Prefix '{}' is not a valid key: {}", prefix, source))]
    InvalidPrefix {
        prefix: String,
        #[snafu(source(from(datastore::Error, Box::new)))]
        source: Box<datastore::Error>,
    },

    #[snafu(display("Config applier was unable to fork child, returned {}", code))]
    ConfigApplierFork { code: String },

    #[snafu(display("Unable to start config applier: {} ", source))]
    ConfigApplierStart { source: io::Error },

    #[snafu(display("Unable to use config applier, couldn't get stdin"))]
    ConfigApplierStdin {},

    #[snafu(display(
        "Waiting on config applier failed; something else may have awaited it: {} ",
        source
    ))]
    ConfigApplierWait { source: io::Error },

    #[snafu(display("Unable to send input to config applier: {}", source))]
    ConfigApplierWrite { source: io::Error },

    #[snafu(display("Unable to start shutdown: {}", source))]
    Shutdown { source: io::Error },

    #[snafu(display("Failed to reboot, exit code: {}, stderr: {}", exit_code, stderr))]
    Reboot { exit_code: i32, stderr: String },

    #[snafu(display("Unable to generate report: {}", source))]
    ReportExec { source: io::Error },

    #[snafu(display(
        "Failed to generate report, exit code: {}, stderr: {}",
        exit_code,
        stderr
    ))]
    ReportResult { exit_code: i32, stderr: String },

    #[snafu(display("Report type must be specified"))]
    ReportTypeMissing {},

    #[snafu(display("Report type '{}' is not supported", report_type))]
    ReportNotSupported { report_type: String },

    // =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=

    // Update related errors
    #[snafu(display("Unable to start the update dispatcher: {} ", source))]
    UpdateDispatcher { source: io::Error },

    #[snafu(display("Unable to open update lock file: {}", source))]
    UpdateLockOpen { source: io::Error },

    #[snafu(display("Update lock held"))]
    UpdateLockHeld,

    #[snafu(display("Unable to obtain shared lock for reading update status: {}", source))]
    UpdateShareLock { source: io::Error },

    #[snafu(display("Previously chosen Update no longer exists"))]
    UpdateDoesNotExist,

    #[snafu(display("No update image applied to staging partition"))]
    NoStagedImage,

    #[snafu(display("Update action not allowed according to update state"))]
    DisallowCommand,

    #[snafu(display("Update dispatcher failed"))]
    UpdateError,

    #[snafu(display("Update status is uninitialized, refresh-updates to initialize it"))]
    UninitializedUpdateStatus,

    #[snafu(display("Failed to parse update status: {} ", source))]
    UpdateStatusParse { source: serde_json::Error },

    #[snafu(display(
        "Failed to parse update information from '{}': {} ",
        String::from_utf8_lossy(stdout),
        source
    ))]
    UpdateInfoParse {
        stdout: Vec<u8>,
        source: serde_json::Error,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<Error> for actix_web::HttpResponse {
    fn from(e: Error) -> Self {
        // Include the error message in the response.  The Bottlerocket API is only
        // exposed locally, and only on the host filesystem and to authorized containers,
        // so we're not worried about exposing error details.
        HttpResponseBuilder::new(e.status_code()).body(format!("{}", e))
    }
}
