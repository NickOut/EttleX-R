//! Store-layer model types (distinct from ettlex-core domain models).

pub mod ettle_record;
pub use ettle_record::{EttleCursor, EttleListItem, EttleListOpts, EttleListPage, EttleRecord};

pub mod relation_record;
pub use relation_record::{
    GroupMemberRecord, GroupRecord, RelationListOpts, RelationRecord, RelationTypeEntry,
};
