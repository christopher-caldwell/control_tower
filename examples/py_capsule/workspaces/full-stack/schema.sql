-- Explicit application setup; Control Tower checkpoints still use local SQLite.
CREATE TABLE IF NOT EXISTS public.gallery_records (
    run_id uuid PRIMARY KEY,
    record_id text NOT NULL,
    label text NOT NULL
);
