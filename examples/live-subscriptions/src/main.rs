use factstore::{EventFilter, EventQuery, EventStore, NewEvent, SubscriptionHandlerError};
use factstore_memory::MemoryStore;
use serde_json::json;
use std::sync::{Arc, Mutex};

#[derive(Debug, Default)]
struct AccountNamesView {
    names_by_account_id: std::collections::BTreeMap<String, String>,
}

impl AccountNamesView {
    fn apply_committed_batch(&mut self, committed_batch: &[factstore::EventRecord]) {
        for event_record in committed_batch {
            let account_id = event_record
                .payload
                .get("accountId")
                .and_then(|value| value.as_str())
                .expect("example events should carry accountId")
                .to_owned();
            let owner_or_name = event_record
                .payload
                .get("owner")
                .or_else(|| event_record.payload.get("name"))
                .and_then(|value| value.as_str())
                .expect("example events should carry owner or name")
                .to_owned();

            self.names_by_account_id.insert(account_id, owner_or_name);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();
    let account_names_view = Arc::new(Mutex::new(AccountNamesView::default()));
    let projection_view = Arc::clone(&account_names_view);
    let subscription = store.subscribe_to(
        &EventQuery::all().with_filters([EventFilter::for_event_types([
            "account-opened",
            "account-renamed",
        ])]),
        Arc::new(move |committed_batch| {
            println!(
                "projection handler received filtered committed batch with {} events",
                committed_batch.len()
            );
            for event_record in &committed_batch {
                println!("  {event_record:#?}");
            }

            projection_view
                .lock()
                .expect("example projection lock should succeed")
                .apply_committed_batch(&committed_batch);

            Ok::<(), SubscriptionHandlerError>(())
        }),
    )?;

    store.append(vec![
        NewEvent::new(
            "account-opened",
            json!({
                "accountId": "a1",
                "owner": "Rico"
            }),
        ),
        NewEvent::new(
            "account-credited",
            json!({
                "accountId": "a1",
                "amount": 100
            }),
        ),
        NewEvent::new(
            "account-renamed",
            json!({
                "accountId": "a1",
                "name": "FACTSTR"
            }),
        ),
    ])?;

    println!(
        "updated query model: {:#?}",
        account_names_view
            .lock()
            .expect("example projection lock should succeed")
    );
    subscription.unsubscribe();

    Ok(())
}
