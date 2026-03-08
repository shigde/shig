#[macro_export]
macro_rules! lobby_error {
    ($lobby_id:expr, $($arg:tt)*) => {
        log::error!(
            "[Lobby uuid={}] {}",
            $lobby_id,
            format_args!($($arg)*)
        );
    };
}

#[macro_export]
macro_rules! lobby_info {
    ($lobby_id:expr, $($arg:tt)*) => {
        log::info!(
            "[Lobby uuid={}] {}",
            $lobby_id,
            format!($($arg)*)
        );
    };
}
