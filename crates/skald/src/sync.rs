use crate::rng::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncType {
    None,
    Locked,
    Deck,
    Cdeck,
    Forward,
    Reverse,
    NoRepeat,
    Ping,
    Pong,
}

#[derive(Debug, Clone)]
pub struct SyncState {
    pub kind: SyncType,
    pub index: i64,
    pub order: Vec<usize>,
    pub last: i64,
}

impl SyncState {
    pub fn new(kind: SyncType) -> Self {
        Self {
            kind,
            index: 0,
            order: Vec::new(),
            last: -1,
        }
    }
}

pub fn parse_sync_type(s: &str) -> SyncType {
    match s.trim().to_ascii_lowercase().as_str() {
        "locked" => SyncType::Locked,
        "deck" => SyncType::Deck,
        "cdeck" => SyncType::Cdeck,
        "forward" => SyncType::Forward,
        "reverse" => SyncType::Reverse,
        "no-repeat" => SyncType::NoRepeat,
        "ping" => SyncType::Ping,
        "pong" => SyncType::Pong,
        _ => SyncType::None,
    }
}

fn shuffle(n: usize, rng: &mut Rng) -> Vec<usize> {
    let mut a: Vec<usize> = (0..n).collect();
    for i in (1..a.len()).rev() {
        let j = rng.int((i + 1) as i64) as usize;
        a.swap(i, j);
    }
    a
}

pub fn next_index(state: &mut SyncState, count: usize, rng: &mut Rng) -> usize {
    if count == 0 {
        return 0;
    }
    match state.kind {
        SyncType::Locked => {
            if state.last < 0 || state.last as usize >= count {
                state.last = rng.int(count as i64);
            }
            state.last as usize
        }
        SyncType::Forward => {
            let i = (state.index as usize) % count;
            state.index += 1;
            i
        }
        SyncType::Reverse => {
            let i = (count - 1 - (state.index as usize % count)) % count;
            state.index += 1;
            i
        }
        SyncType::Deck | SyncType::Cdeck => {
            if state.order.is_empty() {
                state.order = shuffle(count, rng);
            }
            let i = (state.index as usize) % state.order.len();
            let picked = state.order[i];
            state.index += 1;
            if state.index as usize >= state.order.len() {
                state.index = 0;
                if state.kind == SyncType::Deck {
                    state.order.clear();
                }
            }
            picked % count
        }
        SyncType::NoRepeat => {
            if count == 1 {
                return 0;
            }
            let mut i = rng.int(count as i64);
            if i == state.last {
                i = (i + 1) % count as i64;
            }
            state.last = i;
            i as usize
        }
        SyncType::Ping => ping_pong(state, count, true),
        SyncType::Pong => ping_pong(state, count, false),
        SyncType::None => rng.int(count as i64) as usize,
    }
}

fn ping_pong(state: &mut SyncState, count: usize, ping: bool) -> usize {
    if count <= 1 {
        return 0;
    }
    let period = (count - 1) * 2;
    let i = (state.index as usize) % period;
    state.index += 1;
    let up = if i < count { i } else { period - i };
    if ping { up } else { count - 1 - up }
}
