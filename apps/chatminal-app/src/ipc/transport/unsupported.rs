use super::ReadWriteStream;

pub(super) fn connect_local_stream(_endpoint: &str) -> Result<Box<dyn ReadWriteStream>, String> {
    Err("chatminal-app scaffold currently supports unix platforms only".to_string())
}
