use mongodb::{
    bson::{doc, Document},
    options::{ClientOptions, UpdateOptions},
    Client, Database,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Restriction {
    user_id: i64,
    login: String
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatInfo {
    chat_id: i64,
    restricted: Vec<Restriction>
}

#[derive(Clone)]
pub(crate) struct MongoDatabase {
    db: Database
}

const COLLECTION: &str = "restrictions";

impl MongoDatabase {
    pub(crate) async fn from_connection_string(
        con_str: &str,
        database: &str,
    ) -> Result<MongoDatabase, anyhow::Error> {
        let client_options = ClientOptions::parse(con_str).await?;
        let database_client = Client::with_options(client_options)?;
        let database = database_client.database(&database);

        Ok(MongoDatabase { db: database })
    }

    pub(crate) async fn restriction_list(&self, chat_id: i64) -> anyhow::Result<Vec<String>> {
        let col = self.db.collection::<ChatInfo>(COLLECTION);
        let query = doc! {
            "chat_id": chat_id
        };
        let option: Option<ChatInfo> = col.find_one(query, None).await?;
        if let Some(chat) = option {
            Ok(chat.restricted.into_iter().map(|chat| chat.login.to_string()).collect())
        } else {
            Ok(Vec::new())
        }
    }

    pub(crate) async fn add_to_restrictions(&self, chat_id: i64, user_id: i64, login: &str) -> anyhow::Result<()> {
        let col = self.db.collection::<Document>(COLLECTION);
        let options = UpdateOptions::builder().upsert(true).build();
        col
            .update_one(
                doc!{"chat_id": chat_id},
                doc! {
                "$addToSet": {"restricted": {"user_id": user_id, "login": login}}
            }, options).await?;
        Ok(())
    }
    pub(crate) async fn remove_from_restrictions(&self, chat_id: i64, login: &str) -> anyhow::Result<()>{
        let col = self.db.collection::<Document>(COLLECTION);
        let options = UpdateOptions::builder().upsert(true).build();

        col
            .update_one(
                doc!{"chat_id": chat_id},
                doc! {
                "$pull": {"restricted": {"login": login}}
            }, options).await?;
        Ok(())
    }
    pub(crate) async fn is_chat_restricted(&self, chat_id: i64, id: i64) -> anyhow::Result<bool> {
        let col = self.db.collection::<ChatInfo>(COLLECTION);
        let query = doc! {
            "chat_id": &chat_id
        };
        let option: Option<ChatInfo> = col.find_one(query, None).await?;
        Ok(if let Some(chat) = option {
            chat.restricted.into_iter().any(|restr| restr.user_id == id)
        } else {
            false
        })
    }
}