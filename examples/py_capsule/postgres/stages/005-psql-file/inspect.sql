-- psql quotes the supplied run_id as a SQL literal using :'run_id'.
BEGIN READ ONLY;
SELECT json_build_object(
    'run_id', run_id,
    'record_id', record_id,
    'label', label,
    'label_length', char_length(label)
)
FROM public.gallery_records
WHERE run_id = :'run_id'::uuid;
COMMIT;
