use crate::domain::{
    models::{Activity, Conversation, Message},
    ports::ConversationRepository,
};
use rusqlite::{params, Connection};
use std::path::Path;
use uuid::Uuid;

pub struct Store(pub Connection);
impl Store {
    pub fn open(path: &Path) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        Self::initialize(conn)
    }
    fn initialize(conn: Connection) -> Result<Self, String> {
        conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;
            CREATE TABLE IF NOT EXISTS conversations (
              id TEXT PRIMARY KEY, title TEXT NOT NULL, updated_at INTEGER NOT NULL DEFAULT (unixepoch()));
            CREATE TABLE IF NOT EXISTS messages (
              seq INTEGER PRIMARY KEY AUTOINCREMENT, id TEXT UNIQUE NOT NULL,
              conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
              role TEXT NOT NULL CHECK(role IN ('user','assistant')), content TEXT NOT NULL,
              status TEXT NOT NULL, created_at INTEGER NOT NULL DEFAULT (unixepoch()));
            CREATE INDEX IF NOT EXISTS messages_conversation ON messages(conversation_id, seq);
            CREATE TABLE IF NOT EXISTS activity (
              seq INTEGER PRIMARY KEY AUTOINCREMENT, id TEXT UNIQUE NOT NULL,
              conversation_id TEXT REFERENCES conversations(id) ON DELETE CASCADE,
              model TEXT NOT NULL, status TEXT NOT NULL, detail TEXT NOT NULL,
              created_at INTEGER NOT NULL DEFAULT (unixepoch()));
            UPDATE messages SET status='interrupted' WHERE status='streaming';
            UPDATE activity SET status='interrupted', detail='Application closed during generation'
              WHERE status='running';
            PRAGMA user_version=1;") .map_err(|e| e.to_string())?;
        Ok(Self(conn))
    }
}
impl ConversationRepository for Store {
    fn conversations(&self) -> Result<Vec<Conversation>, String> {
        let mut stmt = self
            .0
            .prepare(
                "SELECT id,title,updated_at FROM conversations ORDER BY updated_at DESC,rowid DESC",
            )
            .map_err(|e| e.to_string())?;
        let result = stmt
            .query_map([], |r| {
                Ok(Conversation {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    updated_at: r.get(2)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string());
        result
    }
    fn create(&self) -> Result<Conversation, String> {
        let id = Uuid::new_v4().to_string();
        self.0
            .execute(
                "INSERT INTO conversations(id,title) VALUES (?1,'New conversation')",
                [&id],
            )
            .map_err(|e| e.to_string())?;
        self.conversations()?
            .into_iter()
            .find(|c| c.id == id)
            .ok_or("Conversation creation failed".into())
    }
    fn messages(&self, id: &str) -> Result<Vec<Message>, String> {
        let mut stmt=self.0.prepare("SELECT id,conversation_id,role,content,status,created_at FROM messages WHERE conversation_id=?1 ORDER BY seq").map_err(|e|e.to_string())?;
        let result = stmt
            .query_map([id], |r| {
                Ok(Message {
                    id: r.get(0)?,
                    conversation_id: r.get(1)?,
                    role: r.get(2)?,
                    content: r.get(3)?,
                    status: r.get(4)?,
                    created_at: r.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string());
        result
    }
    fn begin(
        &mut self,
        conversation: &str,
        content: &str,
        model: &str,
        task: &str,
    ) -> Result<String, String> {
        let assistant = Uuid::new_v4().to_string();
        let tx = self.0.transaction().map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO messages(id,conversation_id,role,content,status) VALUES (?1,?2,'user',?3,'complete')",params![Uuid::new_v4().to_string(),conversation,content]).map_err(|e|e.to_string())?;
        tx.execute("INSERT INTO messages(id,conversation_id,role,content,status) VALUES (?1,?2,'assistant','','streaming')",params![assistant,conversation]).map_err(|e|e.to_string())?;
        let title: String = content.chars().take(60).collect();
        tx.execute("UPDATE conversations SET updated_at=unixepoch(),title=CASE WHEN title='New conversation' THEN ?2 ELSE title END WHERE id=?1",params![conversation,title]).map_err(|e|e.to_string())?;
        tx.execute("INSERT INTO activity(id,conversation_id,model,status,detail) VALUES (?1,?2,?3,'running','Generating a local response')",params![task,conversation,model]).map_err(|e|e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(assistant)
    }
    fn checkpoint(&self, id: &str, content: &str) -> Result<(), String> {
        self.0
            .execute(
                "UPDATE messages SET content=?2 WHERE id=?1",
                params![id, content],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    fn finish(
        &mut self,
        id: &str,
        task: &str,
        content: &str,
        status: &str,
        detail: &str,
    ) -> Result<Message, String> {
        let tx = self.0.transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE messages SET content=?2,status=?3 WHERE id=?1",
            params![id, content, status],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE activity SET status=?2,detail=?3 WHERE id=?1",
            params![task, status, detail],
        )
        .map_err(|e| e.to_string())?;
        tx.execute("UPDATE conversations SET updated_at=unixepoch() WHERE id=(SELECT conversation_id FROM messages WHERE id=?1)",[id]).map_err(|e|e.to_string())?;
        let message=tx.query_row("SELECT id,conversation_id,role,content,status,created_at FROM messages WHERE id=?1",[id],|r| Ok(Message {id:r.get(0)?,conversation_id:r.get(1)?,role:r.get(2)?,content:r.get(3)?,status:r.get(4)?,created_at:r.get(5)?})).map_err(|e|e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(message)
    }
    fn delete(&self, id: &str) -> Result<(), String> {
        self.0
            .execute("DELETE FROM conversations WHERE id=?1", [id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    fn activity(&self) -> Result<Vec<Activity>, String> {
        let mut stmt=self.0.prepare("SELECT id,conversation_id,model,status,detail,created_at FROM activity ORDER BY seq DESC LIMIT 100").map_err(|e|e.to_string())?;
        let result = stmt
            .query_map([], |r| {
                Ok(Activity {
                    id: r.get(0)?,
                    conversation_id: r.get(1)?,
                    model: r.get(2)?,
                    status: r.get(3)?,
                    detail: r.get(4)?,
                    created_at: r.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string());
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn completed_history_survives_closing_and_reopening_database_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("history.db");
        let conversation_id;
        {
            let mut store = Store::open(&path).unwrap();
            let conversation = store.create().unwrap();
            conversation_id = conversation.id;
            let assistant = store
                .begin(&conversation_id, "Hello", "qwen", "task")
                .unwrap();
            store
                .finish(
                    &assistant,
                    "task",
                    "Saved across restart 🦀",
                    "complete",
                    "Done",
                )
                .unwrap();
        }
        let store = Store::open(&path).unwrap();
        assert_eq!(store.conversations().unwrap()[0].title, "Hello");
        let messages = store.messages(&conversation_id).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[1].content, "Saved across restart 🦀");
        assert_eq!(messages[1].status, "complete");
        assert_eq!(store.activity().unwrap()[0].status, "complete");
    }
    #[test]
    fn history_transactions_cascade_and_recovery() {
        let mut store = Store::initialize(Connection::open_in_memory().unwrap()).unwrap();
        assert!(store
            .begin("missing", "hello", "model", "bad-task")
            .is_err());
        assert!(store.activity().unwrap().is_empty());
        let c = store.create().unwrap();
        let a = store
            .begin(&c.id, "hello 'quoted' 🦀", "qwen", "task")
            .unwrap();
        store.checkpoint(&a, "partial").unwrap();
        let store = Store::initialize(store.0).unwrap();
        assert_eq!(store.messages(&c.id).unwrap()[1].status, "interrupted");
        assert_eq!(store.messages(&c.id).unwrap()[1].content, "partial");
        assert_eq!(store.activity().unwrap()[0].status, "interrupted");
        store.delete(&c.id).unwrap();
        assert!(store.messages(&c.id).unwrap().is_empty());
        assert!(store.activity().unwrap().is_empty());
    }
}
