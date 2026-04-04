use crate::connection::ConnectionOps;
use crate::Connection;

pub(crate) fn prefer_swrast() -> bool {
    #[cfg(windows)]
    {
        if crate::os::windows::is_running_in_rdp_session() {
            // Using OpenGL in RDP has problematic behavior upon
            // disconnect, so we force the use of software rendering.
            log::trace!("Running in an RDP session, use SWRAST");
            return true;
        }
    }
    Connection::get()
        .map(|conn| conn.config())
        .unwrap_or_else(config::current_config_handle)
        .front_end
        == config::FrontEndSelection::Software
}
