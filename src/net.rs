mod tcp_listener;
mod tcp_stream;
mod udp_socket;

pub use tcp_listener::*;
pub use tcp_stream::*;
pub use udp_socket::*;

macro_rules! poll_net {
    ($stream:expr, $timeout:expr, $struct_name:ident::$function_name:ident($($param:expr),*)) => {
        if let Ok(Some(duration)) = $timeout {
            if duration.is_zero() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Timeout duration can't be zero",
                ));
            }

            let instant = Instant::now() + duration;

            poll_fn(|_context| {
                if instant.checked_duration_since(Instant::now()).is_none() {
                    return Poll::Ready(Err(Error::new(
                        ErrorKind::TimedOut,
                        format!("{} timed out", stringify!($struct_name)),
                    )));
                }

                match $stream.$function_name($($param,)*) {
                    Ok(length) => Poll::Ready(Ok(length)),
                    Err(error) => match error.kind() {
                        ErrorKind::WouldBlock => Poll::Pending,
                        _ => Poll::Ready(Err(error)),
                    },
                }
            })
            .await
        } else {
            poll_fn(|_context| match $stream.$function_name($($param,)*) {
                Ok(length) => Poll::Ready(Ok(length)),
                Err(error) => match error.kind() {
                    ErrorKind::WouldBlock => Poll::Pending,
                    _ => Poll::Ready(Err(error)),
                },
            })
            .await
        }
    };
}

pub(self) use poll_net;
