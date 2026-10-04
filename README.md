# Users CRUD

A small web app for managing users. The server is written in Rust (Axum) and stores data in a local SQLite file, `users.db`. The admin page is served from the same process, so you do not install a separate database or frontend.

Open [http://localhost:3001](http://localhost:3001) after the server starts.

## Walkthrough

1. Start the server (see [Run the application](#run-the-application)).
2. The terminal prints `Server → http://localhost:3001`. On the first launch, an empty database is filled with 100 sample users.
3. Open [http://localhost:3001](http://localhost:3001) in a browser.

The page is a user list with these actions:

- **Search.** Type a name or email. The list filters as you type and returns to page 1.
- **Browse.** Ten users are shown per page. Use the page controls at the bottom of the table.
- **Add a user.** Click **Add User**. Enter full name, email, date of birth, and a password of at least 8 characters. Email addresses must be unique.
- **Edit a user.** Click the pencil icon on a row. Change the fields you need. Leave the password blank to keep the current one. A new password must still be at least 8 characters.
- **Delete a user.** Click the trash icon and confirm. The row is soft-deleted: it disappears from the list, and the record stays in `users.db` with a `deleted_at` timestamp.

The table shows name, email, date of birth, created time, and updated time. Passwords are not shown.

Stop the server with `Ctrl+C` in the terminal where it is running.

To start over with a fresh sample set, stop the server, delete `users.db` (and `users.db-wal` / `users.db-shm` if they exist), then start the server again.

## What you need

| Requirement | Why |
| --- | --- |
| Rust (`rustc` and `cargo`) | Builds and runs the server |
| A C compiler and linker | `rusqlite` compiles SQLite from source (`bundled` feature) |
| This project | `Cargo.toml`, `Cargo.lock`, `src/main.rs`, and `index.html` |

You do not install SQLite yourself. `index.html` is compiled into the binary. `users.db` is created in the project directory the first time the server runs.

## Install Rust

Install Rust with [rustup](https://rustup.rs). After it finishes, open a new terminal so `cargo` is on your `PATH`, then check:

```bash
rustc --version
cargo --version
```

### macOS

1. Install the Xcode Command Line Tools if you do not already have them (this also provides the C compiler):

   ```bash
   xcode-select --install
   ```

2. Install Rust:

   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

3. Accept the defaults, then load Cargo into the current shell (or open a new terminal):

   ```bash
   source "$HOME/.cargo/env"
   ```

### Linux

1. Install a C compiler and the usual build tools.

   Debian / Ubuntu:

   ```bash
   sudo apt update
   sudo apt install build-essential curl
   ```

   Fedora:

   ```bash
   sudo dnf install gcc curl
   ```

   Arch Linux:

   ```bash
   sudo pacman -S base-devel curl
   ```

2. Install Rust:

   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

3. Accept the defaults, then load Cargo into the current shell (or open a new terminal):

   ```bash
   source "$HOME/.cargo/env"
   ```

### Windows

1. Install the Microsoft C++ build tools. The default Rust toolchain on Windows (`x86_64-pc-windows-msvc`) needs them to compile SQLite.

   - Download [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/).
   - In the installer, select the workload **Desktop development with C++**.
   - Finish the install and reboot if the installer asks you to.

2. Install Rust. Either:

   - Download and run [rustup-init.exe](https://win.rustup.rs/x86_64), or
   - In PowerShell:

     ```powershell
     winget install Rustlang.Rustup
     ```

   Accept the default MSVC toolchain when rustup asks.

3. Close and reopen PowerShell or Command Prompt, then check `rustc --version` and `cargo --version`.

## Run the application

From the project directory (the folder that contains `Cargo.toml`):

### macOS and Linux

```bash
./run_application
```

`run_application` is a small shell script that runs `cargo run` from the project directory. You can call Cargo directly instead:

```bash
cargo run
```

The first build downloads dependencies and compiles SQLite, so it takes longer than later runs. When the terminal shows `Server → http://localhost:3001`, open that address in a browser.

If `./run_application` prints `Permission denied`, make it executable once:

```bash
chmod +x run_application
```

### Windows

In PowerShell or Command Prompt, from the project directory:

```powershell
cargo run
```

`run_application` is a Bash script. It works in Git Bash or WSL. In PowerShell or Command Prompt, use `cargo run`.

When the terminal shows `Server → http://localhost:3001`, open that address in a browser.

## API

The page calls these routes. Responses use JSON with `"success": true` or `"success": false`.

| Method | Path | Purpose |
| --- | --- | --- |
| `GET` | `/` | Admin page |
| `GET` | `/api/users?page=1&per_page=10&search=` | List active users |
| `GET` | `/api/users/{id}` | One user |
| `POST` | `/api/users` | Create a user (`name`, `email`, `dob`, `password`) |
| `PUT` | `/api/users/{id}` | Update a user (`password` is optional) |
| `DELETE` | `/api/users/{id}` | Soft-delete a user |

`per_page` is limited to 1–100. List results omit passwords.
