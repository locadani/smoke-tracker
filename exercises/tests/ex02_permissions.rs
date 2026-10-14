use smoke_exercises::ex02_permissions::{Action, Actor, Role, can, can_see};

const NOW: u64 = 1_000;
const ME: u32 = 1;
const OTHER: u32 = 2;

fn actor(role: Role, expires_at: Option<u64>) -> Actor {
    Actor {
        id: ME,
        role,
        expires_at,
    }
}

fn every_action() -> [Action; 9] {
    [
        Action::LogEntry,
        Action::ViewAll,
        Action::ViewOwn,
        Action::EditEntry { owner: ME },
        Action::EditEntry { owner: OTHER },
        Action::ManageEvent { owner: ME },
        Action::ManageEvent { owner: OTHER },
        Action::ManageUsers,
        Action::EditLists,
    ]
}

#[test]
fn a_resident_may_do_everything() {
    let resident = actor(Role::Resident, None);
    for action in every_action() {
        assert!(can(&resident, action, NOW), "{action:?}");
    }
}

#[test]
fn an_active_guest_logs_and_manages_only_what_is_theirs() {
    let guest = actor(Role::Guest, Some(NOW + 1));
    let allowed = [
        Action::LogEntry,
        Action::ViewOwn,
        Action::EditEntry { owner: ME },
        Action::ManageEvent { owner: ME },
    ];
    for action in every_action() {
        assert_eq!(
            can(&guest, action, NOW),
            allowed.contains(&action),
            "{action:?}"
        );
    }
}

#[test]
fn an_expired_guest_can_only_view_their_own_data() {
    let guest = actor(Role::Guest, Some(NOW));
    for action in every_action() {
        assert_eq!(
            can(&guest, action, NOW),
            action == Action::ViewOwn,
            "{action:?}"
        );
    }
}

#[test]
fn an_active_viewer_views_everything_and_changes_nothing() {
    let viewer = actor(Role::Viewer, None);
    for action in every_action() {
        let expected = matches!(action, Action::ViewAll | Action::ViewOwn);
        assert_eq!(can(&viewer, action, NOW), expected, "{action:?}");
    }
}

#[test]
fn an_expired_viewer_can_do_nothing() {
    let viewer = actor(Role::Viewer, Some(NOW - 1));
    for action in every_action() {
        assert!(!can(&viewer, action, NOW), "{action:?}");
    }
}

#[test]
fn an_account_expires_exactly_at_its_expiry_second() {
    let guest = actor(Role::Guest, Some(NOW));
    assert!(!guest.is_expired(NOW - 1));
    assert!(guest.is_expired(NOW));
    assert!(!actor(Role::Resident, None).is_expired(u64::MAX));
}

#[test]
fn only_an_expired_viewer_cannot_sign_in() {
    assert!(actor(Role::Viewer, Some(NOW + 1)).can_sign_in(NOW));
    assert!(!actor(Role::Viewer, Some(NOW)).can_sign_in(NOW));
    assert!(actor(Role::Guest, Some(NOW)).can_sign_in(NOW));
    assert!(actor(Role::Resident, None).can_sign_in(NOW));
}

#[test]
fn people_see_what_their_role_allows() {
    let resident = actor(Role::Resident, None);
    let guest = actor(Role::Guest, Some(NOW + 1));
    let expired_guest = actor(Role::Guest, Some(NOW));
    let viewer = actor(Role::Viewer, None);
    let expired_viewer = actor(Role::Viewer, Some(NOW));

    assert!(can_see(&resident, OTHER, NOW));
    assert!(can_see(&viewer, OTHER, NOW));
    assert!(!can_see(&expired_viewer, OTHER, NOW));
    assert!(!can_see(&expired_viewer, ME, NOW));
    assert!(can_see(&guest, ME, NOW));
    assert!(!can_see(&guest, OTHER, NOW));
    assert!(can_see(&expired_guest, ME, NOW));
    assert!(!can_see(&expired_guest, OTHER, NOW));
}
