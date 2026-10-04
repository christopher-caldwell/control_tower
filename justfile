# All operations require an explicit local workflow. There is no live database.
db-bootstrap-local workflow:
    cargo run -p control-tower-database --bin control-tower-db -- bootstrap-local '{{workflow}}'

db-migrate-local workflow:
    cargo run -p control-tower-database --bin control-tower-db -- migrate-local '{{workflow}}'

db-verify-local workflow:
    cargo run -p control-tower-database --bin control-tower-db -- verify-local '{{workflow}}'
