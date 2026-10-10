//! Azure DevOps ticket lookup for "#1234" in line descriptions, through the `az` CLI.
//!
//! Lookup is on only when `az` is installed and logged in to an account with an Azure DevOps
//! organization; otherwise every search is [`Search::Off`]. Calls run on a worker thread, so the UI
//! never waits for the network.

use std::collections::{HashMap, HashSet};
use std::io::Write as _;
use std::ops::Range;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use serde::Deserialize;

/// Most tickets listed for one search.
const MAX_RESULTS: usize = 8;
/// Digits of the largest ticket number searched.
const MAX_DIGITS: usize = 7;
/// Wait after a keystroke before searching, so fast typing sends one search.
const DEBOUNCE: Duration = Duration::from_millis(250);
/// Application ID of Azure DevOps in Microsoft Entra ID, for `az rest --resource`. Public and the
/// same for every tenant: it names the service the token is for, not an account.
const DEVOPS_RESOURCE: &str = "499b84ac-1321-427f-aa17-267ca6975798";

#[derive(Clone, Debug, PartialEq)]
pub struct Ticket {
    pub id: u32,
    pub title: String,
}

impl Ticket {
    /// Description text for the ticket: "#42107 : Invoice PDF layout".
    pub fn label(&self) -> String {
        format!("#{} : {}", self.id, self.title)
    }
}

/// Outcome of [`Tickets::search`].
#[derive(Debug, PartialEq)]
pub enum Search<'a> {
    /// Lookup is unavailable (no `az`, not logged in, or still checking).
    Off,
    Loading,
    Found(&'a [Ticket]),
}

enum Reply {
    Ready,
    Found(String, Vec<Ticket>),
    /// A newer search replaced this one before it ran.
    Dropped(String),
}

enum Source {
    Off,
    Az {
        queries: Sender<String>,
        replies: Receiver<Reply>,
    },
    #[cfg(test)]
    Fixed(Vec<Ticket>),
}

/// Ticket search results, cached for the session.
pub struct Tickets {
    source: Source,
    ready: bool,
    found: HashMap<String, Vec<Ticket>>,
    pending: HashSet<String>,
}

impl Tickets {
    /// Never finds anything, as without `az`.
    #[cfg(test)]
    pub fn off() -> Self {
        Self::new(Source::Off, false)
    }

    /// Checks `az` in the background, then serves searches. `wake` runs when a result arrives.
    pub fn start(wake: impl Fn() + Send + 'static) -> Self {
        let (queries, worker_queries) = mpsc::channel();
        let (worker_replies, replies) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("tickets".into())
            .spawn(move || worker(&worker_queries, &worker_replies, wake));
        let source = match spawned {
            Ok(_) => Source::Az { queries, replies },
            Err(_) => Source::Off,
        };
        Self::new(source, false)
    }

    /// Finds tickets in `tickets` at once, as if `az` were logged in.
    #[cfg(test)]
    pub fn fixed(tickets: Vec<Ticket>) -> Self {
        Self::new(Source::Fixed(tickets), true)
    }

    fn new(source: Source, ready: bool) -> Self {
        Self {
            source,
            ready,
            found: HashMap::new(),
            pending: HashSet::new(),
        }
    }

    /// Tickets whose number starts with `digits`, most recently changed first, an exact match on top.
    pub fn search(&mut self, digits: &str) -> Search<'_> {
        if let Source::Az { replies, .. } = &self.source {
            for reply in replies.try_iter() {
                match reply {
                    Reply::Ready => self.ready = true,
                    Reply::Found(q, tickets) => {
                        self.pending.remove(&q);
                        self.found.insert(q, tickets);
                    }
                    Reply::Dropped(q) => {
                        self.pending.remove(&q);
                    }
                }
            }
        }
        if !self.ready {
            return Search::Off;
        }
        if !self.found.contains_key(digits) {
            match &self.source {
                Source::Off => return Search::Off,
                Source::Az { queries, .. } => {
                    if self.pending.insert(digits.to_owned())
                        && queries.send(digits.into()).is_err()
                    {
                        // The worker stopped: lookup is off for the rest of the session.
                        self.ready = false;
                        return Search::Off;
                    }
                    return Search::Loading;
                }
                #[cfg(test)]
                Source::Fixed(all) => {
                    let mut hits: Vec<Ticket> = all
                        .iter()
                        .filter(|t| t.id.to_string().starts_with(digits))
                        .cloned()
                        .collect();
                    exact_first(&mut hits, digits);
                    self.found.insert(digits.to_owned(), hits);
                }
            }
        }
        Search::Found(&self.found[digits])
    }
}

/// The ticket number being typed before `cursor` (a char index): `#` then 1 to 7 digits, at the start
/// of the text or after a non-alphanumeric character, with no digit right after the cursor. Returns
/// the char range of `#…` and its digits.
pub fn query_at(text: &str, cursor: usize) -> Option<(Range<usize>, String)> {
    let chars: Vec<char> = text.chars().collect();
    if cursor > chars.len() || chars.get(cursor).is_some_and(char::is_ascii_digit) {
        return None;
    }
    let digits_start = chars[..cursor]
        .iter()
        .rposition(|c| !c.is_ascii_digit())
        .map_or(0, |i| i + 1);
    let digits: String = chars[digits_start..cursor].iter().collect();
    let hash = digits_start.checked_sub(1)?;
    let valid = chars[hash] == '#'
        && (1..=MAX_DIGITS).contains(&digits.len())
        && !digits.starts_with('0')
        && (hash == 0 || !chars[hash - 1].is_alphanumeric());
    valid.then_some((hash..cursor, digits))
}

/// `text` with the chars in `range` replaced by the ticket's label, and the char index after it.
pub fn complete(text: &str, range: Range<usize>, ticket: &Ticket) -> (String, usize) {
    let label = ticket.label();
    let mut out: String = text.chars().take(range.start).collect();
    out.push_str(&label);
    let cursor = out.chars().count();
    out.extend(text.chars().skip(range.end));
    (out, cursor)
}

/// Moves the ticket numbered exactly `digits` to the top, keeping the others in order.
fn exact_first(tickets: &mut [Ticket], digits: &str) {
    tickets.sort_by_key(|t| t.id.to_string() != digits);
}

/// WIQL query for tickets whose number starts with `digits`: the number itself, then each range of
/// longer numbers with that prefix (4210, 42100–42109, 421000–421099, …).
fn wiql(digits: &str) -> String {
    let n: u64 = digits.parse().unwrap_or(0);
    let mut clauses = vec![format!("[System.Id] = {n}")];
    let (mut lo, mut hi) = (n, n);
    for _ in digits.len()..MAX_DIGITS {
        lo *= 10;
        hi = hi * 10 + 9;
        clauses.push(format!("([System.Id] >= {lo} AND [System.Id] <= {hi})"));
    }
    format!(
        "SELECT [System.Id] FROM WorkItems WHERE {} ORDER BY [System.ChangedDate] DESC",
        clauses.join(" OR ")
    )
}

fn worker(queries: &Receiver<String>, replies: &Sender<Reply>, wake: impl Fn()) {
    // Returning drops `replies`, which leaves lookup off.
    let Some(az) = Az::find() else { return };
    let Some(org) = az.organization() else { return };
    if replies.send(Reply::Ready).is_err() {
        return;
    }
    wake();
    while let Ok(mut digits) = queries.recv() {
        std::thread::sleep(DEBOUNCE);
        for newer in queries.try_iter() {
            let _ = replies.send(Reply::Dropped(std::mem::replace(&mut digits, newer)));
        }
        // A failed search lists nothing, like a search without matches.
        let found = az.search(&org, &digits).unwrap_or_default();
        if replies.send(Reply::Found(digits, found)).is_err() {
            return;
        }
        wake();
    }
}

/// The `az` executable.
struct Az(PathBuf);

impl Az {
    fn find() -> Option<Self> {
        let name = if cfg!(windows) { "az.cmd" } else { "az" };
        // An app opened from Finder does not get the shell's PATH, so also try the default
        // install folders.
        let defaults = if cfg!(windows) {
            [
                r"C:\Program Files\Microsoft SDKs\Azure\CLI2\wbin",
                r"C:\Program Files (x86)\Microsoft SDKs\Azure\CLI2\wbin",
            ]
        } else {
            ["/opt/homebrew/bin", "/usr/local/bin"]
        };
        let path = std::env::var_os("PATH").unwrap_or_default();
        std::env::split_paths(&path)
            .chain(defaults.map(PathBuf::from))
            .map(|dir| dir.join(name))
            .find(|p| p.is_file())
            .map(Self)
    }

    /// Name of the first Azure DevOps organization of the logged-in user.
    fn organization(&self) -> Option<String> {
        #[derive(Deserialize)]
        struct Profile {
            id: String,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Account {
            account_name: String,
        }
        #[derive(Deserialize)]
        struct Accounts {
            value: Vec<Account>,
        }
        let me: Profile = self.rest(
            "https://app.vssps.visualstudio.com/_apis/profile/profiles/me?api-version=7.1",
            None,
        )?;
        let accounts: Accounts = self.rest(
            &format!(
                "https://app.vssps.visualstudio.com/_apis/accounts?memberId={}&api-version=7.1",
                me.id
            ),
            None,
        )?;
        accounts.value.into_iter().next().map(|a| a.account_name)
    }

    fn search(&self, org: &str, digits: &str) -> Option<Vec<Ticket>> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Wiql {
            work_items: Vec<Ref>,
        }
        #[derive(Deserialize)]
        struct Ref {
            id: u32,
        }
        #[derive(Deserialize)]
        struct Items {
            value: Vec<Item>,
        }
        #[derive(Deserialize)]
        struct Item {
            id: u32,
            fields: Fields,
        }
        #[derive(Deserialize)]
        struct Fields {
            #[serde(rename = "System.Title")]
            title: String,
        }
        let body = serde_json::json!({ "query": wiql(digits) }).to_string();
        let refs: Wiql = self.rest(
            &format!(
                "https://dev.azure.com/{org}/_apis/wit/wiql?$top={MAX_RESULTS}&api-version=7.1"
            ),
            Some(&body),
        )?;
        if refs.work_items.is_empty() {
            return Some(Vec::new());
        }
        let ids: Vec<String> = refs.work_items.iter().map(|r| r.id.to_string()).collect();
        let items: Items = self.rest(
            &format!(
                "https://dev.azure.com/{org}/_apis/wit/workitems?ids={}&fields=System.Title&api-version=7.1",
                ids.join(",")
            ),
            None,
        )?;
        // Keep the query's order (most recently changed first).
        let mut tickets: Vec<Ticket> = refs
            .work_items
            .iter()
            .filter_map(|r| items.value.iter().find(|i| i.id == r.id))
            .map(|i| Ticket {
                id: i.id,
                title: i.fields.title.clone(),
            })
            .collect();
        exact_first(&mut tickets, digits);
        Some(tickets)
    }

    /// Runs `az rest` against Azure DevOps: a POST of `body` when given, else a GET. `None` when `az`
    /// fails (not logged in, no network) or the answer does not parse.
    fn rest<T: serde::de::DeserializeOwned>(&self, uri: &str, body: Option<&str>) -> Option<T> {
        let mut cmd = Command::new(&self.0);
        cmd.args([
            "rest",
            "--only-show-errors",
            "--resource",
            DEVOPS_RESOURCE,
            "--uri",
            uri,
        ])
        .stdin(if body.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
        if body.is_some() {
            // The body goes through stdin: Windows runs az.cmd through cmd.exe, which would
            // misread the quotes and `<`, `>` of a JSON argument.
            cmd.args(["--method", "post", "--body", "@-"]);
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt as _;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let mut child = cmd.spawn().ok()?;
        if let Some(body) = body {
            child.stdin.take()?.write_all(body.as_bytes()).ok()?;
        }
        let out = child.wait_with_output().ok()?;
        if !out.status.success() {
            return None;
        }
        serde_json::from_slice(&out.stdout).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticket(id: u32) -> Ticket {
        Ticket {
            id,
            title: format!("Ticket {id}"),
        }
    }

    #[test]
    fn finds_the_number_typed_before_the_cursor() {
        let q = query_at;
        assert_eq!(q("#508", 4), Some((0..4, "508".into())));
        assert_eq!(q("Fix #42107 now", 10), Some((4..10, "42107".into())));
        assert_eq!(q("(#5", 3), Some((1..3, "5".into())));
        assert_eq!(q("Réunion #12", 11), Some((8..11, "12".into())));
        // Not a ticket number.
        assert_eq!(q("#", 1), None);
        assert_eq!(q("#05", 3), None);
        assert_eq!(q("a#12", 4), None);
        assert_eq!(q("12", 2), None);
        assert_eq!(q("#12345678", 9), None);
        // Cursor elsewhere than at the end of the number.
        assert_eq!(q("#508", 2), None);
        assert_eq!(q("#508 x", 6), None);
        assert_eq!(q("#508", 9), None);
    }

    #[test]
    fn completion_replaces_the_number_with_the_label() {
        let t = Ticket {
            id: 42107,
            title: "Invoice PDF layout".into(),
        };
        assert_eq!(
            complete("#508", 0..4, &t),
            ("#42107 : Invoice PDF layout".into(), 27)
        );
        let (text, cursor) = complete("Réunion #5 puis", 8..10, &t);
        assert_eq!(text, "Réunion #42107 : Invoice PDF layout puis");
        assert_eq!(cursor, 35);
    }

    #[test]
    fn wiql_covers_every_number_with_the_prefix() {
        assert_eq!(
            wiql("42107"),
            "SELECT [System.Id] FROM WorkItems WHERE [System.Id] = 42107 \
             OR ([System.Id] >= 421070 AND [System.Id] <= 421079) \
             OR ([System.Id] >= 4210700 AND [System.Id] <= 4210799) \
             ORDER BY [System.ChangedDate] DESC"
        );
        assert!(!wiql("1234567").contains(">="));
    }

    #[test]
    fn exact_match_comes_first() {
        let mut found = vec![ticket(42108), ticket(4210), ticket(42107)];
        exact_first(&mut found, "4210");
        assert_eq!(found, [ticket(4210), ticket(42108), ticket(42107)]);
    }

    #[test]
    fn off_never_searches() {
        assert_eq!(Tickets::off().search("12"), Search::Off);
    }

    #[test]
    fn fixed_finds_by_prefix() {
        let mut tickets = Tickets::fixed(vec![ticket(42107), ticket(4210), ticket(4)]);
        assert_eq!(
            tickets.search("4210"),
            Search::Found(&[ticket(4210), ticket(42107)])
        );
        assert_eq!(tickets.search("9"), Search::Found(&[]));
    }
}
