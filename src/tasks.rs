use chrono::{format::StrftimeItems, Days, Local, Months, NaiveDate};

const DONE_PREFIX: &str = "x ";
const PENDING_PREFIX: &str = "☐ ";
const DUE_KEY: &str = "due:";
const REC_KEY: &str = "rec:";
pub const DATE_FORMAT_STR: &str = "%Y-%m-%d";
pub const DATE_FORMAT_CONST: StrftimeItems<'_> = StrftimeItems::new(DATE_FORMAT_STR);

#[derive(Clone, Debug)]
pub struct Task {
    pub text: String,
    pub arr: Vec<TaskSection>,
    pub done: bool,
}

#[derive(Clone, Debug)]
pub enum TaskStringTag {
    Other,
    Context,
    Priority,
    Project,
    KV,
}

#[derive(Clone, Debug)]
pub struct TaskSection(pub TaskStringTag, pub String);

impl Task {
    pub fn new(text: &str) -> Self {
        let text = text.trim();
        let done = text.starts_with("x ");

        let text = if done || text.starts_with(PENDING_PREFIX) {
            text.to_string()
        } else {
            (PENDING_PREFIX.to_string() + text).to_string()
        };

        let arr = parse_task(&text);
        Self { done, text, arr }
    }

    pub fn toggle_done(&mut self) -> Option<String> {
        const PRIORITY_KEY: &str = "Pri:";
        if self.done {
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
            let date = if !completion_date.is_empty() && !start_date.is_empty() {
                start_date.to_string() + " "
            } else {
                "".to_string()
            };
            let rest = rest.trim();
            self.text = format!("{PENDING_PREFIX}{pri_new}{date}{rest}").replace(&pri_old, "");
            self.arr = parse_task(&self.text);
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
                let local = Local::now();
                let completion_date = local.format("%Y-%m-%d").to_string();
                if completion_date.is_empty() {
                    start_date.to_string() + " "
                } else {
                    completion_date + " " + start_date + " "
                }
            };
            let rest = rest.trim().trim_start();
            let due_date = try_rec(rest);

            self.text = format!("{DONE_PREFIX}{date}{rest}{priority}");
            self.arr = parse_task(&self.text);
            if let Some((old, new)) = due_date {
                let text = text.replace(&old, &new);
                return Some(text);
            }
        };
        None
    }
}

fn get_priority(input: &str) -> (&str, &str) {
    let input = input.trim_start();
    let word = input.get(..3).unwrap_or("");
    if word.starts_with("(") && word.ends_with(")") {
        if let Some(pri) = word.get(1..2) {
            if pri == pri.to_uppercase() {
                (input.get(3..).unwrap_or(""), pri)
            } else {
                (input, "")
            }
        } else {
            (input, "")
        }
    } else {
        (input, "")
    }
}

fn get_date(input: &str) -> (&str, &str) {
    let def = (input, "");
    match input.get(..10) {
        Some(date) => {
            let chars: Vec<char> = date.chars().collect();
            if chars[0].is_ascii_digit()
                && chars[1].is_ascii_digit()
                && chars[2].is_ascii_digit()
                && chars[3].is_ascii_digit()
                && chars[4] == '-'
                && chars[5].is_ascii_digit()
                && chars[6].is_ascii_digit()
                && chars[7] == '-'
                && chars[8].is_ascii_digit()
                && chars[9].is_ascii_digit()
            {
                (input.get(10..).unwrap_or(""), date)
            } else {
                def
            }
        }
        None => def,
    }
}

fn try_rec(input: &str) -> Option<(String, String)> {
    if let Some(rec) = input
        .split_whitespace()
        .filter(|e| e.starts_with(REC_KEY))
        .last()
    {
        if let Some(old_date_str) = input
            .split_whitespace()
            .filter(|e| e.starts_with(DUE_KEY))
            .last()
        {
            let old_date_str = old_date_str.strip_prefix(DUE_KEY).unwrap();
            let rec = rec.strip_prefix(REC_KEY).unwrap();
            if let Some((strict, num, duration)) = parse_rec(rec) {
                let old_date = if strict {
                    // strict means due date is calculated based on the last due date
                    match NaiveDate::parse_from_str(old_date_str, DATE_FORMAT_STR) {
                        Ok(t) => t,
                        Err(_) => return None,
                    }
                } else {
                    // else due date is based on the current date
                    let local = Local::now();
                    local.date_naive()
                };
                let new_date = match duration {
                    'w' => old_date.checked_add_days(Days::new(num * 7)),
                    'm' => {
                        let num = match u32::try_from(num) {
                            Ok(m) => m,
                            Err(_) => return None,
                        };
                        old_date.checked_add_months(Months::new(num))
                    }
                    'y' => {
                        let num = match u32::try_from(num) {
                            Ok(m) => m,
                            Err(_) => return None,
                        };
                        old_date.checked_add_months(Months::new(num * 12))
                    }
                    // => old_date.checked_add_days(Days::new(num)),
                    _ => old_date.checked_add_days(Days::new(num)),
                };
                let new_date = match new_date {
                    Some(d) => d,
                    None => return None,
                };

                let new_date_str = new_date.format_with_items(DATE_FORMAT_CONST).to_string();
                Some((old_date_str.to_owned(), new_date_str))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
}

fn parse_rec(input: &str) -> Option<(bool, u64, char)> {
    let (input, strict) = {
        if input.starts_with("+") {
            (input.get(1..).unwrap(), true)
        } else {
            (input, false)
        }
    };

    let (input, duration) = {
        let last = input.chars().last().unwrap_or('d');
        if last.is_ascii_digit() {
            (input, 'd')
        } else {
            (input.get(..input.len() - 1).unwrap(), last)
        }
    };

    let num = match input.parse() {
        Ok(t) => t,
        Err(_) => return None,
    };

    Some((strict, num, duration))
}

fn parse_task(text: &str) -> Vec<TaskSection> {
    let prefix = if text.starts_with(PENDING_PREFIX) {
        PENDING_PREFIX
    } else {
        DONE_PREFIX
    };
    let mut arr: Vec<TaskSection> = vec![TaskSection(TaskStringTag::Other, prefix.to_string())];
    let text = text.strip_prefix(prefix).unwrap();
    let (rest, pri) = get_priority(text);
    if !pri.is_empty() {
        arr.push(TaskSection(TaskStringTag::Priority, format!("({pri})")));
    };
    let mut current_state = String::new();

    for word in rest.split_ascii_whitespace() {
        if word.starts_with("@") {
            if !current_state.is_empty() {
                arr.push(TaskSection(TaskStringTag::Other, current_state));
                current_state = String::new();
            }
            arr.push(TaskSection(TaskStringTag::Context, " ".to_string() + word));
        } else if word.starts_with("+") {
            if !current_state.is_empty() {
                arr.push(TaskSection(TaskStringTag::Other, current_state));
                current_state = String::new();
            }
            arr.push(TaskSection(TaskStringTag::Project, " ".to_string() + word));
        } else if word.contains(":") {
            if !current_state.is_empty() {
                arr.push(TaskSection(TaskStringTag::Other, current_state));
                current_state = String::new();
            }
            arr.push(TaskSection(TaskStringTag::KV, " ".to_string() + word));
        } else {
            current_state.push_str(" ");
            current_state.push_str(word);
        }
    }

    if !current_state.is_empty() {
        arr.push(TaskSection(TaskStringTag::Other, current_state));
    }
    arr
}

#[cfg(test)]
mod test {

    use crate::tasks::{Task, PENDING_PREFIX};
    use chrono::{Days, Local, Months, NaiveDate};

    use super::DATE_FORMAT_CONST;

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

        let local = Local::now();
        let date = local.format_with_items(DATE_FORMAT_CONST).to_string();

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

    #[test]
    fn tasks_recurring() {
        let today = Local::now().date_naive();
        let today_str = today.format_with_items(DATE_FORMAT_CONST).to_string();
        let due = today
            .checked_add_days(Days::new(2))
            .unwrap()
            .format_with_items(DATE_FORMAT_CONST)
            .to_string();

        let list: Vec<(String, String)> = vec![
            format!("recurrent task rec:10 due:{due}"),
            format!("strict recurrent task rec:+10 due:{due}"),
            format!("recurrent task with days rec:10d due:{due}"),
            format!("strict recurrent task with days rec:+10d due:{due}"),
            format!("recurrent task with months rec:2m due:{due}"),
            format!("strict recurrent task with months rec:+2m due:{due}"),
            format!("recurrent task with years rec:2y due:{due}"),
            format!("strict recurrent task with years rec:+2y due:{due}"),
            format!("2024-08-27 recurrent task with start dates rec:10 due:{due}"),
            format!("2024-08-27 strict recurrent task with start dates rec:+10 due:{due}"),
        ]
        .iter()
        .map(|t| Task::new(&t))
        .map(|mut t1| {
            let t2 = t1.toggle_done().unwrap();
            (t1.text.to_string(), t2)
        })
        .collect();

        fn get_date_string(date: Option<NaiveDate>) -> String {
            date.unwrap()
                .format_with_items(DATE_FORMAT_CONST)
                .to_string()
        }

        let due_days = get_date_string(today.checked_add_days(Days::new(10)));
        let due_days_strict = get_date_string(today.checked_add_days(Days::new(12)));
        let due_m = get_date_string(today.checked_add_months(Months::new(2)));
        let due_strict_m = get_date_string(
            today
                .checked_add_days(Days::new(2))
                .unwrap()
                .checked_add_months(Months::new(2)),
        );
        let due_y = get_date_string(today.checked_add_months(Months::new(12 * 2)));
        let due_strict_y = get_date_string(
            today
                .checked_add_days(Days::new(2))
                .unwrap()
                .checked_add_months(Months::new(12 * 2)),
        );

        // the newly created task from that
        let expected: Vec<(String, String)> = vec![
            (
                format!("x recurrent task rec:10 due:{due}"),
                format!("{PENDING_PREFIX}recurrent task rec:10 due:{due_days}"),
            ),
            (
                format!("x strict recurrent task rec:+10 due:{due}"),
                format!("{PENDING_PREFIX}strict recurrent task rec:+10 due:{due_days_strict}"),
            ),
            (
                format!("x recurrent task with days rec:10d due:{due}"),
                format!("{PENDING_PREFIX}recurrent task with days rec:10d due:{due_days}"),
            ),
            (
                format!("x strict recurrent task with days rec:+10d due:{due}"),
                format!(
                    "{PENDING_PREFIX}strict recurrent task with days rec:+10d due:{due_days_strict}"
                ),
            ),
            (
                format!("x recurrent task with months rec:2m due:{due}"),
                format!("{PENDING_PREFIX}recurrent task with months rec:2m due:{due_m}"),
            ),
            (
                format!("x strict recurrent task with months rec:+2m due:{due}"),
                format!(
                    "{PENDING_PREFIX}strict recurrent task with months rec:+2m due:{due_strict_m}"
                ),
            ),
            (
                format!("x recurrent task with years rec:2y due:{due}"),
                format!("{PENDING_PREFIX}recurrent task with years rec:2y due:{due_y}"),
            ),
            (
                format!("x strict recurrent task with years rec:+2y due:{due}"),
                format!(
                    "{PENDING_PREFIX}strict recurrent task with years rec:+2y due:{due_strict_y}"
                ),
            ),
            (
                format!("x {today_str} 2024-08-27 recurrent task with start dates rec:10 due:{due}"),
                format!("{PENDING_PREFIX}2024-08-27 recurrent task with start dates rec:10 due:{due_days}"),
            ),
            (
                format!("x {today_str} 2024-08-27 strict recurrent task with start dates rec:+10 due:{due}"),
                format!("{PENDING_PREFIX}2024-08-27 strict recurrent task with start dates rec:+10 due:{due_days_strict}"),
            ),
        ];

        list.iter()
            .zip(expected)
            .for_each(|e| assert_eq!(*e.0, e.1));
    }
}
