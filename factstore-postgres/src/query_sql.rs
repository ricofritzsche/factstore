use factstore::{EventFilter, EventQuery};
use sqlx::{Postgres, QueryBuilder};

pub(crate) fn push_query_conditions(
    query_builder: &mut QueryBuilder<'_, Postgres>,
    event_query: &EventQuery,
    include_min_sequence_number: bool,
) {
    let mut wrote_condition = false;

    if let Some(filters) = &event_query.filters {
        if !filters.is_empty() {
            query_builder.push(" WHERE (");

            for (index, event_filter) in filters.iter().enumerate() {
                if index > 0 {
                    query_builder.push(" OR ");
                }

                query_builder.push("(");
                push_filter_condition(query_builder, event_filter);
                query_builder.push(")");
            }

            query_builder.push(")");
            wrote_condition = true;
        }
    }

    if include_min_sequence_number {
        if let Some(min_sequence_number) = event_query.min_sequence_number {
            if wrote_condition {
                query_builder.push(" AND ");
            } else {
                query_builder.push(" WHERE ");
            }

            // The current Rust contract treats min_sequence_number as an
            // exclusive read cursor for returned rows.
            query_builder
                .push("sequence_number > ")
                .push_bind(min_sequence_number as i64);
        }
    }
}

fn push_filter_condition(
    query_builder: &mut QueryBuilder<'_, Postgres>,
    event_filter: &EventFilter,
) {
    let mut wrote_condition = false;

    if let Some(event_types) = &event_filter.event_types {
        if event_types.is_empty() {
            // `Some(Vec::new())` is an explicit empty match set, not omission.
            query_builder.push("FALSE");
            return;
        } else {
            query_builder
                .push("(")
                .push("event_type = ANY(")
                .push_bind(event_types.clone())
                .push(")");
            query_builder.push(")");
            wrote_condition = true;
        }
    }

    if let Some(payload_predicates) = &event_filter.payload_predicates {
        if wrote_condition {
            query_builder.push(" AND ");
        }

        if payload_predicates.is_empty() {
            query_builder.push("FALSE");
            return;
        }

        query_builder.push("(");
        for (index, payload_predicate) in payload_predicates.iter().enumerate() {
            if index > 0 {
                query_builder.push(" OR ");
            }

            query_builder
                .push("payload @> ")
                .push_bind(payload_predicate.clone());
        }
        query_builder.push(")");
    } else if !wrote_condition {
        query_builder.push("TRUE");
    }
}
