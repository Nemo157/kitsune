//! `SeaORM` Entity. Generated by sea-orm-codegen 0.11.0

use crate::custom::ActorType;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "accounts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(column_type = "Text", nullable)]
    pub display_name: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub note: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub username: String,
    pub locked: bool,
    pub local: bool,
    #[sea_orm(column_type = "Text")]
    pub domain: String,
    pub actor_type: ActorType,
    #[sea_orm(column_type = "Text", nullable, unique)]
    pub url: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub featured_collection_url: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub followers_url: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub following_url: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub inbox_url: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub outbox_url: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub shared_inbox_url: Option<String>,
    #[sea_orm(column_type = "Text", unique)]
    pub public_key_id: String,
    #[sea_orm(column_type = "Text")]
    pub public_key: String,
    pub created_at: TimeDateTimeWithTimeZone,
    pub updated_at: TimeDateTimeWithTimeZone,
    pub avatar_id: Option<Uuid>,
    pub header_id: Option<Uuid>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::media_attachments::Entity",
        from = "Column::AvatarId",
        to = "super::media_attachments::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    MediaAttachments2,
    #[sea_orm(
        belongs_to = "super::media_attachments::Entity",
        from = "Column::HeaderId",
        to = "super::media_attachments::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    MediaAttachments1,
    #[sea_orm(has_many = "super::posts::Entity")]
    Posts,
    #[sea_orm(has_many = "super::posts_favourites::Entity")]
    PostsFavourites,
    #[sea_orm(has_one = "super::users::Entity")]
    Users,
}

impl Related<super::posts_favourites::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PostsFavourites.def()
    }
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Users.def()
    }
}

impl Related<super::posts::Entity> for Entity {
    fn to() -> RelationDef {
        super::posts_mentions::Relation::Posts.def()
    }
    fn via() -> Option<RelationDef> {
        Some(super::posts_mentions::Relation::Accounts.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
