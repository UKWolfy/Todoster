use chrono::{Duration, Local};

use todo_ron_cli::*; // <-- import from the crate root

#[test]
fn parse_index_list_handles_single_indexes() {
    let indices = parse_index_list("0,2,4");
    assert_eq!(indices, vec![0, 2, 4]);
}

#[test]
fn parse_index_list_trims_spaces_and_ignores_empty() {
    let indices = parse_index_list(" 0,  2 , , 4 ,");
    assert_eq!(indices, vec![0, 2, 4]);
}

#[test]
fn parse_index_list_handles_simple_range() {
    let indices = parse_index_list("1-3");
    assert_eq!(indices, vec![1, 2, 3]);
}

#[test]
fn parse_index_list_handles_mixed_ranges_and_indexes() {
    let indices = parse_index_list("0,2-4,7");
    assert_eq!(indices, vec![0, 2, 3, 4, 7]);
}

#[test]
fn parse_index_list_handles_reversed_range() {
    let indices = parse_index_list("5-3");
    assert_eq!(indices, vec![3, 4, 5]);
}

#[test]
fn repeating_task_resets_after_due_time() {
    let now = Local::now();
    let mut item = TodoItem::new("Feed gecko".into(), Some(2));

    item.mark_complete(now - Duration::days(3));
    item.reset_if_due(now);

    assert_eq!(item.complete, false);
    assert!(item.complete_date.is_none());
}

#[test]
fn non_repeating_task_does_not_reset() {
    let now = Local::now();
    let mut item = TodoItem::new("One-off task".into(), None);

    item.mark_complete(now - Duration::days(10));
    item.reset_if_due(now);

    assert_eq!(item.complete, true);
    assert!(item.complete_date.is_some());
}
