//! A place to keep text that is slow to obtain.
//!
//! Some facts take long to read, such as the textures of several thousand
//! meshes. An adapter can keep what it read in a [`TextCache`] and ask for
//! it again on the next scan. The cache itself lives elsewhere (ADR 0006);
//! adapters only see this trait.

/// Keeps texts under a namespace and a key, each with a fingerprint.
///
/// The fingerprint stands for the state of whatever the text was derived
/// from, for example the size and date of a file. Asking with another
/// fingerprint finds nothing, so an entry goes stale by itself when its
/// source changes.
pub trait TextCache {
    /// The text kept under `namespace` and `key`, if it was stored with
    /// this `fingerprint`.
    fn get(&self, namespace: &str, key: &str, fingerprint: &str) -> Option<String>;

    /// Keeps `value` under `namespace` and `key`, replacing what was there.
    /// A cache that cannot store it simply does not.
    fn put(&self, namespace: &str, key: &str, fingerprint: &str, value: &str);
}

/// The [`TextCache`] that remembers nothing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoCache;

impl TextCache for NoCache {
    fn get(&self, _namespace: &str, _key: &str, _fingerprint: &str) -> Option<String> {
        None
    }

    fn put(&self, _namespace: &str, _key: &str, _fingerprint: &str, _value: &str) {}
}
