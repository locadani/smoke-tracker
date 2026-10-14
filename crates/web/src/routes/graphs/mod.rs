//! The Graphs screen (spec §9 screen 5, look-and-feel spec §7). The filters travel in the
//! address ([`filters`]); each chart is a `<canvas>` whose Chart.js configuration travels as
//! JSON in its `data-chart` attribute ([`charts`]), which `static/js/graphs.js` draws, so no
//! page carries an inline script. The room × hour heatmap is a plain table, coloured by CSS
//! classes. The numbers come from `smoke_domain::graphs`, over what the viewer may see: guests
//! get graphs of their own data only (spec §4).

mod charts;
mod filters;

use std::collections::HashMap;

use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use smoke_domain::graphs::{
    Granularity, area_levels, events_per_bucket, intensity_lines, room_hours,
};
use smoke_domain::permissions::{Action, can, can_see};
use smoke_domain::repos::Store;
use smoke_domain::types::UserId;

use self::charts::{
    HeatRow, area_chart, bucket_labels, count_chart, heat_rows, intensity_chart, symptom_chart,
};
use self::filters::FiltersView;
use crate::auth::CurrentUser;
use crate::error::{AppError, page};
use crate::forms::QueryPairs;
use crate::header::{SignedIn, Tab};
use crate::i18n::Tr;
use crate::state::AppState;

#[derive(Template)]
#[template(path = "graphs.html")]
struct GraphsPage {
    tr: Tr,
    signed_in: Option<SignedIn>,
    filters: FiltersView,
    /// Guests see only their own data; the page says so.
    own_only: bool,
    empty: bool,
    intensity: String,
    events_key: &'static str,
    events: String,
    hours: std::ops::Range<u8>,
    heatmap: Vec<HeatRow>,
    symptoms: Option<String>,
    areas: String,
}

pub(crate) async fn show<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    QueryPairs(query): QueryPairs,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    let schedule = &state.config.schedule;
    let actor = user.actor();
    let own_only = !can(&actor, Action::ViewAll, now);
    let users = state.store.users().await?;
    let people = (!own_only).then_some(users.as_slice());
    let selection = filters::read(&query, schedule.local_date(now), people.unwrap_or_default());
    let (period, filter) = (selection.period, &selection.filter);
    let span = period.span(schedule);
    let rooms = state.store.rooms().await?;
    let symptoms = state.store.symptoms().await?;

    let visible = state
        .store
        .reports_between(span.0, span.1)
        .await?
        .into_iter()
        .filter(|r| can_see(&actor, r.user_id, now))
        .collect();
    let reports = filter.reports(visible, &rooms, schedule);
    let visible = state
        .store
        .readings_between(span.0, span.1)
        .await?
        .into_iter()
        .filter(|r| can_see(&actor, r.user_id, now))
        .collect();
    let readings = filter.readings(visible, schedule);
    let names: HashMap<UserId, String> = users
        .iter()
        .map(|u| (u.id, u.username.to_string()))
        .collect();

    let labels = bucket_labels(tr, &period.buckets());
    let events = events_per_bucket(&reports, period, schedule)
        .into_iter()
        .map(|(_, count)| count)
        .collect();
    let areas = area_levels(&reports, &rooms, period, schedule);
    let lines = intensity_lines(&reports);
    let intensity = intensity_chart(tr, &lines, &names, &readings, period, span, schedule);
    page(
        StatusCode::OK,
        &GraphsPage {
            tr,
            signed_in: SignedIn::of(&user, now, Tab::Graphs),
            filters: filters::view(tr, &selection, &rooms, people),
            own_only,
            empty: reports.is_empty() && readings.is_empty(),
            intensity: intensity.to_string(),
            events_key: match period.granularity() {
                Granularity::Day => "graphs-events-day",
                Granularity::Week => "graphs-events-week",
            },
            events: count_chart(tr.t("graphs-events-label"), labels.clone(), events, false)
                .to_string(),
            hours: 0..24,
            heatmap: heat_rows(tr, room_hours(&reports, schedule), &rooms),
            symptoms: symptom_chart(tr, &reports, &symptoms).map(|chart| chart.to_string()),
            areas: area_chart(tr, labels, &areas).to_string(),
        },
    )
}

/// Every key passed to the template through a variable, for the translation tests.
#[cfg(test)]
pub(crate) fn keys() -> Vec<&'static str> {
    let mut keys = vec!["graphs-events-day", "graphs-events-week"];
    keys.extend(filters::keys());
    keys
}
