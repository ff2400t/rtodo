use time::{format_description::BorrowedFormatItem, macros::format_description, OffsetDateTime};

const DONE_PREFIX: &str = "x ";
const PENDING_PREFIX: &str = "☐ ";
const DATE_FORMAT_STR: &[BorrowedFormatItem] = format_description!("[year]-[month]-[day]");

#[derive(Clone, Debug)]
pub struct Task {
    pub text: String,
    pub done: bool,
}

impl Task {
    pub fn new(text: &str) -> Self {
        let text = text.trim();
        let done = text.starts_with("x ");
        let text = if done {
            text.to_string()
        } else {
            (PENDING_PREFIX.to_string() + text).to_string()
        };
        Self { done, text }
    }

    pub fn toggle_done(&mut self) {
        const PRIORITY_KEY: &str = "Pri:";
        self.text = if self.done {
            self.done = false;
            let text = self.text.clone();
            let priority_kv = text
                .split_whitespace()
                .skip(1)
                .find(|word| word.starts_with(PRIORITY_KEY));
            let (pri_old, pri_new) = match priority_kv {
                Some(kv) => {
                    let pri = kv.strip_prefix(PRIORITY_KEY).unwrap();
                    let pri_new = format!("({}) ", pri);
                    (" ".to_string() + kv, pri_new)
                }
                None => ("".to_string(), "".to_string()),
            };
            let rest = text.get(2..).unwrap();
            let (rest, completion_date) = get_date(rest.trim_start());
            let (rest, start_date) = get_date(rest.trim_start());
            let date = if completion_date != "" && start_date != "" {
                start_date.to_string() + " "
            } else {
                "".to_string()
            };
            let rest = rest.trim();

            format!("{PENDING_PREFIX}{pri_new}{date}{rest}").replace(&pri_old, "")
        } else {
            self.done = true;
            let text = self.text.clone();
            let (_, rest) = text.split_once(PENDING_PREFIX).unwrap();
            let (rest, priority) = get_priority(rest.trim_start());
            let priority = if priority.is_empty() {
                "".to_string()
            } else {
                " ".to_string() + PRIORITY_KEY + priority
            };
            let (rest, start_date) = get_date(rest.trim_start());
            let date = if start_date.is_empty() {
                start_date.to_string()
            } else {
                let local = OffsetDateTime::now_utc();
                let date = local.format(&DATE_FORMAT_STR).unwrap_or("".to_string());
                if date.is_empty() {
                    start_date.to_string()
                } else {
                    date + " " + start_date + " "
                }
            };
            let rest = rest.trim().trim_start();

            format!("{DONE_PREFIX}{date}{rest}{priority}")
        }
    }
}

fn get_priority(input: &str) -> (&str, &str) {
    let input = input.trim_start();
    let word = input.get(..3).unwrap_or("");
    if word.starts_with("(") && word.ends_with(")") {
        if let Some(pri) = word.get(1..2).clone() {
            if pri == pri.to_uppercase() {
                return (input.get(3..).unwrap_or(""), &pri);
            } else {
                return (input, "");
            }
        } else {
            return (input, "");
        }
    } else {
        return (input, "");
    }
}

fn get_date(input: &str) -> (&str, &str) {
    let def = (input, "");
    match input.get(..10) {
        Some(date) => {
            let chars: Vec<char> = date.chars().collect();
            if chars[0].is_digit(10)
                && chars[1].is_digit(10)
                && chars[2].is_digit(10)
                && chars[3].is_digit(10)
                && chars[4] == '-'
                && chars[5].is_digit(10)
                && chars[6].is_digit(10)
                && chars[7] == '-'
                && chars[8].is_digit(10)
                && chars[9].is_digit(10)
            {
                return (input.get(10..).unwrap_or(""), date);
            } else {
                return def;
            }
        }
        None => {
            return def;
        }
    }
}

#[cfg(test)]
mod test {
    use time::OffsetDateTime;

    use crate::tasks::{Task, PENDING_PREFIX};

    use super::DATE_FORMAT_STR;

    #[test]
    fn simple_tasks() {
        let list: Vec<String> = vec![
            "task",
            "x done",
            "(A) task with priority",
            "x done task with priority Pri:A",
        ]
        .iter()
        .map(|t| Task::new(t))
        .map(|mut t| {
            t.toggle_done();
            t.text
                .strip_prefix(PENDING_PREFIX)
                .unwrap_or(&t.text)
                .to_string()
        })
        .collect();

        let expected: Vec<String> = vec![
            "x task",
            "done",
            "x task with priority Pri:A",
            "(A) done task with priority",
        ]
        .iter_mut()
        .map(|m| m.to_string())
        .collect();

        list.iter()
            .zip(expected)
            .for_each(|e| assert_eq!(*e.0, e.1));
    }

    #[test]
    fn tasks_with_date() {
        let list: Vec<String> = vec![
            "  2024-08-14   task with start date",
            "x  2024-08-15  2024-08-14  done task with start date",
            "(A)   2024-08-14   task with priority and start date",
            "  x   2024-08-14   2024-08-14 task with priority and start date Pri:A",
        ]
        .iter()
        .map(|t| Task::new(t))
        .map(|mut t| {
            t.toggle_done();
            t.text
                .strip_prefix(PENDING_PREFIX)
                .unwrap_or(&t.text)
                .to_string()
        })
        .collect();

        let local = OffsetDateTime::now_utc();
        let date = local.format(&DATE_FORMAT_STR).unwrap_or("".to_string());

        let expected: Vec<String> = vec![
            &format!("x {date} 2024-08-14 task with start date"),
            "2024-08-14 done task with start date",
            &format!("x {date} 2024-08-14 task with priority and start date Pri:A"),
            "(A) 2024-08-14 task with priority and start date",
        ]
        .iter_mut()
        .map(|m| m.to_string())
        .collect();
        list.iter()
            .zip(expected)
            .for_each(|e| assert_eq!(*e.0, e.1));
    }
}
