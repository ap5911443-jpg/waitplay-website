//! Local coding / waiting-time insights. Nothing leaves the machine and nothing but timings is stored: no prompts, no terminal
//! contents, no paths. The recorder watches the same session list the island shows; finished work goes to an append-only
//! `insights.jsonl` in the app-support folder, and [`summarize`] turns those lines into the numbers the Insights page draws.
//!
//! Pure on purpose (time and observations are passed in), so every rule is unit tested.

use crate::paths;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

/// Longest gap between two observations that counts as elapsed time. A longer one means the app or the Mac was asleep.
const MAX_GAP_MS: i64 = 30_000;
/// An open session that shows no activity for this long is closed as abandoned (a crashed agent, a closed terminal).
const ABANDON_AFTER_MS: i64 = 60_000;
/// Longest a single record may claim; a guard against corrupt clocks.
const MAX_SPAN_MS: i64 = 48 * 3_600_000;
/// The file is rewritten without records older than this when it grows large.
const KEEP_DAYS: i64 = 400;
const COMPACT_ABOVE_BYTES: u64 = 2_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Working,
    Waiting,
    Completed,
    Failed,
    /// Not active and not finished: the bridge knows the session but it is quiet (or it vanished).
    Idle,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Obs {
    pub id: String,
    /// Agent id (`claude-code`, `codex`, ...).
    pub agent: String,
    pub phase: Phase,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Completed,
    Failed,
    Abandoned,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "k")]
pub enum Record {
    /// One task of one session, from the moment it was first seen active to the moment it ended.
    #[serde(rename = "s")]
    Session { id: String, agent: String, start: i64, end: i64, work: i64, wait: i64, ent: i64, outcome: Outcome },
    /// One stretch with the entertainment window open. `auto`: an agent was already active when it opened.
    #[serde(rename = "e")]
    Entertainment { site: String, start: i64, end: i64, auto: bool },
}

struct Open {
    agent: String,
    start: i64,
    phase: Phase,
    work: i64,
    wait: i64,
    ent: i64,
    last_active: i64,
}

struct EntSpan {
    site: String,
    start: i64,
    auto: bool,
}

#[derive(Default)]
pub struct Recorder {
    open: HashMap<String, Open>,
    ent: Option<EntSpan>,
    last: Option<i64>,
}

impl Recorder {
    pub fn new() -> Recorder {
        Recorder::default()
    }

    /// Feeds one observation of every known session and of the entertainment window (`Some(site)` while it is open). Returns the
    /// records that finished with it.
    pub fn observe(&mut self, now: i64, obs: &[Obs], entertainment: Option<&str>) -> Vec<Record> {
        let mut done = Vec::new();
        let dt = self.last.map_or(0, |l| (now - l).clamp(0, MAX_GAP_MS));
        self.last = Some(now);
        let ent_open = self.ent.is_some();

        // time since the previous observation belongs to the phase the session was in then
        for o in self.open.values_mut() {
            match o.phase {
                Phase::Working => o.work += dt,
                Phase::Waiting => o.wait += dt,
                _ => {}
            }
            if ent_open && matches!(o.phase, Phase::Working | Phase::Waiting) {
                o.ent += dt;
            }
            if matches!(o.phase, Phase::Working | Phase::Waiting) {
                o.last_active = now;
            }
        }

        let mut seen: Vec<&str> = Vec::new();
        for o in obs {
            seen.push(&o.id);
            match o.phase {
                Phase::Working | Phase::Waiting => {
                    let e = self.open.entry(o.id.clone()).or_insert(Open { agent: o.agent.clone(), start: now, phase: o.phase, work: 0, wait: 0, ent: 0, last_active: now });
                    e.phase = o.phase;
                    e.last_active = now;
                }
                Phase::Completed | Phase::Failed => {
                    if let Some(s) = self.open.remove(&o.id) {
                        let outcome = if o.phase == Phase::Completed { Outcome::Completed } else { Outcome::Failed };
                        done.push(finish(&o.id, s, now, outcome));
                    }
                }
                Phase::Idle => {
                    if let Some(e) = self.open.get_mut(&o.id) {
                        e.phase = Phase::Idle;
                    }
                }
            }
        }
        // a session the bridge no longer lists is quiet; quiet ones are closed once they have been inactive long enough
        for (id, s) in self.open.iter_mut() {
            if !seen.contains(&id.as_str()) {
                s.phase = Phase::Idle;
            }
        }
        let stale: Vec<String> = self
            .open
            .iter()
            .filter(|(_, s)| !matches!(s.phase, Phase::Working | Phase::Waiting) && now - s.last_active >= ABANDON_AFTER_MS)
            .map(|(id, _)| id.clone())
            .collect();
        for id in stale {
            if let Some(s) = self.open.remove(&id) {
                let end = s.last_active;
                done.push(finish(&id, s, end, Outcome::Abandoned));
            }
        }

        // the entertainment window
        let any_active = self.open.values().any(|s| matches!(s.phase, Phase::Working | Phase::Waiting));
        match (self.ent.take(), entertainment) {
            (Some(e), Some(site)) if e.site == site => self.ent = Some(e),
            (prev, now_site) => {
                if let Some(e) = prev {
                    done.push(Record::Entertainment { site: e.site, start: e.start, end: now, auto: e.auto });
                }
                if let Some(site) = now_site {
                    self.ent = Some(EntSpan { site: site.to_string(), start: now, auto: any_active });
                }
            }
        }
        done
    }

    /// Closes everything that is open (the app is quitting). Sessions still running are recorded as abandoned.
    pub fn flush(&mut self, now: i64) -> Vec<Record> {
        let mut out = Vec::new();
        for (id, s) in self.open.drain() {
            let end = s.last_active.max(s.start);
            out.push(finish(&id, s, end, Outcome::Abandoned));
        }
        if let Some(e) = self.ent.take() {
            out.push(Record::Entertainment { site: e.site, start: e.start, end: now, auto: e.auto });
        }
        out
    }
}

fn finish(id: &str, s: Open, end: i64, outcome: Outcome) -> Record {
    let span = (end - s.start).clamp(0, MAX_SPAN_MS);
    Record::Session { id: id.to_string(), agent: s.agent, start: s.start, end: s.start + span, work: s.work.min(span), wait: s.wait.min(span), ent: s.ent.min(span), outcome }
}

// ── storage ─────────────────────────────────────────────────────────────────────────────────────────────────────────────

pub fn path() -> PathBuf {
    paths::app_support_dir().join("insights.jsonl")
}

pub fn append(records: &[Record]) {
    append_to(&path(), records)
}

pub fn append_to(p: &std::path::Path, records: &[Record]) {
    if records.is_empty() {
        return;
    }
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) else { return };
    let mut text = String::new();
    for r in records {
        if let Ok(line) = serde_json::to_string(r) {
            text.push_str(&line);
            text.push('\n');
        }
    }
    let _ = f.write_all(text.as_bytes());
}

/// Every readable record; a damaged line is skipped, never fatal.
pub fn load() -> Vec<Record> {
    load_from(&path())
}

pub fn load_from(p: &std::path::Path) -> Vec<Record> {
    let Ok(text) = std::fs::read_to_string(p) else { return Vec::new() };
    text.lines().filter_map(|l| serde_json::from_str::<Record>(l).ok()).collect()
}

/// Drops records older than [`KEEP_DAYS`] when the file has grown large. Called once at start-up.
pub fn compact(now: i64) {
    compact_at(&path(), now)
}

pub fn compact_at(p: &std::path::Path, now: i64) {
    if std::fs::metadata(p).map_or(true, |m| m.len() < COMPACT_ABOVE_BYTES) {
        return;
    }
    let cutoff = now - KEEP_DAYS * 86_400_000;
    let keep: Vec<Record> = load_from(p).into_iter().filter(|r| record_end(r) >= cutoff).collect();
    let mut text = String::new();
    for r in &keep {
        if let Ok(line) = serde_json::to_string(r) {
            text.push_str(&line);
            text.push('\n');
        }
    }
    let _ = paths::write_atomic(p, text.as_bytes());
}

fn record_end(r: &Record) -> i64 {
    match r {
        Record::Session { end, .. } | Record::Entertainment { end, .. } => *end,
    }
}

// ── summary ─────────────────────────────────────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Range {
    Today,
    Days7,
    Days30,
    All,
}

impl Range {
    pub fn parse(s: &str) -> Option<Range> {
        match s {
            "today" => Some(Range::Today),
            "7d" => Some(Range::Days7),
            "30d" => Some(Range::Days30),
            "all" => Some(Range::All),
            _ => None,
        }
    }
    /// First local day included, as a day number (see [`day_of`]); `None` for all time.
    fn first_day(self, today: i64) -> Option<i64> {
        match self {
            Range::Today => Some(today),
            Range::Days7 => Some(today - 6),
            Range::Days30 => Some(today - 29),
            Range::All => None,
        }
    }
}

const DAY_MS: i64 = 86_400_000;

/// Local day number (days since 1970-01-01) of `ms`, for a zone `tz_min` minutes east of UTC.
pub fn day_of(ms: i64, tz_min: i32) -> i64 {
    (ms + tz_min as i64 * 60_000).div_euclid(DAY_MS)
}

/// `YYYY-MM-DD` of a day number (Howard Hinnant's civil-from-days).
pub fn day_label(day: i64) -> String {
    let z = day + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{:04}-{:02}-{:02}", if m <= 2 { y + 1 } else { y }, m, d)
}

/// The numbers behind the Insights page for `range`, days cut at the user's local midnight.
pub fn summarize(records: &[Record], now: i64, range: Range, tz_min: i32) -> Value {
    let today = day_of(now, tz_min);
    let first = range.first_day(today);
    let in_range = |t: i64| first.map_or(true, |f| day_of(t, tz_min) >= f);

    let (mut sessions, mut completed, mut failed, mut abandoned) = (0u64, 0u64, 0u64, 0u64);
    let (mut work, mut wait, mut total, mut longest, mut with_ent) = (0i64, 0i64, 0i64, 0i64, 0i64);
    let mut agents: HashMap<String, (u64, i64)> = HashMap::new();
    let mut days: HashMap<i64, (u64, i64, i64)> = HashMap::new();
    let (mut ent_total, mut ent_count, mut ent_auto) = (0i64, 0u64, 0u64);
    let mut sites: HashMap<String, (u64, i64)> = HashMap::new();

    for r in records {
        match r {
            Record::Session { agent, start, end, work: w, wait: wt, ent, outcome, .. } if in_range(*start) => {
                sessions += 1;
                match outcome {
                    Outcome::Completed => completed += 1,
                    Outcome::Failed => failed += 1,
                    Outcome::Abandoned => abandoned += 1,
                }
                let span = (end - start).max(0);
                work += w;
                wait += wt;
                total += span;
                longest = longest.max(span);
                with_ent += ent;
                let a = agents.entry(agent.clone()).or_default();
                a.0 += 1;
                a.1 += span;
                let d = days.entry(day_of(*start, tz_min)).or_default();
                d.0 += 1;
                d.1 += w;
                d.2 += wt;
            }
            Record::Entertainment { site, start, end, auto } if in_range(*start) => {
                let span = (end - start).max(0);
                ent_total += span;
                ent_count += 1;
                ent_auto += *auto as u64;
                let s = sites.entry(site.clone()).or_default();
                s.0 += 1;
                s.1 += span;
            }
            _ => {}
        }
    }

    let mut agent_rows: Vec<(String, u64, i64)> = agents.into_iter().map(|(k, (n, ms))| (k, n, ms)).collect();
    agent_rows.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
    let mut site_rows: Vec<(String, u64, i64)> = sites.into_iter().map(|(k, (n, ms))| (k, n, ms)).collect();
    site_rows.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));

    // one bar per day, empty days included, so the chart has a continuous axis
    let (from, to) = match (first, days.keys().min()) {
        (Some(f), _) => (f, today),
        (None, Some(m)) => (*m, today),
        (None, None) => (today, today),
    };
    let day_rows: Vec<Value> = (from..=to)
        .take(800)
        .map(|d| {
            let (n, w, wt) = days.get(&d).copied().unwrap_or_default();
            json!({ "day": day_label(d), "sessions": n, "workMs": w, "waitMs": wt })
        })
        .collect();

    json!({
        "range": match range { Range::Today => "today", Range::Days7 => "7d", Range::Days30 => "30d", Range::All => "all" },
        "sessions": sessions, "completed": completed, "failed": failed, "abandoned": abandoned,
        "workMs": work, "waitMs": wait, "totalMs": total,
        "avgMs": if sessions == 0 { 0 } else { total / sessions as i64 },
        "longestMs": longest, "withEntertainmentMs": with_ent,
        "entertainment": {
            "totalMs": ent_total, "launches": ent_count, "autoLaunches": ent_auto,
            "avgMs": if ent_count == 0 { 0 } else { ent_total / ent_count as i64 },
            "top": site_rows.first().map(|s| s.0.clone()),
            "platforms": site_rows.iter().map(|(k, n, ms)| json!({ "site": k, "count": n, "ms": ms })).collect::<Vec<_>>(),
        },
        "agents": agent_rows.iter().map(|(k, n, ms)| json!({ "agent": k, "sessions": n, "ms": ms })).collect::<Vec<_>>(),
        "days": day_rows,
    })
}

/// The most recent finished sessions (newest first) for the Sessions pane.
pub fn recent(records: &[Record], limit: usize) -> Value {
    let mut rows: Vec<&Record> = records.iter().filter(|r| matches!(r, Record::Session { .. })).collect();
    rows.sort_by_key(|r| std::cmp::Reverse(record_end(r)));
    let list: Vec<Value> = rows
        .into_iter()
        .take(limit)
        .filter_map(|r| match r {
            Record::Session { agent, start, end, work, wait, outcome, .. } => Some(json!({ "agent": agent, "start": start, "end": end, "workMs": work, "waitMs": wait, "outcome": outcome })),
            _ => None,
        })
        .collect();
    json!({ "sessions": list })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ob(id: &str, phase: Phase) -> Obs {
        Obs { id: id.into(), agent: "claude-code".into(), phase }
    }
    fn sessions(r: &[Record]) -> Vec<(&str, i64, i64, Outcome)> {
        r.iter()
            .filter_map(|x| match x {
                Record::Session { id, work, wait, outcome, .. } => Some((id.as_str(), *work, *wait, *outcome)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn working_then_waiting_then_completed_splits_the_time() {
        let mut r = Recorder::new();
        assert!(r.observe(0, &[ob("a", Phase::Working)], None).is_empty());
        assert!(r.observe(10_000, &[ob("a", Phase::Working)], None).is_empty());
        assert!(r.observe(20_000, &[ob("a", Phase::Waiting)], None).is_empty());
        assert!(r.observe(30_000, &[ob("a", Phase::Waiting)], None).is_empty());
        let out = r.observe(40_000, &[ob("a", Phase::Completed)], None);
        assert_eq!(sessions(&out), [("a", 20_000, 20_000, Outcome::Completed)]);
    }

    #[test]
    fn a_completed_session_never_seen_active_is_not_recorded() {
        let mut r = Recorder::new();
        assert!(r.observe(0, &[ob("old", Phase::Completed)], None).is_empty());
    }

    #[test]
    fn sessions_are_independent_and_finish_in_any_order() {
        let mut r = Recorder::new();
        r.observe(0, &[ob("a", Phase::Working), ob("b", Phase::Working)], None);
        let out = r.observe(10_000, &[ob("a", Phase::Working), ob("b", Phase::Completed)], None);
        assert_eq!(sessions(&out), [("b", 10_000, 0, Outcome::Completed)]);
        let out = r.observe(25_000, &[ob("a", Phase::Failed), ob("b", Phase::Completed)], None);
        assert_eq!(sessions(&out), [("a", 25_000, 0, Outcome::Failed)]);
    }

    #[test]
    fn a_new_task_in_the_same_session_is_a_new_record() {
        let mut r = Recorder::new();
        r.observe(0, &[ob("a", Phase::Working)], None);
        assert_eq!(sessions(&r.observe(5_000, &[ob("a", Phase::Completed)], None)).len(), 1);
        r.observe(6_000, &[ob("a", Phase::Working)], None);
        let out = r.observe(9_000, &[ob("a", Phase::Completed)], None);
        assert_eq!(sessions(&out), [("a", 3_000, 0, Outcome::Completed)]);
    }

    #[test]
    fn a_vanished_session_is_closed_as_abandoned_after_the_grace_period() {
        let mut r = Recorder::new();
        r.observe(0, &[ob("a", Phase::Working)], None);
        r.observe(10_000, &[ob("a", Phase::Working)], None);
        assert!(r.observe(20_000, &[], None).is_empty());
        assert!(r.observe(60_000, &[], None).is_empty(), "inside the grace period");
        let out = r.observe(85_000, &[], None);
        assert_eq!(sessions(&out), [("a", 20_000, 0, Outcome::Abandoned)]);
        assert!(r.observe(90_000, &[], None).is_empty(), "closed once");
    }

    #[test]
    fn a_long_gap_is_sleep_not_work() {
        let mut r = Recorder::new();
        r.observe(0, &[ob("a", Phase::Working)], None);
        r.observe(8 * 3_600_000, &[ob("a", Phase::Working)], None);
        let out = r.observe(8 * 3_600_000 + 5_000, &[ob("a", Phase::Completed)], None);
        assert_eq!(sessions(&out), [("a", 35_000, 0, Outcome::Completed)], "30 s for the gap at most, then 5 s");
    }

    #[test]
    fn entertainment_spans_and_auto_launches() {
        let mut r = Recorder::new();
        r.observe(0, &[ob("a", Phase::Working)], None);
        assert!(r.observe(1_000, &[ob("a", Phase::Working)], Some("instagram")).is_empty());
        let out = r.observe(11_000, &[ob("a", Phase::Working)], Some("youtube"));
        assert_eq!(out, [Record::Entertainment { site: "instagram".into(), start: 1_000, end: 11_000, auto: true }]);
        let out = r.observe(21_000, &[ob("a", Phase::Completed)], None);
        assert_eq!(out.len(), 2);
        assert!(out.contains(&Record::Entertainment { site: "youtube".into(), start: 11_000, end: 21_000, auto: true }));
        assert!(out.iter().any(|x| matches!(x, Record::Session { ent: 20_000, .. })), "{out:?}");
    }

    #[test]
    fn flush_records_what_is_open() {
        let mut r = Recorder::new();
        r.observe(0, &[ob("a", Phase::Working)], Some("x"));
        r.observe(10_000, &[ob("a", Phase::Working)], Some("x"));
        let out = r.flush(12_000);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn day_labels_and_zones() {
        assert_eq!(day_label(0), "1970-01-01");
        assert_eq!(day_label(day_of(1_709_164_800_000, 0)), "2024-02-29");
        // 23:30 UTC is already the next day at UTC+1
        let t = 1_709_164_800_000 + 23 * 3_600_000 + 30 * 60_000;
        assert_eq!(day_label(day_of(t, 60)), "2024-03-01");
        assert_eq!(day_label(day_of(t, 0)), "2024-02-29");
    }

    #[test]
    fn summary_filters_ranges_and_groups_by_agent() {
        let now = 1_709_164_800_000 + 12 * 3_600_000; // 2024-02-29 12:00 UTC
        let s = |agent: &str, start: i64, work: i64, wait: i64, outcome| Record::Session { id: "x".into(), agent: agent.into(), start, end: start + work + wait, work, wait, ent: 0, outcome };
        let recs = vec![
            s("claude-code", now - 3_600_000, 600_000, 60_000, Outcome::Completed),
            s("codex", now - 7_200_000, 300_000, 0, Outcome::Failed),
            s("claude-code", now - 3 * DAY_MS, 100_000, 0, Outcome::Completed),
            s("cursor", now - 40 * DAY_MS, 100_000, 0, Outcome::Completed),
            Record::Entertainment { site: "youtube".into(), start: now - 3_000_000, end: now - 2_400_000, auto: true },
        ];
        let today = summarize(&recs, now, Range::Today, 0);
        assert_eq!(today["sessions"], 2);
        assert_eq!(today["completed"], 1);
        assert_eq!(today["failed"], 1);
        assert_eq!(today["workMs"], 900_000);
        assert_eq!(today["waitMs"], 60_000);
        assert_eq!(today["longestMs"], 660_000);
        assert_eq!(today["entertainment"]["top"], "youtube");
        assert_eq!(today["entertainment"]["autoLaunches"], 1);
        assert_eq!(today["agents"][0]["agent"], "claude-code");
        assert_eq!(today["days"].as_array().unwrap().len(), 1);

        assert_eq!(summarize(&recs, now, Range::Days7, 0)["sessions"], 3);
        assert_eq!(summarize(&recs, now, Range::Days7, 0)["days"].as_array().unwrap().len(), 7);
        assert_eq!(summarize(&recs, now, Range::Days30, 0)["sessions"], 3);
        assert_eq!(summarize(&recs, now, Range::All, 0)["sessions"], 4);
        let empty = summarize(&[], now, Range::All, 0);
        assert_eq!((empty["sessions"].as_u64(), empty["avgMs"].as_i64()), (Some(0), Some(0)));
    }

    #[test]
    fn store_round_trips_and_skips_damaged_lines() {
        let dir = std::env::temp_dir().join(format!("wp-insights-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let p = dir.join("insights.jsonl");
        assert!(load_from(&p).is_empty());
        let rec = Record::Session { id: "a".into(), agent: "codex".into(), start: 1, end: 5, work: 3, wait: 1, ent: 0, outcome: Outcome::Completed };
        append_to(&p, &[rec.clone()]);
        std::fs::OpenOptions::new().append(true).open(&p).unwrap().write_all(b"{ broken\n").unwrap();
        append_to(&p, &[Record::Entertainment { site: "x".into(), start: 1, end: 2, auto: false }]);
        let back = load_from(&p);
        assert_eq!(back.len(), 2);
        assert_eq!(back[0], rec);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn recent_lists_the_newest_finished_sessions_first() {
        let s = |end: i64, o| Record::Session { id: "x".into(), agent: "codex".into(), start: end - 10, end, work: 5, wait: 1, ent: 0, outcome: o };
        let recs = vec![s(100, Outcome::Completed), s(300, Outcome::Failed), s(200, Outcome::Completed), Record::Entertainment { site: "x".into(), start: 1, end: 999, auto: false }];
        let r = recent(&recs, 2);
        let ends: Vec<i64> = r["sessions"].as_array().unwrap().iter().map(|x| x["end"].as_i64().unwrap()).collect();
        assert_eq!(ends, [300, 200]);
        assert_eq!(r["sessions"][0]["outcome"], "failed");
    }
}
