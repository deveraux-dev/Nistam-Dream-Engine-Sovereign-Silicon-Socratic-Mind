//! Lane WELD — `example_empty`'s handler, the worked reference example for
//! the `Empty` payload kind. Not wired into live dispatch.

use crate::protocol::DaemonReply;

/// The worked example: an `Empty`-payload handler takes no arguments.
pub fn handle() -> DaemonReply {
    DaemonReply::with_data("example:empty-payload-kind")
}
