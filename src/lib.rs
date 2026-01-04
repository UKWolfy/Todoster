use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Local, TimeZone};
use clap::{Parser, Subcommand};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

/// Simple RON-based to-do app.
#[derive(Parser)]
#[command(name = "todo")]
#[command(about = "RON-backed todo CLI", long_about = None)]
pub struct Cli {
    /// Path to the RON storage file (default: ~/.config/todoster/todos.ron)
    #[arg(short, long)]
    pub file: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List all tasks (incomplete first, then complete)
    List,

    /// Add a new task
    Add {
        /// The task text
        text: String,
        /// Repeat interval in days
        #[arg(short, long)]
        repeat: Option<i64>,
    },

    /// Mark a task as complete by index (as shown in `list`)
    Complete {
        /// Index of the task to complete
        indexes: String,
    },

    /// Mark a task as incomplete again
    Undo {
        /// Index of the task to mark incomplete
        index: usize,
    },

    /// Edit an existing task
    Edit {
        /// Index of the task to edit
        index: usize,

        /// New text for the task
        #[arg(long)]
        text: Option<String>,

        /// New repeat interval in days
        #[arg(long)]
        repeat: Option<i64>,

        /// Clear the repeat interval
        #[arg(long)]
        clear_repeat: bool,
    },

    /// Delete one or more tasks (comma-separated indexes and ranges)
    Delete {
        /// Comma-separated list of indexes/ranges, e.g. "0,2,5-7"
        indexes: String,

        /// Actually perform deletion (otherwise just show what would be deleted)
        #[arg(long)]
        confirm: bool,
    },

    /// Show a table of available commands
    Commands,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TodoItem {
    pub text: String,
    pub complete: bool,
    pub complete_date: Option<DateTime<Local>>,
    pub repeat_days: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct TodoList {
    pub items: Vec<TodoItem>,
}

impl TodoItem {
    pub fn new(text: String, repeat_days: Option<i64>) -> Self {
        Self {
            text,
            complete: false,
            complete_date: None,
            repeat_days,
        }
    }

    pub fn mark_complete(&mut self, now: DateTime<Local>) {
        self.complete = true;
        self.complete_date = Some(now);
    }

    pub fn mark_incomplete(&mut self) {
        self.complete = false;
        self.complete_date = None;
    }

    /// Returns the next due moment as the start of the due day (midnight local time).
    fn next_due_start(&self) -> Option<DateTime<Local>> {
        let done_at = self.complete_date?;
        let days = self.repeat_days?;

        // Due *date* is based on completion date, not time-of-day.
        let due_date = done_at.date_naive() + Duration::days(days);

        // Consider it due from midnight (start of that day) in local time.
        let naive_midnight = due_date.and_hms_opt(0, 0, 0)?;
        Local.from_local_datetime(&naive_midnight).single()
    }

    pub fn should_reset(&self, now: DateTime<Local>) -> bool {
        if !self.complete {
            return false;
        }

        match self.next_due_start() {
            Some(due_start) => now >= due_start,
            None => false,
        }
    }

    pub fn reset_if_due(&mut self, now: DateTime<Local>) {
        if self.should_reset(now) {
            self.complete = false;
            self.complete_date = None;
        }
    }

    pub fn time_until_next_repeat(&self, now: DateTime<Local>) -> Option<Duration> {
        if !self.complete {
            return None;
        }

        let due_start = self.next_due_start()?;
        Some(due_start - now)
    }
}

impl TodoList {
    fn load(path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(TodoList::default());
        }

        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        let list: TodoList =
            ron::from_str(&contents).with_context(|| "Failed to parse RON data")?;

        Ok(list)
    }

    fn save(&self, path: &PathBuf) -> Result<()> {
        // Make sure the directory exists (for ~/.config/todoster/todos.ron)
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        let pretty = PrettyConfig::new()
            .separate_tuple_members(true)
            .enumerate_arrays(true);

        let ron_string =
            ron::ser::to_string_pretty(self, pretty).with_context(|| "Failed to serialize RON")?;

        let mut file = fs::File::create(path)
            .with_context(|| format!("Failed to create file: {}", path.display()))?;
        file.write_all(ron_string.as_bytes())
            .with_context(|| "Failed to write RON data")?;
        Ok(())
    }

    fn auto_reset_repeating(&mut self, now: DateTime<Local>) {
        for item in &mut self.items {
            item.reset_if_due(now);
        }
    }

    fn add(&mut self, text: String, repeat_days: Option<i64>) {
        self.items.push(TodoItem::new(text, repeat_days));
    }
}

fn default_file_path() -> PathBuf {
    let base = env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|_| env::var("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|_| PathBuf::from("."));

    base.join("todoster").join("todos.ron")
}

fn print_list(list: &TodoList, now: DateTime<Local>) {
    let mut incomplete: Vec<(usize, &TodoItem)> = Vec::new();
    let mut complete: Vec<(usize, &TodoItem)> = Vec::new();

    for (idx, item) in list.items.iter().enumerate() {
        if item.complete {
            complete.push((idx, item));
        } else {
            incomplete.push((idx, item));
        }
    }

    println!("=== Incomplete tasks ===");
    if incomplete.is_empty() {
        println!("(none)");
    } else {
        for (idx, item) in incomplete {
            let repeat_info = match item.repeat_days {
                Some(days) => format!("(Repeat: {}d)", days),
                None => String::new(),
            };

            if repeat_info.is_empty() {
                println!("[{}] {}", idx, item.text);
            } else {
                println!("[{}] {} {}", idx, item.text, repeat_info);
            }
        }
    }

    println!();
    println!("=== Complete tasks ===");
    if complete.is_empty() {
        println!("(none)");
    } else {
        for (idx, item) in complete {
            let repeat_info = match item.time_until_next_repeat(now) {
                Some(diff) => {
                    // diff is time until *midnight* at the start of the due day
                    if diff.num_seconds() <= 0 {
                        // Due day is today (or earlier) -> considered due from midnight
                        let overdue_days = (-diff).num_days();
                        if overdue_days <= 0 {
                            "(repeat: due today)".to_string()
                        } else {
                            format!("(repeat: overdue by {}d)", overdue_days)
                        }
                    } else {
                        let days = diff.num_days();
                        let hours = (diff - Duration::days(days)).num_hours();

                        if days >= 1 {
                            format!("(repeat in {}d, {}hrs)", days, hours)
                        } else {
                            // Due day is today, and per rule it's considered due from midnight
                            "(repeat: due today)".to_string()
                        }
                    }
                }
                None => {
                    if item.repeat_days.is_some() {
                        "(repeat: no completion date yet)".to_string()
                    } else {
                        "(no repeat)".to_string()
                    }
                }
            };

            println!("[{}] {} {}", idx, item.text, repeat_info);
        }
    }
}

fn print_command_table() {
    println!("=== Todoster Commands ===\n");

    println!("{:<45} {}", "todo", "List tasks (default)");
    println!("{:<45} {}", "todo list", "List tasks");

    println!("{:<45} {}", "todo add \"<text>\"", "Add a new task");
    println!(
        "{:<45} {}",
        "todo add \"<text>\" --repeat <days>", "Add repeating task"
    );

    println!(
        "{:<45} {}",
        "todo complete <i1,i2,1-4>", "Mark task(s) complete (supports ranges)"
    );
    println!(
        "{:<45} {}",
        "todo undo <index>", "Mark a task incomplete again"
    );

    println!(
        "{:<45} {}",
        "todo edit <index> --text \"<new>\"", "Edit task text"
    );
    println!(
        "{:<45} {}",
        "todo edit <index> --repeat <days>", "Change repeat interval"
    );
    println!(
        "{:<45} {}",
        "todo edit <index> --clear-repeat", "Remove repeat interval"
    );

    println!(
        "{:<45} {}",
        "todo delete <i1,i2,i3>", "Delete multiple tasks (by index)"
    );
    println!(
        "{:<45} {}",
        "todo delete 1-4,7", "Supports ranges (inclusive)"
    );
    println!(
        "{:<45} {}",
        "todo delete 0,2-3,7 --confirm", "Actually perform deletion"
    );
    println!(
        "{:<45} {}",
        "todo delete 0,2-3,7", "Dry-run (shows what would be deleted)"
    );

    println!(
        "{:<45} {}",
        "todo --file <path> <command>", "Use a custom RON file"
    );

    println!("\nIndexes are currently 0-based (first item = 0).");
}

// Make this public so tests (and main.rs) can use it.
pub fn parse_index_list(spec: &str) -> Vec<usize> {
    let mut result = Vec::new();

    for part in spec.split(',') {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }

        if let Some((start_s, end_s)) = p.split_once('-') {
            let start = match start_s.trim().parse::<usize>() {
                Ok(v) => v,
                Err(_) => continue,
            };
            let end = match end_s.trim().parse::<usize>() {
                Ok(v) => v,
                Err(_) => continue,
            };

            if start <= end {
                for i in start..=end {
                    result.push(i);
                }
            } else {
                for i in end..=start {
                    result.push(i);
                }
            }
        } else if let Ok(v) = p.parse::<usize>() {
            result.push(v);
        }
    }

    result
}

/// Public entry point that main.rs will call.
pub fn run_cli() -> Result<()> {
    let cli = Cli::parse();
    let path = cli.file.clone().unwrap_or_else(default_file_path);

    let now = Local::now();
    let mut list = TodoList::load(&path)?;

    // Auto-reset repeating items that are due
    list.auto_reset_repeating(now);

    match cli.command.unwrap_or(Commands::List) {
        Commands::List => {
            print_list(&list, now);
        }

        Commands::Add { text, repeat } => {
            list.add(text, repeat);
            list.save(&path)?;
            println!("Task added.");
        }
        Commands::Complete { indexes } => {
            let mut indices = parse_index_list(&indexes);

            if indices.is_empty() {
                eprintln!("No valid indexes supplied.");
                return Ok(());
            }

            // For completing, order doesn't matter, but we should dedupe.
            indices.sort_unstable();
            indices.dedup();

            let mut completed_any = false;

            for idx in indices {
                if let Some(item) = list.items.get_mut(idx) {
                    item.mark_complete(now);
                    println!("Marked complete [{}] {}", idx, item.text);
                    completed_any = true;
                } else {
                    eprintln!("No task with index {} — skipping.", idx);
                }
            }

            if completed_any {
                list.save(&path)?;
            }
        }

        Commands::Undo { index } => {
            if let Some(item) = list.items.get_mut(index) {
                item.mark_incomplete();
                list.save(&path)?;
                println!("Task {} marked incomplete.", index);
            } else {
                eprintln!("No task with index {}", index);
            }
        }

        Commands::Edit {
            index,
            text,
            repeat,
            clear_repeat,
        } => {
            if let Some(item) = list.items.get_mut(index) {
                if let Some(new_text) = text {
                    item.text = new_text;
                }

                if clear_repeat {
                    item.repeat_days = None;
                } else if let Some(new_repeat) = repeat {
                    item.repeat_days = Some(new_repeat);
                }

                list.save(&path)?;
                println!("Task {} updated.", index);
            } else {
                eprintln!("No task with index {}", index);
            }
        }

        Commands::Delete { indexes, confirm } => {
            let mut indices = parse_index_list(&indexes);

            if indices.is_empty() {
                eprintln!("No valid indexes supplied.");
                return Ok(());
            }

            indices.sort_unstable_by(|a, b| b.cmp(a));
            indices.dedup();

            if !confirm {
                println!(
                    "The following tasks would be deleted (run again with --confirm to proceed):\n"
                );

                for idx in &indices {
                    if *idx < list.items.len() {
                        println!("[{}] {}", idx, list.items[*idx].text);
                    } else {
                        println!("[{}] (does not exist)", idx);
                    }
                }

                println!("\nNothing deleted. Add --confirm to actually delete.");
                return Ok(());
            }

            for idx in &indices {
                if *idx < list.items.len() {
                    let removed = list.items.remove(*idx);
                    println!("Deleted [{}] {}", idx, removed.text);
                } else {
                    eprintln!("Index {} does not exist — skipping.", idx);
                }
            }

            list.save(&path)?;
        }

        Commands::Commands => {
            print_command_table();
        }
    }

    Ok(())
}
