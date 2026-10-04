#!/usr/bin/env rust-script
//! Single-file Rust CRUD script for a `users` table backed by SQLite.
//!
//! Run directly (requires `rust-script`):
//!   cargo install rust-script
//!   rust-script users_crud.rs
//!
//! Or as a normal Cargo project — copy this file to `src/main.rs` and use
//! the Cargo.toml block below as your manifest.
//!
//! ```cargo
//! [package]
//! name    = "users_crud"
//! version = "0.1.0"
//! edition = "2021"
//!
//! [dependencies]
//! rusqlite = { version = "0.31", features = ["bundled"] }
//! chrono   = "0.4"
//! fake     = { version = "2.9", features = ["derive"] }
//! rand     = "0.8"
//! ```

// ─── Imports ─────────────────────────────────────────────────────────────────

use chrono::Local;
use fake::{
    faker::{internet::en::FreeEmail, name::en::Name},
    Fake,
};
use rand::Rng;
use rusqlite::{params, Connection, Result};

// ─── Model ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct User {
    pub id:         i64,
    pub name:       String,
    pub email:      String,
    pub dob:        String,           // stored as "YYYY-MM-DD"
    pub password:   String,           // ⚠ store a hash (bcrypt/argon2) in prod
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,   // NULL → active; timestamp → soft-deleted
}

// ─── Row mapper ──────────────────────────────────────────────────────────────

fn row_to_user(row: &rusqlite::Row<'_>) -> rusqlite::Result<User> {
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

fn now_str() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

// ─── Database init ───────────────────────────────────────────────────────────

pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;

         CREATE TABLE IF NOT EXISTS users (
             id         INTEGER PRIMARY KEY AUTOINCREMENT,
             name       TEXT    NOT NULL,
             email      TEXT    NOT NULL UNIQUE,
             dob        TEXT    NOT NULL,
             password   TEXT    NOT NULL,
             created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S','now')),
             updated_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S','now')),
             deleted_at TEXT
         );

         CREATE INDEX IF NOT EXISTS idx_users_email      ON users(email);
         CREATE INDEX IF NOT EXISTS idx_users_deleted_at ON users(deleted_at);",
    )
}

// ─── CREATE ──────────────────────────────────────────────────────────────────

/// Insert a new active user, return its auto-generated id.
pub fn create_user(
    conn: &Connection,
    name: &str,
    email: &str,
    dob: &str,
    password: &str,
) -> Result<i64> {
    let now = now_str();
    conn.execute(
        "INSERT INTO users (name, email, dob, password, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![name, email, dob, password, now, now],
    )?;
    Ok(conn.last_insert_rowid())
}

// ─── READ ────────────────────────────────────────────────────────────────────

/// Fetch one active (non-deleted) user by id.
pub fn get_user(conn: &Connection, id: i64) -> Result<Option<User>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, email, dob, password, created_at, updated_at, deleted_at
           FROM users
          WHERE id = ?1 AND deleted_at IS NULL",
    )?;
    let mut rows = stmt.query(params![id])?;
    match rows.next()? {
        Some(row) => Ok(Some(row_to_user(row)?)),
        None      => Ok(None),
    }
}

/// Fetch one user by email (active only).
pub fn get_user_by_email(conn: &Connection, email: &str) -> Result<Option<User>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, email, dob, password, created_at, updated_at, deleted_at
           FROM users
          WHERE email = ?1 AND deleted_at IS NULL",
    )?;
    let mut rows = stmt.query(params![email])?;
    match rows.next()? {
        Some(row) => Ok(Some(row_to_user(row)?)),
        None      => Ok(None),
    }
}

/// List all active users, optional LIMIT/OFFSET for pagination.
pub fn list_users(
    conn: &Connection,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<User>> {
    let lim = limit.unwrap_or(i64::MAX);
    let off = offset.unwrap_or(0);
    let mut stmt = conn.prepare(
        "SELECT id, name, email, dob, password, created_at, updated_at, deleted_at
           FROM users
          WHERE deleted_at IS NULL
          ORDER BY id
          LIMIT ?1 OFFSET ?2",
    )?;
    let result = stmt.query_map(params![lim, off], row_to_user)?
        .collect::<Result<Vec<_>>>();
    result
}

/// List all users including soft-deleted (admin view).
pub fn list_all_users_including_deleted(conn: &Connection) -> Result<Vec<User>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, email, dob, password, created_at, updated_at, deleted_at
           FROM users
          ORDER BY id",
    )?;
    let result = stmt.query_map([], row_to_user)?
        .collect::<Result<Vec<_>>>();
    result
}

/// Count of active users.
pub fn count_active_users(conn: &Connection) -> Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM users WHERE deleted_at IS NULL",
        [],
        |row| row.get(0),
    )
}

// ─── UPDATE ──────────────────────────────────────────────────────────────────

/// Update all mutable fields of an active user. Returns rows affected.
pub fn update_user(
    conn: &Connection,
    id: i64,
    name: &str,
    email: &str,
    dob: &str,
    password: &str,
) -> Result<usize> {
    conn.execute(
        "UPDATE users
            SET name = ?1, email = ?2, dob = ?3, password = ?4, updated_at = ?5
          WHERE id = ?6 AND deleted_at IS NULL",
        params![name, email, dob, password, now_str(), id],
    )
}

/// Update only the name of an active user.
pub fn update_name(conn: &Connection, id: i64, name: &str) -> Result<usize> {
    conn.execute(
        "UPDATE users SET name = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL",
        params![name, now_str(), id],
    )
}

/// Update only the password of an active user.
pub fn update_password(conn: &Connection, id: i64, password: &str) -> Result<usize> {
    conn.execute(
        "UPDATE users SET password = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL",
        params![password, now_str(), id],
    )
}

// ─── DELETE ──────────────────────────────────────────────────────────────────

/// Soft-delete: sets deleted_at timestamp, user stays in DB. Returns rows affected.
pub fn soft_delete_user(conn: &Connection, id: i64) -> Result<usize> {
    let now = now_str();
    conn.execute(
        "UPDATE users SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
        params![now, id],
    )
}

/// Restore a previously soft-deleted user.
pub fn restore_user(conn: &Connection, id: i64) -> Result<usize> {
    conn.execute(
        "UPDATE users SET deleted_at = NULL, updated_at = ?1 WHERE id = ?2",
        params![now_str(), id],
    )
}

/// Hard-delete: permanently removes the row from the database.
pub fn hard_delete_user(conn: &Connection, id: i64) -> Result<usize> {
    conn.execute("DELETE FROM users WHERE id = ?1", params![id])
}

// ─── SEEDER ──────────────────────────────────────────────────────────────────

/// Insert `count` fake users. Skips on duplicate e-mail collision (very rare).
pub fn seed_users(conn: &Connection, count: usize) -> Result<()> {
    let mut rng = rand::thread_rng();
    let mut inserted = 0usize;
    let mut attempts = 0usize;

    println!("  Seeding {} users …", count);

    while inserted < count {
        attempts += 1;
        if attempts > count * 3 {
            eprintln!("  ⚠  Stopped after {} attempts ({} inserted).", attempts, inserted);
            break;
        }

        let name:       String = Name().fake();
        let base_email: String = FreeEmail().fake();
        // Guarantee uniqueness by prefixing with a monotonic counter.
        let email = format!("u{}_{}", attempts, base_email);

        let year  = rng.gen_range(1960u32..=2005);
        let month = rng.gen_range(1u32..=12);
        let day   = rng.gen_range(1u32..=28);   // capped at 28 to stay valid
        let dob   = format!("{}-{:02}-{:02}", year, month, day);

        // In production replace with bcrypt::hash / argon2::hash_password.
        let password = format!("$2b$12$fakehash{:06}", rng.gen_range(0..=999999u32));

        match create_user(conn, &name, &email, &dob, &password) {
            Ok(_)  => inserted += 1,
            Err(e) => eprintln!("  skip ({}): {}", email, e),
        }

        if inserted % 25 == 0 && inserted > 0 {
            println!("    … {} / {} done", inserted, count);
        }
    }

    println!("  ✓ Seeded {} users.\n", inserted);
    Ok(())
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn sep(label: &str) {
    println!("\n{:═<70}", format!("══ {} ", label));
}

fn print_user(u: &User) {
    println!(
        "  id={:<5} name={:<25} email={:<35}\n         dob={} | created={} | deleted={:?}",
        u.id, u.name, u.email, u.dob, u.created_at, u.deleted_at
    );
}

// ─── Main ────────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let db_path = "users.db";
    let conn    = Connection::open(db_path)?;
    init_db(&conn)?;
    println!("Database: {}", db_path);

    // ── SEED ─────────────────────────────────────────────────────────────────
    sep("SEED · 100 fake rows");
    seed_users(&conn, 100)?;
    println!("  Active users in DB: {}", count_active_users(&conn)?);

    // ── CREATE ───────────────────────────────────────────────────────────────
    sep("CREATE · insert a manual user");
    let new_id = create_user(
        &conn,
        "Alice Johnson",
        "alice@example.com",
        "1990-05-15",
        "$2b$12$realHashWouldGoHereInProd",
    )?;
    println!("  Created user with id = {}", new_id);

    // ── READ · single ────────────────────────────────────────────────────────
    sep("READ · fetch user by id");
    match get_user(&conn, new_id)? {
        Some(ref u) => print_user(u),
        None        => println!("  Not found."),
    }

    sep("READ · fetch user by email");
    match get_user_by_email(&conn, "alice@example.com")? {
        Some(ref u) => print_user(u),
        None        => println!("  Not found."),
    }

    // ── READ · list with pagination ──────────────────────────────────────────
    sep("READ · paginated list (first 5 users)");
    let page = list_users(&conn, Some(5), Some(0))?;
    println!("  Showing {} of {} active users:", page.len(), count_active_users(&conn)?);
    for u in &page {
        print_user(u);
    }

    // ── UPDATE · full ────────────────────────────────────────────────────────
    sep("UPDATE · full record update");
    let rows = update_user(
        &conn,
        new_id,
        "Alice M. Johnson",
        "alice.m@example.com",
        "1990-05-15",
        "$2b$12$newHashAfterPasswordChange",
    )?;
    println!("  Rows affected: {}", rows);
    if let Some(ref u) = get_user(&conn, new_id)? {
        print_user(u);
    }

    // ── UPDATE · partial ─────────────────────────────────────────────────────
    sep("UPDATE · partial — name only");
    update_name(&conn, new_id, "Alice M. Johnson-Smith")?;
    if let Some(ref u) = get_user(&conn, new_id)? {
        println!("  New name: {}", u.name);
    }

    // ── SOFT DELETE ──────────────────────────────────────────────────────────
    sep("SOFT DELETE · sets deleted_at");
    let rows = soft_delete_user(&conn, new_id)?;
    println!("  Rows soft-deleted: {}", rows);
    match get_user(&conn, new_id)? {
        Some(_) => println!("  ✗ User still visible (bug!)"),
        None    => println!("  ✓ User id={} hidden from active queries.", new_id),
    }
    // Confirm the row still physically exists:
    let all = list_all_users_including_deleted(&conn)?;
    let ghost = all.iter().find(|u| u.id == new_id);
    if let Some(u) = ghost {
        println!("  Row still in DB: deleted_at = {:?}", u.deleted_at);
    }

    // ── RESTORE ──────────────────────────────────────────────────────────────
    sep("RESTORE · clears deleted_at");
    let rows = restore_user(&conn, new_id)?;
    println!("  Rows restored: {}", rows);
    match get_user(&conn, new_id)? {
        Some(ref u) => println!("  ✓ Restored: {} <{}>", u.name, u.email),
        None        => println!("  ✗ Not found (bug!)"),
    }

    // ── HARD DELETE ──────────────────────────────────────────────────────────
    sep("HARD DELETE · permanent removal");
    let rows = hard_delete_user(&conn, new_id)?;
    println!("  Rows permanently deleted: {}", rows);
    match get_user(&conn, new_id)? {
        Some(_) => println!("  ✗ Row still exists (bug!)"),
        None    => println!("  ✓ Row gone from DB."),
    }

    // ── Summary ──────────────────────────────────────────────────────────────
    sep("SUMMARY");
    println!(
        "  Active users: {}  |  DB file: {}",
        count_active_users(&conn)?,
        db_path
    );
    println!();

    Ok(())
}




