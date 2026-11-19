diesel::table! {
    spike_entries (id) {
        id -> Int4,
        amount -> Numeric,
        note -> Text,
        created_at -> Timestamptz,
    }
}
