//! GPIO abstraction layer

mod traits;
mod mock;

#[cfg(feature = "real-gpio")]
mod rppal;

pub use traits::*;
pub use mock::MockGpio;

#[cfg(feature = "real-gpio")]
pub use self::rppal::RppalGpio;

/// Default GPIO implementation based on features
#[cfg(feature = "mock-gpio")]
pub type DefaultGpio = MockGpio;

#[cfg(feature = "real-gpio")]
pub type DefaultGpio = RppalGpio;
