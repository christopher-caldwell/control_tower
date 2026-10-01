# All operations require an explicit local workspace. There is no live database.
db-bootstrap-local workspace:
    cargo run -p control-tower-database --bin control-tower-db -- bootstrap-local '{{workspace}}'

db-migrate-local workspace:
    cargo run -p control-tower-database --bin control-tower-db -- migrate-local '{{workspace}}'

db-verify-local workspace:
    cargo run -p control-tower-database --bin control-tower-db -- verify-local '{{workspace}}'
