#[path = "../util/mod.rs"]
#[macro_use]
mod util;
use util::*;

mod no_server;
mod stream;

use interprocess::local_socket::NameTypeSupport;

#[test]
fn local_socket_stream() -> TestResult {
    use stream::*;
    install_color_eyre();
    // If only one name type is supported, this one will choose the supported one. If both are
    // supported, this will try paths first.
    util::drive_server_and_multiple_clients(|s, n| server(s, n, false), client)?;
    if NameTypeSupport::query() == NameTypeSupport::Both {
        // Try the namespace now.
        util::drive_server_and_multiple_clients(|s, n| server(s, n, true), client)?;
    }
    Ok(())
}
#[test]
fn local_socket_no_server() -> TestResult {
    install_color_eyre();
    // Same as above.
    no_server::run_and_verify_error(false)?;
    if NameTypeSupport::query() == NameTypeSupport::Both {
        no_server::run_and_verify_error(true)?;
    }
    Ok(())
}
