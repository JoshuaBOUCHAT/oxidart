//! Monoio integration for OxidArt.
//!
//! This module provides async utilities for single-threaded monoio runtimes,
//! including automatic timestamp management for TTL functionality.
//!
//! # Example
//!
//! ```rust,ignore
//! use oxidart::OxidArt;
//! use std::time::Duration;
//!
//! #[monoio::main(enable_timer = true)]
//! async fn main() {
//!     // Recommended: creates shared tree with automatic ticker
//!     let tree = OxidArt::shared_with_ticker(Duration::from_millis(100));
//!
//!     // Your server loop here...
//!     tree.borrow_mut().set(/* ... */);
//! }
//! ```

use crate::OxidArt;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// Shared OxidArt type for monoio (single-threaded).
pub type SharedArt = Rc<RefCell<OxidArt>>;

impl OxidArt {
    /// Creates a new shared OxidArt with an automatic background ticker.
    ///
    /// This is the recommended constructor when using TTL features with monoio.
    /// It returns an `Rc<RefCell<OxidArt>>` and spawns a background task that
    /// periodically updates the internal timestamp.
    ///
    /// # Arguments
    ///
    /// * `interval` - How often to update the timestamp (e.g., 100ms)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use oxidart::OxidArt;
    /// use std::time::Duration;
    ///
    /// #[monoio::main(enable_timer = true)]
    /// async fn main() {
    ///     let tree = OxidArt::shared_with_ticker(Duration::from_millis(100));
    ///
    ///     tree.borrow_mut().set_ttl(
    ///         Bytes::from_static(b"key"),
    ///         Duration::from_secs(60),
    ///         Bytes::from_static(b"value"),
    ///     );
    /// }
    /// ```
    pub fn shared_with_ticker(interval: Duration) -> SharedArt {
        let art = Rc::new(RefCell::new(Self::new()));
        art.borrow_mut().tick(); // Initial tick
        spawn_ticker(art.clone(), interval);
        art
    }

    /// Updates the internal timestamp to the current system time.
    ///
    /// This is a convenience method for single-threaded async runtimes.
    /// Call this at the start of each event loop iteration, or use
    /// [`shared_with_ticker`](Self::shared_with_ticker) to automate this.
    #[inline]
    pub fn tick(&mut self) {
        self.now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before UNIX epoch")
            .as_secs();
    }
}

/// Spawns a background task that periodically updates the tree's internal timestamp.
///
/// This is designed for single-threaded monoio runtimes. The ticker runs
/// cooperatively with other tasks in the same thread, updating at each
/// `interval` to keep TTL checks accurate.
///
/// # Arguments
///
/// * `art` - A shared reference to the tree (typically `Rc<RefCell<OxidArt>>`)
/// * `interval` - How often to update the timestamp (e.g., 100ms)
///
/// # Example
///
/// ```rust,ignore
/// use oxidart::OxidArt;
/// use std::cell::RefCell;
/// use std::rc::Rc;
/// use std::time::Duration;
///
/// #[monoio::main]
/// async fn main() {
///     let shared_art = Rc::new(RefCell::new(OxidArt::new()));
///
///     // Spawn ticker - updates every 100ms
///     oxidart::monoio::spawn_ticker(shared_art.clone(), Duration::from_millis(100));
///
///     loop {
///         // handle connections...
///         // No need to manually call tick(), it's handled automatically
///     }
/// }
/// ```
pub fn spawn_ticker(art: Rc<RefCell<OxidArt>>, interval: Duration) {
    monoio::spawn(async move {
        loop {
            monoio::time::sleep(interval).await;
            art.borrow_mut().tick();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[monoio::test(enable_timer = true)]
    async fn test_ttl_expiration_with_ticker() {
        let art = Rc::new(RefCell::new(OxidArt::new()));

        // Spawn the ticker (updates every 100ms)
        spawn_ticker(art.clone(), Duration::from_millis(100));

        // Initial tick to set current time
        art.borrow_mut().tick();

        // batch:1 expires in 1 second
        art.borrow_mut().set_ttl(
            Bytes::from_static(b"batch:1"),
            Duration::from_secs(1),
            Bytes::from_static(b"expires_soon"),
        );

        // batch:2 never expires
        art.borrow_mut().set(
            Bytes::from_static(b"batch:2"),
            Bytes::from_static(b"forever"),
        );

        // Both should exist initially
        let results = art.borrow().getn(Bytes::from_static(b"batch:"));
        assert_eq!(results.len(), 2, "should have 2 entries before expiration");

        // Wait 2 seconds for batch:1 to expire
        monoio::time::sleep(Duration::from_secs(2)).await;

        // Yield to let the ticker task run and update the timestamp
        monoio::time::sleep(Duration::from_millis(150)).await;

        // Only batch:2 should remain
        let results = art.borrow().getn(Bytes::from_static(b"batch:"));
        assert_eq!(results.len(), 1, "should have 1 entry after expiration");
        assert_eq!(
            results[0],
            (
                Bytes::from_static(b"batch:2"),
                Bytes::from_static(b"forever")
            )
        );
    }

    #[monoio::test(enable_timer = true)]
    async fn test_shared_with_ticker_constructor() {
        // Use the convenience constructor
        let art = OxidArt::shared_with_ticker(Duration::from_millis(100));

        // Set a key with TTL
        art.borrow_mut().set_ttl(
            Bytes::from_static(b"test"),
            Duration::from_secs(1),
            Bytes::from_static(b"value"),
        );

        // Should exist initially
        assert!(art.borrow_mut().get(Bytes::from_static(b"test")).is_some());

        // Wait for expiration
        monoio::time::sleep(Duration::from_secs(2)).await;
        monoio::time::sleep(Duration::from_millis(150)).await;

        // Should be expired
        assert!(art.borrow_mut().get(Bytes::from_static(b"test")).is_none());
    }
}
