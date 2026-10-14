use smoke_exercises::ex05_event_repo::{
    Conflict, Event, EventRepo, MemoryEvents, close_open_event,
};

#[test]
fn a_new_event_is_open_and_numbered_from_one() {
    let mut repo = MemoryEvents::default();
    let event = repo.create_event(7, 100).unwrap();
    assert_eq!(
        event,
        Event {
            id: 1,
            owner: 7,
            opened_at: 100,
            closed_at: None
        }
    );
    assert_eq!(repo.open_event(), Some(event));
    assert_eq!(repo.event(1), Some(event));
    assert_eq!(repo.event(2), None);
}

#[test]
fn only_one_event_can_be_open() {
    let mut repo = MemoryEvents::default();
    repo.create_event(1, 100).unwrap();
    assert_eq!(repo.create_event(2, 200), Err(Conflict::AnotherEventOpen));
}

#[test]
fn closing_frees_the_slot_for_the_next_event() {
    let mut repo = MemoryEvents::default();
    repo.create_event(1, 100).unwrap();
    let closed = repo.close_event(1, 150).unwrap();
    assert_eq!(closed.closed_at, Some(150));
    assert_eq!(repo.open_event(), None);
    assert_eq!(repo.event(1), Some(closed));
    assert_eq!(repo.create_event(1, 200).unwrap().id, 2);
}

#[test]
fn closing_checks_the_id_then_the_state_then_the_time() {
    let mut repo = MemoryEvents::default();
    assert_eq!(repo.close_event(1, 10), Err(Conflict::UnknownEvent));
    repo.create_event(1, 100).unwrap();
    assert_eq!(repo.close_event(1, 99), Err(Conflict::RuleViolated));
    assert_eq!(repo.open_event().map(|e| e.id), Some(1));
    repo.close_event(1, 100).unwrap();
    assert_eq!(repo.close_event(1, 200), Err(Conflict::EventNotOpen));
}

#[test]
fn close_open_event_closes_what_is_open() {
    let mut repo = MemoryEvents::default();
    repo.create_event(1, 100).unwrap();
    let closed = close_open_event(&mut repo, 180).unwrap().unwrap();
    assert_eq!(closed.closed_at, Some(180));
    assert_eq!(repo.open_event(), None);
}

#[test]
fn close_open_event_with_nothing_open_is_not_an_error() {
    let mut repo = MemoryEvents::default();
    assert_eq!(close_open_event(&mut repo, 180), Ok(None));
}

/// A repository that is not `MemoryEvents` at all: `close_open_event` must work with it too.
struct Recording {
    open: Option<Event>,
    closed_with: Vec<(u32, u64)>,
}

impl EventRepo for Recording {
    fn open_event(&self) -> Option<Event> {
        self.open
    }

    fn event(&self, _id: u32) -> Option<Event> {
        self.open
    }

    fn create_event(&mut self, _owner: u32, _opened_at: u64) -> Result<Event, Conflict> {
        Err(Conflict::AnotherEventOpen)
    }

    fn close_event(&mut self, id: u32, at: u64) -> Result<Event, Conflict> {
        self.closed_with.push((id, at));
        let mut event = self.open.take().ok_or(Conflict::UnknownEvent)?;
        event.closed_at = Some(at);
        Ok(event)
    }
}

#[test]
fn close_open_event_only_uses_the_trait() {
    let mut repo = Recording {
        open: Some(Event {
            id: 9,
            owner: 1,
            opened_at: 5,
            closed_at: None,
        }),
        closed_with: Vec::new(),
    };
    close_open_event(&mut repo, 50).unwrap();
    assert_eq!(repo.closed_with, [(9, 50)]);
}
