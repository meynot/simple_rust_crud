use axum::{
    extract::{Path, Query, State},
    response::Html,
    routing::get,
    Json, Router,
};
use chrono::Local;
use fake::{faker::{internet::en::FreeEmail, name::en::Name}, Fake};
use rand::Rng;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

// ─── Model ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id:         i64,
    name:       String,
    email:      String,
    dob:        String,
    password:   String,   // ⚠ use bcrypt/argon2 in production
    created_at: String,
    updated_at: String,
    deleted_at: Option<String>,
}

/// Public DTO — password is never sent to the client.
#[derive(Debug, Clone, Serialize)]
struct UserDto {
    id:         i64,
    name:       String,
    email:      String,
    dob:        String,
    created_at: String,
    updated_at: String,
}

impl From<User> for UserDto {
    fn from(u: User) -> Self {
        Self {
            id: u.id, name: u.name, email: u.email, dob: u.dob,
            created_at: u.created_at, updated_at: u.updated_at,
        }
    }
}

fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<User> {
    Ok(User {
        id:         row.get(0)?,
        name:       row.get(1)?,
        email:      row.get(2)?,
        dob:        row.get(3)?,
        password:   row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
        deleted_at: row.get(7)?,
    })
}

fn now() -> String { Local::now().format("%Y-%m-%d %H:%M:%S").to_string() }

// ─── DB init ─────────────────────────────────────────────────────────────────

fn init_db(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         CREATE TABLE IF NOT EXISTS users (
             id         INTEGER PRIMARY KEY AUTOINCREMENT,
             name       TEXT NOT NULL,
             email      TEXT NOT NULL UNIQUE,
             dob        TEXT NOT NULL,
             password   TEXT NOT NULL,
             created_at TEXT NOT NULL,
             updated_at TEXT NOT NULL,
             deleted_at TEXT
         );
         CREATE INDEX IF NOT EXISTS idx_users_email      ON users(email);
         CREATE INDEX IF NOT EXISTS idx_users_deleted_at ON users(deleted_at);",
    )
}

// ─── DB layer ────────────────────────────────────────────────────────────────

fn db_insert(c: &Connection, name: &str, email: &str, dob: &str, pw: &str) -> rusqlite::Result<i64> {
    let t = now();
    c.execute(
        "INSERT INTO users (name,email,dob,password,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6)",
        params![name, email, dob, pw, t, t],
    )?;
    Ok(c.last_insert_rowid())
}

fn db_find(c: &Connection, id: i64) -> rusqlite::Result<Option<User>> {
    let mut st = c.prepare(
        "SELECT id,name,email,dob,password,created_at,updated_at,deleted_at
           FROM users WHERE id=?1 AND deleted_at IS NULL",
    )?;
    let mut rows = st.query(params![id])?;
    Ok(rows.next()?.and_then(|r| map_row(r).ok()))
}

fn db_list(c: &Connection, limit: i64, offset: i64, q: &str) -> rusqlite::Result<Vec<User>> {
    if q.is_empty() {
        let mut st = c.prepare(
            "SELECT id,name,email,dob,password,created_at,updated_at,deleted_at
               FROM users WHERE deleted_at IS NULL ORDER BY id DESC LIMIT ?1 OFFSET ?2",
        )?;
        let result = st.query_map(params![limit, offset], map_row)?.collect::<rusqlite::Result<Vec<_>>>();
        result
    } else {
        let pat = format!("%{q}%");
        let mut st = c.prepare(
            "SELECT id,name,email,dob,password,created_at,updated_at,deleted_at
               FROM users WHERE deleted_at IS NULL
               AND (name LIKE ?1 OR email LIKE ?1) ORDER BY id DESC LIMIT ?2 OFFSET ?3",
        )?;
        let result = st.query_map(params![pat, limit, offset], map_row)?.collect::<rusqlite::Result<Vec<_>>>();
        result
    }
}

fn db_count(c: &Connection, q: &str) -> rusqlite::Result<i64> {
    if q.is_empty() {
        c.query_row("SELECT COUNT(*) FROM users WHERE deleted_at IS NULL", [], |r| r.get(0))
    } else {
        let pat = format!("%{q}%");
        c.query_row(
            "SELECT COUNT(*) FROM users WHERE deleted_at IS NULL AND (name LIKE ?1 OR email LIKE ?1)",
            params![pat], |r| r.get(0),
        )
    }
}

fn db_update(c: &Connection, id: i64, name: &str, email: &str, dob: &str, pw: &str) -> rusqlite::Result<usize> {
    c.execute(
        "UPDATE users SET name=?1,email=?2,dob=?3,password=?4,updated_at=?5
          WHERE id=?6 AND deleted_at IS NULL",
        params![name, email, dob, pw, now(), id],
    )
}

fn db_soft_delete(c: &Connection, id: i64) -> rusqlite::Result<usize> {
    let t = now();
    c.execute(
        "UPDATE users SET deleted_at=?1,updated_at=?1 WHERE id=?2 AND deleted_at IS NULL",
        params![t, id],
    )
}

fn db_seed(c: &Connection, n: usize) -> rusqlite::Result<()> {
    let mut rng = rand::thread_rng();
    let (mut done, mut att) = (0usize, 0usize);
    while done < n {
        att += 1;
        if att > n * 5 { break; }
        let name: String  = Name().fake();
        let base: String  = FreeEmail().fake();
        let email = format!("u{att}_{base}");
        let dob   = format!("{}-{:02}-{:02}",
            rng.gen_range(1960..=2005u32),
            rng.gen_range(1..=12u32),
            rng.gen_range(1..=28u32));
        let pw = format!("$2b$12$fake{:08}", rng.gen_range(0..=99_999_999u32));
        if db_insert(c, &name, &email, &dob, &pw).is_ok() { done += 1; }
    }
    println!("✓ Seeded {done} users.");
    Ok(())
}

// ─── Request / Response types ─────────────────────────────────────────────────

#[derive(Deserialize)]
struct CreateBody { name: String, email: String, dob: String, password: String }

#[derive(Deserialize)]
struct UpdateBody { name: String, email: String, dob: String, password: Option<String> }

#[derive(Deserialize)]
struct ListQuery { page: Option<i64>, per_page: Option<i64>, search: Option<String> }

type Db = Arc<Mutex<Connection>>;

fn ok<T: Serialize>(data: T) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "success": true, "data": data }))
}

fn fail(msg: impl std::fmt::Display) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "success": false, "message": msg.to_string() }))
}

// ─── Handlers ────────────────────────────────────────────────────────────────

const HTML: &str = include_str!("../index.html");
async fn route_index() -> Html<&'static str> { Html(HTML) }

async fn route_list(
    State(db): State<Db>,
    Query(p): Query<ListQuery>,
) -> Json<serde_json::Value> {
    let c      = db.lock().unwrap();
    let per    = p.per_page.unwrap_or(10).clamp(1, 100);
    let page   = p.page.unwrap_or(1).max(1);
    let search = p.search.unwrap_or_default();
    let offset = (page - 1) * per;

    match db_list(&c, per, offset, &search) {
        Err(e)   => fail(e),
        Ok(rows) => {
            let total = db_count(&c, &search).unwrap_or(0);
            let pages = ((total as f64) / (per as f64)).ceil() as i64;
            let dtos: Vec<UserDto> = rows.into_iter().map(Into::into).collect();
            Json(serde_json::json!({
                "success": true, "data": dtos,
                "total": total, "page": page, "per_page": per, "pages": pages
            }))
        }
    }
}

async fn route_get(State(db): State<Db>, Path(id): Path<i64>) -> Json<serde_json::Value> {
    let c = db.lock().unwrap();
    match db_find(&c, id) {
        Ok(Some(u)) => ok(UserDto::from(u)),
        Ok(None)    => fail("User not found"),
        Err(e)      => fail(e),
    }
}

async fn route_create(
    State(db): State<Db>,
    Json(b): Json<CreateBody>,
) -> Json<serde_json::Value> {
    let c = db.lock().unwrap();
    match db_insert(&c, &b.name, &b.email, &b.dob, &b.password) {
        Err(e) => fail(e),
        Ok(id) => match db_find(&c, id) {
            Ok(Some(u)) => ok(UserDto::from(u)),
            _           => ok(serde_json::json!({ "id": id })),
        },
    }
}

async fn route_update(
    State(db): State<Db>,
    Path(id): Path<i64>,
    Json(b): Json<UpdateBody>,
) -> Json<serde_json::Value> {
    let c  = db.lock().unwrap();
    let pw = match &b.password {
        Some(p) if !p.trim().is_empty() => p.clone(),
        _ => match db_find(&c, id) {
            Ok(Some(u)) => u.password,
            _           => return fail("User not found"),
        },
    };
    match db_update(&c, id, &b.name, &b.email, &b.dob, &pw) {
        Ok(0)  => fail("User not found"),
        Err(e) => fail(e),
        Ok(_)  => match db_find(&c, id) {
            Ok(Some(u)) => ok(UserDto::from(u)),
            _           => fail("User not found"),
        },
    }
}

async fn route_delete(State(db): State<Db>, Path(id): Path<i64>) -> Json<serde_json::Value> {
    let c = db.lock().unwrap();
    match db_soft_delete(&c, id) {
        Ok(0)  => fail("User not found"),
        Ok(_)  => Json(serde_json::json!({ "success": true, "message": "User deleted" })),
        Err(e) => fail(e),
    }
}

// ─── Main ────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let conn = Connection::open("users.db").expect("open DB");
    init_db(&conn).expect("init DB");

    let n: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0)).unwrap_or(0);
    if n == 0 {
        println!("Database empty — seeding 100 users…");
        db_seed(&conn, 100).ok();
    }

    let db: Db = Arc::new(Mutex::new(conn));

    let app = Router::new()
        .route("/",               get(route_index))
        .route("/api/users",      get(route_list).post(route_create))
        .route("/api/users/{id}", get(route_get).put(route_update).delete(route_delete))
        .with_state(db);

    println!("🚀  Server → http://localhost:3001");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}


