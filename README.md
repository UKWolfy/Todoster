# Todoster — RON-backed CLI To-Do App

A simple, lightweight command-line to-do manager written in Rust.  
Tasks are stored in a human-readable **RON file** at:

```
~/.config/todoster/todos.ron
```

The app supports repeating tasks with auto-reset, editing, undo, multi-delete with ranges, and safe confirmation mode.

---

## ✨ Features

- Add tasks with optional repeat interval (days)
- Auto-reset repeating tasks when they become due
- Mark complete / undo completion
- Edit task text or repeat settings
- Delete multiple tasks (supports ranges like `1-4,7`)
- Safe delete preview with `--confirm`
- XDG config storage (`~/.config/todoster/`)
- Integration tests for core behaviour

---

## 🧰 Installation

Build from source:

```bash
cargo install --path .
```

(or run locally with)

```bash
cargo run
```

---

## 🕹 Usage

List tasks:

```bash
todo
```

Add a task:

```bash
todo add "Feed the gecko"
```

Add a repeating task:

```bash
todo add "Clean tank" --repeat 7
```

Mark complete:

```bash
todo complete 0
```

Undo completion:

```bash
todo undo 0
```

Edit a task:

```bash
todo edit 1 --text "Feed the gecko & mist tank"
todo edit 1 --repeat 3
todo edit 1 --clear-repeat
```

Delete tasks (supports commas & ranges):

```bash
todo delete 0,2-4,7        # dry-run preview
todo delete 0,2-4,7 --confirm
```

Show command summary:

```bash
todo commands
```

Use a custom data file:

```bash
todo --file work.ron list
```

---

## 🧪 Tests

Run tests with:

```bash
cargo test
```

---

## 📂 Data Format (RON)

Example stored task entry:

```ron
(
  items: [
    (
      text: "Feed the gecko",
      complete: false,
      complete_date: None,
      repeat_days: Some(2),
    ),
  ],
)
```

---

## 🚧 Roadmap Ideas

- 1-based UI indexes (keep internal 0-based)
- colourised CLI output
- tags / priorities
- `stats` command
- optional SQLite backend

---

## 🦀 Built With

- Rust
- clap
- chrono
- ron
- anyhow
- serde
