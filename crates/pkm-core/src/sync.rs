//! Sync eligibility and conflict detection.
//!
//! Determines whether objects can be safely synced and detects conflicts
//! when two versions diverge from the same parent.

use crate::Timestamp;

/// A conflict detection result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncConflict {
    /// No conflict; one version can supersede the other.
    NoConflict,
    /// Conflict: both versions diverged from the same parent independently.
    Conflict,
}

/// Trait for objects that can be synced and checked for conflicts.
pub trait SyncEligible {
    /// Get the current version number.
    fn version(&self) -> u32;

    /// Get the timestamp of the last update.
    fn updated_at(&self) -> Timestamp;

    /// Check if this object is eligible for syncing.
    fn is_sync_eligible(&self) -> bool {
        self.version() >= 1
    }

    /// Detect conflict between this version and another.
    ///
    /// Returns `Conflict` if both versions diverged from the same parent
    /// independently (concurrent edits). Returns `NoConflict` if one clearly
    /// precedes the other (last-write-wins).
    fn detect_conflict(&self, other: &Self) -> SyncConflict {
        // Different versions: not a direct conflict (yet).
        if self.version() != other.version() {
            return SyncConflict::NoConflict;
        }

        // Both claim to be version 1: concurrent creation (conflict).
        if self.version() == 1 {
            return SyncConflict::Conflict;
        }

        // Same version, different timestamps: last-write-wins.
        if self.updated_at() != other.updated_at() {
            return SyncConflict::NoConflict;
        }

        // Same version, same timestamp: concurrent edit (conflict).
        SyncConflict::Conflict
    }

    /// Merge this version with another, preferring the newer one.
    ///
    /// This is a simple last-write-wins merge. If there's a conflict,
    /// the caller should handle it manually (via user review).
    fn merge_prefer_newer(&self, other: &Self) -> Self
    where
        Self: Clone,
    {
        if other.updated_at() > self.updated_at() {
            other.clone()
        } else {
            self.clone()
        }
    }
}

/// A reference to a synced object (id + version + timestamp).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct SyncRef {
    /// Version number at the time of sync.
    pub version: u32,
    /// Timestamp of the last update.
    pub updated_at: Timestamp,
}

impl SyncRef {
    pub fn new(version: u32, updated_at: Timestamp) -> Self {
        Self {
            version,
            updated_at,
        }
    }

    /// Check if this sync ref represents a conflict with another.
    pub fn detect_conflict(&self, other: &SyncRef) -> SyncConflict {
        // Different versions: not a conflict yet.
        if self.version != other.version {
            return SyncConflict::NoConflict;
        }

        // Both at version 1: concurrent creation.
        if self.version == 1 {
            return SyncConflict::Conflict;
        }

        // Same version, different timestamps: last-write-wins.
        if self.updated_at != other.updated_at {
            return SyncConflict::NoConflict;
        }

        // Same version, same timestamp: concurrent edit.
        SyncConflict::Conflict
    }

    /// Determine which version is newer.
    pub fn is_newer_than(&self, other: &SyncRef) -> bool {
        self.version > other.version
            || (self.version == other.version && self.updated_at > other.updated_at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock struct for testing SyncEligible.
    #[derive(Clone)]
    struct MockSyncable {
        version: u32,
        updated_at: Timestamp,
    }

    impl SyncEligible for MockSyncable {
        fn version(&self) -> u32 {
            self.version
        }

        fn updated_at(&self) -> Timestamp {
            self.updated_at
        }
    }

    #[test]
    fn version_1_is_eligible() {
        let obj = MockSyncable {
            version: 1,
            updated_at: Timestamp::now_utc(),
        };
        assert!(obj.is_sync_eligible());
    }

    #[test]
    fn version_0_is_not_eligible() {
        let obj = MockSyncable {
            version: 0,
            updated_at: Timestamp::now_utc(),
        };
        assert!(!obj.is_sync_eligible());
    }

    #[test]
    fn different_versions_no_conflict() {
        let now = Timestamp::now_utc();
        let obj_v1 = MockSyncable {
            version: 1,
            updated_at: now,
        };
        let obj_v2 = MockSyncable {
            version: 2,
            updated_at: now,
        };

        assert_eq!(obj_v1.detect_conflict(&obj_v2), SyncConflict::NoConflict);
    }

    #[test]
    fn concurrent_creation_is_conflict() {
        let now = Timestamp::now_utc();
        let obj_a = MockSyncable {
            version: 1,
            updated_at: now,
        };
        let obj_b = MockSyncable {
            version: 1,
            updated_at: now,
        };

        assert_eq!(obj_a.detect_conflict(&obj_b), SyncConflict::Conflict);
    }

    #[test]
    fn same_version_different_time_no_conflict() {
        let now = Timestamp::now_utc();
        let later = now.saturating_add(time::Duration::seconds(10));

        let obj_a = MockSyncable {
            version: 2,
            updated_at: now,
        };
        let obj_b = MockSyncable {
            version: 2,
            updated_at: later,
        };

        assert_eq!(obj_a.detect_conflict(&obj_b), SyncConflict::NoConflict);
    }

    #[test]
    fn same_version_same_time_is_conflict() {
        let now = Timestamp::now_utc();
        let obj_a = MockSyncable {
            version: 2,
            updated_at: now,
        };
        let obj_b = MockSyncable {
            version: 2,
            updated_at: now,
        };

        assert_eq!(obj_a.detect_conflict(&obj_b), SyncConflict::Conflict);
    }

    #[test]
    fn sync_ref_conflict_detection() {
        let t1 = Timestamp::now_utc();
        let t2 = t1.saturating_add(time::Duration::seconds(1));

        let ref_a = SyncRef::new(2, t1);
        let ref_b = SyncRef::new(2, t2);

        assert_eq!(ref_a.detect_conflict(&ref_b), SyncConflict::NoConflict);
    }

    #[test]
    fn sync_ref_concurrent_edit_conflict() {
        let now = Timestamp::now_utc();
        let ref_a = SyncRef::new(2, now);
        let ref_b = SyncRef::new(2, now);

        assert_eq!(ref_a.detect_conflict(&ref_b), SyncConflict::Conflict);
    }

    #[test]
    fn is_newer_than() {
        let t1 = Timestamp::now_utc();

        let newer = SyncRef::new(3, t1);
        let older = SyncRef::new(2, t1);

        assert!(newer.is_newer_than(&older));
        assert!(!older.is_newer_than(&newer));
    }

    #[test]
    fn is_newer_than_same_version_different_time() {
        let t1 = Timestamp::now_utc();
        let t2 = t1.saturating_add(time::Duration::seconds(10));

        let newer = SyncRef::new(2, t2);
        let older = SyncRef::new(2, t1);

        assert!(newer.is_newer_than(&older));
        assert!(!older.is_newer_than(&newer));
    }
}
